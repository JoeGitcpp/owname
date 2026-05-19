use std::process::Command;
use std::sync::Arc;

use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::PredictorLifecycleState;

#[test]
fn observer_compact_source_exposes_only_short_numeric_trace_columns() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer_helpers/compact.rs"
    ))
    .expect("observer compact helper must exist");

    for required in [
        "compact_header",
        "compact_row",
        "code_frames",
        "egress_frames",
        "egress_pressure",
        "egress_deviation",
        "pair_similarity",
        "pairs",
        "probe_i",
        "evidence_p",
        "gap_p",
        "gap_c",
        "gap_r",
        "gap_cycles",
        "gap_samples",
        "voc_tE",
    ] {
        assert!(
            src.contains(required),
            "missing compact trace field: {required}"
        );
    }

    for forbidden in ["prompt", "chat", "answer", "meaning", "token_id"] {
        assert!(
            !src.contains(forbidden),
            "compact observer must not introduce {forbidden}"
        );
    }
}

#[test]
fn full_observer_summarizes_vpc_trace_evidence_instead_of_dumping_all_entries() {
    let vp = Arc::new(Viewport::new());
    for i in 0..700 {
        let publish_t_us = 10_000 + i * 10_000;
        let target_t_us = publish_t_us + 10_000;
        vp.record_vpc_axis_predictor_trace(
            1,
            0,
            publish_t_us,
            target_t_us,
            PredictorLifecycleState::OpenPublication,
        );
        vp.record_vpc_axis_publication(1, 0, publish_t_us, target_t_us);
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_observer_vpc_summary_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp runtime");
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write snapshot");

    let output = Command::new(env!("CARGO_BIN_EXE_observer"))
        .arg(&dir)
        .args(["1", "1"])
        .output()
        .expect("run observer");
    assert!(
        output.status.success(),
        "observer should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let row = stdout.lines().last().expect("observer row");
    assert!(
        row.len() <= 20_000,
        "full observer row must remain bounded; len={}",
        row.len()
    );
    assert!(row.contains("vpc:n"));
    assert!(row.contains("tr_recent["));
    assert!(row.contains("pub_recent["));
    assert!(row.contains("open_recent["));
}
