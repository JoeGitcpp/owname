//! 可疑-2 fix (2026-06-22): action_suppression accrues from PREDICTEDNESS (the
//! acoustic competence of a vocalized closure), not a categorical vocalized?-label
//! gate. The old law deposited suppression only on quiet (vocalized==false)
//! closures, so the 13-min call's 5931 all-vocalized closures left
//! action_suppression pinned at 0.000 (a dead column). Now a well-predicted
//! vocalization habituates; a novel/poorly-predicted one does not.
//!
//! Marsland/PNAS habituation as a continuous function of the act's own outcome;
//! Bays/Wolpert sensory attenuation by forward-model prediction.

use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use std::sync::Arc;

const SEC: u64 = 1_000_000;

fn voice_matching(vp: &Arc<Viewport>, t: u64) {
    vp.begin_voicing(t, 0.8, 0.5, 0.5);
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
    let (bands, _) = vp.read_self_vocal_bands();
    let (mel64, _) = vp.read_self_vocal_mel64();
    vp.observe_vocal_target_full(t, 0.8, 0.5, 0.5, 0.5, bands, mel64); // target == heard self → fit≈1
}

/// The exact 5931-all-vocalized condition that produced action_suppression=0:
/// it must now become positive.
#[test]
fn all_vocalized_session_accrues_suppression() {
    let vp = Arc::new(Viewport::new());
    let mut t = SEC;
    for _ in 0..100u64 {
        voice_matching(&vp, t);
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true); // ALL vocalized, never quiet
        t += SEC + 100_000; // >1s so the 1/s suppression throttle deposits
    }
    assert!(
        vp.read_learned_bias().action_suppression > 0.0,
        "an all-vocalized evidence session must accrue suppression (was pinned at 0), got {}",
        vp.read_learned_bias().action_suppression
    );
}

/// Gradedness (not a label): a well-predicted (high-fit) session deposits MORE
/// suppression than a poorly-predicted (low-fit) one.
#[test]
fn suppression_scales_with_predictedness() {
    // Low-fit: self and target are unlike shapes (high shape_dist → low fit → low c).
    let mut low = [0.0f64; VIB_BANDS];
    let mut high = [0.0f64; VIB_BANDS];
    for i in 0..4 {
        low[i] = 1.0;
        high[VIB_BANDS - 1 - i] = 1.0;
    }
    let vp_lo = Arc::new(Viewport::new());
    vp_lo.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, low); // target = low
    let mut t = SEC;
    for _ in 0..100u64 {
        vp_lo.begin_voicing(t, 0.8, 0.5, 0.5);
        *vp_lo.self_vocal_bands.write().unwrap() = (high, t); // heard = unlike
        vp_lo.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
        t += SEC + 100_000;
    }
    let sup_lo = vp_lo.read_learned_bias().action_suppression;

    // High-fit: heard self == target (fit≈1 → high c).
    let vp_hi = Arc::new(Viewport::new());
    vp_hi.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, low); // target = low
    let mut t = SEC;
    for _ in 0..100u64 {
        vp_hi.begin_voicing(t, 0.8, 0.5, 0.5);
        *vp_hi.self_vocal_bands.write().unwrap() = (low, t); // heard == target
        vp_hi.observe_vocal_reflex_closure(t + 1_000, 0.8, true);
        t += SEC + 100_000;
    }
    let sup_hi = vp_hi.read_learned_bias().action_suppression;

    assert!(
        sup_hi > sup_lo,
        "well-predicted (high-fit) closures must deposit more suppression than \
         poorly-predicted ones: high-fit={sup_hi}, low-fit={sup_lo}"
    );
}
