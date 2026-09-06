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
    let cells: Vec<_> = sample
        .cells
        .iter()
        .map(|cell| {
            cell.map(|agent| json!({"id": agent.id.to_string(), "value": agent.value.to_string()}))
        })
        .collect();
    json!({
        "width": sample.width, "height": sample.height,
        "tick": sample.tick.to_string(), "count": sample.count.to_string(),
        "status": status_name(sample.status), "cells": cells,
        "totals": {
            "moves": sample.totals.moves.to_string(),
            "creations": sample.totals.creations.to_string(),
            "removals": sample.totals.removals.to_string(),
            "value_changes": sample.totals.value_changes.to_string()
        }
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandInput {
    command: String,
}

#[derive(Clone)]
struct Api {
    cache: Cache,
    controls: Controls,
    port: u16,
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
                let snapshot = self.cache.lock().expect("cache lock").clone();
                json_response(StatusCode::OK, snapshot)
            }
            ("POST", "/api/control") => self.control(request).await,
            _ => error(StatusCode::NOT_FOUND, "unknown route or method"),
        };
        Ok(reply)
    }

    async fn control(&self, request: Request<Incoming>) -> Response<Body> {
        if request
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            != Some("application/json")
        {
            return error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "use application/json");
        }
        let bytes = match Limited::new(request.into_body(), BODY_LIMIT)
            .collect()
            .await
        {
            Ok(body) => body.to_bytes(),
            Err(_) => {
                return error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "invalid or oversized request body",
                );
            }
        };
        let input: CommandInput = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => return error(StatusCode::BAD_REQUEST, "invalid JSON"),
        };
        let command = match input.command.as_str() {
            "pause" => Command::Pause,
            "resume" => Command::Resume,
            "step" => Command::Step,
            _ => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "command must be pause, resume, or step; no other fields",
                );
            }
        };
        let receiver = match self.controls.request(command) {
            Ok(receiver) => receiver,
            Err(message) => {
                let status = if message == "control queue is full" {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::CONFLICT
                };
                return error(status, &message);
            }
        };
        // HTTP tasks can wait for a receipt; the simulation never waits for a reader.
        // There are at most MAX_CONNECTIONS tasks and one receipt per request.
        match tokio::task::spawn_blocking(move || receiver.recv_timeout(CONTROL_TIMEOUT)).await {
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
        }
    }
}

fn start_experiment(
    config: Config,
    port: u16,
) -> Result<(Worker, Api, thread::JoinHandle<()>), String> {
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
    let api = Api {
        cache,
        controls: worker.controls(),
        port,
    };
    Ok((worker, api, collector))
}

/// Serve one experiment until explicit shutdown. Completion keeps its final cache.
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
    let (worker, api, collector) = start_experiment(config, port)?;
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
    let stopped = worker.stop();
    collector
        .join()
        .map_err(|_| "snapshot collector panicked")?;
    stopped?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Agent, Events};

    #[test]
    fn serialization_keeps_u64_and_signed_extremes_exact() {
        let sample = Snapshot {
            width: 3,
            height: 3,
            tick: u64::MAX,
            count: 2,
            status: Status::Completed,
            totals: Events {
                moves: u64::MAX,
                creations: 9_007_199_254_740_993,
                removals: 0,
                value_changes: u64::MAX - 1,
            },
            cells: vec![
                Some(Agent {
                    id: u64::MAX - 1,
                    value: i64::MIN,
                }),
                Some(Agent {
                    id: 9_007_199_254_740_993,
                    value: i64::MAX,
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
    }

    #[test]
    fn cache_completes_without_http_and_retained_response_never_holds_its_lock() {
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            for retain_response in [false, true] {
                let (worker, api, collector) = start_experiment(
                    Config {
                        ticks: 100,
                        tick_interval: Duration::ZERO,
                        ..Config::default()
                    },
                    0,
                )
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

    #[tokio::test]
    async fn a_backpressured_http_response_does_not_delay_final_cache_publication() {
        let (worker, api, collector) = start_experiment(
            Config {
                ticks: 100,
                tick_interval: Duration::ZERO,
                ..Config::default()
            },
            1234,
        )
        .unwrap();
        let controls = api.controls.clone();
        let cache = api.cache.clone();
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
            let _ = done.send((world.tick(), last));
        });
        let result = timeout(Duration::from_secs(10), finished).await;
        let still_blocked = !response_task.is_finished();
        // Release even on failure so incorrect lock-holding cannot hang cleanup.
        drop(reader);
        let _ = timeout(REQUEST_TIMEOUT, response_task).await;
        let (tick, last) = result
            .expect("completion must not wait for response consumption")
            .unwrap();
        assert!(
            still_blocked,
            "the test must actually hold an unfinished response"
        );
        assert_eq!(tick, 100);
        assert_eq!(last["tick"], "100");
        assert_eq!(last["status"], "completed");
    }
}
