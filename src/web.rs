//! HTTP is an optional adapter. The worker still owns the world on a normal thread.
use std::{
    convert::Infallible,
    future::Future,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header::HeaderValue,
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinSet, time::timeout};

use crate::runner::{Command, Config, Controls, SNAPSHOT_CAPACITY, Snapshot, Status, Worker};

pub const MAX_CONNECTIONS: usize = 16;
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const CONTROL_TIMEOUT: Duration = Duration::from_secs(1);
const BODY_LIMIT: usize = 128;
const RUN_HEADER: &str = "x-virtuallife-run";

type Body = Full<Bytes>;
type Cache = Arc<Mutex<Value>>;

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Paused => "paused",
        Status::Running => "running",
        Status::Completed => "completed",
    }
}

/// Decimal strings preserve every bit of Rust's integer domains in JSON/JS.
/// Only small grid dimensions/indices become JavaScript numbers.
pub fn snapshot_json(sample: &Snapshot) -> Value {
    let experiment = sample.experiment.as_ref().map(|info| {
        let counts = info.counts(&sample.cells);
        json!({
            "seed": info.config.seed.to_string(),
            "generator": crate::experiment::GENERATOR, "version": env!("CARGO_PKG_VERSION"),
            "survival": info.config.survival(), "protocol": info.config.protocol(),
            "maintenance": info.config.maintenance.map(|rules| json!({"maximum":rules.maximum,"upkeep":rules.upkeep,"move_wear":rules.move_wear,"copy_wear":rules.copy_wear,"repair":rules.repair})),
            "occupancy": format!("{}.{:06}", info.config.occupancy / 1_000_000, info.config.occupancy % 1_000_000),
            "groups": info.groups.iter().enumerate().map(|(index, group)| json!({
                "weights": group.weights.0, "proportion": group.proportion.to_string(),
                "initial_count": group.initial_count.to_string(), "count": counts[index].to_string()
            })).collect::<Vec<_>>()
        })
    });
    let cells: Vec<_> = sample
        .cells
        .iter()
        .map(|cell| {
            cell.map(|agent| {
                if sample.experiment.is_none() {
                    return json!({"id": agent.id.to_string(), "value": agent.value.to_string()});
                }
                let group = sample.experiment.as_ref().map(|info| info.groups.iter().position(|g| g.weights == agent.weights).expect("configured group"));
                let mut cell = json!({"id": agent.id.to_string(), "value": agent.value.to_string(), "weights": agent.weights.0, "group": group});
                if sample.experiment.as_ref().is_some_and(|info| info.config.maintenance.is_some()) { cell["integrity"] = json!(agent.integrity); }
                cell
            })
        })
        .collect();
    let mut totals = json!({
        "moves":sample.totals.moves.to_string(),"creations":sample.totals.creations.to_string(),
        "removals":sample.totals.removals.to_string(),"value_changes":sample.totals.value_changes.to_string()
    });
    if sample
        .experiment
        .as_ref()
        .is_some_and(|info| info.config.maintenance.is_some())
    {
        totals["repairs"] = json!(sample.totals.repairs.to_string());
        totals["failures"] = json!(sample.totals.failures.to_string());
    }
    json!({
        "width": sample.width, "height": sample.height,
        "tick": sample.tick.to_string(), "count": sample.count.to_string(),
        "status": status_name(sample.status), "cells": cells,
        "end_tick": sample.end_tick.to_string(), "experiment": experiment,
        "failure_history": {
            "limit": crate::engine::FAILURE_LIMIT, "discarded": sample.discarded_failures.to_string(),
            "records": sample.failures.iter().map(|failure| json!({
                "id": failure.id.to_string(), "tick": failure.tick.to_string(),
                "position": {"x":failure.position.x,"y":failure.position.y},
                "reason": failure.reason.label(), "integrity_before":failure.integrity_before,
                "upkeep":failure.upkeep,"action_wear":failure.action_wear
            })).collect::<Vec<_>>()
        },
        "totals": totals
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandInput {
    command: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum RestartInput {
    Seed(String),
    Random(bool),
}

fn restart_seed(bytes: &[u8]) -> Result<Option<u64>, &'static str> {
    let input: RestartInput = serde_json::from_slice(bytes)
        .map_err(|_| "use exactly one decimal seed string or random: true")?;
    match input {
        RestartInput::Seed(seed)
            if !seed.is_empty() && seed.bytes().all(|b| b.is_ascii_digit()) =>
        {
            seed.parse()
                .map(Some)
                .map_err(|_| "seed must be between 0 and 18446744073709551615")
        }
        RestartInput::Random(true) => Ok(None),
        _ => Err("use a decimal seed string from 0 to 18446744073709551615, or random: true"),
    }
}

#[derive(Clone)]
struct RunView {
    cache: Cache,
    controls: Controls,
    run_id: HeaderValue,
}

struct Experiment {
    worker: Worker,
    collector: thread::JoinHandle<()>,
}

impl Experiment {
    fn stop(self) -> Result<(), String> {
        let stopped = self.worker.stop();
        self.collector
            .join()
            .map_err(|_| "snapshot collector panicked")?;
        stopped.map(|_| ())
    }
}

#[derive(Clone)]
struct Api {
    current: Arc<Mutex<RunView>>,
    // Only replacement/shutdown owns these handles. Snapshot reads and control
    // enqueueing use the short current lock, never the initialization/join lock.
    experiment: Arc<Mutex<Option<Experiment>>>,
    config: Config,
    port: u16,
}

fn identified(mut response: Response<Body>, run_id: &HeaderValue) -> Response<Body> {
    response.headers_mut().insert(RUN_HEADER, run_id.clone());
    response
}

fn response(status: StatusCode, content_type: &str, body: impl Into<Bytes>) -> Response<Body> {
    Response::builder().status(status)
        .header("content-type", content_type)
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .header("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")
        .body(Full::new(body.into())).expect("constant valid response headers")
}

fn json_response(status: StatusCode, value: Value) -> Response<Body> {
    response(status, "application/json", value.to_string())
}

fn error(status: StatusCode, message: &str) -> Response<Body> {
    json_response(status, json!({"error": message}))
}

impl Api {
    fn allowed(&self, request: &Request<Incoming>) -> bool {
        let headers = request.headers();
        let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) else {
            return false;
        };
        if headers.get_all("host").iter().count() != 1
            || ![
                format!("127.0.0.1:{}", self.port),
                format!("localhost:{}", self.port),
            ]
            .contains(&host.to_owned())
        {
            return false;
        }
        let origin = headers.get("origin").and_then(|v| v.to_str().ok());
        if headers.get_all("origin").iter().count() > 1
            || (headers.contains_key("origin") && origin != Some(format!("http://{host}").as_str()))
        {
            return false;
        }
        if let Some(site) = headers.get("sec-fetch-site")
            && site != "same-origin"
            && site != "none"
        {
            return false;
        }
        // Browser controls must come from this origin. Local API clients supply it explicitly.
        request.method() != hyper::Method::POST || origin.is_some()
    }

    async fn handle(self, request: Request<Incoming>) -> Result<Response<Body>, Infallible> {
        if !self.allowed(&request) {
            return Ok(error(
                StatusCode::FORBIDDEN,
                "Host or Origin is not allowed",
            ));
        }
        let run = self.current.lock().expect("current run lock").clone();
        let reply = match (request.method().as_str(), request.uri().path()) {
            ("GET", "/") => response(
                StatusCode::OK,
                "text/html; charset=utf-8",
                include_str!("../web/index.html"),
            ),
            ("GET", "/app.js") => response(
                StatusCode::OK,
                "text/javascript",
                include_str!("../web/app.js"),
            ),
            ("GET", "/display.js") => response(
                StatusCode::OK,
                "text/javascript",
                include_str!("../web/display.js"),
            ),
            ("GET", "/style.css") => {
                response(StatusCode::OK, "text/css", include_str!("../web/style.css"))
            }
            ("GET", "/api/snapshot") => {
                // Copy under the short cache lock; serialization and network I/O happen after release.
                let snapshot = run.cache.lock().expect("cache lock").clone();
                json_response(StatusCode::OK, snapshot)
            }
            ("POST", "/api/control") => return Ok(self.control(request).await),
            ("POST", "/api/restart") => return Ok(self.restart(request).await),
            _ => error(StatusCode::NOT_FOUND, "unknown route or method"),
        };
        Ok(identified(reply, &run.run_id))
    }

    async fn control(&self, request: Request<Incoming>) -> Response<Body> {
        let headers = request.headers().clone();
        let bytes = match request_body(request).await {
            Ok(bytes) => bytes,
            Err(reply) => return self.current_response(reply),
        };
        let (run_id, receiver) = {
            let run = self.current.lock().expect("current run lock");
            // Browser commands name the run they were sent for. Local API clients
            // may omit this precondition for compatibility with existing scripts.
            if let Some(run_id) = headers.get(RUN_HEADER)
                && (run_id != run.run_id || headers.get_all(RUN_HEADER).iter().count() != 1)
            {
                return identified(
                    error(
                        StatusCode::CONFLICT,
                        "experiment changed; read the new snapshot",
                    ),
                    &run.run_id,
                );
            }
            let input: CommandInput = match serde_json::from_slice(&bytes) {
                Ok(value) => value,
                Err(_) => {
                    return identified(error(StatusCode::BAD_REQUEST, "invalid JSON"), &run.run_id);
                }
            };
            let command = match input.command.as_str() {
                "pause" => Command::Pause,
                "resume" => Command::Resume,
                "step" => Command::Step,
                _ => {
                    return identified(
                        error(
                            StatusCode::BAD_REQUEST,
                            "command must be pause, resume, or step; no other fields",
                        ),
                        &run.run_id,
                    );
                }
            };
            let receiver = match run.controls.request(command) {
                Ok(receiver) => receiver,
                Err(message) => {
                    let status = if message == "control queue is full" {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::CONFLICT
                    };
                    return identified(error(status, &message), &run.run_id);
                }
            };
            (run.run_id.clone(), receiver)
        };
        // HTTP tasks can wait for a receipt; the simulation never waits for a reader.
        // There are at most MAX_CONNECTIONS tasks and one receipt per request.
        let reply = match tokio::task::spawn_blocking(move || {
            receiver.recv_timeout(CONTROL_TIMEOUT)
        })
        .await
        {
            Ok(Ok(receipt)) => json_response(
                if receipt.applied {
                    StatusCode::OK
                } else {
                    StatusCode::CONFLICT
                },
                json!({
                    "applied": receipt.applied, "tick": receipt.tick.to_string(), "status": status_name(receipt.status),
                    "message": if receipt.applied { "Applied between ticks" } else { "Control not applicable in this state" }
                }),
            ),
            Ok(Err(mpsc::RecvTimeoutError::Disconnected)) => error(
                StatusCode::CONFLICT,
                "worker has completed or stopped; read the current snapshot",
            ),
            _ => error(
                StatusCode::GATEWAY_TIMEOUT,
                "control outcome unknown; inspect the current tick before another command; do not retry automatically",
            ),
        };
        identified(reply, &run_id)
    }

    fn current_response(&self, reply: Response<Body>) -> Response<Body> {
        identified(
            reply,
            &self.current.lock().expect("current run lock").run_id,
        )
    }

    async fn restart(&self, request: Request<Incoming>) -> Response<Body> {
        let headers = request.headers().clone();
        let bytes = match request_body(request).await {
            Ok(bytes) => bytes,
            Err(reply) => return self.current_response(reply),
        };
        let seed = match restart_seed(&bytes) {
            Ok(seed) => seed,
            Err(message) => return self.current_response(error(StatusCode::BAD_REQUEST, message)),
        };
        let api = self.clone();
        // Initialization and joins must outlive a lost HTTP reply. Only one
        // replacement can be in progress; no detached backlog of workers grows.
        match tokio::task::spawn_blocking(move || api.replace_run(&headers, seed)).await {
            Ok(reply) => reply,
            Err(_) => self.current_response(error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "restart outcome unknown; read the current snapshot; do not retry automatically",
            )),
        }
    }

    fn replace_run(&self, headers: &hyper::HeaderMap, seed: Option<u64>) -> Response<Body> {
        let Ok(mut owned) = self.experiment.try_lock() else {
            return self.current_response(error(
                StatusCode::CONFLICT,
                "restart already in progress; read the current snapshot",
            ));
        };
        {
            let run = self.current.lock().expect("current run lock");
            if headers.get_all(RUN_HEADER).iter().count() != 1
                || headers.get(RUN_HEADER) != Some(&run.run_id)
            {
                return identified(
                    error(
                        StatusCode::CONFLICT,
                        "experiment changed; read the new snapshot",
                    ),
                    &run.run_id,
                );
            }
        }
        if owned.is_none() {
            return self
                .current_response(error(StatusCode::SERVICE_UNAVAILABLE, "server is stopping"));
        }
        let mut config = self.config.clone();
        let Some(settings) = &mut config.experiment else {
            return self.current_response(error(
                StatusCode::CONFLICT,
                "seeded restart is available only in autonomous mode",
            ));
        };
        settings.seed = match seed {
            Some(seed) => seed,
            None => {
                let mut bytes = [0; 8];
                if getrandom::fill(&mut bytes).is_err() {
                    return self.current_response(error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "cannot generate a seed; current run retained",
                    ));
                }
                u64::from_ne_bytes(bytes)
            }
        };
        config.start_paused = true;
        // Prepare fully before touching the old run. A failed initialization
        // drops/stops its worker and leaves the current run usable.
        let (worker, next, collector) = match start_experiment(config) {
            Ok(started) => started,
            Err(message) => {
                return self.current_response(error(StatusCode::INTERNAL_SERVER_ERROR, &message));
            }
        };
        // Both threads are joined even if the old simulation itself failed.
        let _ = owned.take().expect("active experiment").stop();
        let snapshot = next.cache.lock().expect("cache lock").clone();
        let reply = identified(json_response(StatusCode::OK, snapshot), &next.run_id);
        *owned = Some(Experiment { worker, collector });
        *self.current.lock().expect("current run lock") = next;
        reply
    }
}

