use bionic_v0::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RELATION,
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

fn seed_surface_residue(vp: &Viewport, t_us: u64, edge_id: i16) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[wave(
            PREDICTION_WAVE_GAP,
            1,
            edge_id,
            0.42,
            0.50,
            0.60,
            t_us,
        )],
    );
    vp.refresh_active_surface(t_us + 1_000);
}

fn seed_relation(vp: &Viewport, t_us: u64, edge_id: usize, pressure: f64, confidence: f64) {
    let mut relations = vp.global_relations.write().unwrap();
    relations.edges[edge_id].src_slot_id = 1;
    relations.edges[edge_id].dst_slot_id = 2;
    relations.edges[edge_id].active = true;
    relations.edges[edge_id].relation_confidence = confidence;
    relations.edges[edge_id].predictive_score = pressure;
    relations.edges[edge_id].pred.relation_surprise = pressure;
    relations.edges[edge_id].last_update_t_us = t_us;
    relations.n_active_edges = 1;
    relations.last_update_t_us = t_us;
}

fn seed_memory(vp: &Viewport, recurrence: f64, stability: f64, t_us: u64) {
    let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
    memory.pattern_recurrence_score = recurrence;
    memory.routing_stability_score = stability;
    memory.persistence_score = recurrence;
    memory.dominant_focus_persistence = recurrence.max(0.25);
    memory.history_fill_ratio = 0.80;
    memory.last_update_t_us = t_us;
}

#[test]
fn background_relation_resonance_low_match_attempts_decay_without_entering_surface() {
    let vp = Viewport::new();
    seed_surface_residue(&vp, 1_000_000, 3);
    seed_memory(&vp, 0.20, 0.20, 1_010_000);

    for step in 0..10 {
        let t_us = 1_020_000 + step * 20_000;
        seed_relation(&vp, t_us, 3, 0.08, 0.12);
        vp.tick_active_surface_relation_resonance(t_us + 1_000);
        vp.refresh_active_surface(t_us + 2_000);
    }

    let surface = vp.read_active_surface();
    assert!(surface.relation_resonance_traces_alive >= 1);
    assert!(surface.low_match_decay_count > 0);
    assert!(surface.wrong_match_leakage <= 0.01);
    assert_eq!(surface.sudden_match_count, 0);
    assert_eq!(surface.sudden_match_enters_surface_count, 0);
}

#[test]
fn background_relation_resonance_high_match_spikes_and_enters_active_surface() {
    let vp = Viewport::new();
    seed_surface_residue(&vp, 2_000_000, 4);
    seed_memory(&vp, 0.88, 0.82, 2_010_000);

    seed_relation(&vp, 2_020_000, 4, 0.93, 0.91);
    vp.tick_active_surface_relation_resonance(2_030_000);
    vp.refresh_active_surface(2_040_000);

    let surface = vp.read_active_surface();
    assert!(surface.relation_resonance_traces_alive >= 1);
    assert!(surface.sudden_match_count > 0);
    assert_eq!(surface.high_match_spike_observed(), 1);
    assert_eq!(surface.sudden_match_enters_surface(), 1);
    assert!(surface.traces.iter().any(|trace| {
        trace.source_kind == PREDICTION_WAVE_RELATION
            && (trace.aperture_share > 0.0 || trace.echo_share > 0.0)
    }));
}

#[test]
fn background_relation_resonance_keeps_attempts_bounded_under_mixed_pressure() {
    let vp = Viewport::new();
    seed_surface_residue(&vp, 3_000_000, 5);
    seed_memory(&vp, 0.72, 0.65, 3_010_000);

    for step in 0..96 {
        let t_us = 3_020_000 + step * 10_000;
        let edge = (step % 12) as usize;
        let high = step % 11 == 0;
        seed_relation(
            &vp,
            t_us,
            edge,
            if high { 0.92 } else { 0.18 },
            if high { 0.90 } else { 0.24 },
        );
        vp.tick_active_surface_relation_resonance(t_us + 1_000);
    }

    let surface = vp.read_active_surface();
    assert!(surface.relation_resonance_traces_alive <= 16);
    assert!(surface.sudden_match_count > 0);
    assert!(surface.low_match_decay_count > 0);
}

#[test]
fn background_relation_resonance_body_runtime_orders_resonance_before_active_surface() {
    let body_source = include_str!("../src/bin/body.rs");
    let field_pos = body_source
        .find("tick_prediction_pressure_field_from_substrate")
        .expect("pressure field tick is present");
    let resonance_pos = body_source
        .find("tick_active_surface_relation_resonance")
        .expect("relation resonance tick is present");
    let surface_pos = body_source
        .find("refresh_active_surface")
        .expect("active surface refresh is present");
    let probe_pos = body_source
        .find("refresh_probe_law")
        .expect("probe law refresh is present");

    assert!(field_pos < resonance_pos);
    assert!(resonance_pos < surface_pos);
    assert!(surface_pos < probe_pos);
}

#[test]
fn background_relation_resonance_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/active_surface.rs"),
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
                "background relation resonance source contains rejected term: {forbidden}"
            );
        }
    }
}
