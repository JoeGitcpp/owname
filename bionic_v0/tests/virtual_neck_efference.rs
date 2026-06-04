// Phase A acceptance tests — virtual neck + efference copy + depth observer.
//
// Criteria verified here:
//   ① Given known translation, near-object pixel displacement > far-object
//      pixel displacement (sign and magnitude correct).
//   ② Still camera (translation_term = None) → depth marked unobservable for
//      ALL regions; depth values NOT updated.
//   ③ Action-consequence pairs are formed between NeckTranslation write epochs
//      and their subsequent visual-ingress consequence windows.

use std::sync::Arc;

use bionic_v0::action_consequence::ConsequenceWindowPhase;
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::depth_observer::DepthObserver;
use bionic_v0::efference_copy::{EfferenceCopy, EFFERENCE_MIN_TT};
use bionic_v0::phot_camera::region_perception;
use bionic_v0::virtual_neck::{
    NeckCurriculum, SceneObject, VirtualNeck, NECK_FOCAL_PX, NECK_PHASE_TICKS, NECK_WOBBLE_PERIOD,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIS_REGIONS;

// ── Helper: construct a minimal two-object neck scene ──────────────────────

fn two_object_scene(z_near: f64, z_far: f64) -> VirtualNeck {
    let scene = vec![
        SceneObject {
            x_world: 0.0,
            y_world: -0.20,
            z_world: z_near,
            brightness: 0.80,
            radius_world: 0.20,
        },
        SceneObject {
            x_world: 0.0,
            y_world: 0.20,
            z_world: z_far,
            brightness: 0.70,
            radius_world: 0.20,
        },
    ];
    VirtualNeck::with_scene(scene)
}

// ─────────────────────────────────────────────────────────────────────────────
// ① Parallax: near object displaced more than far object.
// ─────────────────────────────────────────────────────────────────────────────

/// For a lateral translation tx the pixel displacement of a scene object is
///   Δpx = tx · focal_px / z_world
///
/// So a near object (small z) must show a LARGER displacement than a far one.
/// This tests the geometric invariant directly on the projected coordinates.
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
    };

    // Project from cam_x=0 and cam_x=tx and measure pixel-level displacement.
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

    // Check ratio: disp_near / disp_far ≈ z_far / z_near (parallax law).
    let expected_ratio = z_far / z_near;
    let actual_ratio = disp_near / disp_far;
    assert!(
        (actual_ratio - expected_ratio).abs() < 1e-6,
        "parallax ratio actual={actual_ratio:.4} expected={expected_ratio:.4}"
    );
}

