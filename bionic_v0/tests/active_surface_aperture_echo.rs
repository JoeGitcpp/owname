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

fn seed_parallel_pressure(vp: &Viewport, t_us: u64) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[
            wave(PREDICTION_WAVE_GAP, 1, 2, 0.86, 0.55, 0.78, t_us),
            wave(PREDICTION_WAVE_RELATION, 2, 4, 0.72, 0.92, 0.72, t_us),
            wave(PREDICTION_WAVE_RESIDUE, 3, 6, 0.48, 0.65, 0.58, t_us),
        ],
    );
}

#[test]
fn active_surface_aperture_echo_keeps_parallel_traces_but_one_aperture() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 1_000_000);
    vp.refresh_active_surface(1_010_000);

    let surface = vp.read_active_surface();
    assert!(surface.traces_alive >= 3);
    assert_eq!(surface.active_aperture_count(), 1);
    assert_eq!(surface.echo_focus_present(), 1);
    assert_eq!(surface.echo_pressure_below_aperture(), 1);
    assert!(surface.aperture_trace_index >= 0);
    assert!(surface.echo_trace_index >= 0);
    assert_ne!(surface.aperture_trace_index, surface.echo_trace_index);
    assert!(surface.aperture_pressure > 0.0);
    assert!(surface.effective_probe_pressure() > surface.aperture_pressure);
}

#[test]
fn active_surface_aperture_echo_places_dominant_trace_in_aperture_and_retains_others() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 2_000_000);
    vp.refresh_active_surface(2_010_000);

    let surface = vp.read_active_surface();
    let aperture = surface.traces[surface.aperture_trace_index as usize];
    assert_eq!(aperture.source_kind, PREDICTION_WAVE_GAP);
    assert!(surface
        .traces
        .iter()
        .any(|trace| trace.source_kind == PREDICTION_WAVE_RELATION && trace.echo_share > 0.0));
    assert!(surface
        .traces
        .iter()
        .any(|trace| trace.source_kind == PREDICTION_WAVE_RESIDUE && trace.residue_strength > 0.0));
}

#[test]
fn active_surface_aperture_echo_closure_reduces_aperture_without_deleting_parallel_traces() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 3_000_000);
    vp.refresh_active_surface(3_010_000);
    let before = vp.read_active_surface();

    vp.apply_active_surface_closure(0.80);
    let after = vp.read_active_surface();

    assert!(after.closure_reductions > before.closure_reductions);
    assert!(after.aperture_pressure < before.aperture_pressure);
    assert!(after.traces_alive >= 2);
    assert_eq!(after.non_aperture_focus_retained(), 1);
}

#[test]
fn active_surface_aperture_echo_event_can_displace_aperture_and_old_focus_retains_residue() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 4_000_000);
    vp.refresh_active_surface(4_010_000);
    let before = vp.read_active_surface();
    let before_kind = before.traces[before.aperture_trace_index as usize].source_kind;

    vp.refresh_prediction_pressure_field(
        4_040_000,
        &[wave(
            PREDICTION_WAVE_EVENT,
            0,
            9,
            0.98,
            0.70,
            0.86,
            4_040_000,
        )],
    );
    vp.refresh_active_surface(4_050_000);
    let after = vp.read_active_surface();
    let after_kind = after.traces[after.aperture_trace_index as usize].source_kind;

    assert_eq!(after_kind, PREDICTION_WAVE_EVENT);
    assert_ne!(after_kind, before_kind);
    assert!(after.aperture_displacements > before.aperture_displacements);
    assert!(after
        .traces
        .iter()
        .any(|trace| trace.source_kind == before_kind && trace.residue_strength > 0.0));
}

