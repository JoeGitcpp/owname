//! A-fix (2026-06-22): the developmental clock advances on competence
//! IMPROVEMENT (learning progress), not absolute competence level — so it can no
//! longer ratchet to ~1.0 while real skill is flat/declining (the verified
//! hallucinated-maturity bug from the 13-min call: m→0.9955 while efficacy fell
//! 0.83→0.43 and shape_dist stayed flat ~0.235).
//!
//! Grounded in Oudeyer/Kaplan/Hafner 2007 (IAC: reward the error-DECREASE, not
//! the level) and mirrors the in-repo vision sibling observe_visual_reflex_closure.

use bionic_v0::viewport::Viewport;
use std::sync::Arc;

const T0: u64 = 1_000_000;
const SEC: u64 = 1_000_000;

/// Voice → self-hear (efference copy) → feed the heard contour back as the
/// target (best-case acoustic fit), so evidence closures carry real competence.
fn voice_and_hear(vp: &Arc<Viewport>, t: u64) {
    vp.begin_voicing(t, 0.8, 0.5, 0.5);
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
    let (bands, _) = vp.read_self_vocal_bands();
    let (mel64, _) = vp.read_self_vocal_mel64();
    vp.observe_vocal_target_full(t, 0.8, 0.5, 0.5, 0.5, bands, mel64);
}

/// Unit law: maturity advances on a positive improvement, holds on zero/negative,
/// and saturates below 1 (monotone high-water-mark; never fabricates from noise).
#[test]
fn progress_law_credits_improvement_only() {
    let vp = Viewport::new();
    assert_eq!(vp.developmental_maturity(), 0.0);

    vp.advance_maturity_by_progress(0.0);
    assert_eq!(vp.developmental_maturity(), 0.0, "zero improvement → no maturity");

    vp.advance_maturity_by_progress(-0.3);
    assert_eq!(vp.developmental_maturity(), 0.0, "negative improvement → no maturity");

    vp.advance_maturity_by_progress(0.1);
    let m1 = vp.developmental_maturity();
    assert!(m1 > 0.0, "positive improvement must advance the clock, got {m1}");

    // Realistic cumulative improvement (Σ ≈ a competence high-water-mark ≤ 1):
    // monotone-rising, the (1−m) term keeps it below 1.
    for _ in 0..3 {
        vp.advance_maturity_by_progress(0.1);
    }
    let m2 = vp.developmental_maturity();
    assert!(m2 > m1 && m2 < 1.0, "saturating-monotone, got {m2}");

    // Sustained large improvement approaches the ceiling GRADUALLY (the per-tick
    // cap means no single second can clamp to 1.0 — it asymptotes over many
    // steps), and never overflows past 1.0.
    for _ in 0..200 {
        vp.advance_maturity_by_progress(0.5);
    }
    let m3 = vp.developmental_maturity();
    assert!(
        m3 > 0.99 && m3 <= 1.0,
        "sustained improvement asymptotes toward (but is bounded by) 1.0, got {m3}"
    );
}

/// Defense-in-depth: a single pathological improvement (the stale-baseline
/// overshoot that clamped maturity to 1.0 in one tick post-deploy) must NOT
/// saturate the clock — the per-tick cap bounds the step.
#[test]
fn large_improvement_cannot_saturate_in_one_tick() {
    let vp = Viewport::new();
    vp.advance_maturity_by_progress(0.40); // the t=24s overshoot value
    let m = vp.developmental_maturity();
    assert!(
        m > 0.0 && m < 0.1,
        "one tick must not saturate — capped step bounds it well below 1.0, got {m}"
    );
}

/// Regression for the post-deploy t=24s jump: vision closures advance the SHARED
/// maturity throttle cursor ~50×/s and used to starve the vocal-progress baseline
/// refresh, so when efficacy had climbed and vision finally yielded the slot the
/// stale gap clamped maturity to exactly 1.0 in one tick. With an own cursor +
/// high-water-mark baseline + per-tick cap, a burst of vision closures between
/// vocal closures must produce only small, smooth maturity steps.
#[test]
fn vision_burst_cannot_starve_vocal_baseline_into_a_jump() {
    let vp = Arc::new(Viewport::new());
    let mut t = T0;
    let mut prev_m = 0.0f64;
    let mut max_step = 0.0f64;
    let mut mse = 0.9f64;
    for _ in 0..120u64 {
        // a burst of improving vision closures this second (advance the shared cursor)
        for k in 0..40u64 {
            mse *= 0.999;
            vp.observe_visual_reflex_closure(t + k * 1_000, mse, 0.5);
        }
        // then one genuine vocal evidence closure (efficacy keeps climbing)
        voice_and_hear(&vp, t);
        vp.observe_vocal_reflex_closure(t + 60_000, 0.8, true);
        let m = vp.developmental_maturity();
        max_step = max_step.max(m - prev_m);
        prev_m = m;
        t += SEC + 100_000;
    }
    assert!(
        max_step < 0.2,
        "maturity must rise smoothly; a vision-starved stale baseline caused a unit \
         jump (largest single-step Δm was {max_step})"
    );
    assert!(
        vp.developmental_maturity() < 0.999,
        "must not saturate to 1.0, got {}",
        vp.developmental_maturity()
    );
}

/// Core anti-ratchet property: a genuine learner matures during the COMPETENCE
/// RISE, then maturity HOLDS once competence plateaus — it does not keep
/// climbing on flat skill. (Old law: every evidence closure ratcheted m up
/// regardless of whether skill improved.)
#[test]
fn maturity_holds_when_competence_plateaus() {
    let vp = Arc::new(Viewport::new());

    // Phase 1 — rising competence: closures ≥1 s apart so each is a throttle tick.
    let mut t = T0;
    for _ in 0..150u64 {
        voice_and_hear(&vp, t);
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
        t += SEC + 100_000;
    }
    let m_after_rise = vp.developmental_maturity();
    let eff_after_rise = vp.read_learned_bias().vocal_efficacy;
    assert!(
        m_after_rise > 0.1,
        "a genuine learner must mature during the competence rise, got m={m_after_rise}"
    );

    // Phase 2 — plateau: same perfect-match closures, but efficacy has converged
    // so there is little further improvement to credit.
    for _ in 0..150u64 {
        voice_and_hear(&vp, t);
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
        t += SEC + 100_000;
    }
    let m_after_plateau = vp.developmental_maturity();
    let eff_after_plateau = vp.read_learned_bias().vocal_efficacy;

    let rise_gain = m_after_rise; // from 0
    let plateau_gain = m_after_plateau - m_after_rise;
    assert!(
        plateau_gain < rise_gain * 0.5,
        "plateau (Δm={plateau_gain}) must earn far less maturity than the rise \
         (Δm={rise_gain}); efficacy {eff_after_rise:.3}→{eff_after_plateau:.3}. \
         A flat-competence ratchet is the hallucinated-maturity bug."
    );
    assert!(
        m_after_plateau < 0.999,
        "maturity must not saturate to ~1.0 from flat-competence closures, got {m_after_plateau}"
    );
}
