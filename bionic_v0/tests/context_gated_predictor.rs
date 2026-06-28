use std::sync::Arc;
use bionic_v0::predictor::{ContextGatedPredictor, N_CTX, N_TAPS};
use bionic_v0::viewport::Viewport;

fn feed_sequence(vp: &Arc<Viewport>, axis: usize, values: &[f64], start_t: u64, dt: u64) {
    for (i, &v) in values.iter().enumerate() {
        vp.push_observation(axis, start_t + i as u64 * dt, v);
    }
}

fn train_ticks(p: &mut ContextGatedPredictor, vp: &Arc<Viewport>, n: usize) {
    for _ in 0..n {
        p.tick(vp);
    }
}

#[test]
fn t1_rising_sequence_predicts_up() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Train on rising sequence
    for cycle in 0..200 {
        let base = cycle as f64 * 0.01;
        let vals: Vec<f64> = (0..8).map(|i| base + i as f64 * 0.1).collect();
        let t = cycle as u64 * 80_000;
        feed_sequence(&vp, axis, &vals, t, 10_000);
        for _ in 0..8 {
            p.tick(&vp);
        }
    }

    // Predict after a rising sequence
    let t = 200 * 80_000;
    feed_sequence(&vp, axis, &[1.0, 1.1, 1.2, 1.3], t, 10_000);
    p.tick(&vp);

    let ps = vp.predictors[axis].read().unwrap();
    // Context should be 3 (rising+rising)
    assert_eq!(ps.last_ctx, 3, "rising sequence should select ctx=3");
}

#[test]
fn t2_falling_sequence_predicts_down() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Train on falling sequence
    for cycle in 0..200 {
        let base = 2.0 - cycle as f64 * 0.005;
        let vals: Vec<f64> = (0..8).map(|i| base - i as f64 * 0.1).collect();
        let t = cycle as u64 * 80_000;
        feed_sequence(&vp, axis, &vals, t, 10_000);
        for _ in 0..8 {
            p.tick(&vp);
        }
    }

    let t = 200 * 80_000;
    feed_sequence(&vp, axis, &[1.5, 1.4, 1.3, 1.2], t, 10_000);
    p.tick(&vp);

    let ps = vp.predictors[axis].read().unwrap();
    assert_eq!(ps.last_ctx, 0, "falling sequence should select ctx=0");
}

#[test]
fn t3_different_ctx_different_prediction() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Manually set different coefficients for different contexts
    p.coeffs[0] = [-0.5, 0.1, 0.0, 0.0]; // falling ctx: predict decrease
    p.coeffs[3] = [1.5, -0.3, 0.0, 0.0]; // rising ctx: predict increase

    // Feed same x0,x1 but different x2 to get different contexts
    let t = 100_000;
    // Rising context: x=[0.5, 0.4, 0.3, 0.2] → bit_0=(0.5>0.4)=1, bit_1=(0.4>0.3)=1 → ctx=3
    feed_sequence(&vp, axis, &[0.2, 0.3, 0.4, 0.5], t, 10_000);
    p.tick(&vp);
    let pred_rising = vp.latest_prediction(axis);

    // Reset for falling context
    let mut p2 = ContextGatedPredictor::new(axis);
    p2.coeffs[0] = [-0.5, 0.1, 0.0, 0.0];
    p2.coeffs[3] = [1.5, -0.3, 0.0, 0.0];

    let vp2 = Arc::new(Viewport::new());
    let t2 = 100_000;
    // Falling context: x=[0.5, 0.6, 0.7, 0.8] → bit_0=(0.5>0.6)=0, bit_1=(0.6>0.7)=0 → ctx=0
    feed_sequence(&vp2, axis, &[0.8, 0.7, 0.6, 0.5], t2, 10_000);
    p2.tick(&vp2);
    let pred_falling = vp2.latest_prediction(axis);

    assert!(
        pred_rising != pred_falling,
        "different contexts should give different predictions"
    );
}

