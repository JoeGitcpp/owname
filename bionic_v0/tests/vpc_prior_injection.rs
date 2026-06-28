//! B1-1 step1: prior injection channel (VPC mechanism 5, low-layer half).
//!
//! A high (cross-layer) predictor writes a `prior` to the viewport. The
//! low-layer AR-2 predictor reads it and adds it to its prediction as an INPUT.
//!
//! WHAT "NO BACKPROP" ACTUALLY MEANS HERE (corrected 2026-05-30):
//! The prior is part of the low layer's *world/input*, like glasses that make
//! the world clearer. The low layer only ever: reads its current world (incl.
//! prior) → predicts → sees its own local error → adjusts its own a/b. The high
//! layer NEVER reaches into the low layer's coefficients. The prior lowering the
//! low layer's error IS the point (frozen doc 机制5: "prior 写回 → 低层利用高阶
//! 信息 → 间接降低 err"). So "prior must not change the coefficients" is the
//! WRONG test (it would forbid the very mechanism). The real no-BP invariants
//! are: (a) no cross-layer chain回传 — the high layer writes only to the prior
//! channel, never to a/b; (b) each predictor updates only from its own local
//! error; (c) a correct prior actually reduces the low layer's error.
//!
//! Contracts verified:
//!   1. prior channel defaults to 0 (no prior = unchanged behavior).
//!   2. injecting a prior shifts the published prediction by that prior.
//!   3. NO chain回传: tick() only writes the predicted ring / a,b via local
//!      update — the prior channel is read-only to the low layer (it never
//!      writes its own prior; only an external high layer does).
//!   4. a correct prior reduces realized error vs no prior (机制5 目的).

use std::sync::Arc;

use bionic_v0::predictor::ContextGatedPredictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::AXIS_VIB;

fn push_obs(vp: &Arc<Viewport>, axis: usize, t_us: u64, v: f64) {
    vp.push_observation(axis, t_us, v);
}

#[test]
fn prior_defaults_to_zero() {
    let vp = Arc::new(Viewport::new());
    assert_eq!(
        vp.read_prior(AXIS_VIB),
        0.0,
        "prior channel must default to 0 (no prior)"
    );
}

#[test]
fn injected_prior_shifts_prediction() {
    let vp = Arc::new(Viewport::new());
    let axis = AXIS_VIB;

    // Feed a flat signal so the bare prediction is ~0 (a,b start at 0).
    let mut t = 1_000u64;
    push_obs(&vp, axis, t, 0.0);
    t += 10_000;
    push_obs(&vp, axis, t, 0.0);

    let mut p = ContextGatedPredictor::new(axis);
    p.tick(&vp);
    let pred_no_prior = vp.latest_prediction(axis).unwrap_or(f64::NAN);

    vp.push_prior(axis, 0.3);
    t += 10_000;
    push_obs(&vp, axis, t, 0.0);
    t += 10_000;
    push_obs(&vp, axis, t, 0.0);
    let mut p2 = ContextGatedPredictor::new(axis);
    p2.tick(&vp);
    let pred_with_prior = vp.latest_prediction(axis).unwrap_or(f64::NAN);

    assert!(
        (pred_with_prior - pred_no_prior - 0.3).abs() < 1e-9,
        "prediction must shift by the injected prior: no_prior={pred_no_prior}, with_prior={pred_with_prior}"
    );
}

#[test]
fn low_layer_never_writes_its_own_prior() {
    // No-BP invariant (a): the low layer must NOT write the prior channel.
    // Only an external high layer writes priors. So after many low-layer ticks
    // with no high layer present, the prior stays exactly where it started.
    let vp = Arc::new(Viewport::new());
    let axis = AXIS_VIB;
    let mut p = ContextGatedPredictor::new(axis);

    let standing = 0.15;
    vp.push_prior(axis, standing); // pretend a high layer set it once

    let mut t = 1_000u64;
    for k in 0..50u64 {
        let v = 0.1 + 0.05 * ((k % 7) as f64);
        push_obs(&vp, axis, t, v);
        p.tick(&vp);
        t += 10_000;
    }

    // The low layer adapted its a/b, but it never touched the prior channel.
    assert_eq!(
        vp.read_prior(axis),
        standing,
        "low layer must never write the prior channel (no chain回传); only a high layer does"
    );
}

#[test]
fn correct_prior_reduces_error() {
    // 机制5 目的: a prior that points the prediction the right way must lower
    // the low layer's realized error vs no prior. We use a signal with a
    // constant offset the bare AR-2 (a,b from 0) is slow to capture; a prior
    // equal to that offset should immediately cut the first-step error.
    let offset = 0.5;

    // --- no prior ---
    let vp0 = Arc::new(Viewport::new());
    let axis = AXIS_VIB;
    let mut p0 = ContextGatedPredictor::new(axis);
    let mut t = 1_000u64;
    push_obs(&vp0, axis, t, offset);
    t += 10_000;
    push_obs(&vp0, axis, t, offset);
    p0.tick(&vp0); // publishes a prediction for the next step
    t += 10_000;
    push_obs(&vp0, axis, t, offset); // the value the prediction targets
    p0.tick(&vp0); // resolves → pushes err
    let err_no_prior = vp0.latest_err(axis).map(f64::abs).unwrap_or(f64::NAN);

    // --- with a prior equal to the offset ---
    let vp1 = Arc::new(Viewport::new());
    let mut p1 = ContextGatedPredictor::new(axis);
    vp1.push_prior(axis, offset);
    let mut t1 = 1_000u64;
    push_obs(&vp1, axis, t1, offset);
    t1 += 10_000;
    push_obs(&vp1, axis, t1, offset);
    p1.tick(&vp1);
    t1 += 10_000;
    push_obs(&vp1, axis, t1, offset);
    p1.tick(&vp1);
    let err_with_prior = vp1.latest_err(axis).map(f64::abs).unwrap_or(f64::NAN);

    assert!(
        err_with_prior < err_no_prior,
        "a correct prior must reduce realized error (机制5 目的): no_prior={err_no_prior}, with_prior={err_with_prior}"
    );
}