async fn request_body(request: Request<Incoming>) -> Result<Bytes, Response<Body>> {
    if request.headers().get_all("content-type").iter().count() != 1
        || request
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            != Some("application/json")
    {
        return Err(error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "use application/json",
        ));
    }
    Limited::new(request.into_body(), BODY_LIMIT)
        .collect()
        .await
        .map(|body| body.to_bytes())
        .map_err(|_| {
            error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid or oversized request body",
            )
        })
}

fn start_experiment(config: Config) -> Result<(Worker, RunView, thread::JoinHandle<()>), String> {
    // Identity belongs to this adapter instance, not to model state or tick time.
    // OS randomness avoids clock/PID reuse; no simulation randomness is added.
    let mut identity = [0_u8; 16];
    getrandom::fill(&mut identity).map_err(|e| format!("cannot identify experiment: {e}"))?;
    let run_id = format!("{:032x}", u128::from_be_bytes(identity))
        .parse()
        .expect("hexadecimal run ID is a valid header");
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(config, Some(sender))?;
    // Publish even when no HTTP client exists. This collector is not browser-owned.
    let first = receiver
        .recv_timeout(REQUEST_TIMEOUT)
        .map_err(|e| e.to_string())?;
    let cache = Arc::new(Mutex::new(snapshot_json(&first)));
    let collector_cache = cache.clone();
    let collector = thread::Builder::new()
        .name("snapshot-cache".into())
        .spawn(move || {
            for sample in receiver {
                let value = snapshot_json(&sample);
                *collector_cache.lock().expect("cache lock") = value;
            }
            let mut value = collector_cache.lock().expect("cache lock");
            if value["status"] != "completed" {
                value["status"] = json!("failed");
                value["error"] = json!("Simulation worker stopped before completion");
            }
        })
        .map_err(|e| e.to_string())?;
    let api = RunView {
        cache,
        controls: worker.controls(),
        run_id,
    };
    Ok((worker, api, collector))
}

