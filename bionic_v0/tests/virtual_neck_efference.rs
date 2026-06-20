// Virtual neck + efference copy + action-consequence acceptance tests.
//
// Per-Gaussian depth-from-parallax (Phase 2) is tested elsewhere:
//   * phot_gaussian unit tests — the local rule (converges to d_obs/tt,
//     near>far ρ, still→ρ unchanged & unobservable);
//   * tests/gaussian_depth_body.rs — the body pipeline (self-motion→observable,
//     stillness→unobservable).
// This file keeps: projection-parallax geometry, efference motion-prior
// injection, and NeckTranslation action-consequence pairing.

use std::sync::Arc;

use bionic_v0::action_consequence::ConsequenceWindowPhase;
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::efference_copy::EfferenceCopy;
use bionic_v0::virtual_neck::{NeckCurriculum, VirtualNeck, NECK_FOCAL_PX, NECK_WOBBLE_PERIOD};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIS_REGIONS;

// ─────────────────────────────────────────────────────────────────────────────
// ① Parallax geometry: near object displaced more than far object.
// ─────────────────────────────────────────────────────────────────────────────

/// For a lateral translation tx the pixel displacement of a scene object is
///   Δpx = tx · focal_px / z_world
/// So a near object (small z) must show a LARGER displacement than a far one.
#[test]
fn near_object_displaced_more_than_far() {
    let z_near = 1.0;
    let z_far = 4.0;
    let tx = 0.05; // 5 % of frame width

    let neck = VirtualNeck {
        scene: vec![],
        curriculum: NeckCurriculum::default(),
        tick_count: 0,
        wobble_period: NECK_WOBBLE_PERIOD,
        focal_px: NECK_FOCAL_PX,
        cam_x_prev: 0.0,
        commanded_bias: 0.0,
        commanded_floor: bionic_v0::GAZE_FLOOR_NEWBORN,
    };

    let (px0_near, _) = neck.project(0.0, 0.0, z_near, 0.0);
    let (px1_near, _) = neck.project(0.0, 0.0, z_near, tx);
    let disp_near = (px1_near - px0_near).abs();

    let (px0_far, _) = neck.project(0.0, 0.0, z_far, 0.0);
    let (px1_far, _) = neck.project(0.0, 0.0, z_far, tx);
    let disp_far = (px1_far - px0_far).abs();

    assert!(
        disp_near > disp_far,
        "near (z={z_near}) disp={disp_near:.4} must exceed far (z={z_far}) disp={disp_far:.4}"
    );

    // Parallax law: disp_near / disp_far ≈ z_far / z_near.
    let expected_ratio = z_far / z_near;
    let actual_ratio = disp_near / disp_far;
    assert!(
        (actual_ratio - expected_ratio).abs() < 1e-6,
        "parallax ratio actual={actual_ratio:.4} expected={expected_ratio:.4}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Efference motion-prior injection (action-conditioned prediction).
// ─────────────────────────────────────────────────────────────────────────────

/// When a translation is active, inject_efference_motion_priors writes non-zero
/// priors on VIS_SUB_MOTION axes (with an empty Gaussian field ρ defaults to the
/// neutral 1.0, so the expected prior is tt·1.0). When still, priors are 0.
#[test]
fn efference_prior_injection_motion_vs_still() {
    let vp = Arc::new(Viewport::new());

    // Moving camera: tt = 0.05, neutral ρ = 1.0 → expected prior = 0.05.
    vp.push_efference_copy(EfferenceCopy { tx: 0.05, ty: 0.0, tz: 0.0, t_us: 1000 });
    vp.inject_efference_motion_priors();

    for region in 0..VIS_REGIONS {
        let prior = vp.read_prior(bionic_v0::vis_axis(region, bionic_v0::VIS_SUB_MOTION));
        let expected = 0.05 * 1.0; // tt * neutral ρ (no active Gaussians yet)
        assert!(
            (prior - expected).abs() < 1e-9,
            "region {region}: prior {prior:.6} expected {expected:.6}"
        );
    }

    // Still camera: all motion priors must be 0.
    vp.push_efference_copy(EfferenceCopy { tx: 0.0, ty: 0.0, tz: 0.0, t_us: 2000 });
    vp.inject_efference_motion_priors();

    for region in 0..VIS_REGIONS {
        let prior = vp.read_prior(bionic_v0::vis_axis(region, bionic_v0::VIS_SUB_MOTION));
        assert_eq!(prior, 0.0, "region {region}: still camera must produce zero motion prior");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ③ Action-consequence pairing for NeckTranslation.
// ─────────────────────────────────────────────────────────────────────────────

/// A NeckTranslation Begin event followed by visual-ingress accumulation must
/// yield a temporal pair within TEMPORAL_PAIR_MAX_DELAY_US.
#[test]
fn neck_translation_action_consequence_pair_formed() {
    let vp = Arc::new(Viewport::new());

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1_000,
    });
    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 1, "one NeckTranslation epoch expected");
    let epoch = &ao.write_epoch_history[0];
    let write_seq = epoch.write_seq;

    // Visual consequence arriving 50 ms later.
    let consequence_t = 50_000_u64;
    vp.accumulate_ingress_change(consequence_t, 0.25);

    // Close the consequence window via quiescence (gap > 30 ms).
    vp.tick_ingress_accumulation(100_000, 30_000);

    let ac = vp.read_action_consequence();
    assert!(
        ac.windows.iter().any(|w| w.phase == ConsequenceWindowPhase::Closed),
        "consequence window should be closed after quiescence"
    );

    let paired = vp.try_auto_pair_latest_epoch_to_window(100_000);
    assert!(paired, "temporal pair must be formed");

    let ac2 = vp.read_action_consequence();
    assert_eq!(ac2.n_pairs, 1, "exactly one pair expected");
    let pair = &ac2.pairs[0];
    assert_eq!(pair.write_seq, write_seq, "pair must reference the NeckTranslation epoch");
    assert!(
        pair.delay_us < bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US,
        "pair delay {} µs must be within max {}",
        pair.delay_us,
        bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US
    );
}

/// Multiple NeckTranslation events paired with their visual consequences.
#[test]
fn multiple_translation_pairs_form_correctly() {
    let vp = Arc::new(Viewport::new());

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000,
    });
    vp.accumulate_ingress_change(40_000, 0.2);
    vp.tick_ingress_accumulation(80_000, 30_000);
    let paired1 = vp.try_auto_pair_latest_epoch_to_window(80_000);
    assert!(paired1, "first pair must form");

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 100_000,
    });
    vp.accumulate_ingress_change(150_000, 0.3);
    vp.tick_ingress_accumulation(200_000, 30_000);
    let paired2 = vp.try_auto_pair_latest_epoch_to_window(200_000);
    assert!(paired2, "second pair must form");

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_pairs, 2, "two pairs expected");
    let seqs: Vec<u64> = ac.pairs.iter().map(|p| p.write_seq).collect();
    assert_ne!(seqs[0], seqs[1], "pairs must reference different write epochs");
}
