//! Pure-vision developmental maturity: the body earns maturity from SEEING
//! alone (prediction-error reduction on its Gaussian reconstruction), not only
//! from audio-visual co-occurrence.
//!
//! Guards the highest-principle constraints of observe_visual_reflex_closure:
//!   * no fabricated maturity (empty/near-empty field credits nothing);
//!   * no 0×x cold-start trap (credit only MSE *improvement*, not absolute MSE);
//!   * shared 1/s maturity throttle (developmental pace unchanged).

use std::sync::Arc;

use bionic_v0::phot_camera::{CameraFrame, CAM_H, CAM_W};
use bionic_v0::viewport::Viewport;

const SEC: u64 = 1_000_000;

/// An empty/near-empty field (active_frac below the evidence floor) must never
/// advance the developmental clock — that would be fabricated maturity.
#[test]
fn empty_field_earns_no_maturity() {
    let vp = Viewport::new();
    for step in 0..50u64 {
        // active_frac = 0.0 (no structure), even though mse "improves".
        vp.observe_visual_reflex_closure(step * SEC, 0.9 - step as f64 * 0.01, 0.0);
    }
    assert_eq!(
        vp.developmental_maturity(),
        0.0,
        "no visual structure → no maturity (no fabrication)"
    );
}

/// A field with real structure whose reconstruction MSE keeps FALLING must
/// advance maturity above the newborn floor.
#[test]
fn improving_reconstruction_advances_maturity() {
    let vp = Viewport::new();
    // First call only sets the EMA baseline (no prior to improve against).
    vp.observe_visual_reflex_closure(1 * SEC, 0.8, 0.5);
    assert_eq!(vp.developmental_maturity(), 0.0, "baseline call credits nothing");

    // Subsequent calls, each ≥1 s apart, with monotonically falling MSE.
    let mut mse = 0.8;
    for step in 2..12u64 {
        mse *= 0.8; // each frame the body reconstructs better
        vp.observe_visual_reflex_closure(step * SEC, mse, 0.5);
    }
    assert!(
        vp.developmental_maturity() > 0.0,
        "falling recon-MSE with real structure must mature the body, got {}",
        vp.developmental_maturity()
    );
}

/// A perfectly stable scene (constant MSE = no improvement) must NOT advance
/// maturity, even though the absolute MSE is low — crediting absolute MSE would
/// be the 0×x cold-start trap. Only prediction-error REDUCTION earns credit.
#[test]
fn stable_scene_does_not_fabricate_maturity() {
    let vp = Viewport::new();
    vp.observe_visual_reflex_closure(1 * SEC, 0.05, 0.5); // baseline, low MSE
    for step in 2..30u64 {
        // constant low MSE — no improvement
        vp.observe_visual_reflex_closure(step * SEC, 0.05, 0.5);
    }
    assert_eq!(
        vp.developmental_maturity(),
        0.0,
        "stable (non-improving) scene must not advance maturity even at low MSE"
    );
}

/// The shared 1/s throttle: two improving closures less than a second apart may
/// advance the clock at most once.
#[test]
fn maturity_throttled_to_one_per_second() {
    let vp = Viewport::new();
    vp.observe_visual_reflex_closure(10 * SEC, 0.8, 0.5); // baseline
    vp.observe_visual_reflex_closure(11 * SEC, 0.4, 0.5); // advance #1
    let after_one = vp.developmental_maturity();
    assert!(after_one > 0.0, "first improving closure should advance");

    // Three more improving closures within the SAME second → all throttled out.
    vp.observe_visual_reflex_closure(11 * SEC + 100_000, 0.2, 0.5);
    vp.observe_visual_reflex_closure(11 * SEC + 300_000, 0.1, 0.5);
    vp.observe_visual_reflex_closure(11 * SEC + 900_000, 0.05, 0.5);
    assert_eq!(
        vp.developmental_maturity(),
        after_one,
        "sub-second improving closures must not advance the clock again"
    );
}

/// End-to-end through the live visual pipeline: feeding a static bright blob
/// into tick_gaussian_field repeatedly (≥1 s apart) makes the field learn to
/// reconstruct it (falling MSE), which must mature the body — proving the hook
/// is wired and vision earns maturity with NO audio and NO co-occurrence.
#[test]
fn live_gaussian_field_path_matures_the_body() {
    let vp = Arc::new(Viewport::new());
    let frame = central_blob_frame();
    for step in 0..30u64 {
        // ≥1 s apart so the shared throttle admits the early improving frames.
        vp.tick_gaussian_field(&frame, 1 * SEC + step * 1_100_000);
    }
    assert!(
        vp.read_gaussian_field().active_count() >= 1,
        "a bright blob must birth Gaussians (else there is no structure to credit)"
    );
    assert!(
        vp.developmental_maturity() > 0.0,
        "learning to reconstruct a seen scene must mature the body (vision-only), got {}",
        vp.developmental_maturity()
    );
}

/// The treadmill fix (2026-06-22): a live, ever-changing scene must NOT keep
/// advancing maturity. The old law credited every frame whose error beat the
/// recent baseline — adaptation-to-change endlessly mistaken for cumulative skill,
/// which saturated the global clock over a long call. The high-water mark credits
/// only a genuinely-new-best sustained reconstruction.
#[test]
fn changing_scene_does_not_treadmill_maturity() {
    let vp = Viewport::new();
    let mut t = 0u64;
    vp.observe_visual_reflex_closure(t, 0.8, 0.5); // seed the baseline high
    // Phase 1: sustained low recon error → the baseline drops to a new all-time
    // best → genuine cumulative visual skill → maturity rises.
    for _ in 0..150 {
        t += SEC;
        vp.observe_visual_reflex_closure(t, 0.1, 0.5);
    }
    let m_learned = vp.developmental_maturity();
    assert!(m_learned > 0.0, "sustained recon improvement must mature, got {m_learned}");
    // Phase 2: a CHANGING scene whose error oscillates ABOVE the best — per-frame
    // down-swings the old law credited endlessly, but none beats the all-time best,
    // so the high-water mark earns nothing → maturity holds (no treadmill).
    for k in 0..120u64 {
        t += SEC;
        let osc = if k % 2 == 0 { 0.5 } else { 0.35 }; // both > the ~0.1 best
        vp.observe_visual_reflex_closure(t, osc, 0.5);
    }
    let m_churn = vp.developmental_maturity();
    assert!(
        m_churn - m_learned < 0.02,
        "a changing scene that never beats the all-time-best reconstruction must NOT \
         keep maturing the body (treadmill): learned={m_learned}, after churn={m_churn}"
    );
}

fn central_blob_frame() -> CameraFrame {
    let mut f = CameraFrame::zeros(0);
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
