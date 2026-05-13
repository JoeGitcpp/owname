use bionic_v0::evolution_priors::{BiasAxis, BiasPriors};
use bionic_v0::learned_bias::{BiasLocator, LearnedBiasEventInput, LearnedBiasState};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

#[test]
fn learned_updates_do_not_mutate_base_priors() {
    let priors = BiasPriors::default();
    let before = priors.pressure_weights.multiplier;
    let mut state = LearnedBiasState::default();

    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Global,
            requested_delta: 0.9,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: 7,
            event_t_us: 1_000,
        },
    );

    assert_eq!(priors.pressure_weights.multiplier, before);
    assert!(state.curves[0].delta_value > 0.0);
}

#[test]
fn protected_axis_delta_is_clamped_by_protected_cap() {
    let priors = BiasPriors::default();
    let mut state = LearnedBiasState::default();

    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            axis: BiasAxis::PressureMultiplier,
            locator: BiasLocator::Global,
            requested_delta: 0.9,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: 8,
            event_t_us: 2_000,
        },
    );

    let cap = priors.modify_rate_cap_for(BiasAxis::PressureMultiplier);
    assert!(state.curves[0].delta_value.abs() <= cap);
}

#[test]
fn repeated_support_sediments_curve_and_deviation_weakens_it() {
    let priors = BiasPriors::default();
    let mut state = LearnedBiasState::default();
    let input = LearnedBiasEventInput {
        axis: BiasAxis::CrossChannelGain,
        locator: BiasLocator::Global,
        requested_delta: 0.2,
        support: 1.0,
        deviation: 0.0,
        evidence_ref: 9,
        event_t_us: 3_000,
    };

    state.apply_event(&priors, input);
    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            event_t_us: 4_000,
            ..input
        },
    );
    let strengthened = state.curves[0].delta_value;

    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            requested_delta: 0.2,
            support: 0.0,
            deviation: 1.0,
            event_t_us: 5_000,
            ..input
        },
    );

    assert!(state.curves[0].delta_value < strengthened);
    assert!(state.curves[0].deviation_count > 0);
}

#[test]
fn effective_value_is_base_plus_delta_clamped_to_range() {
    let priors = BiasPriors::default();
    let mut state = LearnedBiasState::default();

    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Global,
            requested_delta: 0.2,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: 10,
            event_t_us: 6_000,
        },
    );

    let effective = state.effective_scalar(
        BiasAxis::CrossChannelGain,
        BiasLocator::Global,
        1.25,
        0.0,
        2.0,
    );
    assert!(effective > 1.25);
    assert!(effective <= 2.0);
}

#[test]
fn learned_bias_checkpoint_roundtrip_preserves_numeric_state() {
    let priors = BiasPriors::default();
    let mut state = LearnedBiasState::default();
    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Band(2),
            requested_delta: 0.2,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: 11,
            event_t_us: 7_000,
        },
    );

    let dir = std::env::temp_dir().join(format!("learned-bias-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = bionic_v0::learned_bias::LearnedBiasCheckpointPaths::new(&dir);

    bionic_v0::learned_bias::write_learned_bias_checkpoint(&state, &paths).unwrap();
    let loaded = bionic_v0::learned_bias::read_learned_bias_checkpoint(&paths.final_path).unwrap();

    assert_eq!(loaded.n_curves, 1);
    assert_eq!(loaded.n_recent_events, 1);
    assert_eq!(loaded.total_events, 1);
    assert_eq!(loaded.curves[0].locator, BiasLocator::Band(2));
}

#[test]
fn body_runtime_loads_learned_bias_checkpoint_before_snapshot() {
    use std::process::{Command, Stdio};

    let priors = BiasPriors::default();
    let mut state = LearnedBiasState::default();
    state.apply_event(
        &priors,
        LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Global,
            requested_delta: 0.2,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: 71,
            event_t_us: 1_000,
        },
    );

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_body_load_learned_bias_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let paths = bionic_v0::learned_bias::LearnedBiasCheckpointPaths::new(&dir);
    bionic_v0::learned_bias::write_learned_bias_checkpoint(&state, &paths).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_body"))
        .arg(&dir)
        .arg("2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run body");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    let snap = read_snapshot(&SnapshotPaths::new(&dir).final_path).unwrap();
    assert_eq!(snap.learned_bias.total_events, 1);
    assert_eq!(snap.learned_bias.n_curves, 1);
}

