//! B1-2: learning-progress gate = noise rejection.
//!
//! The CrossLayerPredictor tracks whether it is actually LEARNING to predict
//! the target's error (its err_pred_mse going DOWN over time = learning
//! progress). It gates the prior it writes by that progress:
//!   - target error has predictable structure → mse drops → progress > 0 →
//!     prior is written normally.
//!   - target error is pure noise → mse stays high & flat → progress ≈ 0 →
//!     prior gate → 0 → it stops writing a prior (actively rejects the noise,
//!     not just weight≈0 but no prior at all).
//!
//! This is the "blank/learning-progress drive" and "noise rejection" the design
//! requires (治暗室 + 治噪声成瘾). Still fully compliant: progress is computed
//! from the predictor's OWN local mse; it never touches the low layer.
//!
//! Note on the two signals (confirmed by probing):
//!   - `learning_progress` (d(mse)/dt downward) is the DRIVE signal: positive
//!     WHILE learning, but → 0 once converged. So at end-state both a mastered
//!     target and a noise target show progress ≈ 0 — progress alone cannot
//!     distinguish them.
//!   - the PRIOR GATE is the noise discriminator: explained-fraction
//!     (1 - mse/var). ≈1 for a learnable (now-mastered) target → prior written;
//!     ≈0 for noise (residual mse ≈ the error's own variance) → prior gated off.
//!
//! Contracts:
//!   1. A learnable target is actually learned (residual mse becomes small) and
//!      gets a non-zero prior (gate open).
//!   2. A pure-noise target is gated off (near-zero prior) because its residual
//!      mse stays ≈ its variance (nothing explained).

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::vpc_cross_layer::CrossLayerPredictor;
use bionic_v0::VIB_BANDS;

// Deterministic pseudo-noise in [0.1, 0.9], no relation to `step`'s structure.
fn pseudo_noise(seed: u64) -> f64 {
    let h = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    0.1 + 0.8 * (((h >> 33) % 1000) as f64 / 1000.0)
}

#[test]
fn learnable_target_has_progress_and_writes_prior() {
    assert!(VIB_BANDS >= 2);
    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b) = (0usize, 1usize);
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b]);

    let mut t = 1_000u64;
    for step in 0..400u64 {
        let b = 0.5 + 0.4 * (((step % 8) as f64) / 8.0 - 0.5);
        let a_err = 0.3 * (b - 0.5); // A's error is a clean function of B
        vp.push_observation(axis_b, t, b);
        vp.push_err(axis_a, t, a_err);
        clp.tick(&vp);
        t += 10_000;
    }

    // It actually learned the structure → residual error-prediction mse small.
    assert!(
        clp.err_pred_mse() < 0.005,
        "learnable target must be learned (small residual mse): {}",
        clp.err_pred_mse()
    );
    // Gate open → a real prior is written.
    assert!(
        vp.read_prior(axis_a).abs() > 1e-6,
        "learnable target must get a non-zero prior (gate open)"
    );
}

#[test]
fn pure_noise_target_is_gated_off() {
    assert!(VIB_BANDS >= 2);
    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b) = (0usize, 1usize);
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b]);

    let mut t = 1_000u64;
    for step in 0..400u64 {
        // Source B is a clean signal, BUT A's error is pure noise unrelated to B.
        let b = 0.5 + 0.4 * (((step % 8) as f64) / 8.0 - 0.5);
        let a_err = pseudo_noise(step) - 0.5; // unpredictable from B
        vp.push_observation(axis_b, t, b);
        vp.push_err(axis_a, t, a_err);
        clp.tick(&vp);
        t += 10_000;
    }

    // It cannot learn to predict noise → progress stays ~0 → prior gated off.
    assert!(
        clp.learning_progress() < 0.01,
        "pure-noise target must show ~zero learning progress: {}",
        clp.learning_progress()
    );
    assert!(
        vp.read_prior(axis_a).abs() < 0.05,
        "pure-noise target must be gated off (near-zero prior): {}",
        vp.read_prior(axis_a)
    );
}
