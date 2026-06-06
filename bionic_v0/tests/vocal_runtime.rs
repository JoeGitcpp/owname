use bionic_v0::action_output::ActionOutputPhase;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_adapter::sample_vocal_waveform;
use bionic_v0::{MOD_VIB, VIB_BANDS};
use std::sync::Arc;

#[test]
fn snapshot_roundtrip_preserves_vocal_substrate_state() {
    let dir = std::env::temp_dir().join(format!("bionic_v0_vocal_snap_test_{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("bionic_v0_vocal_observer_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.9, 0.3, 0.5, 0.2);
    vp.tick_vocal_imitation_loop(2_000);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();
    assert!(snap.vocal_imitation.imitation_pressure >= 0.0);
}

#[test]
fn observer_renders_vocal_snapshot_as_numeric_trace() {
    let observer_src = include_str!("../src/bin/observer.rs");
    assert!(
        observer_src.contains("voc:tE"),
        "observer must expose vocal target/organ/imitation trace for live tuning"
    );
    assert!(observer_src.contains("op{:.3}"));
    assert!(observer_src.contains("ob{:.3}"));
    assert!(observer_src.contains("or{:.3}"));
    assert!(observer_src.contains("ip{:.3}"));

    let lowered = observer_src.to_ascii_lowercase();
    for forbidden in ["tts", "speak", "word", "meaning", "sentence"] {
        assert!(
            !lowered.contains(forbidden),
            "observer vocal trace must not introduce text ontology: {forbidden}"
        );
    }
}

#[test]
fn vocal_output_path_uses_cpal_stream_and_avoids_text_or_stdout_diagnostics() {
    let src = include_str!("../src/vocal_adapter.rs");
    assert!(src.contains("cpal::default_host"));
    assert!(src.contains("default_output_device"));
    assert!(src.contains("default_output_config"));
    assert!(src.contains("build_output_stream"));
    assert!(src.contains("SampleFormat::F32"));
    assert!(src.contains("SampleFormat::I16"));
    assert!(src.contains("SampleFormat::U16"));
    assert!(src.contains(".play()"));

    let lowered = src.to_ascii_lowercase();
    assert!(!lowered.contains("text-to-speech"));
    assert!(!lowered.contains("tts"));
    assert!(!src.contains("println!"));
    assert!(!src.contains("eprintln!"));
}

#[test]
fn begin_voicing_records_action_output_epoch_for_vocal_write_out() {
    let vp = Arc::new(Viewport::new());

    vp.begin_voicing(1_000, 0.6, 0.4, 0.7);

    let out = vp.read_action_output();
    assert_eq!(out.n_channels, 1);
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Active);
    assert_eq!(out.write_epoch_history[0].start_t_us, 1_000);
}

#[test]
fn vocal_imitation_loop_records_action_epoch_when_target_starts_voicing() {
    let vp = Arc::new(Viewport::new());

    vp.observe_vocal_target(1_000, 0.8, 0.35, 0.60, 0.20);
    vp.tick_vocal_imitation_loop(2_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.write_epoch_history[0].start_t_us, 2_000);
}

/// Tile a 4-band pattern across the 16 VIB bands (M1.5 widened VIB_BANDS 4→16).
fn vib16(p: [f64; 4]) -> [f64; VIB_BANDS] {
    std::array::from_fn(|i| p[i % 4])
}

fn push_vib_frame(vp: &Arc<Viewport>, t_us: u64, bands: [f64; VIB_BANDS]) {
    for axis in 0..VIB_BANDS {
        vp.push_observation(axis, t_us, bands[axis]);
    }
    let energy = bands.iter().map(|b| b * b).sum::<f64>();
    vp.write_ambient_for(MOD_VIB, energy, 0.05, Some(t_us), true);
}

#[test]
fn vib_ingress_populates_vocal_target_when_no_recent_body_write_exists() {
    let vp = Arc::new(Viewport::new());

    push_vib_frame(&vp, 10_000, vib16([0.05, 0.12, 0.34, 0.18]));
    vp.tick_vocal_target_from_vib(10_000);

    let target = vp.read_vocal_target();
    assert!(target.active_energy > 0.01);
    assert_eq!(target.last_update_t_us, 10_000);
    assert!(target.pitch_hint > 0.0);
    assert!(target.resonance_hint > 0.0);
}

#[test]
fn recent_body_write_coupled_vib_does_not_overwrite_vocal_target() {
    let vp = Arc::new(Viewport::new());

    vp.begin_voicing(20_000, 0.7, 0.3, 0.4);
    push_vib_frame(&vp, 20_050, vib16([0.06, 0.15, 0.28, 0.16]));
    vp.tick_vocal_target_from_vib(20_050);

    let target = vp.read_vocal_target();
    assert_eq!(target.last_update_t_us, 0);
    assert_eq!(target.active_energy, 0.0);
}
