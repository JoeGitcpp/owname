//! 阶段1 (病根修复): vision flows into the SHARED perception axes, the same
//! substrate hearing uses — not a per-slot shadow.
//!
//! Verifies: after a camera frame is processed, the shared observation axes in
//! the visual band [AXIS_VIS_BASE .. AXIS_VIS_BASE+VIS_AXES) carry real values
//! (brightness/RGB/motion per coarse region). Before this redesign these axes
//! did not exist and vision never reached the shared coordinate system.

use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter};
use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::viewport::Viewport;
use bionic_v0::{vis_axis, AXIS_VIS_BASE, VIB_BANDS, VIS_AXES, VIS_REGIONS, VIS_SUB_BRIGHTNESS};

#[test]
fn vision_reaches_shared_observation_axes() {
    let vp = Arc::new(Viewport::new());

    // A bright full-field-ish blob, then keep it, so regions light up.
    let mut script = vec![SynthCamStep::Idle];
    for _ in 0..4 {
        script.push(SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.6,
            brightness: 0.95,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let mut t = 1_000_000u64;
    for _ in 0..5 {
        adapter.tick(&vp, t);
        t += 33_333;
    }

    // The visual band of the shared axes must now carry observations.
    let mut any_visual = false;
    for region in 0..VIS_REGIONS {
        let axis = vis_axis(region, VIS_SUB_BRIGHTNESS);
        if let Some(obs) = vp.latest_observation(axis) {
            if obs.valid && obs.value > 0.0 {
                any_visual = true;
            }
        }
    }
    assert!(
        any_visual,
        "vision must reach the SHARED observation axes (some region brightness > 0)"
    );

    // Sanity: hearing axes [0..VIB_BANDS) and vision axes are in ONE coordinate
    // system — the visual band sits right after the hearing band.
    assert_eq!(AXIS_VIS_BASE, VIB_BANDS, "vision axes follow hearing axes in one space");
    assert!(VIS_AXES > 0);
}