#[test]
fn t4_insufficient_context_degrades() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Feed only 2 observations
    vp.push_observation(axis, 10_000, 0.5);
    vp.push_observation(axis, 20_000, 0.6);
    p.tick(&vp);

    // Should still produce a prediction (fills missing taps with last value)
    let pred = vp.latest_prediction(axis);
    assert!(pred.is_some(), "should produce prediction even with 2 observations");
}

#[test]
fn t5_learning_only_updates_active_ctx() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Save initial coefficients for ctx=0
    let ctx0_before = p.coeffs[0];

    // Bootstrap surprise so effective LR is non-zero
    vp.set_surprise(axis, 1.0);

    // Feed rising sequence (ctx=3)
    for cycle in 0..50 {
        let t = cycle as u64 * 20_000;
        feed_sequence(&vp, axis, &[0.1 + cycle as f64 * 0.01, 0.2 + cycle as f64 * 0.01, 0.3 + cycle as f64 * 0.01, 0.4 + cycle as f64 * 0.01], t, 10_000);
        p.tick(&vp);
    }

    // ctx=0 coefficients should be unchanged
    for tap in 0..N_TAPS {
        assert!(
            (p.coeffs[0][tap] - ctx0_before[tap]).abs() < 1e-12,
            "ctx=0 tap {} should be unchanged (was {}, now {})",
            tap, ctx0_before[tap], p.coeffs[0][tap],
        );
    }

    // ctx=3 should have changed
    let changed = p.coeffs[3].iter().any(|&c| c.abs() > 1e-8);
    assert!(changed, "ctx=3 coefficients should have been updated");
}

#[test]
fn t6_maturity_annealing() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;

    // At m=0, ar2_lr_scale should be 3.0 (newborn boost)
    let scale_m0 = bionic_v0::ar2_lr_scale(0.0);
    let scale_m1 = bionic_v0::ar2_lr_scale(1.0);
    assert!(
        scale_m0 > scale_m1,
        "newborn scale ({}) should exceed mature scale ({})",
        scale_m0, scale_m1,
    );
}

#[test]
fn t7_convergence_on_sinusoid() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    let dt = 10_000u64;
    let freq = 0.1;

    vp.set_surprise(axis, 1.0);

    for i in 0..200 {
        let t = i as u64 * dt;
        let v = (i as f64 * freq * std::f64::consts::TAU).sin();
        vp.push_observation(axis, t, v);
        p.tick(&vp);
    }

    let initial_mse = {
        let ps = vp.predictors[axis].read().unwrap();
        ps.mean_sq_err
    };

    // Feed 200 more
    for i in 200..400 {
        let t = i as u64 * dt;
        let v = (i as f64 * freq * std::f64::consts::TAU).sin();
        vp.push_observation(axis, t, v);
        p.tick(&vp);
    }

    let final_mse = {
        let ps = vp.predictors[axis].read().unwrap();
        ps.mean_sq_err
    };

    assert!(
        final_mse < initial_mse || initial_mse < 0.01,
        "MSE should decrease over training: initial={}, final={}",
        initial_mse, final_mse,
    );
}

#[test]
fn t8_prior_adds_correctly() {
    let vp = Arc::new(Viewport::new());
    let axis = 0;
    let mut p = ContextGatedPredictor::new(axis);

    // Feed observations
    let t = 100_000;
    feed_sequence(&vp, axis, &[0.0, 0.0, 0.0, 0.0], t, 10_000);
    p.tick(&vp);
    let pred_no_prior = vp.latest_prediction(axis).unwrap_or(0.0);

    // Now set a prior
    let vp2 = Arc::new(Viewport::new());
    let mut p2 = ContextGatedPredictor::new(axis);
    feed_sequence(&vp2, axis, &[0.0, 0.0, 0.0, 0.0], t, 10_000);
    vp2.push_prior(axis, 0.5);
    p2.tick(&vp2);
    let pred_with_prior = vp2.latest_prediction(axis).unwrap_or(0.0);

    let diff = (pred_with_prior - pred_no_prior - 0.5).abs();
    assert!(
        diff < 0.01,
        "prior should add 0.5 to prediction: no_prior={}, with_prior={}, diff_from_expected={}",
        pred_no_prior, pred_with_prior, diff,
    );
}