#[test]
fn active_surface_aperture_echo_suppressed_focus_can_reenter_after_quiet_rehydration() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 5_000_000);
    vp.refresh_active_surface(5_010_000);

    vp.refresh_prediction_pressure_field(
        5_040_000,
        &[wave(
            PREDICTION_WAVE_EVENT,
            0,
            9,
            0.98,
            0.70,
            0.86,
            5_040_000,
        )],
    );
    vp.refresh_active_surface(5_050_000);
    let displaced = vp.read_active_surface();

    for step in 0..4 {
        vp.refresh_prediction_pressure_field(5_080_000 + step * 20_000, &[]);
        vp.refresh_active_surface(5_080_000 + step * 20_000);
    }
    vp.refresh_prediction_pressure_field(
        5_180_000,
        &[wave(
            PREDICTION_WAVE_RESIDUE,
            3,
            6,
            0.62,
            0.95,
            0.70,
            5_180_000,
        )],
    );
    vp.refresh_active_surface(5_190_000);
    let reentered = vp.read_active_surface();

    assert!(reentered.reentry_count > displaced.reentry_count);
    assert!(reentered
        .traces
        .iter()
        .any(|trace| trace.source_kind == PREDICTION_WAVE_RESIDUE && trace.aperture_share > 0.0));
}

#[test]
fn active_surface_aperture_echo_probe_drive_tracks_aperture_with_echo() {
    let vp = Viewport::new();
    seed_parallel_pressure(&vp, 6_000_000);
    vp.refresh_active_surface(6_010_000);
    let surface = vp.read_active_surface();
    let expected_drive = surface.effective_probe_pressure();

    vp.refresh_probe_law(6_020_000);
    vp.tick_probe_output_from_law(6_020_000);

    let probe = vp.read_probe_law();
    assert!(expected_drive > surface.aperture_pressure);
    assert!(probe.intensity > surface.aperture_pressure * 0.20);
    assert!(vp.read_action_output().n_write_epochs >= 1);
}

#[test]
fn active_surface_aperture_echo_stress_bounds_switching_and_trace_count() {
    let vp = Viewport::new();
    for step in 0..96 {
        let t_us = 7_000_000 + step * 10_000;
        vp.refresh_prediction_pressure_field(
            t_us,
            &[
                wave(
                    PREDICTION_WAVE_GAP,
                    (step % 4) as i8,
                    (step % 12) as i16,
                    if step % 3 == 0 { 0.90 } else { 0.44 },
                    0.55,
                    0.74,
                    t_us,
                ),
                wave(
                    PREDICTION_WAVE_RELATION,
                    ((step + 1) % 4) as i8,
                    ((step + 3) % 12) as i16,
                    if step % 5 == 0 { 0.86 } else { 0.50 },
                    0.82,
                    0.70,
                    t_us,
                ),
                wave(
                    PREDICTION_WAVE_RESIDUE,
                    ((step + 2) % 4) as i8,
                    ((step + 6) % 12) as i16,
                    0.40,
                    0.70,
                    0.52,
                    t_us,
                ),
            ],
        );
        vp.refresh_active_surface(t_us + 1_000);
        let surface = vp.read_active_surface();
        assert!(surface.traces_alive <= 16);
        assert_eq!(surface.active_aperture_count(), 1);
        assert_eq!(surface.echo_pressure_below_aperture(), 1);
    }

    let surface = vp.read_active_surface();
    assert_eq!(surface.aperture_switches_bounded(96), 1);
}

#[test]
fn active_surface_aperture_echo_body_runtime_orders_surface_before_probe_law() {
    let body_source = include_str!("../src/bin/body.rs");
    let field_pos = body_source
        .find("tick_prediction_pressure_field_from_substrate")
        .expect("pressure field body tick is present");
    let surface_pos = body_source
        .find("refresh_active_surface")
        .expect("active surface refresh is present");
    let probe_pos = body_source
        .find("refresh_probe_law")
        .expect("probe law refresh is present");

    assert!(field_pos < surface_pos);
    assert!(surface_pos < probe_pos);
}

#[test]
fn active_surface_aperture_echo_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/active_surface.rs"),
        include_str!("../src/probe_law.rs"),
        include_str!("../src/bin/body.rs"),
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
        "struct Thought",
        "struct Desire",
        "struct Mind",
        "struct Conscious",
        "struct Task",
        "struct Goal",
    ] {
        for source in sources {
            assert!(
                !source.contains(forbidden),
                "active surface source contains rejected term: {forbidden}"
            );
        }
    }
}
