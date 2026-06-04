//! Phase 1 (keystone ①) Stage B integration tests.
//!
//! DoD (c): asserts that the Gaussian field is driven by the adapter-registry
//! path (the same path body.rs uses) and that its outputs actually land on the
//! unified tier-0 axis stream.
//!
//! These tests use the real RealPhotAdapter + SyntheticCameraFrameProvider so
//! they exercise the exact same code path as body.rs — they are NOT isolated
//! unit tests of the Gaussian core.

use std::sync::Arc;
use bionic_v0::{
    GAUSS_RECON_MSE_AXIS, GAUSS_ACTIVE_FRAC_AXIS,
    adapter::{ModalityAdapter, RealPhotAdapter},
    phot_camera::{SyntheticCameraFrameProvider, SynthCamStep},
    viewport::Viewport,
};

/// Helper: build a RealPhotAdapter backed by a synthetic provider and run
/// `n_ticks` ticks against the supplied Viewport.  Returns the adapter so
/// callers can inspect it if needed.
fn drive_adapter(vp: &Arc<Viewport>, steps: Vec<SynthCamStep>, us_per_tick: u64) {
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        steps,
        0,
        us_per_tick,
    ));
    let mut adapter = RealPhotAdapter::new(provider);
    let n = 200u64;   // enough ticks to settle
    for i in 0..n {
        adapter.tick(vp, i * us_per_tick);
    }
}

/// (DoD a+b+c): The adapter is the same object body.rs registers via
/// spawn_registry_driver.  After driving it, the Gaussian summary axes must
/// carry fresh tier-0 observations — proving the body→adapter→tick_gaussian_field
/// →push_observation chain is intact.
#[test]
fn adapter_drives_gaussian_field_and_axes_receive_observations() {
    let vp = Arc::new(Viewport::new());

    // A bright static blob — easy stable scene; Gaussians should spawn.
    let steps: Vec<SynthCamStep> = (0..200)
        .map(|_| SynthCamStep::Blob { cx: 0.5, cy: 0.5, r: 0.2, brightness: 0.8 })
        .collect();

    drive_adapter(&vp, steps, 33_333);

    // GAUSS_RECON_MSE must have an observation.
    let mse_obs = vp.latest_observation(GAUSS_RECON_MSE_AXIS);
    assert!(
        mse_obs.is_some(),
        "GAUSS_RECON_MSE_AXIS must carry an observation after adapter ticks"
    );
    // GAUSS_ACTIVE_FRAC must have an observation.
    let frac_obs = vp.latest_observation(GAUSS_ACTIVE_FRAC_AXIS);
    assert!(
        frac_obs.is_some(),
        "GAUSS_ACTIVE_FRAC_AXIS must carry an observation after adapter ticks"
    );
    // active_frac must be in [0,1].
    let frac = frac_obs.unwrap().value;
    assert!(
        (0.0..=1.0).contains(&frac),
        "GAUSS_ACTIVE_FRAC must be in [0,1], got {frac}"
    );
    // At least one Gaussian must have been born on a blob scene.
    let field = vp.read_gaussian_field();
    assert!(
        field.active_count() >= 1,
        "at least one Gaussian must be active after steady blob input"
    );
}

/// Reconstruction MSE must fall on a stable repeated scene — the field is
/// learning.  Measured on the SAME Viewport: read MSE after the first frame,
/// then after 150 more frames on the same blob, and assert it fell.
#[test]
fn recon_mse_falls_on_stable_scene() {
    use bionic_v0::phot_camera::{CameraFrame, CAM_W, CAM_H};

    let vp = Arc::new(Viewport::new());
    let cx = 0.4f64; let cy = 0.6f64; let r = 0.15f64; let bright = 0.75f64;

    let make_blob_frame = |t_us: u64| -> CameraFrame {
        let mut f = CameraFrame::zeros(t_us);
        for py in 0..CAM_H {
            for px in 0..CAM_W {
                let xn = px as f64 / CAM_W as f64;
                let yn = py as f64 / CAM_H as f64;
                let d2 = (xn-cx).powi(2) + (yn-cy).powi(2);
                let v = (-(0.5*d2)/(r*r)).exp() * bright;
                let i = py*CAM_W+px;
                f.luma[i] = v; f.r[i] = v; f.g[i] = v*0.5; f.b[i] = v*0.2;
            }
        }
        f
    };

    // First frame: field is fresh, MSE should be high.
    vp.tick_gaussian_field(&make_blob_frame(1_000), 1_000);
    let first_mse = vp.latest_observation(GAUSS_RECON_MSE_AXIS)
        .map(|s| s.value)
        .unwrap_or(1.0);

    // 150 more frames of the same blob.
    for i in 1u64..=150 {
        vp.tick_gaussian_field(&make_blob_frame(i * 33_333), i * 33_333);
    }
    let late_mse = vp.latest_observation(GAUSS_RECON_MSE_AXIS)
        .map(|s| s.value)
        .unwrap_or(1.0);

    assert!(
        late_mse < first_mse,
        "reconstruction MSE must fall on a stable scene: first={first_mse:.5} late={late_mse:.5}"
    );
}

/// Encoder-probe signal: after sufficient stable exposure, GAUSS_RECON_MSE
/// must beat the "copy-nothing" baseline (all-black prediction MSE).
#[test]
fn recon_mse_beats_copy_nothing_baseline_after_training() {
    use bionic_v0::phot_camera::CameraFrame;
    use bionic_v0::phot_gaussian::{GaussianField, GAUSS_REC_W, GAUSS_REC_H, GAUSS_PIX};

    let vp = Arc::new(Viewport::new());
    let blob = || SynthCamStep::Blob { cx: 0.5, cy: 0.5, r: 0.18, brightness: 0.85 };
    let steps: Vec<SynthCamStep> = (0..200).map(|_| blob()).collect();
    drive_adapter(&vp, steps, 33_333);

    // Compute the "copy-nothing" (all-zero prediction) baseline MSE for this blob.
    // We do this by building a fresh empty field and ticking it once on the same blob.
    use bionic_v0::phot_camera::{CAM_W, CAM_H, CAM_PIX};
    let mut frame = CameraFrame::zeros(0);
    let cx = 0.5f64; let cy = 0.5f64; let r = 0.18f64; let bright = 0.85f64;
    for py in 0..CAM_H {
        for px in 0..CAM_W {
            let xn = px as f64 / CAM_W as f64;
            let yn = py as f64 / CAM_H as f64;
            let d2 = (xn-cx).powi(2) + (yn-cy).powi(2);
            let v = (-(0.5*d2)/(r*r)).exp() * bright;
            let i = py*CAM_W+px;
            frame.luma[i] = v; frame.r[i] = v; frame.g[i] = v*0.2; frame.b[i] = v*0.2;
        }
    }
    let mut empty = GaussianField::new();
    let baseline = empty.tick_frame(&frame); // first tick on fresh field = copy-nothing MSE

    let trained_mse = vp.latest_observation(GAUSS_RECON_MSE_AXIS)
        .map(|s| s.value)
        .unwrap_or(1.0);

    assert!(
        trained_mse < 0.6 * baseline,
        "trained field MSE ({trained_mse:.5}) must beat 60% of the copy-nothing baseline ({baseline:.5})"
    );
}