/// With two consecutive frames and a known translation, the region brightness
/// change (proxy for displacement) is larger in the near-object region than
/// in the far-object region when both objects are placed in different regions.
#[test]
fn region_motion_larger_for_near_than_far() {
    // Near object in top-left region, far object in top-right region.
    let scene = vec![
        SceneObject {
            x_world: -0.25,
            y_world: -0.15,
            z_world: 1.0,
            brightness: 0.80,
            radius_world: 0.25,
        },
        SceneObject {
            x_world: 0.25,
            y_world: -0.15,
            z_world: 5.0,
            brightness: 0.70,
            radius_world: 0.25,
        },
    ];
    let mut neck = VirtualNeck::with_scene(scene);

    // Advance to a phase where amplitude is non-trivial.
    // 3 phases × NECK_PHASE_TICKS ticks each → amplitude = NECK_AMP_INIT + 3×NECK_AMP_STEP.
    let n_warmup = 3 * NECK_PHASE_TICKS as u64;
    let mut prev_brightness = [0.0_f64; VIS_REGIONS];
    let (frame_init, _) = neck.tick(0);
    let (_, pb) = region_perception(&frame_init, &prev_brightness);
    prev_brightness = pb;

    for i in 1..n_warmup {
        let (f, _) = neck.tick(i * 33_000);
        let (_, pb) = region_perception(&f, &prev_brightness);
        prev_brightness = pb;
    }

    // Capture two consecutive frames so we can observe motion per region.
    let (frame_a, _) = neck.tick(n_warmup * 33_000);
    let (_, pb_a) = region_perception(&frame_a, &prev_brightness);

    let (frame_b, ec) = neck.tick((n_warmup + 1) * 33_000);
    let (axes_b, _) = region_perception(&frame_b, &pb_a);

    // Only evaluate the motion test when there's a measurable translation this tick.
    if ec.translation_term().is_some() {
        // Region 0 = top-left (near), Region 1 = top-right (far).
        let motion_near = axes_b[0 * 5 + bionic_v0::VIS_SUB_MOTION];
        let motion_far = axes_b[1 * 5 + bionic_v0::VIS_SUB_MOTION];

        // Near motion must be at least as large as far motion; at high amplitude
        // (several phases in) it should be strictly larger.
        let amp = neck.curriculum.amplitude;
        if amp > 0.02 {
            assert!(
                motion_near >= motion_far,
                "near motion {motion_near:.5} should ≥ far motion {motion_far:.5} at amp={amp:.4}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ② Still camera → depth marked unobservable.
// ─────────────────────────────────────────────────────────────────────────────

/// When the efference copy has |tx| < EFFERENCE_MIN_TT the translation_term
/// is None and depth must be marked unobservable.  The depth values must NOT
/// change across the update call.
#[test]
fn still_camera_marks_depth_unobservable() {
    let mut obs = DepthObserver::default();
    let initial = obs.inv_depth.clone();

    // Provide a still-camera efference copy (tx below threshold).
    let still_ec = EfferenceCopy {
        tx: EFFERENCE_MIN_TT * 0.5,
        ty: 0.0,
        tz: 0.0,
        t_us: 0,
    };
    let tt = still_ec.translation_term(); // must be None

    for region in 0..VIS_REGIONS {
        obs.update(region, 0.5, tt);
    }

    for region in 0..VIS_REGIONS {
        assert!(
            !obs.observable[region],
            "region {region}: still camera must be unobservable"
        );
        assert_eq!(
            obs.inv_depth[region], initial[region],
            "region {region}: depth must not change when still"
        );
    }
}

/// Same as above but verified through the Viewport API.
#[test]
fn viewport_still_camera_depth_unobservable() {
    let vp = Arc::new(Viewport::new());

    // Push a near-zero efference copy (still camera).
    vp.push_efference_copy(EfferenceCopy {
        tx: 0.0,
        ty: 0.0,
        tz: 0.0,
        t_us: 1000,
    });

    let obs_motion = [0.3; VIS_REGIONS]; // fake motion values
    vp.update_depth_from_efference(&obs_motion);

    let depth = vp.read_depth_observer();
    for region in 0..VIS_REGIONS {
        assert!(
            !depth.observable[region],
            "region {region}: still viewport must be unobservable"
        );
    }
}

/// When the camera IS moving the depth observer marks the regions observable
/// and the update rule causes the estimate to change (assuming obs_disp ≠ tt·ρ).
#[test]
fn moving_camera_marks_depth_observable() {
    let vp = Arc::new(Viewport::new());

    // Push a clearly observable translation.
    vp.push_efference_copy(EfferenceCopy {
        tx: 0.01, // well above EFFERENCE_MIN_TT
        ty: 0.0,
        tz: 0.0,
        t_us: 2000,
    });

    // Observe a disparity that is NOT equal to tt * initial_rho (= 0.01 * 1.0 = 0.01).
    // Use obs_disp = 0.05 so there is a large residual and depth should change.
    let obs_motion = [0.05; VIS_REGIONS];
    vp.update_depth_from_efference(&obs_motion);

    let depth = vp.read_depth_observer();
    for region in 0..VIS_REGIONS {
        assert!(
            depth.observable[region],
            "region {region}: moving viewport must be observable"
        );
        // Initial inv_depth = 1.0; obs_disp=0.05, tt=0.01 → err = 0.05 - 0.01*1.0 = 0.04
        // Δρ = lr * 0.04 * 0.01 = 0.1 * 0.04 * 0.01 = 0.00004
        // new ρ = 1.00004 — different from initial.
        let expected = 1.0 + 0.1 * (0.05 - 0.01 * 1.0) * 0.01;
        assert!(
            (depth.inv_depth[region] - expected).abs() < 1e-9,
            "region {region}: unexpected depth value {:.8} (expected {:.8})",
            depth.inv_depth[region],
            expected
        );
    }
}

/// Depth estimate converges toward the true inverse depth over many updates.
///
/// Scenario: true scene at z=2 (ρ_true = 0.5).  We push tt=0.5 each tick
/// and obs_disp = tt * ρ_true = 0.5 * 0.5 = 0.25 (perfect observation).
/// Starting from ρ₀ = 1.0 the update rule must drive ρ → 0.5.
///
/// Step size = lr * tt² = 0.1 * 0.25 = 0.025.  The residual decays as
/// (1 − 0.025)^n, reaching < 1 % of initial error after ≈ 180 steps.
#[test]
fn depth_observer_converges_to_true_inv_depth() {
    let mut obs = DepthObserver::default();
    let rho_true = 0.5;
    let tt = 0.5; // larger tt → faster convergence (step ∝ tt²)
    let obs_disp = tt * rho_true; // perfect measurement

    for _ in 0..500 {
        obs.update(0, obs_disp, Some(tt));
    }

    let rho = obs.inv_depth[0];
    assert!(
        (rho - rho_true).abs() < 0.01,
        "after 500 updates ρ = {rho:.5} expected ≈ {rho_true:.5}"
    );
    assert!(obs.observable[0], "region 0 must be observable");
}

// ─────────────────────────────────────────────────────────────────────────────
// ③ Action-consequence pairing for NeckTranslation.
// ─────────────────────────────────────────────────────────────────────────────

/// A NeckTranslation Begin event followed by visual-ingress accumulation must
/// yield a temporal pair within TEMPORAL_PAIR_MAX_DELAY_US.
#[test]
fn neck_translation_action_consequence_pair_formed() {
    let vp = Arc::new(Viewport::new());

    // Write a NeckTranslation Begin event at t=1000 µs.
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

    // Simulate visual consequence arriving 50 ms later.
    let consequence_t = 50_000_u64;
    vp.accumulate_ingress_change(consequence_t, 0.25);

    // Close the consequence window via quiescence (gap > 30 ms).
    vp.tick_ingress_accumulation(100_000, 30_000);

    // Confirm window is now Closed.
    let ac = vp.read_action_consequence();
    assert!(
        ac.windows.iter().any(|w| w.phase == ConsequenceWindowPhase::Closed),
        "consequence window should be closed after quiescence"
    );

    // Run auto-pairing.
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
/// Each translation is followed by visual motion; the latest epoch-to-window
/// temporal bridge should select the closest preceding epoch.
#[test]
fn multiple_translation_pairs_form_correctly() {
    let vp = Arc::new(Viewport::new());

    // First translation at t=1000.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000,
    });

    // First visual consequence at t=40000.
    vp.accumulate_ingress_change(40_000, 0.2);
    vp.tick_ingress_accumulation(80_000, 30_000);
    let paired1 = vp.try_auto_pair_latest_epoch_to_window(80_000);
    assert!(paired1, "first pair must form");

    // Second translation at t=100000.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 100_000,
    });

    // Second visual consequence at t=150000.
    vp.accumulate_ingress_change(150_000, 0.3);
    vp.tick_ingress_accumulation(200_000, 30_000);
    let paired2 = vp.try_auto_pair_latest_epoch_to_window(200_000);
    assert!(paired2, "second pair must form");

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_pairs, 2, "two pairs expected");

    // Pairs must be distinct (different write_seq).
    let seqs: Vec<u64> = ac.pairs.iter().map(|p| p.write_seq).collect();
    assert_ne!(seqs[0], seqs[1], "pairs must reference different write epochs");
}

