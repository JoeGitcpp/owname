//! D-fix (2026-06-22): the vocal closure evidence gate requires FRESH
//! efference-copy self-hearing (the body voiced within the 150ms DIVA window),
//! not merely a non-silent latched self-bands buffer. The old presence gate
//! (self_bands.sum()>1e-6) passed ~100% of closures — including the dense pair
//! path firing during silent moments — rubber-stamping credit (the 13-min call
//! had closures == closures_with_evidence on every row).
//!
//! Brown/Friston active inference: self-generated feedback is precision-weighted
//! by confidence, never counted at full strength merely because it is present.

use bionic_v0::viewport::Viewport;
use std::sync::Arc;

const T0: u64 = 1_000_000;

fn voice_and_hear(vp: &Arc<Viewport>, t: u64) {
    vp.begin_voicing(t, 0.8, 0.5, 0.5);
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
    let (bands, _) = vp.read_self_vocal_bands();
    let (mel64, _) = vp.read_self_vocal_mel64();
    vp.observe_vocal_target_full(t, 0.8, 0.5, 0.5, 0.5, bands, mel64);
}

/// A closure within 150ms of self-voicing carries evidence and credits competence.
#[test]
fn fresh_self_hearing_credits_competence() {
    let vp = Arc::new(Viewport::new());
    for i in 0..30u64 {
        let t = T0 + i * 300_000;
        voice_and_hear(&vp, t);
        vp.observe_vocal_reflex_closure(t + 1_000, 0.8, true); // 1ms after voicing → fresh
    }
    assert!(
        vp.read_learned_bias().vocal_efficacy > 0.0,
        "fresh self-hearing closures must credit competence"
    );
}

/// A closure long AFTER voicing — with the self-bands buffer still non-silent
/// (latched) — must carry NO evidence. This is the exact leak the old presence
/// gate allowed: the pair path firing in a silent moment off a stale buffer.
#[test]
fn stale_self_hearing_credits_nothing() {
    let vp = Arc::new(Viewport::new());
    // Voice once to fill the self-bands buffer (so it is NON-silent thereafter).
    voice_and_hear(&vp, T0);
    let (bands, _) = vp.read_self_vocal_bands();
    assert!(
        bands.iter().sum::<f64>() > 1e-6,
        "precondition: the latched self-bands buffer is non-silent (old gate would pass)"
    );

    // Now fire closures 500ms+ after voicing, never re-voicing → stale.
    for i in 0..30u64 {
        let t = T0 + 500_000 + i * 300_000; // ≥500ms since the single voicing
        vp.observe_vocal_reflex_closure(t, 0.8, true);
    }
    assert_eq!(
        vp.read_learned_bias().vocal_efficacy,
        0.0,
        "stale self-hearing (no voicing within 150ms) must credit NO competence, \
         even though the latched buffer is non-silent"
    );
    assert_eq!(
        vp.developmental_maturity(),
        0.0,
        "stale closures must not advance the developmental clock"
    );
}
