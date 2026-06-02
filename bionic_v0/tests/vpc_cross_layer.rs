//! B1-1 step2: CrossLayerPredictor (VPC mechanism 5, high-layer half).
//!
//! A cross-layer predictor learns to predict a TARGET axis's error from a
//! sparse set of SOURCE axes (gated by learned relevance — only sources that
//! actually correlate with the target's error get a non-trivial weight). It
//! then writes a prior back to the viewport so the low layer can use it.
//!
//! NO-BP invariants (same as step1): the cross-layer predictor only updates its
//! OWN src_weights from its OWN local error (predicted-vs-actual target err).
//! It writes only to the prior channel — never to the low layer's a/b. Each
//! tick reads a SPARSE source set, not a global matrix.
//!
//! Contracts verified (synthetic two-axis coupling, B leads A):
//!   1. The cross-layer predictor learns: its own err-prediction MSE drops.
//!   2. After learning, it writes a non-zero prior for the target axis.
//!   3. An uncorrelated noise source earns ~zero weight (sparse / noise reject).

use std::sync::Arc;

use bionic_v0::vpc_cross_layer::CrossLayerPredictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;

// Synthetic coupling: source axis B leads, and the LOW-LAYER ERROR on target
// axis A is a function of B's recent value. (We drive A's error stream directly
// here to isolate the cross-layer learning from the AR-2 dynamics.)
fn drive(vp: &Arc<Viewport>, axis_a: usize, axis_b: usize, axis_noise: usize, t: u64, step: u64) {
    // B is a clean oscillation.
    let b = 0.5 + 0.4 * (((step % 8) as f64) / 8.0 - 0.5);
    // A's error is driven by B (the structure the cross-layer should find).
    let a_err = 0.3 * (b - 0.5);
    // Noise axis: unrelated pseudo-random-ish value, no relation to A's err.
    let noise = 0.5 + 0.4 * (((step.wrapping_mul(2654435761) % 7) as f64) / 7.0 - 0.5);

    vp.push_observation(axis_b, t, b);
    vp.push_observation(axis_noise, t, noise);
    // Publish A's low-layer error onto its err_stream (what the high layer reads).
    vp.push_err(axis_a, t, a_err);
}

#[test]
fn cross_layer_learns_and_writes_prior() {
    assert!(VIB_BANDS >= 3, "need >=3 axes for a/b/noise");
    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b, axis_noise) = (0usize, 1usize, 2usize);

    // Cross-layer predictor for target A, candidate sources {B, noise}.
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b, axis_noise]);

    let mut mse_first = None;
    let mut mse_last = 0.0;
    let mut t = 1_000u64;
    for step in 0..400u64 {
        drive(&vp, axis_a, axis_b, axis_noise, t, step);
        clp.tick(&vp);
        let m = clp.err_pred_mse();
        // Sample "early" after warmup + a few cycles so the MSE EMA has settled
        // past its near-zero cold start (cold weights ≈ 0 → tiny initial mse).
        if step == 60 {
            mse_first = Some(m);
        }
        mse_last = m;
        t += 10_000;
    }

    // 1. It learned to predict A's error → its MSE dropped from early to late.
    let mse_early = mse_first.expect("should have early mse");
    assert!(
        mse_last < mse_early,
        "cross-layer err-prediction MSE must drop as it learns: early={mse_early}, last={mse_last}"
    );

    // 2. It writes a non-zero prior for the target axis.
    assert!(
        vp.read_prior(axis_a).abs() > 1e-6,
        "after learning, cross-layer must write a non-zero prior for target axis"
    );

    // 3. Noise source earns far less weight than the real source B.
    let w_b = clp.weight_for(axis_b).abs();
    let w_noise = clp.weight_for(axis_noise).abs();
    assert!(
        w_b > w_noise,
        "real coupled source B must earn more weight than uncorrelated noise: w_b={w_b}, w_noise={w_noise}"
    );
}

#[test]
fn cross_layer_does_not_touch_low_layer_coefficients() {
    // No-BP: the cross-layer predictor must only write the prior channel.
    // The low-layer predictor state (a/b) is never written by the high layer.
    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b, axis_noise) = (0usize, 1usize, 2usize);
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b, axis_noise]);

    let a_before = {
        let ps = vp.predictors[axis_a].read().unwrap();
        (ps.a, ps.b)
    };
    let mut t = 1_000u64;
    for step in 0..100u64 {
        drive(&vp, axis_a, axis_b, axis_noise, t, step);
        clp.tick(&vp);
        t += 10_000;
    }
    let a_after = {
        let ps = vp.predictors[axis_a].read().unwrap();
        (ps.a, ps.b)
    };
    assert_eq!(
        a_before, a_after,
        "cross-layer predictor must NOT write low-layer a/b coefficients (no BP)"
    );
}