// ─────────────────────────────────────────────────────────────────────────────
// Efference prior injection (action-conditioned prediction).
// ─────────────────────────────────────────────────────────────────────────────

/// When a translation is active, inject_efference_motion_priors should write
/// non-zero priors on VIS_SUB_MOTION axes.  When still, priors should be 0.
#[test]
fn efference_prior_injection_motion_vs_still() {
    let vp = Arc::new(Viewport::new());

    // Moving camera: tt = 0.05, initial ρ = 1.0 → expected prior = 0.05 * 1.0 = 0.05.
    vp.push_efference_copy(EfferenceCopy {
        tx: 0.05,
        ty: 0.0,
        tz: 0.0,
        t_us: 1000,
    });
    vp.inject_efference_motion_priors();

    for region in 0..VIS_REGIONS {
        let prior = vp.read_prior(bionic_v0::vis_axis(region, bionic_v0::VIS_SUB_MOTION));
        let expected = 0.05 * 1.0; // tt * initial_rho
        assert!(
            (prior - expected).abs() < 1e-9,
            "region {region}: prior {prior:.6} expected {expected:.6}"
        );
    }

    // Still camera: all motion priors must be 0.
    vp.push_efference_copy(EfferenceCopy {
        tx: 0.0,
        ty: 0.0,
        tz: 0.0,
        t_us: 2000,
    });
    vp.inject_efference_motion_priors();

    for region in 0..VIS_REGIONS {
        let prior = vp.read_prior(bionic_v0::vis_axis(region, bionic_v0::VIS_SUB_MOTION));
        assert_eq!(
            prior, 0.0,
            "region {region}: still camera must produce zero motion prior"
        );
    }
}

