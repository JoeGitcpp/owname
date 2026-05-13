use std::process::Command;

#[test]
fn curriculum_report_binary_emits_repeatable_episode_metrics() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .args(["--episodes", "3"])
        .output()
        .expect("run curriculum_report");

    assert!(
        output.status.success(),
        "curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("episodes_run=3"));
    assert!(stdout.contains("courses_run=9"));
    assert!(stdout.contains("total_steps_run="));
    assert!(stdout.contains("mean_worldline_err="));
    assert!(stdout.contains("total_pairs_formed=3"));
    assert!(stdout.contains("total_learned_events="));
    assert!(stdout.contains("max_learned_abs_delta="));
}

#[test]
fn persistent_curriculum_report_emits_same_body_metrics() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .args(["--episodes", "4", "--persistent"])
        .output()
        .expect("run curriculum_report persistent");

    assert!(
        output.status.success(),
        "persistent curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=persistent"));
    assert!(stdout.contains("episodes_run=4"));
    assert!(stdout.contains("courses_run=12"));
    assert!(stdout.contains("final_pairs=4"));
    assert!(stdout.contains("final_learned_events="));
    assert!(stdout.contains("max_learned_abs_delta="));
}

#[test]
fn curriculum_report_source_stays_observer_side_and_numeric() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/curriculum_report.rs"
    ))
    .expect("read curriculum_report.rs");

    for forbidden in [
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "request_response",
        "answer_model",
        "target_label",
        "result_label",
        "personality",
        "owner",
        "ethics",
    ] {
        assert!(
            !src.contains(forbidden),
            "curriculum_report must not introduce rejected ontology term `{}`",
            forbidden
        );
    }
}