/// Serve successive experiments until shutdown. Completion keeps its final cache.
/// The caller binds loopback (checked here too); port 0 is useful for isolated tests.
pub async fn serve(
    listener: TcpListener,
    config: Config,
    shutdown: impl Future<Output = ()>,
) -> Result<(), String> {
    if !listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .ip()
        .is_loopback()
    {
        return Err("the viewer must bind to loopback".into());
    }
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let (worker, run, collector) = start_experiment(config.clone())?;
    let api = Api {
        current: Arc::new(Mutex::new(run)),
        experiment: Arc::new(Mutex::new(Some(Experiment { worker, collector }))),
        config,
        port,
    };
    let mut connections = JoinSet::new();
    tokio::pin!(shutdown);
    let result = loop {
        tokio::select! {
            _ = &mut shutdown => break Ok(()),
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            accepted = listener.accept() => {
                let (stream, _) = match accepted { Ok(pair) => pair, Err(e) => break Err(e.to_string()) };
                if connections.len() >= MAX_CONNECTIONS {
                    // Refuse overload before parsing/buffering another request.
                    drop(stream);
                    continue;
                }
                let api = api.clone();
                connections.spawn(async move {
                    let service = service_fn(move |request| api.clone().handle(request));
                    let mut builder = http1::Builder::new();
                    builder.keep_alive(false).max_buf_size(8192).max_headers(32)
                        .timer(TokioTimer::new()).header_read_timeout(REQUEST_TIMEOUT);
                    // The deadline bounds headers, body, receipt wait, AND a slow response.
                    let _ = timeout(REQUEST_TIMEOUT, builder.serve_connection(TokioIo::new(stream), service)).await;
                });
            }
        }
    };
    // Allow already-started responses to finish, bounded by their connection deadline.
    while connections.join_next().await.is_some() {}
    // Also wait for a replacement whose HTTP response hit its deadline.
    tokio::task::spawn_blocking(move || {
        api.experiment
            .lock()
            .expect("experiment lock")
            .take()
            .expect("active experiment")
            .stop()
    })
    .await
    .map_err(|_| "experiment shutdown panicked")??;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Agent, Events};

    fn autonomous_api() -> Api {
        let config = Config {
            ticks: 12,
            sample_every: 7,
            tick_interval: Duration::from_secs(3600),
            experiment: Some(crate::experiment::ExperimentConfig {
                width: 8,
                height: 6,
                ..Default::default()
            }),
            ..Config::default()
        };
        let (worker, run, collector) = start_experiment(config.clone()).unwrap();
        Api {
            current: Arc::new(Mutex::new(run)),
            experiment: Arc::new(Mutex::new(Some(Experiment { worker, collector }))),
            config,
            port: 1234,
        }
    }

    fn precondition(api: &Api) -> hyper::HeaderMap {
        let mut headers = hyper::HeaderMap::new();
        headers.insert(RUN_HEADER, api.current.lock().unwrap().run_id.clone());
        headers
    }

    #[test]
    fn failed_restart_initialization_retains_an_usable_run_and_replacements_join_old_workers() {
        let (done, finished) = mpsc::channel();
        // Include every replacement, join and destructor in the deadlock guard.
        thread::spawn(move || {
            check_restart_initialization_and_cleanup();
            let _ = done.send(());
        });
        finished
            .recv_timeout(Duration::from_secs(10))
            .expect("restart initialization and cleanup exceeded deadlock guard");
    }

    fn check_restart_initialization_and_cleanup() {
        let mut api = autonomous_api();
        let initial = api.current.lock().unwrap().clone();
        let headers = precondition(&api);
        // Exercise the real initializer's rejection without touching the valid worker.
        api.config.sample_every = 0;
        assert_eq!(
            api.replace_run(&headers, Some(42)).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(api.current.lock().unwrap().run_id, initial.run_id);
        assert!(
            initial
                .controls
                .request(Command::Step)
                .unwrap()
                .recv_timeout(REQUEST_TIMEOUT)
                .unwrap()
                .applied
        );
        api.config.sample_every = 7;
        for seed in 0..24 {
            let old = api.current.lock().unwrap().clone();
            let headers = precondition(&api);
            let reply = api.replace_run(&headers, Some(seed));
            assert_eq!(reply.status(), StatusCode::OK);
            assert_ne!(reply.headers()[RUN_HEADER], old.run_id);
            // Disconnected command channels prove replacement has joined, rather
            // than leaving an old paused/completed worker waiting indefinitely.
            assert!(old.controls.request(Command::Step).is_err());
            let current = api.current.lock().unwrap().clone();
            assert_eq!(current.cache.lock().unwrap()["tick"], "0");
            assert_eq!(
                current.cache.lock().unwrap()["experiment"]["seed"],
                seed.to_string()
            );
        }
        api.experiment
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .stop()
            .unwrap();
    }

    #[test]
    fn retained_http_snapshots_and_control_receipts_keep_their_old_identity_after_restart() {
        let (done, finished) = mpsc::channel();
        // Runtime teardown can wait for blocking tasks, so guard it along with
        // the response checks and all worker/collector cleanup.
        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(check_retained_http_responses_after_restart());
            let _ = done.send(());
        });
        finished
            .recv_timeout(Duration::from_secs(20))
            .expect("restart HTTP response checks exceeded deadlock guard");
    }

    async fn check_retained_http_responses_after_restart() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for control in [false, true] {
            let api = autonomous_api();
            let old = api.current.lock().unwrap().run_id.clone();
            let (mut reader, writer) = tokio::io::duplex(64);
            let connection_api = api.clone();
            let response_task = tokio::spawn(async move {
                http1::Builder::new()
                    .keep_alive(false)
                    .serve_connection(
                        TokioIo::new(writer),
                        service_fn(move |request| connection_api.clone().handle(request)),
                    )
                    .await
                    .unwrap();
            });
            let request = if control {
                let body = r#"{"command":"step"}"#;
                format!(
                    "POST /api/control HTTP/1.1\r\nHost: 127.0.0.1:1234\r\nOrigin: http://127.0.0.1:1234\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-VirtualLife-Run: {}\r\n\r\n{body}",
                    body.len(),
                    old.to_str().unwrap()
                )
            } else {
                "GET /api/snapshot HTTP/1.1\r\nHost: 127.0.0.1:1234\r\n\r\n".to_owned()
            };
            reader.write_all(request.as_bytes()).await.unwrap();
            let mut first = [0];
            timeout(REQUEST_TIMEOUT, reader.read_exact(&mut first))
                .await
                .unwrap()
                .unwrap();
            // One response byte proves the old snapshot/receipt was produced;
            // the 64-byte transport holds the rest across a real replacement.
            let headers = precondition(&api);
            let replacement = api.clone();
            let reply =
                tokio::task::spawn_blocking(move || replacement.replace_run(&headers, Some(42)))
                    .await
                    .unwrap();
            assert_eq!(reply.status(), StatusCode::OK);
            assert!(!response_task.is_finished());
            let mut bytes = first.to_vec();
            timeout(REQUEST_TIMEOUT, reader.read_to_end(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            response_task.await.unwrap();
            let text = String::from_utf8(bytes).unwrap();
            let (headers, body) = text.split_once("\r\n\r\n").unwrap();
            assert!(headers.contains(&format!("{RUN_HEADER}: {}", old.to_str().unwrap())));
            let value: Value = serde_json::from_str(body).unwrap();
            assert_eq!(value["tick"], if control { "1" } else { "0" });
            if control {
                assert_eq!(value["applied"], true);
            } else {
                assert_eq!(value["experiment"]["seed"], "1");
            }
            let run = api.current.lock().unwrap().clone();
            assert_ne!(run.run_id, old);
            assert_eq!(run.cache.lock().unwrap()["tick"], "0");
            assert_eq!(run.cache.lock().unwrap()["experiment"]["seed"], "42");
            api.experiment
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .stop()
                .unwrap();
        }
    }

    #[test]
    fn serialization_keeps_u64_and_signed_extremes_exact() {
        let sample = Snapshot {
            width: 3,
            height: 3,
            tick: u64::MAX,
            count: 2,
            status: Status::Completed,
            end_tick: u64::MAX,
            experiment: None,
            failures: Vec::new(),
            discarded_failures: 0,
            totals: Events {
                moves: u64::MAX,
                creations: 9_007_199_254_740_993,
                removals: 0,
                value_changes: u64::MAX - 1,
                ..Events::default()
            },
            cells: vec![
                Some(Agent {
                    id: u64::MAX - 1,
                    value: i64::MIN,
                    ..Agent::default()
                }),
                Some(Agent {
                    id: 9_007_199_254_740_993,
                    value: i64::MAX,
                    ..Agent::default()
                }),
            ],
        };
        let encoded = snapshot_json(&sample).to_string();
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["tick"], "18446744073709551615");
        assert_eq!(value["cells"][0]["id"], "18446744073709551614");
        assert_eq!(value["cells"][0]["value"], "-9223372036854775808");
        assert_eq!(value["cells"][1]["id"], "9007199254740993");
        assert_eq!(value["cells"][1]["value"], "9223372036854775807");
        assert_eq!(value["totals"]["moves"], "18446744073709551615");
        assert_eq!(value["totals"]["creations"], "9007199254740993");
        assert_eq!(value["totals"]["value_changes"], "18446744073709551614");

        let mut wear_sample = sample;
        let (_, _, info) = crate::experiment::ExperimentConfig {
            bundles: vec![crate::engine::Weights::default()],
            proportions: vec![1],
            ..crate::experiment::ExperimentConfig::default()
        }
        .initialize()
        .unwrap();
        wear_sample.experiment = Some(Arc::new(info));
        wear_sample.totals.repairs = u64::MAX;
        wear_sample.totals.failures = u64::MAX - 1;
        wear_sample.discarded_failures = u64::MAX - 2;
        wear_sample.failures.push(crate::engine::Failure {
            id: u64::MAX - 1,
            tick: u64::MAX,
            position: crate::engine::Position::new(1, 1),
            reason: crate::engine::FailureReason::CopyWear,
            integrity_before: u32::MAX,
            upkeep: 1,
            action_wear: u32::MAX,
        });
        let value = snapshot_json(&wear_sample);
        assert_eq!(value["totals"]["repairs"], "18446744073709551615");
        assert_eq!(value["totals"]["failures"], "18446744073709551614");
        assert_eq!(
            value["failure_history"]["discarded"],
            "18446744073709551613"
        );
        assert_eq!(
            value["failure_history"]["records"][0]["tick"],
            "18446744073709551615"
        );
        assert_eq!(
            value["failure_history"]["records"][0]["id"],
            "18446744073709551614"
        );
        assert_eq!(
            value["failure_history"]["records"][0]["integrity_before"],
            u32::MAX
        );
    }

    #[test]
    fn cache_completes_without_http_and_retained_response_never_holds_its_lock() {
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            for retain_response in [false, true] {
                let (worker, api, collector) = start_experiment(Config {
                    ticks: 100,
                    tick_interval: Duration::ZERO,
                    ..Config::default()
                })
                .unwrap();
                let old_response = retain_response.then(|| api.cache.lock().unwrap().clone());
                api.controls
                    .request(Command::Resume)
                    .unwrap()
                    .recv_timeout(REQUEST_TIMEOUT)
                    .unwrap();
                worker.wait_for_completion(REQUEST_TIMEOUT).unwrap();
                let world = worker.join().unwrap();
                collector.join().unwrap();
                assert_eq!(world.tick(), 100);
                let last = api.cache.lock().unwrap().clone();
                assert_eq!(last["tick"], "100");
                assert_eq!(last["status"], "completed");
                assert_eq!(last["count"], "2");
                assert_eq!(last, api.cache.lock().unwrap().clone());
                if let Some(old) = old_response {
                    assert_eq!(old["tick"], "0");
                }
            }
            let _ = done.send(());
        });
        finished
            .recv_timeout(Duration::from_secs(10))
            .expect("adapter must finish independently of HTTP readers");
    }
}