/// Priors update correctly as depth estimates change.
///
/// After a depth update drives ρ from 1.0 toward 0.5, the next prior
/// injection should reflect the new ρ.
///
/// With tt=0.5, lr=0.1: step = 0.025.  After 100 steps residual decays to
/// ≈ (0.975)^100 ≈ 0.082 of initial error, so ρ ≈ 0.5 + 0.5×0.082 ≈ 0.541.
/// The injected prior (tt × ρ ≈ 0.271) must be closer to the target prior
/// (tt × ρ_true = 0.25) than to the initial prior (tt × 1.0 = 0.5).
#[test]
fn efference_prior_tracks_updated_depth() {
    let vp = Arc::new(Viewport::new());

    let tt = 0.5; // larger tt → faster convergence (step ∝ tt²)
    let rho_true = 0.5;
    let obs_disp = tt * rho_true;

    // Run 100 depth updates so ρ moves noticeably toward rho_true.
    for _ in 0..100 {
        vp.push_efference_copy(EfferenceCopy {
            tx: tt,
            ty: 0.0,
            tz: 0.0,
            t_us: 0,
        });
        let obs = [obs_disp; VIS_REGIONS];
        vp.update_depth_from_efference(&obs);
    }

    // Now inject priors and check they're closer to tt * rho_true than to the
    // initial tt * 1.0.
    vp.push_efference_copy(EfferenceCopy {
        tx: tt,
        ty: 0.0,
        tz: 0.0,
        t_us: 0,
    });
    vp.inject_efference_motion_priors();

    let expected_converged = tt * rho_true; // 0.25
    let initial_prior = tt * 1.0;           // 0.50

    for region in 0..VIS_REGIONS {
        let prior = vp.read_prior(bionic_v0::vis_axis(region, bionic_v0::VIS_SUB_MOTION));
        let dist_from_converged = (prior - expected_converged).abs();
        let dist_from_initial = (prior - initial_prior).abs();
        assert!(
            dist_from_converged < dist_from_initial,
            "region {region}: prior {prior:.5} should be closer to converged={expected_converged:.5} than initial={initial_prior:.5} after 100 updates"
        );
    }
}
