//! Phase 2 body-level integration: per-Gaussian depth-from-parallax driven
//! through the live viewport pipeline.
//!
//! DoD (from 2026-06-04 remediation plan, Phase 2):
//!   (a) the body driver actually drives depth;
//!   (c) self-motion → depth observable; stillness → unobservable.
//!
//! True metric depth on the passive real camera is impossible (no efference);
//! the HONEST source of self-motion is the body's own commanded virtual neck.

use std::sync::Arc;

use bionic_v0::phot_camera::{CameraFrame, CAM_H, CAM_W};
use bionic_v0::viewport::Viewport;
use bionic_v0::virtual_neck::{tick_virtual_neck_into_viewport, VirtualNeck, NECK_AMP_MAX};
use bionic_v0::VIS_REGIONS;

/// A static bright disc, used to birth Gaussians without any motion.
fn central_blob_frame(t_us: u64) -> CameraFrame {
    let mut f = CameraFrame::zeros(t_us);
    let (cx, cy) = (CAM_W as f64 / 2.0, CAM_H as f64 / 2.0);
    let r2 = (CAM_H as f64 * 0.2).powi(2);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let d2 = (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2);
            let v = if d2 <= r2 { 0.9 } else { 0.0 };
            let i = y * CAM_W + x;
            f.luma[i] = v;
            f.r[i] = v;
            f.g[i] = v;
            f.b[i] = v;
        }
    }
    f
}

/// Self-motion (the body's commanded virtual-neck wobble) must make at least
/// one Gaussian's depth observable, driven entirely through the body pipeline.
#[test]
fn body_driver_makes_depth_observable_under_self_motion() {
    let vp = Arc::new(Viewport::new());
    let mut neck = VirtualNeck::new_default();
    // Command a clearly-observable lateral amplitude (skip the slow curriculum
    // warm-up — we are testing the wiring, not the amplitude schedule).
    neck.curriculum.amplitude = NECK_AMP_MAX;

    let mut prev = [0.0_f64; VIS_REGIONS];
    let mut motion_ticks = 0usize;
    let mut ever_observable = false;
    for step in 0..400u64 {
        prev = tick_virtual_neck_into_viewport(&vp, &mut neck, &prev, 1_000_000 + step * 33_333);
        if vp.read_efference_copy().translation_term().is_some() {
            motion_ticks += 1;
        }
        let f = vp.read_gaussian_field();
        if f.gaussians.iter().any(|g| g.active && g.depth_observable) {
            ever_observable = true;
        }
    }

    assert!(motion_ticks > 0, "the virtual neck must command observable self-motion");
    let field = vp.read_gaussian_field();
    assert!(field.active_count() >= 1, "the neck scene must birth Gaussians");
    assert!(
        ever_observable,
        "self-motion must make >=1 Gaussian's depth observable in the body loop"
    );
}

/// With no commanded self-motion (default efference tx=0 → translation_term
/// None), depth must stay unobservable for every active Gaussian — world-motion
/// or a static scene must never masquerade as self-motion.
#[test]
fn stillness_leaves_depth_unobservable() {
    let vp = Arc::new(Viewport::new());
    let frame = central_blob_frame(1_000_000);
    for step in 0..30u64 {
        // tick_gaussian_field reads the viewport's efference; we never push one,
        // so tt = None on every tick.
        vp.tick_gaussian_field(&frame, 1_000_000 + step * 33_333);
    }
    let field = vp.read_gaussian_field();
    assert!(field.active_count() >= 1, "a bright blob must birth at least one Gaussian");
    assert!(
        field.gaussians.iter().filter(|g| g.active).all(|g| !g.depth_observable),
        "with no self-motion (tt=None), no Gaussian's depth may be observable"
    );
}