#[cfg(test)]
mod slow_reader_test {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn a_backpressured_http_response_does_not_delay_final_cache_publication() {
        let (finished, completion) = mpsc::channel();
        // Guard the runtime and all worker cleanup, including a broken blocking
        // publisher's destructor. No raw join can hang the test thread.
        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(check_backpressured_response());
            let _ = finished.send(());
        });
        completion
            .recv_timeout(Duration::from_secs(20))
            .expect("HTTP observation checks exceeded deadlock guard");
    }

    async fn check_backpressured_response() {
        for autonomous in [false, true] {
            let config = Config {
                ticks: 100,
                tick_interval: Duration::ZERO,
                experiment: autonomous.then(|| crate::experiment::ExperimentConfig {
                    width: 8,
                    height: 6,
                    ..crate::experiment::ExperimentConfig::default()
                }),
                ..Config::default()
            };
            let expected = Worker::spawn(
                Config {
                    start_paused: false,
                    ..config.clone()
                },
                None,
            )
            .unwrap()
            .join()
            .unwrap();
            let (worker, run, collector) = start_experiment(config.clone()).unwrap();
            let controls = run.controls.clone();
            let cache = run.cache.clone();
            let api = Api {
                current: Arc::new(Mutex::new(run)),
                experiment: Arc::new(Mutex::new(None)),
                config,
                port: 1234,
            };
            // Hyper serves the actual snapshot handler over a 64-byte transport. After
            // one byte is read, the response cannot finish until the reader continues.
            // This gives deterministic backpressure without timing sleeps or huge data.
            let (mut reader, writer) = tokio::io::duplex(64);
            let response_task = tokio::spawn(async move {
                let mut builder = http1::Builder::new();
                builder.keep_alive(false);
                builder
                    .serve_connection(
                        TokioIo::new(writer),
                        service_fn(move |request| api.clone().handle(request)),
                    )
                    .await
            });
            reader
                .write_all(b"GET /api/snapshot HTTP/1.1\r\nHost: 127.0.0.1:1234\r\n\r\n")
                .await
                .unwrap();
            timeout(REQUEST_TIMEOUT, reader.read_exact(&mut [0u8; 1]))
                .await
                .unwrap()
                .unwrap();
            let (done, finished) = tokio::sync::oneshot::channel();
            thread::spawn(move || {
                controls
                    .request(Command::Resume)
                    .unwrap()
                    .recv_timeout(REQUEST_TIMEOUT)
                    .unwrap();
                worker.wait_for_completion(REQUEST_TIMEOUT).unwrap();
                let world = worker.join().unwrap();
                collector.join().unwrap();
                let last = cache.lock().unwrap().clone();
                let _ = done.send((world, last));
            });
            let result = timeout(Duration::from_secs(10), finished).await;
            let still_blocked = !response_task.is_finished();
            // Release even on failure so incorrect lock-holding cannot hang cleanup.
            drop(reader);
            let _ = timeout(REQUEST_TIMEOUT, response_task).await;
            let (world, last) = result
                .expect("completion must not wait for response consumption")
                .unwrap();
            assert!(
                still_blocked,
                "the test must actually hold an unfinished response"
            );
            assert_eq!(world, expected);
            assert_eq!(last["tick"], "100");
            assert_eq!(last["status"], "completed");
            assert_eq!(last["count"], expected.count().to_string());
        }
    }
}
