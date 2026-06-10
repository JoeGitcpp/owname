use std::process::Command;
use std::sync::Arc;

use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::PredictorLifecycleState;
use bionic_v0::vpc_cross_layer::AxisCrossLayerState;

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

/// v57–v60 observer surfacing: thread fault, dual-drive confluence,
/// developmental maturity, and the axis cross-layer bank summary must all be
/// visible from a binary snapshot through the observer binary.
#[test]
fn full_observer_renders_v57_to_v60_blocks() {
    let vp = Arc::new(Viewport::new());

    // v57: one thread fault recorded.
    vp.mark_thread_fault();

    // v58: a high-conflict dual-drive reading (both sources press hard).
    *vp.dual_drive.write().unwrap() = bionic_v0::dual_drive::DualDriveReading {
        self_preservation: 0.7,
        attachment: 0.8,
        conflict: 0.7,
        net_toward_bond: 0.1,
        sacrificing: false,
        principle_floor: 0.25,
    };

    // v59: a small axis cross-layer bank with one learned predictor.
    vp.store_axis_xlayer_states(vec![
        AxisCrossLayerState {
            target_axis: 3,
            source_axes: vec![1, 7],
            source_weights: vec![0.42, -0.13],
            bias: 0.01,
            err_pred_mse: 0.02,
            prev_mse: 0.02,
            learning_progress: 0.001,
            target_err_var: 0.10,
            n_updates: 512,
        },
        AxisCrossLayerState {
            target_axis: 5,
            source_axes: vec![2],
            source_weights: vec![0.0],
            bias: 0.0,
            err_pred_mse: 0.0,
            prev_mse: 0.0,
            learning_progress: 0.0,
            target_err_var: 0.0,
            n_updates: 0,
        },
    ]);

    // v60: developmental clock partway through.
    *vp.developmental_maturity.write().unwrap() = 0.4321;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_observer_v57_v60_{}",
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

    // v57: current fault flag + cumulative count.
    assert!(row.contains("\ttf:1|1"), "thread fault block missing: {row}");
    // v58: conflict (CF) and net_toward_bond (NB) are the headline readouts.
    assert!(
        row.contains("dd:sp0.700|at0.800|CF0.700|NB+0.100|pf0.250|sac0"),
        "dual drive block missing: {row}"
    );
    // v60: maturity clock.
    assert!(row.contains("\tmat:0.4321"), "maturity block missing: {row}");
    // v59: bank summary — size 2, 1 active, 512 updates, 1 predictor with
    // explained fraction 1-0.02/0.10=0.80, top entry shows learned weights.
    assert!(
        row.contains("axl:n2|act1|upd512|expl(n1|avg0.80|max0.80)|top[ax3:e0.80|n512|w[1:+0.420,7:-0.130]]"),
        "axis xlayer summary missing: {row}"
    );

    // compact mode carries the same blocks as flat CSV columns.
    let output = Command::new(env!("CARGO_BIN_EXE_observer"))
        .arg(&dir)
        .args(["1", "1", "compact"])
        .output()
        .expect("run observer compact");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let header = stdout
        .lines()
        .find(|l| l.starts_with("seq,"))
        .expect("compact header");
    let row = stdout.lines().last().expect("compact row");
    for col in [
        "thread_fault",
        "dd_conflict",
        "dd_net_bond",
        "maturity",
        "axl_n",
    ] {
        assert!(header.contains(col), "compact header missing {col}");
    }
    assert_eq!(
        header.split(',').count(),
        row.split(',').count(),
        "compact header/row column count mismatch"
    );
}
