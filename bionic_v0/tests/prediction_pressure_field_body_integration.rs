use bionic_v0::prediction_pressure_field::{
    PREDICTION_WAVE_EVENT, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RELATION, PREDICTION_WAVE_RESIDUE,
};
use bionic_v0::substrate_pressure::PressureSample;
use bionic_v0::viewport::Viewport;

fn seed_gap_pressure(vp: &Viewport, t_us: u64, pressure: f64, slot_id: i8, edge_id: i16) {
    {
        let mut reasoning = vp.global_reasoning.write().unwrap();
        reasoning.global_surprise_load = pressure;
        reasoning.network_instability_score = pressure * 0.8;
        reasoning.routing_focus_slot_ids[0] = slot_id;
        reasoning.routing_focus_edge_ids[0] = edge_id;
        reasoning.routing_weights[0] = pressure;
        reasoning.last_update_t_us = t_us;
    }
    {
        let mut summary = vp.global_reasoning_memory_summary.write().unwrap();
        summary.novelty_score = pressure * 0.6;
        summary.persistence_score = pressure.max(0.35);
        summary.pattern_recurrence_score = pressure.max(0.40);
        summary.routing_stability_score = 0.62;
        summary.dominant_focus_persistence = pressure.max(0.45);
        summary.history_fill_ratio = 0.80;
        summary.last_update_t_us = t_us;
    }
    vp.refresh_active_reasoning_gap(t_us);
}

fn seed_event_pressure(vp: &Viewport, t_us: u64, pressure: f64) {
    vp.tick_evidence_tension(
        PressureSample {
            err: pressure,
            recurrence: pressure * 0.5,
            ambiguity: pressure,
            alignment: 0.15,
            support: 0.0,
            deviation: pressure,
            temporal_gap: pressure,
            cross_channel: pressure * 0.4,
        },
        t_us,
    );
}

fn seed_relation_pressure(vp: &Viewport, t_us: u64, edge_id: usize, pressure: f64) {
    let mut relations = vp.global_relations.write().unwrap();
    relations.edges[edge_id].src_slot_id = 1;
    relations.edges[edge_id].dst_slot_id = 2;
    relations.edges[edge_id].active = true;
    relations.edges[edge_id].relation_confidence = 0.84;
    relations.edges[edge_id].predictive_score = 0.68;
    relations.edges[edge_id].pred.relation_surprise = pressure;
    relations.edges[edge_id].last_update_t_us = t_us;
    relations.n_active_edges = 1;
    relations.last_update_t_us = t_us;
}

fn has_wave(vp: &Viewport, source_kind: u8) -> bool {
    vp.read_prediction_pressure_field()
        .waves
        .iter()
        .any(|wave| wave.source_kind == source_kind)
}

#[test]
fn prediction_pressure_field_body_integration_hydrates_from_existing_substrate_pressures() {
    let vp = Viewport::new();
    let t_us = 10_000_000;
    seed_gap_pressure(&vp, t_us, 0.72, 1, 3);
    seed_event_pressure(&vp, t_us + 1_000, 1.0);
    seed_relation_pressure(&vp, t_us + 2_000, 3, 0.91);

    vp.tick_prediction_pressure_field_from_substrate(t_us + 3_000);

    let field = vp.read_prediction_pressure_field();
    assert!(field.waves_alive >= 4);
    assert_eq!(field.dominant_focus_is_single(), 1);
    assert!(field.dominant_pressure > 0.0);
    assert!(has_wave(&vp, PREDICTION_WAVE_GAP));
    assert!(has_wave(&vp, PREDICTION_WAVE_EVENT));
    assert!(has_wave(&vp, PREDICTION_WAVE_RELATION));
    assert!(has_wave(&vp, PREDICTION_WAVE_RESIDUE));
}

#[test]
fn prediction_pressure_field_body_integration_keeps_probe_quiet_without_pressure() {
    let vp = Viewport::new();
    for step in 0..6 {
        let t_us = 11_000_000 + step * 20_000;
        vp.refresh_active_reasoning_gap(t_us);
        vp.tick_prediction_pressure_field_from_substrate(t_us);
        vp.refresh_probe_law(t_us);
        vp.tick_probe_output_from_law(t_us);
    }

    assert_eq!(vp.read_prediction_pressure_field().dominant_pressure, 0.0);
    assert_eq!(vp.read_action_output().n_write_epochs, 0);
}

#[test]
fn prediction_pressure_field_body_integration_event_pressure_can_drive_probe_without_manual_wave_input(
) {
    let vp = Viewport::new();
    let before = vp.read_action_output().n_write_epochs;
    for step in 0..4 {
        let t_us = 12_000_000 + step * 20_000;
        seed_gap_pressure(&vp, t_us, 0.78, 2, 5);
        seed_event_pressure(&vp, t_us + 1_000, 1.0);
        seed_relation_pressure(&vp, t_us + 2_000, 5, 0.88);
        vp.tick_prediction_pressure_field_from_substrate(t_us + 3_000);
        vp.refresh_probe_law(t_us + 4_000);
        vp.tick_probe_output_from_law(t_us + 4_000);
    }

    assert!(vp.read_action_output().n_write_epochs > before);
}

#[test]
fn prediction_pressure_field_body_integration_stress_keeps_waves_and_switches_bounded() {
    let vp = Viewport::new();
    let mut max_pressure = 0.0_f64;
    for step in 0..96 {
        let t_us = 13_000_000 + step * 10_000;
        let slot_id = (step % 6) as i8;
        let edge_id = (step % 12) as i16;
        let gap = if step % 3 == 0 { 0.90 } else { 0.38 };
        let event = if step % 5 == 0 { 1.0 } else { 0.45 };
        let relation = if step % 7 == 0 { 0.94 } else { 0.52 };

        seed_gap_pressure(&vp, t_us, gap, slot_id, edge_id);
        seed_event_pressure(&vp, t_us + 1_000, event);
        seed_relation_pressure(&vp, t_us + 2_000, (step as usize) % 12, relation);
        vp.tick_prediction_pressure_field_from_substrate(t_us + 3_000);

        let field = vp.read_prediction_pressure_field();
        max_pressure = max_pressure.max(field.dominant_pressure);
        assert!(field.waves_alive <= 16);
        assert!(field
            .waves
            .iter()
            .all(|wave| (0.0..=1.0).contains(&wave.amplitude)));
        assert!(field
            .waves
            .iter()
            .all(|wave| (0.0..=1.0).contains(&wave.resonance)));
    }

    let field = vp.read_prediction_pressure_field();
    assert!(max_pressure > 0.05);
    assert_eq!(field.focus_switches_bounded(96), 1);
}

#[test]
fn prediction_pressure_field_body_runtime_orders_field_before_probe_law() {
    let body_source = include_str!("../src/bin/body.rs");
    let gap_pos = body_source
        .find("refresh_active_reasoning_gap")
        .expect("active gap refresh is present");
    let field_pos = body_source
        .find("tick_prediction_pressure_field_from_substrate")
        .expect("prediction field body tick is present");
    let probe_pos = body_source
        .find("refresh_probe_law")
        .expect("probe law refresh is present");

    assert!(gap_pos < field_pos);
    assert!(field_pos < probe_pos);
}

#[test]
fn prediction_pressure_field_body_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/prediction_pressure_field.rs"),
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
    ] {
        for source in sources {
            assert!(
                !source.contains(forbidden),
                "body prediction pressure source contains rejected term: {forbidden}"
            );
        }
    }
}
