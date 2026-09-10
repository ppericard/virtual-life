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
    assert!(text.contains("protocol=wear-repair crowding v2"), "{text}");
    assert!(
        text.contains("Replaying wear-repair v1 results requires its earlier code."),
        "{text}"
    );
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
