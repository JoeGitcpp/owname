#[test]
fn learned_bias_probe_prints_requested_numeric_curve() {
    use bionic_v0::evolution_priors::{BiasAxis, BiasPriors};
    use bionic_v0::learned_bias::{
        write_learned_bias_checkpoint, BiasLocator, LearnedBiasCheckpointPaths,
        LearnedBiasEventInput, LearnedBiasState,
    };
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_learned_bias_probe_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut state = LearnedBiasState::default();
    let priors = BiasPriors::default();
    for i in 0..3_u64 {
        state.apply_event(
            &priors,
            LearnedBiasEventInput {
                axis: BiasAxis::CrossChannelGain,
                locator: BiasLocator::Channel(3),
                requested_delta: 0.05,
                support: 1.0,
                deviation: 0.0,
                evidence_ref: i + 1,
                event_t_us: 1_000 + i,
            },
        );
    }
    write_learned_bias_checkpoint(&state, &LearnedBiasCheckpointPaths::new(&dir)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_learned_bias_probe"))
        .arg(dir.join("learned_bias.bin"))
        .arg("CrossChannelGain")
        .arg("Channel")
        .arg("3")
        .output()
        .expect("run learned_bias_probe");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("axis=CrossChannelGain"));
    assert!(stdout.contains("locator=Channel(3)"));
    assert!(stdout.contains("delta_value=0.150000"));
    assert!(stdout.contains("stability=0.300000"));
    assert!(stdout.contains("support_count=3"));
    assert!(stdout.contains("deviation_count=0"));
    assert!(stdout.contains("effective_scalar=1.150000"));
    assert!(stdout.contains("ema_effective_scalar=1.015000"));
}
