#[path = "../src/bin/observer_helpers/compact.rs"]
mod observer_compact;
#[path = "../src/bin/observer_helpers/session.rs"]
mod observer_session;

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

fn seed_reasoning_pressure(
    vp: &Viewport,
    surprise: f64,
    instability: f64,
    focus_slot_id: i8,
    focus_edge_id: i16,
) {
    let mut rs = vp.global_reasoning.write().unwrap();
    rs.global_surprise_load = surprise;
    rs.network_instability_score = instability;
    rs.routing_focus_slot_ids[0] = focus_slot_id;
    rs.routing_focus_edge_ids[0] = focus_edge_id;
}

#[test]
fn active_gap_snapshot_roundtrip_preserves_observer_curve() {
    let vp = Arc::new(Viewport::new());
    seed_reasoning_pressure(&vp, 0.8, 0.6, 2, 3);

    for t in [1_000_000, 1_010_000, 1_020_000] {
        vp.refresh_active_reasoning_gap(t);
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_active_gap_snapshot_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp snapshot dir");
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write snapshot");

    let snap = read_snapshot(&paths.final_path).expect("read snapshot");
    assert!(snap.active_reasoning_gap.gap_pressure > 0.0);
    assert!(snap.active_reasoning_gap.history_len >= 3);
    assert!(snap.active_reasoning_gap.total_samples >= 3);

    let latest = snap
        .active_reasoning_gap
        .recent_trace(0)
        .expect("snapshot latest active gap trace");
    assert!(latest.gap_pressure > 0.0);
    assert!(latest.active_cycles >= 1);
}

#[test]
fn active_gap_observer_rows_emit_curve_columns() {
    let vp = Arc::new(Viewport::new());
    seed_reasoning_pressure(&vp, 0.9, 0.7, 1, 4);
    vp.refresh_active_reasoning_gap(2_000_000);

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_active_gap_observer_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp observer dir");
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write snapshot");
    let snap = read_snapshot(&paths.final_path).expect("read snapshot");

    let compact_header = observer_compact::compact_header();
    let compact_row = observer_compact::compact_row(&snap);
    let session_header = observer_session::session_header();
    let session_row = observer_session::session_row(&snap);

    for required in ["gap_p", "gap_c", "gap_r", "gap_cycles", "gap_samples"] {
        assert!(
            compact_header.contains(required),
            "compact header missing {required}"
        );
        assert!(
            session_header.contains(required),
            "session header missing {required}"
        );
    }
    assert!(compact_row.contains("1,4"));
    assert!(session_row.contains("1,4"));
}

#[test]
fn active_gap_observer_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/active_reasoning.rs"),
        include_str!("../src/snapshot.rs"),
        include_str!("../src/bin/observer_helpers/compact.rs"),
        include_str!("../src/bin/observer_helpers/session.rs"),
    ];
    let forbidden = [
        "semantic_readiness",
        "SymbolizerState",
        "concept_id",
        "token_id",
        "word_id",
        "object_name",
        "text_understanding",
        "name_gate",
        "meaning_db",
        "dictionary",
        "parser",
        "tokenizer",
        "image_label",
        "voice_label",
        "prompt_answer",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "println!(\"body",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "active gap observer source contains rejected term: {term}"
            );
        }
    }
}
