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

#[test]
fn api_reports_selected_actions_and_unacted_children_without_implying_success() {
    use virtual_life::{engine::Weights, experiment::ExperimentConfig};
    for (weights, occupancy, selected) in [
        ([1, 0, 0, 0], 1_000_000, "Wait"),
        ([0, 1, 0, 0], 1_000_000, "Move"),
        ([0, 0, 1, 0], 200_000, "Copy"),
        ([0, 0, 0, 1], 1_000_000, "Repair"),
    ] {
        let server = Server::start(Config {
            ticks: 1,
            start_paused: true,
            experiment: Some(ExperimentConfig {
                width: 3,
                height: 3,
                occupancy,
                bundles: vec![Weights(weights)],
                proportions: vec![1],
                ..Default::default()
            }),
            ..Config::default()
        });
        let initial = server.until("0", "paused");
        let initial_ids: Vec<_> = initial["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| !a.is_null())
            .map(|a| {
                assert!(a.as_object().unwrap().contains_key("last_action"));
                assert!(a["last_action"].is_null());
                a["id"].clone()
            })
            .collect();
        assert_eq!(server.control("step").0, 200);
        let final_sample = server.until("1", "completed");
        assert_eq!(
            final_sample["experiment"]["protocol"],
            "wear-repair crowding v3"
        );
        for cell in final_sample["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| !a.is_null())
        {
            if initial_ids.contains(&cell["id"]) {
                assert_eq!(cell["last_action"], selected);
            } else {
                assert_eq!(selected, "Copy");
                assert!(cell["last_action"].is_null());
                assert_eq!(cell["integrity"], 10);
            }
        }
        assert_eq!(final_sample["totals"]["moves"], "0"); // Full world's Moves are all rejected.
        assert_eq!(
            final_sample["totals"]["creations"],
            if selected == "Copy" { "1" } else { "0" }
        );
    }
    for experiment in [None, Some(ExperimentConfig::random())] {
        let server = Server::start(Config {
            ticks: 0,
            experiment,
            ..Config::default()
        });
        for cell in server.until("0", "completed")["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| !a.is_null())
        {
            assert!(!cell.as_object().unwrap().contains_key("last_action"));
        }
    }
}

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
        request(self.address, method, path, headers, body)
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

fn request(address: SocketAddr, method: &str, path: &str, headers: &str, body: &str) -> String {
    let mut stream = TcpStream::connect_timeout(&address, GUARD).unwrap();
    stream.set_read_timeout(Some(GUARD)).unwrap();
    stream.set_write_timeout(Some(GUARD)).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        address,
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
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

fn identity(response: &str) -> String {
    response
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("x-virtuallife-run")
                .then(|| value.trim().to_owned())
        })
        .expect("response run identity")
}

impl Server {
    fn identified_snapshot(&self) -> (String, Value) {
        let response = self.request("GET", "/api/snapshot", "", "");
        (identity(&response), parsed(&response).1)
    }

    fn restart(&self, run: &str, body: &str) -> String {
        self.request("POST", "/api/restart", &format!(
            "Origin: http://{}\r\nContent-Type: application/json\r\nX-VirtualLife-Run: {run}\r\n", self.address
        ), body)
    }
}

#[test]
fn seeded_restart_replays_initial_and_fixed_tick_states_and_preserves_configuration() {
    use virtual_life::{
        engine::{Maintenance, Weights},
        experiment::ExperimentConfig,
    };
    for maintenance in [
        None,
        Some(Maintenance {
            maximum: 17,
            upkeep: 2,
            crowding_threshold: 3,
            crowding_upkeep: 2,
            move_wear: 3,
            copy_wear: 4,
            repair: 6,
        }),
    ] {
        let server = Server::start(Config {
            ticks: 12,
            sample_every: 7,
            tick_interval: Duration::from_secs(3600),
            experiment: Some(ExperimentConfig {
                width: 8,
                height: 6,
                occupancy: 500_000,
                seed: u64::MAX,
                bundles: vec![Weights([1, 2, 3, 4]), Weights([4, 3, 2, 1])],
                proportions: vec![3, 2],
                maintenance,
            }),
            ..Config::default()
        });
        let (mut run, initial) = server.identified_snapshot();
        let mut expected_final = None;
        for _ in 0..2 {
            for tick in 1..=12 {
                assert_eq!(server.control("step").0, 200);
                server.until(
                    &tick.to_string(),
                    if tick == 12 { "completed" } else { "paused" },
                );
            }
            let final_state = server.snapshot();
            if let Some(expected) = &expected_final {
                assert_eq!(&final_state, expected);
            }
            expected_final = Some(final_state);
            let reply = server.restart(&run, r#"{"seed":"18446744073709551615"}"#);
            assert_eq!(parsed(&reply).0, 200, "{reply}");
            let next = identity(&reply);
            assert_ne!(next, run);
            assert_eq!(parsed(&reply).1, initial);
            assert_eq!(
                server.identified_snapshot(),
                (next.clone(), initial.clone())
            );
            run = next;
        }
        // Running at tick zero is a deterministic barrier; pacing is retained.
        assert_eq!(server.control("resume").0, 200);
        server.until("0", "running");
        let reply = server.restart(&run, r#"{"seed":"0"}"#);
        assert_eq!(parsed(&reply).0, 200);
        let (zero_run, zero) = server.identified_snapshot();
        assert_eq!(zero["status"], "paused");
        assert_eq!(zero["experiment"]["seed"], "0");
        let random = server.restart(&zero_run, r#"{"random":true}"#);
        assert_eq!(parsed(&random).0, 200);
        let random_state = parsed(&random).1;
        let replay = server.restart(
            &identity(&random),
            &json!({"seed":random_state["experiment"]["seed"]}).to_string(),
        );
        assert_eq!(parsed(&replay), (200, random_state));
    }
}

#[test]
fn preset_restart_retains_current_bundles_for_seed_only_replay() {
    use virtual_life::{engine::Weights, experiment::ExperimentConfig};
    let server = Server::start(Config {
        ticks: 3,
        sample_every: 7,
        tick_interval: Duration::from_secs(3600),
        experiment: Some(ExperimentConfig {
            width: 8,
            height: 6,
            occupancy: 500_000,
            ..Default::default()
        }),
        ..Config::default()
    });
    let (run, old) = server.identified_snapshot();
    let reply = server.restart(
        &run,
        r#"{"seed":"18446744073709551615","preset":"moderate-movement"}"#,
    );
    assert_eq!(parsed(&reply).0, 200, "{reply}");
    let next = identity(&reply);
    assert_ne!(next, run);
    let initial = parsed(&reply).1;
    assert_eq!(initial["tick"], "0");
    assert_eq!(initial["status"], "paused");
    assert_eq!(initial["experiment"]["preset"], "moderate-movement");
    assert_eq!(
        initial["experiment"]["maintenance"],
        old["experiment"]["maintenance"]
    );
    let bundles = vec![
        Weights([42, 4, 4, 30]),
        Weights([34, 10, 4, 32]),
        Weights([26, 16, 4, 34]),
        Weights([18, 22, 4, 36]),
    ];
    let expected = ExperimentConfig {
        width: 8,
        height: 6,
        occupancy: 500_000,
        seed: u64::MAX,
        bundles,
        proportions: vec![1; 4],
        ..Default::default()
    };
    let (mut world, mut random, _) = expected.initialize().unwrap();
    for tick in 1..=3 {
        let proposals = virtual_life::experiment::proposals(&world, &mut random);
        world.step(&proposals).unwrap();
        assert_eq!(server.control("step").0, 200);
        server.until(
            &tick.to_string(),
            if tick == 3 { "completed" } else { "paused" },
        );
    }
    let actual = server.snapshot();
    for (cell, agent) in actual["cells"]
        .as_array()
        .unwrap()
        .iter()
        .zip(world.cells())
    {
        match agent {
            None => assert!(cell.is_null()),
            Some(agent) => {
                assert_eq!(cell["id"], agent.id.to_string());
                assert_eq!(cell["weights"], json!(agent.weights.0));
                assert_eq!(cell["integrity"], agent.integrity);
            }
        }
    }
    let replay = server.restart(&next, r#"{"seed":"18446744073709551615"}"#);
    assert_eq!(parsed(&replay), (200, initial));
}

#[test]
fn every_preset_matches_ordinary_initialization_and_fixed_ticks_with_custom_settings() {
    use virtual_life::{
        engine::{Maintenance, Weights},
        experiment::{ExperimentConfig, proposals},
        runner::{Snapshot, Status},
    };
    let settings = ExperimentConfig {
        width: 9,
        height: 7,
        occupancy: 400_000,
        seed: u64::MAX,
        bundles: vec![Weights([1, 2, 3, 4])],
        proportions: vec![7],
        maintenance: Some(Maintenance {
            maximum: 17,
            upkeep: 2,
            crowding_threshold: 3,
            crowding_upkeep: 2,
            move_wear: 3,
            copy_wear: 4,
            repair: 6,
        }),
    };
    let server = Server::start(Config {
        ticks: 8,
        sample_every: 7,
        tick_interval: Duration::from_secs(3600),
        experiment: Some(settings.clone()),
        ..Config::default()
    });
    let (mut run, custom) = server.identified_snapshot();
    assert!(custom["experiment"]["preset"].is_null());
    let expected = [
        (
            "original",
            "Original",
            [[2, 4, 1, 3], [2, 2, 2, 4], [4, 1, 1, 4], [1, 5, 2, 2]],
        ),
        (
            "moderate-movement",
            "Moderate movement",
            [
                [42, 4, 4, 30],
                [34, 10, 4, 32],
                [26, 16, 4, 34],
                [18, 22, 4, 36],
            ],
        ),
        (
            "wide-movement-range",
            "Wide movement range",
            [
                [42, 4, 4, 30],
                [31, 12, 4, 33],
                [20, 20, 4, 36],
                [9, 28, 4, 39],
            ],
        ),
        (
            "lower-copying",
            "Lower copying",
            [
                [44, 4, 2, 30],
                [33, 12, 2, 33],
                [22, 20, 2, 36],
                [11, 28, 2, 39],
            ],
        ),
    ];
    let catalog = custom["experiment"]["presets"].as_array().unwrap();
    assert_eq!(catalog.len(), 4);
    for (index, (id, name, bundles)) in expected.into_iter().enumerate() {
        assert_eq!(catalog[index]["id"], id);
        assert_eq!(catalog[index]["name"], name);
        assert_eq!(catalog[index]["bundles"], json!(bundles));
        assert_eq!(catalog[index]["proportions"], json!([1, 1, 1, 1]));
        let reply = server.restart(
            &run,
            &json!({"seed":u64::MAX.to_string(),"preset":id}).to_string(),
        );
        assert_eq!(parsed(&reply).0, 200, "{reply}");
        assert_ne!(identity(&reply), run);
        run = identity(&reply);
        let initial = parsed(&reply).1;
        assert_eq!(initial["experiment"]["preset"], id);
        assert_eq!(
            initial["experiment"]["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["initial_count"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["7", "6", "6", "6"]
        );
        let reference = ExperimentConfig {
            bundles: bundles.map(Weights).to_vec(),
            proportions: vec![1; 4],
            ..settings.clone()
        };
        let (mut world, mut random, info) = reference.initialize().unwrap();
        for tick in 0..=8 {
            if tick > 0 {
                world.step(&proposals(&world, &mut random)).unwrap();
                assert_eq!(server.control("step").0, 200);
            }
            let actual = server.until(
                &tick.to_string(),
                if tick == 8 { "completed" } else { "paused" },
            );
            let expected = web::snapshot_json(&Snapshot {
                width: world.width(),
                height: world.height(),
                tick: world.tick(),
                count: world.count(),
                totals: world.totals(),
                cells: world.cells().to_vec(),
                status: if tick == 8 {
                    Status::Completed
                } else {
                    Status::Paused
                },
                end_tick: 8,
                experiment: Some(std::sync::Arc::new(info.clone())),
                failures: world.failures().iter().copied().collect(),
                discarded_failures: world.discarded_failures(),
            });
            assert_eq!(actual, expected, "{id} at tick {tick}");
        }
        let replay = server.restart(&run, &json!({"seed":u64::MAX.to_string()}).to_string());
        assert_eq!(parsed(&replay), (200, initial));
        run = identity(&replay);
        // Pacing survives replacement: Resume is observed at tick zero before the one-hour deadline.
        assert_eq!(server.control("resume").0, 200);
        server.until("0", "running");
        let random = server.restart(&run, &json!({"random":true,"preset":id}).to_string());
        assert_eq!(parsed(&random).0, 200);
        let random_state = parsed(&random).1;
        assert_eq!(random_state["status"], "paused");
        assert_eq!(random_state["experiment"]["preset"], id);
        let replay = server.restart(
            &identity(&random),
            &json!({"seed":random_state["experiment"]["seed"]}).to_string(),
        );
        assert_eq!(parsed(&replay), (200, random_state));
        run = identity(&replay);
    }
}

#[test]
fn old_moderate_preset_bundles_remain_custom_and_replay_without_migration() {
    use virtual_life::{engine::Weights, experiment::ExperimentConfig};
    let server = Server::start(Config {
        ticks: 1,
        experiment: Some(ExperimentConfig {
            width: 8,
            height: 6,
            occupancy: 500_000,
            bundles: vec![
                Weights([34, 4, 12, 30]),
                Weights([26, 10, 12, 32]),
                Weights([18, 16, 12, 34]),
                Weights([10, 22, 12, 36]),
            ],
            proportions: vec![1; 4],
            seed: 42,
            ..Default::default()
        }),
        ..Config::default()
    });
    let (run, initial) = server.identified_snapshot();
    assert!(initial["experiment"]["preset"].is_null());
    assert_eq!(server.control("step").0, 200);
    server.until("1", "completed");
    let replay = server.restart(&run, r#"{"seed":"42"}"#);
    assert_ne!(identity(&replay), run);
    // The entire initial state, including custom weights/proportions and label, survives replay.
    assert_eq!(parsed(&replay), (200, initial));
}

#[test]
fn presets_reject_unsupported_modes_and_require_exact_properties_for_the_current_label() {
    use virtual_life::experiment::ExperimentConfig;
    for experiment in [None, Some(ExperimentConfig::random())] {
        let server = Server::start(Config {
            ticks: 0,
            experiment,
            ..Config::default()
        });
        let (run, initial) = server.identified_snapshot();
        assert!(initial["experiment"]["presets"].is_null());
        assert!(initial["experiment"]["preset"].is_null());
        assert_eq!(
            parsed(&server.restart(&run, r#"{"seed":"0","preset":"original"}"#)).0,
            409
        );
        assert_eq!(server.identified_snapshot(), (run, initial));
    }
    for (proportions, preset) in [(vec![1; 4], Some("original")), (vec![1, 2, 1, 1], None)] {
        let server = Server::start(Config {
            ticks: 0,
            experiment: Some(ExperimentConfig {
                proportions,
                ..Default::default()
            }),
            ..Config::default()
        });
        let (run, initial) = server.identified_snapshot();
        assert_eq!(initial["experiment"]["preset"], json!(preset));
        let reply = server.restart(&run, r#"{"seed":"0","preset":"lower-copying"}"#);
        assert_eq!(parsed(&reply).0, 200);
        assert_ne!(identity(&reply), run);
        assert_eq!(parsed(&reply).1["status"], "completed");
        assert_eq!(parsed(&reply).1["tick"], "0");
        assert_eq!(parsed(&reply).1["experiment"]["preset"], "lower-copying");
    }
}

#[test]
fn restart_rejects_invalid_seeds_preconditions_and_demo_without_replacing_a_run() {
    let server = Server::start(Config {
        experiment: Some(Default::default()),
        ..Config::default()
    });
    let (run, initial) = server.identified_snapshot();
    let headers = format!(
        "Origin: http://{}\r\nContent-Type: application/json\r\nX-VirtualLife-Run: {run}\r\n",
        server.address
    );
    assert_eq!(parsed(&server.restart(&run, &"x".repeat(129))).0, 413);
    assert_eq!(
        parsed(&server.request(
            "POST",
            "/api/restart",
            &headers.replace("application/json", "text/plain"),
            r#"{"seed":"1"}"#
        ))
        .0,
        415
    );
    for forbidden in [
        headers.replace(
            &format!("Origin: http://{}", server.address),
            "Origin: https://foreign.example",
        ),
        format!("{headers}Sec-Fetch-Site: cross-site\r\n"),
        format!("{headers}Host: foreign.example\r\n"),
    ] {
        let reply = server.request(
            "POST",
            "/api/restart",
            &forbidden,
            r#"{"seed":"1","preset":"original"}"#,
        );
        assert!(
            reply.starts_with("HTTP/1.1 403") || reply.starts_with("HTTP/1.1 400"),
            "{reply}"
        );
    }
    for body in [
        "{}",
        "{",
        r#"{"seed":1}"#,
        r#"{"seed":""}"#,
        r#"{"seed":"-1"}"#,
        r#"{"seed":"+1"}"#,
        r#"{"seed":"1.0"}"#,
        r#"{"seed":"1e2"}"#,
        r#"{"seed":" 1"}"#,
        r#"{"seed":"18446744073709551616"}"#,
        r#"{"seed":"1","seed":"2"}"#,
        r#"{"random":true,"random":true}"#,
        r#"{"random":false}"#,
        r#"{"random":true,"seed":"1"}"#,
        r#"{"seed":"1","extra":0}"#,
        r#"{"seed":null}"#,
        r#"{"random":null}"#,
        r#"{"preset":"original"}"#,
        r#"{"seed":"1","preset":""}"#,
        r#"{"seed":"1","preset":"unknown"}"#,
        r#"{"seed":"1","preset":"Original"}"#,
        r#"{"seed":"1","preset":null}"#,
        r#"{"seed":"1","preset":0}"#,
        r#"{"seed":"1","preset":true}"#,
        r#"{"seed":"1","preset":[]}"#,
        r#"{"seed":"1","preset":{}}"#,
        r#"{"seed":"1","preset":"original","preset":"lower-copying"}"#,
        r#"{"seed":"1","seed":"2","preset":"original"}"#,
        r#"{"random":true,"random":true,"preset":"original"}"#,
        r#"{"seed":null,"preset":"original"}"#,
        r#"{"random":null,"preset":"original"}"#,
        r#"{"seed":"1","random":true,"preset":"original"}"#,
        r#"{"seed":"1","random":null,"preset":"original"}"#,
        r#"{"random":false,"preset":"original"}"#,
        r#"{"seed":"1","preset":"original","bundles":[]}"#,
        r#"{"seed":"1","preset":"original","extra":0}"#,
        r#"["1",null,"original"]"#,
        r#"["1"]"#,
    ] {
        assert_eq!(parsed(&server.restart(&run, body)).0, 400, "{body}");
    }
    for precondition in [
        String::new(),
        "X-VirtualLife-Run: stale\r\n".to_owned(),
        format!("X-VirtualLife-Run: {run}\r\nX-VirtualLife-Run: {run}\r\n"),
    ] {
        assert_eq!(
            parsed(&server.request(
                "POST",
                "/api/restart",
                &format!(
                    "Origin: http://{}\r\nContent-Type: application/json\r\n{precondition}",
                    server.address
                ),
                r#"{"seed":"1","preset":"original"}"#
            ))
            .0,
            409
        );
    }
    assert_eq!(server.identified_snapshot(), (run, initial));
    let demo = Server::start(Config::default());
    let (run, initial) = demo.identified_snapshot();
    assert_eq!(parsed(&demo.restart(&run, r#"{"seed":"1"}"#)).0, 409);
    assert_eq!(demo.identified_snapshot(), (run, initial));
}

#[test]
fn old_preconditions_are_checked_after_a_delayed_request_body() {
    let server = Server::start(Config {
        experiment: Some(Default::default()),
        ..Config::default()
    });
    for (path, body) in [
        ("/api/control", r#"{"command":"step"}"#),
        ("/api/restart", r#"{"seed":"2"}"#),
        (
            "/api/restart",
            r#"{"seed":"2","preset":"wide-movement-range"}"#,
        ),
    ] {
        let (run, _) = server.identified_snapshot();
        let mut delayed = TcpStream::connect_timeout(&server.address, GUARD).unwrap();
        delayed.set_read_timeout(Some(GUARD)).unwrap();
        delayed.set_write_timeout(Some(GUARD)).unwrap();
        // Withhold the final body byte. This request cannot mutate before the
        // explicitly acknowledged replacement, regardless of thread scheduling.
        write!(delayed, "POST {path} HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-VirtualLife-Run: {run}\r\n\r\n{}", server.address, server.address, body.len(), &body[..body.len()-1]).unwrap();
        let reply = server.restart(&run, r#"{"seed":"42","preset":"lower-copying"}"#);
        assert_eq!(parsed(&reply).0, 200);
        let next = identity(&reply);
        let initial = parsed(&reply).1;
        delayed
            .write_all(&body.as_bytes()[body.len() - 1..])
            .unwrap();
        let mut rejected = String::new();
        delayed.read_to_string(&mut rejected).unwrap();
        assert_eq!(parsed(&rejected).0, 409);
        assert_eq!(identity(&rejected), next);
        assert_eq!(server.identified_snapshot(), (next, initial));
    }
}

#[test]
fn zero_tick_restart_stays_completed_and_concurrent_old_run_restarts_cannot_both_apply() {
    let server = Server::start(Config {
        ticks: 0,
        experiment: Some(Default::default()),
        ..Config::default()
    });
    let (run, initial) = server.identified_snapshot();
    let barrier = std::sync::Barrier::new(3);
    let headers = format!(
        "Origin: http://{}\r\nContent-Type: application/json\r\nX-VirtualLife-Run: {run}\r\n",
        server.address
    );
    let address = server.address;
    let replies = thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            request(address, "POST", "/api/restart", &headers, r#"{"seed":"0"}"#)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            request(address, "POST", "/api/restart", &headers, r#"{"seed":"1"}"#)
        });
        barrier.wait();
        [a.join().unwrap(), b.join().unwrap()]
    });
    let mut statuses = replies.iter().map(|r| parsed(r).0).collect::<Vec<_>>();
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let (next, current) = server.identified_snapshot();
    assert_ne!(run, next);
    assert_eq!(current["status"], "completed");
    assert_eq!(current["tick"], "0");
    assert_eq!(initial["end_tick"], current["end_tick"]);
    assert_eq!(parsed(&server.restart(&run, r#"{"seed":"2"}"#)).0, 409);
    let old_command = server.request(
        "POST",
        "/api/control",
        &format!(
            "Origin: http://{}\r\nContent-Type: application/json\r\nX-VirtualLife-Run: {run}\r\n",
            server.address
        ),
        r#"{"command":"step"}"#,
    );
    assert_eq!(parsed(&old_command).0, 409);
    assert_eq!(identity(&old_command), next);
    assert_eq!(server.identified_snapshot(), (next, current));
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
