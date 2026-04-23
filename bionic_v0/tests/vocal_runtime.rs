use std::sync::Arc;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_adapter::sample_vocal_waveform;

#[test]
fn snapshot_roundtrip_preserves_vocal_substrate_state() {
    let dir = std::env::temp_dir().join("bionic_v0_vocal_snap_test");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.7, 0.3, 0.4, 0.2);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();
    assert!(snap.vocal_target.active_energy > 0.0);
}

#[test]
fn vocal_adapter_converts_organ_state_into_nonzero_audio_samples() {
    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(1_000, 0.6, 0.4, 0.7);
    let samples = sample_vocal_waveform(&vp, 128);
    assert!(samples.iter().any(|v: &f64| v.abs() > 0.0));
}

#[test]
fn repeated_target_exposure_drives_organ_toward_target_and_strengthens_memory() {
    let vp = Arc::new(Viewport::new());
    for t in [1_000, 2_000, 3_000, 4_000, 5_000] {
        vp.observe_vocal_target(t, 0.8, 0.35, 0.60, 0.20);
        vp.tick_vocal_imitation_loop(t);
    }
    let organ = vp.read_vocal_organ();
    let mem = vp.read_vocal_memory();
    assert!(organ.breath_drive > 0.0);
    assert!(!mem.prototypes.is_empty());
}

#[test]
fn observer_visible_snapshot_contains_vocal_activity_fields() {
    let dir = std::env::temp_dir().join("bionic_v0_vocal_observer_test");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.9, 0.3, 0.5, 0.2);
    vp.tick_vocal_imitation_loop(2_000);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();
    assert!(snap.vocal_imitation.imitation_pressure >= 0.0);
}