use std::process::Command;

#[test]
fn supported_headless_commands_report_the_fixture_and_extra_wait_ticks() {
    for (ticks, count) in [(0, 3), (5, 2), (8, 2)] {
        let output = Command::new(env!("CARGO_BIN_EXE_headless"))
            .args(["--ticks", &ticks.to_string()])
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
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