#[test]
fn body_runtime_saves_learned_bias_checkpoint_after_live_byte_training() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_body_save_learned_bias_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let body = Command::new(env!("CARGO_BIN_EXE_body"))
        .arg(&dir)
        .arg("4")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn body");
    let loopback = Command::new(env!("CARGO_BIN_EXE_byte_loopback"))
        .arg(&dir)
        .arg("4")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn byte_loopback");

    std::thread::sleep(Duration::from_millis(300));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("live_bytes").join("in.frames"))
        .unwrap();
    file.write_all(b"abcabc001001\n(()())\n").unwrap();
    drop(file);

    let body_out = body.wait_with_output().expect("wait body");
    let loop_out = loopback.wait_with_output().expect("wait loopback");
    assert!(body_out.status.success());
    assert!(loop_out.status.success());
    assert!(body_out.stdout.is_empty());
    assert!(body_out.stderr.is_empty());
    assert!(loop_out.stdout.is_empty());
    assert!(loop_out.stderr.is_empty());

    let paths = bionic_v0::learned_bias::LearnedBiasCheckpointPaths::new(&dir);
    let saved = bionic_v0::learned_bias::read_learned_bias_checkpoint(&paths.final_path).unwrap();
    assert!(saved.total_events > 0);
    assert!(saved.n_curves > 0);
}

#[test]
fn learned_bias_checkpoint_rejects_unknown_version() {
    let dir = std::env::temp_dir().join(format!("learned-bias-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("learned_bias.bin");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&bionic_v0::learned_bias::LEARNED_BIAS_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&999_u64.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();

    assert!(bionic_v0::learned_bias::read_learned_bias_checkpoint(&path).is_err());
}

#[test]
fn learned_bias_checkpoint_rejects_pre_contraction_axis_encoding_version() {
    assert!(
        bionic_v0::learned_bias::LEARNED_BIAS_VERSION > 1,
        "generic pressure-axis checkpoint encoding must not reuse pre-contraction v1"
    );

    let dir = std::env::temp_dir().join(format!("learned-bias-old-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("learned_bias.bin");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&bionic_v0::learned_bias::LEARNED_BIAS_MAGIC.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();

    assert!(bionic_v0::learned_bias::read_learned_bias_checkpoint(&path).is_err());
}

#[test]
fn viewport_starts_with_empty_learned_bias_state() {
    let vp = Viewport::new();
    let state = vp.read_learned_bias();
    assert_eq!(state.n_curves, 0);
    assert_eq!(state.n_recent_events, 0);
}

#[test]
fn snapshot_exposes_numeric_learned_bias_summary() {
    let vp = Arc::new(Viewport::new());
    vp.apply_learned_bias_event(LearnedBiasEventInput {
        axis: BiasAxis::CrossChannelGain,
        locator: BiasLocator::Global,
        requested_delta: 0.2,
        support: 1.0,
        deviation: 0.0,
        evidence_ref: 12,
        event_t_us: 8_000,
    });

    let dir = std::env::temp_dir().join(format!("learned-bias-snapshot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.learned_bias.n_curves, 1);
    assert_eq!(snap.learned_bias.n_recent_events, 1);
    assert!(snap.learned_bias.max_abs_delta > 0.0);
    assert_eq!(snap.learned_bias.last_update_t_us, 8_000);
}

#[test]
fn learned_bias_body_code_avoids_rejected_ontology_fields() {
    let src = std::fs::read_to_string("src/learned_bias.rs").unwrap();
    for needle in [
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "owner",
        "personality",
        "trust_label",
        "truth_label",
        "source_identity",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(needle),
            "learned_bias.rs must not contain rejected body field or output: {}",
            needle
        );
    }
}

#[test]
fn learned_bias_axes_are_generic_pressure_axes_only() {
    let src = std::fs::read_to_string("src/learned_bias.rs").unwrap();
    for forbidden in [
        "CodeStream",
        "ActionConsequence",
        "VisualPrimitive",
        "CommunicationLoop",
        "CrossModalBinding",
        "StructuralTearing",
        "IrreversibleDisruption",
        "EvidenceOverwritePrevention",
    ] {
        assert!(
            !src.contains(forbidden),
            "learned bias must not sediment domain axis `{}`",
            forbidden
        );
    }
}
