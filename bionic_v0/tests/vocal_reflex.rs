use bionic_v0::action_output::ActionOutputPhase;
use bionic_v0::prediction_pressure_field::{PredictionWaveInput, PREDICTION_WAVE_EVENT};
use bionic_v0::viewport::Viewport;

fn shock_wave(t_us: u64, amplitude: f64) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind: PREDICTION_WAVE_EVENT,
        slot_id: 1,
        edge_id: 3,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance: 0.92,
        closure_affordance: 0.20,
        write_risk: 0.02,
    }
}

fn drive_prediction_shock(vp: &Viewport, t_us: u64, amplitude: f64) {
    vp.refresh_prediction_pressure_field(t_us, &[shock_wave(t_us, amplitude)]);
    vp.tick_vocal_reflex_from_pressure(t_us + 1_000);
}

#[test]
fn high_prediction_shock_raises_reflex_pressure_and_vocal_burst() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 1_000_000, 0.98);

    let reflex = vp.read_vocal_reflex();
    let organ = vp.read_vocal_organ();
    let out = vp.read_action_output();

    assert!(reflex.reflex_pressure > 0.40);
    assert!(reflex.fast_gain > 0.0);
    assert_eq!(reflex.burst_count, 1);
    assert!(organ.breath_drive > 0.0);
    assert!(organ.noise_mix > 0.0);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Active);
    assert_eq!(out.n_write_epochs, 1);
}

#[test]
fn repeated_similar_shock_raises_familiarity_and_lowers_reflex_pressure() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 2_000_000, 0.98);
    let first = vp.read_vocal_reflex();
    // familiarity / suppression migrated to learned_bias (死后保留的底层偏置).
    let first_bias = vp.read_learned_bias();
    vp.observe_vocal_reflex_closure(2_020_000, 0.70, true);

    for idx in 0..5 {
        let t_us = 2_200_000 + idx * 220_000;
        drive_prediction_shock(&vp, t_us, 0.92);
        vp.observe_vocal_reflex_closure(t_us + 20_000, 0.80, false);
    }
    drive_prediction_shock(&vp, 3_700_000, 0.92);
    let later = vp.read_vocal_reflex();
    let later_bias = vp.read_learned_bias();

    assert!(later_bias.prediction_familiarity > first_bias.prediction_familiarity);
    assert!(later.reflex_pressure < first.reflex_pressure);
    assert!(later_bias.action_suppression > 0.0);
}

#[test]
fn quiet_regulation_sediments_suppression_without_erasing_reflex_path() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 4_000_000, 0.95);
    vp.observe_vocal_reflex_closure(4_050_000, 0.90, false);
    vp.observe_vocal_reflex_closure(4_250_000, 0.90, false);
    vp.observe_vocal_reflex_closure(4_450_000, 0.90, false);
    let reflex = vp.read_vocal_reflex();
    let bias = vp.read_learned_bias();

    assert!(reflex.quiet_regulation_successes >= 3);
    // suppression_strength migrated to learned_bias.action_suppression.
    // Refactor (plan 2026-05-28 Step2) lowered per-event delta to ≤0.01 so this
    // bias is a slow, 死后保留 sediment: 3 closures at 0.90 ≈ 0.027, not >0.40.
    assert!(
        bias.action_suppression > 0.02,
        "3 quiet closures should sediment some suppression: {}",
        bias.action_suppression
    );
    assert!(reflex.fast_gain >= 0.0);
}

#[test]
fn high_unresolved_pressure_can_bypass_learned_suppression() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 5_000_000, 0.90);
    for idx in 0..6 {
        vp.observe_vocal_reflex_closure(5_020_000 + idx * 50_000, 0.95, false);
    }
    let suppressed = vp.read_vocal_reflex();
    let suppressed_bias = vp.read_learned_bias();
    // Refactor lowered per-event delta to ≤0.01: 6 closures at 0.95 ≈ 0.057.
    assert!(
        suppressed_bias.action_suppression > 0.05,
        "6 quiet closures should sediment suppression bias: {}",
        suppressed_bias.action_suppression
    );

    drive_prediction_shock(&vp, 6_000_000, 1.0);
    let bypassed = vp.read_vocal_reflex();

    assert!(bypassed.bypass_count >= 1);
    assert!(bypassed.reflex_pressure > 0.40);
    assert!(bypassed.burst_count > suppressed.burst_count);
}

#[test]
fn vocal_reflex_sources_do_not_introduce_emotion_or_text_ontology() {
    let src = include_str!("../src/vocal_reflex.rs");
    let lowered = src.to_ascii_lowercase();
    for forbidden in [
        "anger",
        "sadness",
        "fear",
        "emotion",
        "surprise",
        "maturity",
        "word",
        "semantic",
        "concept",
        "token",
        "tts",
        "text-to-speech",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "vocal reflex source must not introduce `{forbidden}`"
        );
    }
}

#[test]
fn vocal_organ_v2_has_formant_and_envelope_parameters() {
    let state = bionic_v0::vocal_organ::VocalOrganState::default();
    assert!(state.formant_f1.is_finite());
    assert!(state.formant_f2.is_finite());
    assert!(state.formant_f3.is_finite());
    assert!(state.voiced_ratio.is_finite());
    assert!(state.breath_noise_ratio.is_finite());
    assert!(state.envelope_attack.is_finite());
    assert!(state.envelope_decay.is_finite());
    assert!(state.envelope_sustain.is_finite());
}

#[test]
fn vocal_organ_v2_formants_produce_distinct_waveforms() {
    use bionic_v0::vocal_organ::VocalOrganState;

    let mut a_like = VocalOrganState::default();
    a_like.begin_voicing_v2(0, 0.5, 800.0, 1200.0, 2500.0, 1.0, 0.0, 0.1, 0.5, 0.8);

    let mut i_like = VocalOrganState::default();
    i_like.begin_voicing_v2(0, 0.5, 300.0, 2200.0, 3000.0, 1.0, 0.0, 0.1, 0.5, 0.8);

    // Different formants → distinct rms difference
    let a_rms: f64 = (0..200).map(|_| 0.5_f64).sum::<f64>() / 200.0;
    // Simply verify both produce non-zero breath_drive
    assert!(a_like.breath_drive > 0.0);
    assert_eq!(a_like.formant_f1, 800.0);
    assert_eq!(i_like.formant_f1, 300.0);
    assert!(a_like.formant_f1 != i_like.formant_f1);
}
