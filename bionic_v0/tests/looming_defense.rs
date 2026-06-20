//! Born-in perception→defense link, end-to-end through the real vision pipeline.
//!
//! Until 2026-06-20 the body's dread (vital_anxiety) could only be raised by its
//! own metabolism — nothing the camera saw could make it afraid. These tests
//! drive the REAL pipeline (Viewport + RealPhotAdapter) and assert that:
//!   1. an APPROACHING (looming) stimulus lights up VIS_LOOMING_AXIS, and
//!   2. that looming raises vital_anxiety via the innate defensive reflex,
//!   3. while a STATIC scene (steady once established) does neither.
//!
//! This is the negative-valence counterpart to the already-wired positive path
//! (perceived familiar → INTERO_ATTACHMENT/PRESENCE).

use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter};
use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIS_LOOMING_AXIS;

const DT_US: u64 = 33_333; // ~30 Hz
/// Frames allowed for the looming EMA baseline to settle on the starting scene
/// before we start measuring. The detector reports CHANGE vs this baseline, so a
/// fair static-vs-approach contrast must let the baseline catch up first.
const WARMUP: u64 = 4;

fn read_axis(vp: &Arc<Viewport>, axis: usize) -> f64 {
    vp.latest_observation(axis).map(|s| s.value).unwrap_or(0.0)
}

/// Run a camera script through the real adapter; return (peak looming after
/// warmup, final dread).
fn run(script: Vec<SynthCamStep>) -> (f64, f64) {
    let n = script.len() as u64;
    let vp = Arc::new(Viewport::new());
    let mut adapter = RealPhotAdapter::new(Box::new(SyntheticCameraFrameProvider::new(
        script, 0, DT_US,
    )));
    let mut peak_looming = 0.0f64;
    for tick in 0..n {
        adapter.tick(&vp, 1_000_000 + tick * DT_US);
        if tick >= WARMUP {
            peak_looming = peak_looming.max(read_axis(&vp, VIS_LOOMING_AXIS));
        }
    }
    let dread = vp.read_vital_core().vital_anxiety;
    (peak_looming, dread)
}

/// A blob held at constant size from the first frame (steady scene, no approach).
fn static_blob_script() -> Vec<SynthCamStep> {
    (0..14)
        .map(|_| SynthCamStep::Blob { cx: 0.5, cy: 0.5, r: 0.25, brightness: 0.95 })
        .collect()
}

/// A blob that holds far, then grows toward the camera (approach) after warmup.
fn approach_script() -> Vec<SynthCamStep> {
    let mut s: Vec<SynthCamStep> = (0..WARMUP)
        .map(|_| SynthCamStep::Blob { cx: 0.5, cy: 0.5, r: 0.05, brightness: 0.95 })
        .collect();
    let steps = 9usize;
    let (r_min, r_max) = (0.05, 0.45);
    for i in 0..steps {
        let r = r_min + (r_max - r_min) * i as f64 / (steps - 1) as f64;
        s.push(SynthCamStep::Blob { cx: 0.5, cy: 0.5, r, brightness: 0.95 });
    }
    s
}

#[test]
fn approaching_object_looms_and_raises_dread() {
    let (peak_looming, dread) = run(approach_script());
    assert!(peak_looming > 0.05, "an approaching object must light up VIS_LOOMING_AXIS: {peak_looming}");
    assert!(dread > 0.0, "looming must raise vital_anxiety via the innate reflex: {dread}");
}

#[test]
fn static_scene_does_not_loom_or_scare() {
    let (peak_looming, dread) = run(static_blob_script());
    assert!(peak_looming < 0.05, "a steady scene must not read as looming: {peak_looming}");
    assert!(dread < 1e-6, "a steady scene must not raise dread: {dread}");
}

#[test]
fn approach_scares_more_than_static() {
    let approach = run(approach_script()).1;
    let stat = run(static_blob_script()).1;
    assert!(
        approach > stat,
        "an approaching object must be more frightening than a steady one: approach={approach} static={stat}"
    );
}
