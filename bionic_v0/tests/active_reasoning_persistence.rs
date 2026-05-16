use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::viewport::Viewport;

fn seed_reasoning_pressure(
    vp: &Viewport,
    surprise: f64,
    instability: f64,
    focus_slot_id: i8,
    focus_edge_id: i16,
) {
    let mut rs = vp.global_reasoning.write().unwrap();
    rs.global_surprise_load = surprise;
    rs.network_instability_score = instability;
    rs.routing_focus_slot_ids[0] = focus_slot_id;
    rs.routing_focus_edge_ids[0] = focus_edge_id;
}

#[test]
fn active_gap_persists_after_pressure_falls_and_drives_probe() {
    let vp = Viewport::new();
    seed_reasoning_pressure(&vp, 0.8, 0.6, 2, 3);

    for t in [1_000_000, 1_010_000, 1_020_000] {
        vp.refresh_active_reasoning_gap(t);
    }

    let active = vp.read_active_reasoning_gap();
    assert!(active.gap_pressure > 0.2);
    assert!(active.persistence_score > 0.0);

    seed_reasoning_pressure(&vp, 0.0, 0.0, 2, 3);
    vp.refresh_active_reasoning_gap(1_030_000);
    let carried = vp.read_active_reasoning_gap();
    assert!(carried.gap_pressure > 0.05);

    vp.refresh_probe_law(1_040_000);
    vp.tick_probe_output_from_law(1_040_000);
    assert!(vp.read_action_output().n_write_epochs > 0);
}

#[test]
fn active_gap_trace_records_observer_curve_samples() {
    let vp = Viewport::new();
    seed_reasoning_pressure(&vp, 0.7, 0.5, 2, 3);

    for t in [1_000_000, 1_010_000, 1_020_000, 1_030_000] {
        vp.refresh_active_reasoning_gap(t);
    }

    let gap = vp.read_active_reasoning_gap();
    assert!(gap.history_len >= 4);
    assert!(gap.total_samples >= 4);

    let latest = gap.recent_trace(0).expect("latest active gap trace");
    assert!(latest.gap_pressure > 0.0);
    assert!(latest.gap_pressure <= 1.0);
    assert!(latest.closure_score >= 0.0 && latest.closure_score <= 1.0);
    assert!(latest.reentry_score >= 0.0 && latest.reentry_score <= 1.0);
    assert!(latest.active_cycles >= 1);
}

#[test]
fn paired_return_evidence_reduces_active_gap_pressure() {
    let vp = Viewport::new();
    seed_reasoning_pressure(&vp, 0.9, 0.8, 1, 4);
    vp.refresh_active_reasoning_gap(2_000_000);
    let before = vp.read_active_reasoning_gap();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.7,
        event_t_us: 2_010_000,
    });
    let write_seq = vp.read_action_output().write_epoch_history[0].write_seq;
    let window_idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 2_011_000,
        end_t_us: 2_012_000,
        magnitude: 0.7,
    });
    let window_id = vp.read_action_consequence().windows[window_idx as usize].window_id;
    assert!(vp.close_action_consequence_window(window_id, 2_013_000));
    assert!(vp.pair_action_consequence(write_seq, window_id, 1_000));

    seed_reasoning_pressure(&vp, 0.0, 0.0, 1, 4);
    vp.refresh_active_reasoning_gap(2_020_000);
    let after = vp.read_active_reasoning_gap();

    assert!(after.closure_score > 0.5);
    assert!(after.gap_pressure < before.gap_pressure);
    assert!(after.last_pair_seq > before.last_pair_seq);
}

#[test]
fn reasoning_history_reenters_a_weak_active_gap() {
    let vp = Viewport::new();
    {
        let mut gap = vp.active_reasoning_gap.write().unwrap();
        gap.gap_pressure = 0.05;
        gap.focus_slot_id = 3;
        gap.focus_edge_id = 5;
    }
    {
        let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
        memory.persistence_score = 0.8;
        memory.pattern_recurrence_score = 0.9;
        memory.dominant_focus_persistence = 0.7;
    }
    seed_reasoning_pressure(&vp, 0.0, 0.0, 3, 5);

    vp.refresh_active_reasoning_gap(3_000_000);
    let after = vp.read_active_reasoning_gap();

    assert!(after.reentry_score > 0.1);
    assert!(after.gap_pressure > 0.05);
}

#[test]
fn active_reasoning_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/active_reasoning.rs"),
        include_str!("../src/probe_law.rs"),
    ];
    let forbidden = [
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
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "active reasoning source contains rejected term: {term}"
            );
        }
    }
}
