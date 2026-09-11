use std::process::Command;

#[test]
fn automaton_cli_reports_lossless_inherited_graphs_and_seeded_replay() {
    let args = [
        "--mode",
        "autonomous",
        "--automaton-preset",
        "mixed",
        "--ticks",
        "3",
    ];
    let a = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args(args)
        .output()
        .unwrap();
    let b = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args(args)
        .output()
        .unwrap();
    assert!(a.status.success());
    assert_eq!(a.stdout, b.stdout);
    let text = String::from_utf8(a.stdout).unwrap();
    assert!(text.contains("unit-action automaton v1"));
    for preset in virtual_life::automaton::PRESETS {
        assert!(text.contains(&preset.machine.specification()));
    }
    assert!(text.contains("state="));
}

#[test]
fn headless_labels_selected_actions_initial_agents_and_newborns_explicitly() {
    for (weights, occupancy, ticks, selected, creations) in [
        ("0,1,0,0", "1", "0", "not-yet-acted", "0"),
        ("0,1,0,0", "1", "1", "Move", "0"),
        ("1,0,0,0", "1", "1", "Wait", "0"),
        ("0,0,0,1", "1", "1", "Repair", "0"),
        ("0,0,1,0", "0.2", "1", "Copy", "1"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_headless"))
            .args([
                "--mode",
                "autonomous",
                "--width",
                "3",
                "--height",
                "3",
                "--occupancy",
                occupancy,
                "--automata",
                &format!("wait:{weights}/{weights}/{weights}/{weights}"),
                "--proportions",
                "1",
                "--ticks",
                ticks,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            text.contains(&format!(
                "last_action={selected} (selected action; success not implied)"
            )),
            "{text}"
        );
        assert!(
            text.contains(&format!("moves=0 creations={creations}")),
            "{text}"
        );
        if selected == "Copy" {
            assert!(
                text.lines()
                    .any(|line| line.starts_with("id=2 ")
                        && line.contains("last_action=not-yet-acted")),
                "{text}"
            );
        }
    }
}

#[test]
fn repair_only_population_maintains_integrity_in_the_default_experiment() {
    let output = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args([
            "--mode",
            "autonomous",
            "--width",
            "3",
            "--height",
            "3",
            "--occupancy",
            "1",
            "--automata",
            "wait:0,0,0,1/0,0,0,1/0,0,0,1/0,0,0,1",
            "--proportions",
            "1",
            "--ticks",
            "2",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("tick=2 count=9"), "{text}");
    assert!(text.contains("repairs=18 failures=0"), "{text}");
    assert!(text.contains("integrity=10/10"), "{text}");
    assert!(text.contains("protocol=unit-action automaton v1"), "{text}");
    assert!(
        text.contains("crowding_threshold=5 crowding_upkeep=1"),
        "{text}"
    );
    assert!(text.contains("current_occupied_neighbors=8 next_tick_upkeep=2 (base=1 crowding=1; displayed neighborhood)"), "{text}");
}

#[test]
fn crowding_cli_reports_exact_wide_costs_in_inspection_and_recorded_failure() {
    // Include process completion (and its worker teardown) inside the guard.
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        check_wide_crowding_output();
        let _ = done.send(());
    });
    finished
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("crowding CLI exceeded its process-completion guard");
}

fn check_wide_crowding_output() {
    for ticks in ["0", "1"] {
        let output = Command::new(env!("CARGO_BIN_EXE_headless"))
            .args([
                "--mode",
                "autonomous",
                "--width",
                "3",
                "--height",
                "3",
                "--occupancy",
                "1",
                "--integrity",
                "4294967295",
                "--upkeep",
                "4294967295",
                "--crowding-upkeep",
                "4294967295",
                "--crowding-threshold",
                "8",
                "--ticks",
                ticks,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        if ticks == "0" {
            assert!(text.contains("current_occupied_neighbors=8 next_tick_upkeep=8589934590 (base=4294967295 crowding=4294967295; displayed neighborhood)"), "{text}");
        } else {
            assert!(text.contains("tick=1 count=0"), "{text}");
            assert!(text.contains("reason=upkeep integrity_before=4294967295 occupied_neighbors=8 base_upkeep=4294967295 crowding_upkeep=4294967295 upkeep=8589934590 action_wear=0"), "{text}");
        }
    }
}

#[test]
fn supported_headless_commands_report_the_fixture_and_extra_wait_ticks() {
    for (ticks, count) in [(0, 3), (5, 2), (8, 2)] {
        let output = Command::new(env!("CARGO_BIN_EXE_headless"))
            .args(["--mode", "demo", "--ticks", &ticks.to_string()])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("Replaying wear-repair v1"), "{text}");
        assert!(text.contains(&format!("tick={ticks} count={count}")));
        if ticks >= 5 {
            assert!(text.contains("moves=2 creations=2 removals=3 value_changes=2"));
            assert!(text.contains("id=2 value=21 at=(2,2)"));
            assert!(text.contains("id=5 value=21 at=(1,2)"));
        }
        if ticks > 5 {
            assert!(text.contains("all-wait"));
        }
    }
}

#[test]
fn invalid_headless_arguments_fail_with_a_message() {
    for arguments in [
        vec!["--ticks", "-1"],
        vec!["--ticks"],
        vec!["--unknown"],
        vec!["--ticks", "18446744073709551616"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_headless"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn mixed_fsm_seed_one_preserves_the_pre_cleanup_world_and_failure_records() {
    let output = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args([
            "--width", "8", "--height", "6", "--seed", "1", "--ticks", "10",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    let (_, state) = text.split_once("tick=10 count=").unwrap();
    let expected = include_str!("fixtures/fsm-seed1-tick10.txt").replace("\r\n", "\n");
    assert_eq!(format!("tick=10 count={state}"), expected);
}
