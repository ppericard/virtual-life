#![cfg(feature = "web")]
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use virtual_life::{runner::Config, web};
const GUARD: Duration = Duration::from_secs(10);

struct Server {
    address: SocketAddr,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    finished: mpsc::Receiver<Result<(), String>>,
}
impl Server {
    fn start(config: Config) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let (shutdown, stop) = tokio::sync::oneshot::channel();
        let (done, finished) = mpsc::channel();
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let result = runtime.block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                web::serve(listener, config, async {
                    let _ = stop.await;
                })
                .await
            });
            let _ = done.send(result);
        });
        Self {
            address,
            shutdown: Some(shutdown),
            finished,
        }
    }
    fn request(&self, method: &str, path: &str, headers: &str, body: &str) -> String {
        let mut stream = TcpStream::connect_timeout(&self.address, GUARD).unwrap();
        stream.set_read_timeout(Some(GUARD)).unwrap();
        stream.set_write_timeout(Some(GUARD)).unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\n{headers}\r\n{body}",
            self.address,
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
    fn control(&self, command: &str) -> (u16, Value) {
        parsed(&self.request(
            "POST",
            "/api/control",
            &format!(
                "Origin: http://{}\r\nContent-Type: application/json\r\n",
                self.address
            ),
            &json!({"command": command}).to_string(),
        ))
    }
    fn snapshot(&self) -> Value {
        let (status, body) = parsed(&self.request("GET", "/api/snapshot", "", ""));
        assert_eq!(status, 200);
        body
    }
    fn until(&self, tick: &str, state: &str) -> Value {
        let deadline = Instant::now() + GUARD;
        loop {
            let snapshot = self.snapshot();
            if snapshot["tick"] == tick && snapshot["status"] == state {
                return snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "observable state did not arrive: {snapshot}"
            );
            thread::yield_now();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let result = self.finished.recv_timeout(GUARD);
        if !thread::panicking() {
            result.expect("server shutdown guard").unwrap();
        }
    }
}
// A test-only raw client makes malformed requests possible; production uses Hyper.
fn parsed(response: &str) -> (u16, Value) {
    let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = response.split_once("\r\n\r\n").unwrap().1;
    (status, serde_json::from_str(body).unwrap())
}

#[test]
fn real_api_steps_to_final_state_and_rejects_the_completion_race() {
    let server = Server::start(Config::default());
    assert_eq!(server.until("0", "paused")["count"], "3");
    for tick in 1..=5 {
        let (status, receipt) = server.control("step");
        assert_eq!(status, 200);
        assert_eq!(receipt["applied"], true);
        assert_eq!(receipt["tick"], tick.to_string());
        server.until(
            &tick.to_string(),
            if tick == 5 { "completed" } else { "paused" },
        );
    }
    let final_state = server.snapshot();
    assert_eq!(final_state["cells"][11], json!({"id":"5","value":"21"}));
    assert_eq!(final_state["cells"][12], json!({"id":"2","value":"21"}));
    assert_eq!(
        final_state["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v.is_null())
            .count(),
        2
    );
    assert_eq!(
        final_state["totals"],
        json!({"moves":"2","creations":"2","removals":"3","value_changes":"2"})
    );
    for command in ["step", "resume", "pause"] {
        assert_eq!(server.control(command).0, 409);
    }
    assert_eq!(server.snapshot(), final_state);
}

#[test]
fn pause_resume_and_single_step_have_applied_receipts() {
    let server = Server::start(Config {
        tick_interval: Duration::from_secs(3600),
        ..Config::default()
    });
    assert_eq!(server.control("resume").0, 200);
    server.until("0", "running");
    let (status, reply) = server.control("step");
    assert_eq!(status, 409);
    assert_eq!(reply["applied"], false);
    assert_eq!(reply["tick"], "0");
    assert_eq!(server.control("pause").0, 200);
    server.until("0", "paused");
    assert_eq!(server.control("step").1["tick"], "1");
    server.until("1", "paused");
}

#[test]
fn slow_incomplete_http_request_does_not_prevent_completion_or_shutdown() {
    let server = Server::start(Config {
        tick_interval: Duration::ZERO,
        ..Config::default()
    });
    let mut slow = TcpStream::connect(server.address).unwrap();
    slow.set_read_timeout(Some(GUARD)).unwrap();
    // A client that never completes headers cannot tie up the engine or its cache.
    write!(
        slow,
        "GET /api/snapshot HTTP/1.1\r\nHost: {}\r\n",
        server.address
    )
    .unwrap();
    assert_eq!(server.control("resume").0, 200);
    let final_state = server.until("5", "completed");
    assert_eq!(final_state["count"], "2");
    let mut bytes = Vec::new();
    let result = slow.read_to_end(&mut bytes);
    assert!(result.is_ok() || result.unwrap_err().kind() == std::io::ErrorKind::ConnectionReset);
    assert_eq!(server.snapshot(), final_state);
}

#[test]
fn rejects_foreign_origins_hosts_oversize_and_invalid_commands_without_mutation() {
    let server = Server::start(Config::default());
    let before = server.snapshot();
    for headers in [
        "Content-Type: application/json\r\n".to_owned(),
        "Origin: https://foreign.example\r\nContent-Type: application/json\r\n".to_owned(),
        format!(
            "Origin: http://{}\r\nSec-Fetch-Site: cross-site\r\nContent-Type: application/json\r\n",
            server.address
        ),
        format!(
            "Origin: http://{}\r\nHost: foreign.example\r\nContent-Type: application/json\r\n",
            server.address
        ),
    ] {
        let reply = server.request("POST", "/api/control", &headers, "{\"command\":\"step\"}");
        assert!(
            reply.starts_with("HTTP/1.1 403") || reply.starts_with("HTTP/1.1 400"),
            "{reply}"
        );
    }
    let headers = format!(
        "Origin: http://{}\r\nContent-Type: application/json\r\n",
        server.address
    );
    for body in [
        "{}",
        "{",
        "{\"command\":\"reset\"}",
        "{\"command\":\"step\",\"extra\":1}",
        "{\"command\":1}",
        "{\"command\":\"resume\",\"command\":\"step\"}",
    ] {
        assert_eq!(
            parsed(&server.request("POST", "/api/control", &headers, body)).0,
            400
        );
    }
    assert_eq!(
        parsed(&server.request("POST", "/api/control", &headers, &"x".repeat(129))).0,
        413
    );
    assert_eq!(
        parsed(&server.request(
            "POST",
            "/api/control",
            &format!("Origin: http://{}\r\n", server.address),
            "{}"
        ))
        .0,
        415
    );
    for path in ["/LICENSE", "/../Cargo.toml", "/api/reset"] {
        assert_eq!(parsed(&server.request("GET", path, "", "")).0, 404);
    }
    assert_eq!(server.snapshot(), before);
}
