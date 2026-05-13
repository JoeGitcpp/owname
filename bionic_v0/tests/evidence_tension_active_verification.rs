use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::probe_law::ProbePhase;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::substrate_pressure::PressureSample;
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

fn tense_sample() -> PressureSample {
    PressureSample {
        err: 0.8,
        ambiguity: 0.7,
        deviation: 0.9,
        recurrence: 0.6,
        cross_channel: 0.7,
        ..Default::default()
    }
}

#[test]
fn evidence_tension_tick_records_unresolved_numeric_pressure() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_sample(), 1_000_000);

    let st = vp.read_evidence_tension();
    assert!(st.current_tension > 0.0);
    assert!(st.unresolved_pressure > 0.0);
    assert_eq!(st.history_len, 1);
    assert_eq!(st.last_update_t_us, 1_000_000);
    assert_eq!(st.history[0].pair_cursor, 0);
    assert!(st.history[0].closure_support <= 1.0);
}

#[test]
fn evidence_tension_quiet_features_decay_without_truth_label() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_sample(), 1_000_000);
    let high = vp.read_evidence_tension().unresolved_pressure;
    vp.tick_evidence_tension(PressureSample::default(), 2_000_000);

    let st = vp.read_evidence_tension();
    assert!(st.unresolved_pressure < high);
    assert_eq!(st.history_len, 2);
}

#[test]
fn unresolved_evidence_tension_contributes_to_probe_pressure() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_sample(), 1_000_000);
    vp.refresh_probe_law(1_100_000);

    let st = vp.read_probe_law();
    assert!(st.intensity > 0.0);
    assert!(
        st.phase == ProbePhase::P1 || st.phase == ProbePhase::P2,
        "unresolved evidence pressure should be eligible to raise probe phase"
    );
}

#[test]
fn new_consequence_pair_reduces_unresolved_evidence_pressure_once() {
    let vp = Viewport::new();
    vp.tick_evidence_tension(tense_sample(), 1_000_000);
    let before = vp.read_evidence_tension().unresolved_pressure;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.8,
        event_t_us: 1_100_000,
    });
    let write_seq = vp.read_action_output().write_epoch_history[0].write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_200_000,
        end_t_us: 1_250_000,
        magnitude: 0.8,
    });
    let window_id = vp.read_action_consequence().windows[0].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, 100_000));

    vp.tick_evidence_tension_from_pairs(1_300_000);
    let after_first = vp.read_evidence_tension().unresolved_pressure;
    vp.tick_evidence_tension_from_pairs(1_400_000);
    let after_second = vp.read_evidence_tension().unresolved_pressure;

    assert!(after_first < before);
    assert!(
        (after_second - after_first).abs() < 1e-12,
        "same pair cursor must not apply closure repeatedly"
    );
}

#[test]
fn evidence_tension_is_snapshot_visible_as_numeric_trace() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_evidence_tension_snapshot_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());

    vp.tick_evidence_tension(tense_sample(), 1_000_000);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert!(snap.evidence_tension.current_tension > 0.0);
    assert!(snap.evidence_tension.unresolved_pressure > 0.0);
    assert_eq!(snap.evidence_tension.history_len, 1);

    let _ = std::fs::remove_file(paths.final_path);
    let _ = std::fs::remove_file(paths.tmp_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn evidence_tension_modules_do_not_contain_rejected_ontology_terms() {
    let evidence = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/evidence_tension.rs"
    ))
    .expect("read evidence_tension.rs");
    let probe = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/probe_law.rs"))
        .expect("read probe_law.rs");
    let forbidden = &[
        "truth_label",
        "true_label",
        "false_label",
        "trust_list",
        "distrust_list",
        "target_object",
        "result_label",
        "semantic_gate",
        "concept_id",
        "token",
    ];
    for kw in forbidden {
        assert!(
            !evidence.contains(kw),
            "evidence_tension.rs must not contain: {}",
            kw
        );
        assert!(!probe.contains(kw), "probe_law.rs must not contain: {}", kw);
    }
}

#[test]
fn body_refresh_consumes_evidence_pair_return_before_probe_refresh() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    let closure_pos = src
        .find("tick_evidence_tension_from_pairs")
        .expect("body refresh must consume evidence pair return");
    let probe_pos = src
        .find("refresh_probe_law")
        .expect("body refresh must still refresh probe law");
    assert!(
        closure_pos < probe_pos,
        "pair-return closure must run before probe-law pressure refresh"
    );
}
