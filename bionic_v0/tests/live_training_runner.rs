use std::process::Command;

#[test]
fn live_training_runner_runs_short_solitary_runtime_and_summarizes_gates() {
    let dir = temp_dir("bionic_v0_live_training_runner_short");
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_live_training_runner"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env(
            "BIONIC_CURRICULUM_REPORT_BIN",
            env!("CARGO_BIN_EXE_curriculum_report"),
        )
        .arg(&dir)
        .args([
            "--solitary-secs",
            "1",
            "--live-secs",
            "0",
            "--sample-interval-secs",
            "1",
        ])
        .output()
        .expect("run live_training_runner");

    assert!(
        output.status.success(),
        "runner should exit successfully: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("mode=live_training_runner"));
    assert!(stdout.contains("snapshot_exists=1"));
    assert!(stdout.contains("body_output_status=silent"));
    assert!(stdout.contains("prelingual_runtime_snapshot_read=1"));
    assert!(stdout.contains("prelingual_growth_gate_pass="));
    assert!(stdout.contains("phot_structure_observed="));
    assert!(stdout.contains("vocal_write_epochs="));
    assert!(stdout.contains("return_windows="));
    assert!(stdout.contains("temporal_pairing_observed="));
    assert!(stdout.contains("closure_or_reentry_observed="));
    assert!(stdout.contains("fire_live_closure_gate_pass="));
    assert!(stdout.contains("mark_text_grounding_allowed="));

    assert!(dir.join("snapshot.bin").exists());
    assert!(dir.join("body.stdout").exists());
    assert!(dir.join("body.stderr").exists());
}

#[test]
fn live_training_runner_records_fixture_fire_gate_as_not_live() {
    let dir = temp_dir("bionic_v0_live_training_runner_fixture");
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_live_training_runner"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env(
            "BIONIC_CURRICULUM_REPORT_BIN",
            env!("CARGO_BIN_EXE_curriculum_report"),
        )
        .arg(&dir)
        .args([
            "--solitary-secs",
            "1",
            "--live-secs",
            "0",
            "--sample-interval-secs",
            "1",
        ])
        .output()
        .expect("run live_training_runner");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("fixture_deterministic_fixture=1"));
    assert!(stdout.contains("fixture_runtime_snapshot_read=0"));
    assert!(stdout.contains("fixture_fire_live_closure_gate_pass=0"));
}

#[test]
fn live_training_runner_fails_when_body_writes_text_output() {
    let dir = temp_dir("bionic_v0_live_training_runner_noisy_body");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let fake_body = dir.join("fake_body.sh");
    std::fs::write(
        &fake_body,
        "#!/bin/sh\nmkdir -p \"$1\"\nprintf 'body-noise'\nexit 0\n",
    )
    .unwrap();
    make_executable(&fake_body);

    let fake_report = dir.join("fake_report.sh");
    std::fs::write(
        &fake_report,
        "#!/bin/sh\nprintf 'curriculum_report_v0\\nmode=fire_live_closure\\ndeterministic_fixture=1\\nruntime_snapshot_read=0\\nfire_live_closure_gate_pass=0\\nprinciple_guard_pass=1\\n'\n",
    )
    .unwrap();
    make_executable(&fake_report);

    let output = Command::new(env!("CARGO_BIN_EXE_live_training_runner"))
        .env("BIONIC_BODY_BIN", &fake_body)
        .env("BIONIC_CURRICULUM_REPORT_BIN", &fake_report)
        .arg(dir.join("runtime"))
        .args([
            "--solitary-secs",
            "1",
            "--live-secs",
            "0",
            "--sample-interval-secs",
            "1",
        ])
        .output()
        .expect("run live_training_runner");

    assert!(
        !output.status.success(),
        "runner must reject body text output"
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("body_output_status=not_silent"));
}

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{prefix}_{}", std::process::id()))
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}
