use bionic_v0::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_EVENT, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RELATION,
    PREDICTION_WAVE_RESIDUE,
};
use bionic_v0::viewport::Viewport;

fn wave(
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    amplitude: f64,
    resonance: f64,
    closure_affordance: f64,
    t_us: u64,
) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind,
        slot_id,
        edge_id,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance,
        closure_affordance,
        write_risk: 0.05,
    }
}

#[test]
fn prediction_pressure_field_keeps_many_waves_but_one_dominant_focus() {
    let vp = Viewport::new();
    vp.refresh_prediction_pressure_field(
        1_000_000,
        &[
            wave(PREDICTION_WAVE_GAP, 1, -1, 0.35, 0.25, 0.60, 1_000_000),
            wave(PREDICTION_WAVE_RELATION, 1, 2, 0.48, 0.45, 0.65, 1_000_000),
            wave(PREDICTION_WAVE_RESIDUE, 2, 3, 0.42, 0.55, 0.55, 1_000_000),
        ],
    );

    let field = vp.read_prediction_pressure_field();
    assert!(field.waves_alive >= 3);
    assert_eq!(field.dominant_focus_is_single(), 1);
    assert!(field.dominant_wave_index >= 0);
    assert!(field.dominant_pressure > 0.0);
}

#[test]
fn prediction_pressure_field_inertia_bounds_focus_switching() {
    let vp = Viewport::new();
    vp.refresh_prediction_pressure_field(
        2_000_000,
        &[wave(
            PREDICTION_WAVE_GAP,
            1,
            -1,
            0.70,
            0.30,
            0.70,
            2_000_000,
        )],
    );
    let first = vp.read_prediction_pressure_field();

    for step in 1..4 {
        vp.refresh_prediction_pressure_field(
            2_000_000 + step * 10_000,
            &[wave(
                PREDICTION_WAVE_EVENT,
                2,
                -1,
                0.66,
                0.30,
                0.70,
                2_000_000 + step * 10_000,
            )],
        );
    }
    let after = vp.read_prediction_pressure_field();

    assert_eq!(after.dominant_wave_index, first.dominant_wave_index);
    assert_eq!(after.focus_switches_bounded(3), 1);
}

#[test]
fn prediction_pressure_field_expresses_quiet_reentry_event_override_and_resonance_spike() {
    let vp = Viewport::new();
    vp.refresh_prediction_pressure_field(
        3_000_000,
        &[wave(
            PREDICTION_WAVE_RESIDUE,
            1,
            5,
            0.30,
            0.35,
            0.50,
            3_000_000,
        )],
    );
    let before_quiet = vp.read_prediction_pressure_field().dominant_pressure;

    vp.refresh_prediction_pressure_field(
        3_040_000,
        &[wave(
            PREDICTION_WAVE_RESIDUE,
            1,
            5,
            0.18,
            0.95,
            0.70,
            3_040_000,
        )],
    );
    let after_quiet = vp.read_prediction_pressure_field();
    assert!(after_quiet.dominant_pressure > before_quiet);
    assert_eq!(after_quiet.quiet_reentry_count > 0, true);

    vp.refresh_prediction_pressure_field(
        3_080_000,
        &[wave(
            PREDICTION_WAVE_EVENT,
            2,
            6,
            0.92,
            0.40,
            0.80,
            3_080_000,
        )],
    );
    let after_event = vp.read_prediction_pressure_field();
    assert_eq!(after_event.event_override_count > 0, true);

    vp.refresh_prediction_pressure_field(
        3_120_000,
        &[wave(
            PREDICTION_WAVE_RELATION,
            3,
            7,
            0.44,
            0.99,
            0.78,
            3_120_000,
        )],
    );
    let after_resonance = vp.read_prediction_pressure_field();
    assert_eq!(after_resonance.resonance_spike_count > 0, true);
}

#[test]
fn prediction_pressure_field_closure_lowers_focus_and_retains_unclosed_residue() {
    let vp = Viewport::new();
    vp.refresh_prediction_pressure_field(
        4_000_000,
        &[
            wave(PREDICTION_WAVE_GAP, 1, -1, 0.82, 0.50, 0.80, 4_000_000),
            wave(PREDICTION_WAVE_RESIDUE, 2, 4, 0.38, 0.55, 0.55, 4_000_000),
        ],
    );
    let before = vp.read_prediction_pressure_field();
    vp.apply_prediction_field_closure(0.80);
    let after_closure = vp.read_prediction_pressure_field();

    assert!(after_closure.dominant_pressure < before.dominant_pressure);
    assert!(after_closure.closure_reductions > before.closure_reductions);

    vp.refresh_prediction_pressure_field(4_050_000, &[]);
    let after_decay = vp.read_prediction_pressure_field();
    assert_eq!(after_decay.retained_unclosed_residue > 0, true);
}

#[test]
fn prediction_pressure_field_dominant_pressure_drives_probe() {
    let vp = Viewport::new();
    vp.refresh_prediction_pressure_field(
        5_000_000,
        &[wave(
            PREDICTION_WAVE_RELATION,
            1,
            8,
            0.85,
            0.92,
            0.90,
            5_000_000,
        )],
    );

    vp.refresh_probe_law(5_010_000);
    vp.tick_probe_output_from_law(5_010_000);

    assert!(vp.read_action_output().n_write_epochs >= 1);
}

#[test]
fn prediction_pressure_field_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/prediction_pressure_field.rs"),
        include_str!("../src/probe_law.rs"),
    ];
    for forbidden in [
        "semantic_readiness",
        "SymbolizerState",
        "concept_id",
        "token_id",
        "word_id",
        "object_name",
        "text_understanding",
        "name_gate",
        "meaning_db",
        "dictionary",
        "parser",
        "tokenizer",
        "image_label",
        "voice_label",
        "prompt_answer",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "println!(\"body",
        "eprintln!",
    ] {
        for source in sources {
            assert!(
                !source.contains(forbidden),
                "prediction pressure field source contains rejected term: {forbidden}"
            );
        }
    }
}
