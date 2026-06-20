//! Gaze as a first-class drive (D-Step3): the virtual-neck self-motion is now
//! commanded by an emergent gaze drive (curiosity = where visual prediction error
//! is high) with an endogenous maturity-annealed exploratory FLOOR — replacing
//! the old scripted sine. These tests guard the highest-principle constraints:
//! anti-0×x bootstrap-deadlock (the floor never collapses) and coarse→fine.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::virtual_neck::{tick_virtual_neck_into_viewport, VirtualNeck};
use bionic_v0::{gaze_babble_floor, GAZE_FLOOR_MATURE, GAZE_FLOOR_NEWBORN, VIS_REGIONS};

#[test]
fn gaze_floor_is_coarse_to_fine_and_never_zero() {
    assert!((gaze_babble_floor(0.0) - GAZE_FLOOR_NEWBORN).abs() < 1e-12);
    assert!((gaze_babble_floor(1.0) - GAZE_FLOOR_MATURE).abs() < 1e-9);
    assert!(
        gaze_babble_floor(0.0) > gaze_babble_floor(1.0),
        "coarse→fine: the exploratory wobble shrinks as the body matures"
    );
    // Anti-deadlock: the floor is strictly > 0 at EVERY maturity, so the
    // no-motion→no-depth→no-error→no-drive loop can never form.
    for i in 0..=10 {
        let f = gaze_babble_floor(i as f64 / 10.0);
        assert!(f > 0.0, "gaze floor must never be 0 (anti-deadlock); m={i} f={f}");
    }
}

#[test]
fn fresh_body_has_nonzero_gaze_floor_and_centred_bias() {
    let vp = Viewport::new();
    let (bias, floor) = vp.compute_gaze_drive(1_000_000);
    assert!(floor > 0.0, "a newborn must have a nonzero exploratory floor, got {floor}");
    assert!((-1.0..=1.0).contains(&bias), "bias must be bounded, got {bias}");
    // No visual prediction error has accumulated yet → no directed pull.
    assert!(bias.abs() < 1e-9, "no visual evidence → centred gaze, got {bias}");
}

#[test]
fn emergent_gaze_keeps_depth_loop_alive() {
    // Drive the neck ONLY through the emergent gaze command (no scripted sine).
    // The endogenous floor must keep producing parallax so depth becomes
    // observable — the anti-0×x guarantee, exercised end-to-end through the same
    // command path body.rs uses.
    let vp = Arc::new(Viewport::new());
    let mut neck = VirtualNeck::new_default();
    let mut prev_b = [0.0_f64; VIS_REGIONS];
    let mut prev_m = [0.0_f64; VIS_REGIONS];
    let mut motion_ticks = 0usize;
    let mut ever_observable = false;
    for step in 0..400u64 {
        let t = 1_000_000 + step * 33_333;
        let (bias, floor) = vp.compute_gaze_drive(t);
        neck.command_gaze(bias, floor);
        let (nb, nm) = tick_virtual_neck_into_viewport(&vp, &mut neck, &prev_b, &prev_m, t);
        prev_b = nb;
        prev_m = nm;
        if vp.read_efference_copy().translation_term().is_some() {
            motion_ticks += 1;
        }
        if vp
            .read_gaussian_field()
            .gaussians
            .iter()
            .any(|g| g.active && g.depth_observable)
        {
            ever_observable = true;
        }
    }
    assert!(motion_ticks > 0, "emergent gaze (floor) must command observable self-motion");
    assert!(
        ever_observable,
        "emergent gaze must make depth observable — the closed loop must survive on the floor alone"
    );
}
