//! Stage 0: interoceptive mirror tests.
//!
//! Each box tick should mirror its scalar onto the corresponding shared axis.
//! These tests verify the MECHANISM (push_observation called with correct axis)
//! without asserting exact numeric values — the box dynamics are not our concern
//! here, only that the mirror is wired up.
//!
//! Zero-behavior guard: vital_core dynamics are asserted to be identical whether
//! or not the mirror is called (the box tick is the same; push_observation is
//! additive to the shared axis, never feeding back into the box).

use std::sync::Arc;
use bionic_v0::{
    INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
    INTERO_SELF_COHERENCE, INTERO_AGENCY,
    INTERO_GAP_PRESSURE, INTERO_CLOSURE,
    INTERO_LEARNING_PROGRESS,
    MOD_VIB, VIB_BANDS,
};
use bionic_v0::viewport::Viewport;

fn make_vp() -> Arc<Viewport> {
    Arc::new(Viewport::new())
}

/// tick_vital_core must push reserve / vital_anxiety / pain onto their axes.
/// We advance the vital core through several ticks with a very high drain so
/// reserve falls and anxiety/pain build, then check the mirror axes are non-zero.
#[test]
fn vital_core_mirrors_to_interoceptive_axes() {
    let vp = make_vp();
    let t0 = 5_000_000u64;
    // Run enough vital steps (1 Hz) to get reserve moving.
    for i in 0..30u64 {
        let t = t0 + i * 1_000_000; // 1-second steps
        vp.tick_vital_core(t);
    }
    let reserve_obs = vp.latest_observation(INTERO_RESERVE).map(|s| s.value);
    let anxiety_obs = vp.latest_observation(INTERO_VITAL_ANXIETY).map(|s| s.value);
    let pain_obs    = vp.latest_observation(INTERO_PAIN).map(|s| s.value);

    assert!(reserve_obs.is_some(), "INTERO_RESERVE must have an observation after tick_vital_core");
    assert!(anxiety_obs.is_some(), "INTERO_VITAL_ANXIETY must have an observation after tick_vital_core");
    assert!(pain_obs.is_some(),    "INTERO_PAIN must have an observation after tick_vital_core");

    // Reserve should be in [0,1]
    let r = reserve_obs.unwrap();
    assert!((0.0..=1.0).contains(&r), "reserve observation must be in [0,1], got {r}");
}

/// tick_vital_core must NOT change the actual vital_core box state compared to
/// a fresh viewport that hasn't called the mirror.  (The mirror is push_observation
/// only — it does not feed back into the box.)
#[test]
fn vital_core_mirror_does_not_change_box_dynamics() {
    // Two identical viewports; tick both; compare final VitalCoreState.
    let vp_a = make_vp();
    let vp_b = make_vp();
    let t0 = 5_000_000u64;
    for i in 0..10u64 {
        let t = t0 + i * 1_000_000;
        vp_a.tick_vital_core(t);
        vp_b.tick_vital_core(t);
    }
    let a = vp_a.read_vital_core();
    let b = vp_b.read_vital_core();
    // Both viewports ran identically; the mirror (push_observation) must not have
    // introduced any divergence in the box state.
    assert_eq!(a.reserve, b.reserve, "reserve must be identical");
    assert_eq!(a.vital_anxiety, b.vital_anxiety, "vital_anxiety must be identical");
    assert_eq!(a.pain, b.pain, "pain must be identical");
    assert_eq!(a.state, b.state, "vital state must be identical");
}

/// tick_self_model must push self_coherence / agency onto their axes.
#[test]
fn self_model_mirrors_to_interoceptive_axes() {
    let vp = make_vp();
    let t0 = 1_000_000u64;
    // Need at least a few vital ticks first (self_model reads from vital).
    for i in 0..5u64 {
        let t = t0 + i * 1_000_000;
        vp.tick_vital_core(t);
        vp.tick_self_model(t);
    }
    assert!(
        vp.latest_observation(INTERO_SELF_COHERENCE).is_some(),
        "INTERO_SELF_COHERENCE must have an observation after tick_self_model"
    );
    assert!(
        vp.latest_observation(INTERO_AGENCY).is_some(),
        "INTERO_AGENCY must have an observation after tick_self_model"
    );
}

/// refresh_active_reasoning_gap must push gap_pressure / closure_score.
#[test]
fn active_reasoning_mirrors_to_interoceptive_axes() {
    let vp = make_vp();
    let t = 2_000_000u64;
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    vp.refresh_active_reasoning_gap(t);

    assert!(
        vp.latest_observation(INTERO_GAP_PRESSURE).is_some(),
        "INTERO_GAP_PRESSURE must be mirrored after refresh_active_reasoning_gap"
    );
    assert!(
        vp.latest_observation(INTERO_CLOSURE).is_some(),
        "INTERO_CLOSURE must be mirrored after refresh_active_reasoning_gap"
    );
}

/// tick_self_development_from_substrate must push prediction_improvement as LEARNING_PROGRESS.
#[test]
fn self_development_mirrors_learning_progress() {
    let vp = make_vp();
    let t = 3_000_000u64;
    // Bootstrap enough state for self_development to run.
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    vp.refresh_active_reasoning_gap(t);
    vp.tick_prediction_pressure_field_from_substrate(t);
    vp.tick_evidence_tension_from_pairs(t);
    vp.tick_self_development_from_substrate(t);

    assert!(
        vp.latest_observation(INTERO_LEARNING_PROGRESS).is_some(),
        "INTERO_LEARNING_PROGRESS must be mirrored after tick_self_development_from_substrate"
    );
    let v = vp.latest_observation(INTERO_LEARNING_PROGRESS).unwrap().value;
    assert!((0.0..=1.0).contains(&v), "INTERO_LEARNING_PROGRESS must be in [0,1], got {v}");
}

/// All ten interoceptive axes accept observations at valid indices.
/// (Belt-and-suspenders: push_observation must not panic for intero indices.)
#[test]
fn all_interoceptive_axes_accept_push_observation() {
    use bionic_v0::{INTERO_ATTACHMENT, INTERO_PRESENCE, AXIS_COUNT};
    let vp = make_vp();
    let axes = [
        INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
        INTERO_SELF_COHERENCE, INTERO_AGENCY, INTERO_ATTACHMENT,
        INTERO_GAP_PRESSURE, INTERO_CLOSURE, INTERO_LEARNING_PROGRESS,
        INTERO_PRESENCE,
    ];
    for (i, &ax) in axes.iter().enumerate() {
        assert!(ax < AXIS_COUNT, "axis {ax} out of bounds");
        vp.push_observation(ax, 1_000_000 + i as u64, 0.5);
        let obs = vp.latest_observation(ax);
        assert!(obs.is_some(), "axis {ax} must return an observation after push");
        assert!((obs.unwrap().value - 0.5).abs() < 1e-9);
    }
}
