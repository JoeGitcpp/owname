// Contract tests for the live body active-probe entry.
//
// Plan: docs/superpowers/plans/2026-04-22-live-body-active-probe-entry.md
//
// These tests verify that the probe-law substrate drives action-output write
// epochs without external commands, that structural suppression inhibits the
// probe, and that probe writes can close into consequence pairs.
// No semantic/target/result fields are introduced by any test.

use bionic_v0::action_consequence::{
    ActionConsequenceWindowEvent, ConsequenceModalityKind, ConsequencePairStatus,
};
use bionic_v0::probe_law::{ProbeLawState, ProbePhase};
use bionic_v0::viewport::Viewport;

#[test]
fn probe_generates_write_epoch_without_external_command() {
    let vp = Viewport::new();
    // Force probe into Active phase by directly writing state
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.7;
        st.suppression = 0.3;
        st.phase = ProbePhase::Active;
    }
    vp.tick_probe_output_from_law(1_000_000);
    let ao = vp.read_action_output();
    assert!(
        ao.n_write_epochs > 0,
        "probe in Active phase must produce write epoch"
    );
}

#[test]
fn suppression_dominant_causes_decay_or_idle() {
    let vp = Viewport::new();
    // First let probe become active
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.7;
        st.suppression = 0.3;
        st.phase = ProbePhase::Active;
    }
    vp.tick_probe_output_from_law(1_000_000);
    // Now drive suppression >> intensity → probe transitions to Decaying via refresh
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.1;
        st.suppression = 0.9;
        // ratio = 0.1/0.9 < 1.0 → from Active → Decaying on next refresh
    }
    vp.refresh_probe_law(2_000_000);
    let st = vp.probe_law.read().unwrap().clone();
    assert!(
        st.phase == ProbePhase::Decaying || st.phase == ProbePhase::Idle,
        "suppression dominant must cause Decaying or Idle phase, got {:?}",
        st.phase
    );
}

#[test]
fn probe_write_can_close_into_consequence_pair() {
    use bionic_v0::action_consequence::ConsequencePairStatus;
    use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};

    let vp = Viewport::new();
    // Generate a write epoch via probe
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.6;
        st.suppression = 0.2;
        st.phase = ProbePhase::Active;
    }
    vp.tick_probe_output_from_law(1_000_000);
    let ao = vp.read_action_output();
    assert!(ao.n_write_epochs > 0);
    let write_seq = ao.write_epoch_history[0].write_seq;

    // Inject a consequence window (simulating ingress change)
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_200_000,
        end_t_us: 1_400_000,
        magnitude: 0.4,
    });
    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 1);
    let wid = ac.windows[0].window_id;

    // Pair them
    vp.pair_action_consequence(write_seq, wid, 200_000);
    let ac2 = vp.read_action_consequence();
    assert_eq!(ac2.n_pairs, 1);
    assert_eq!(ac2.pairs[0].status, ConsequencePairStatus::Paired);
    assert_eq!(ac2.pairs[0].write_seq, write_seq);
}

#[test]
fn no_semantic_fields_in_probe_law_module() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/probe_law.rs",))
        .expect("read probe_law.rs");

    let forbidden = &[
        "target_object",
        "result_label",
        "concept_id",
        "token",
        "semantic_gate",
        "curiosity_threshold",
    ];
    for kw in forbidden {
        assert!(!src.contains(kw), "probe_law.rs must not contain: {}", kw);
    }
}
