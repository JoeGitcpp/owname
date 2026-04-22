// TDD tests for curve_similarity on ActionConsequencePair (imitation laws).
//
// Plan: docs/superpowers/plans/2026-04-22-imitation-laws.md
// Spec: docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md
//
// Run order:
//   1. Tests written first → all fail (RED)
//   2. Implementation lands → all pass (GREEN)

use std::sync::Arc;

use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::viewport::Viewport;

// ─── 1. Similar write and consequence curves yield high similarity ─────────────

#[test]
fn similar_write_and_consequence_curves_yield_high_similarity() {
    let vp = Arc::new(Viewport::new());

    // Write epoch with magnitude 0.5
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.5,
        event_t_us: 1_000,
    });

    // Consequence window with similar magnitude 0.48
    vp.accumulate_ingress_change(1_500, 0.48);
    vp.try_auto_pair_latest_epoch_to_window(1_500);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
    // similarity should be high (magnitudes are close)
    assert!(
        st.pairs[0].curve_similarity > 0.8,
        "Similar magnitudes must yield high curve_similarity"
    );
}

// ─── 2. Dissimilar curves yield low similarity ────────────────────────────────

#[test]
fn dissimilar_curves_yield_low_similarity() {
    let vp = Arc::new(Viewport::new());

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.9,
        event_t_us: 1_000,
    });

    vp.accumulate_ingress_change(1_500, 0.1);
    vp.try_auto_pair_latest_epoch_to_window(1_500);

    let st = vp.read_action_consequence();
    assert!(
        st.pairs[0].curve_similarity < 0.3,
        "Dissimilar magnitudes must yield low curve_similarity"
    );
}
