use std::process::Command;

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
            "--bundles",
            "0,0,0,1",
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
    assert!(text.contains("protocol=wear-repair crowding v3"), "{text}");
    assert!(
        text.contains("crowding_threshold=5 crowding_upkeep=1"),
        "{text}"
    );
    assert!(text.contains("current_occupied_neighbors=8 next_tick_upkeep=2 (base=1 crowding=1; displayed neighborhood)"), "{text}");
    assert!(
        text.contains("Replaying wear-repair v1 requires its earlier code; crowding v2 requires --crowding-upkeep 0 with matching settings."),
        "{text}"
    );
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
            .args(["--ticks", &ticks.to_string()])
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
fn autonomous_cli_records_reproducible_effective_settings_and_final_population() {
    let arguments = [
        "--mode",
        "autonomous",
        "--survival",
        "random",
        "--width",
        "8",
        "--height",
        "6",
        "--seed",
        "1",
        "--ticks",
        "10",
    ];
    let first = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args(arguments)
        .output()
        .unwrap();
    let repeated = Command::new(env!("CARGO_BIN_EXE_headless"))
        .args(arguments)
        .output()
        .unwrap();
    assert!(first.status.success() && repeated.status.success());
    assert_eq!(first.stdout, repeated.stdout);
    let text = String::from_utf8(first.stdout).unwrap();
    assert!(!text.contains("Replaying wear-repair v1"), "{text}");
    for expected in [
        "seed=1 generator=SplitMix64 / VirtualLife sampling v1",
        "protocol=random v1",
        "width=8 height=6 occupancy=0.300000 ticks=10 initial_count=14",
        "group=0 weights=[4, 5, 1, 1] proportion=1 initial_count=4",
        "tick=10 count=9",
        "moves=28 creations=7 removals=12 value_changes=0",
        "group=0 count=2",
        "group=1 count=6",
        "group=2 count=0",
        "group=3 count=1",
    ] {
        assert!(text.contains(expected), "missing {expected} in {text}");
    }
}
