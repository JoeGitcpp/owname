//! 体检 (2026-06-11): end-to-end vocal bootstrap over the REAL closed loop —
//! voice → hear self (efference copy, no microphone) → compare to target →
//! evidence-carrying closures → efficacy EMA + developmental clock climb.
//!
//! Guards the two halves of the cold-start fix:
//! 1. With heard-self evidence and a matching target, a newborn must actually
//!    learn (annealed EMA: 8× alpha at m=0) — the 0×x=0 deadlock is escaped.
//! 2. Without heard-self evidence the same closures must credit NOTHING —
//!    the honesty gate (07a8a9a) stays intact; no hallucinated convergence.

use bionic_v0::viewport::Viewport;
use std::sync::Arc;

const T0: u64 = 1_000_000;
const STEP_US: u64 = 300_000; // > VOCAL_SELF_PLAY_MIN_INTERVAL_US cadence

/// Voice, self-hear through the efference copy, and feed the heard contour
/// back as the external target (a mentor matching the infant's own babble —
/// the best-case acoustic fit).
fn voice_and_hear(vp: &Arc<Viewport>, t: u64) {
    vp.begin_voicing(t, 0.8, 0.5, 0.5);
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
    let (bands, _) = vp.read_self_vocal_bands();
    let (mel64, _) = vp.read_self_vocal_mel64();
    vp.observe_vocal_target_full(t, 0.8, 0.5, 0.5, 0.5, bands, mel64);
}

#[test]
fn heard_self_closures_grow_efficacy_and_maturity() {
    let vp = Arc::new(Viewport::new());
    voice_and_hear(&vp, T0);
    let (bands, _) = vp.read_self_vocal_bands();
    assert!(
        bands.iter().sum::<f64>() > 1e-6,
        "efference copy must produce non-silent heard-self bands while voicing"
    );

    for i in 0..200u64 {
        let t = T0 + (i + 1) * STEP_US;
        voice_and_hear(&vp, t);
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
    }

    let bias = vp.read_learned_bias();
    assert!(
        bias.vocal_efficacy > 0.2,
        "200 evidence-carrying closures must lift newborn efficacy past 0.2 \
         (annealed EMA), got {}",
        bias.vocal_efficacy
    );
    assert!(
        vp.developmental_maturity() > 0.1,
        "earned closures must advance the developmental clock, got {}",
        vp.developmental_maturity()
    );
}

#[test]
fn closures_without_heard_self_credit_nothing() {
    let vp = Arc::new(Viewport::new());
    // Voice (so vocalized=true is honest) but never self-hear: bands stay 0.
    vp.begin_voicing(T0, 0.8, 0.5, 0.5);

    for i in 0..200u64 {
        let t = T0 + (i + 1) * STEP_US;
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
    }

    let bias = vp.read_learned_bias();
    assert!(
        bias.vocal_efficacy == 0.0,
        "no heard-self → no competence credit, got {}",
        bias.vocal_efficacy
    );
    assert!(
        vp.developmental_maturity() == 0.0,
        "no heard-self → developmental clock must not move, got {}",
        vp.developmental_maturity()
    );
}
