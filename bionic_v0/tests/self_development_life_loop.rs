use bionic_v0::action_consequence::{
    ActionConsequencePair, ActionConsequenceState, ActionConsequenceWindow,
    ConsequenceModalityKind, ConsequencePairStatus, ConsequenceWindowPhase,
};
use bionic_v0::action_output::{
    ActionOutputPhase, ActionOutputState, OutputChannelKind, WriteEpochRecord,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::{CONSEQUENCE_WINDOW_CAP, PAIR_HISTORY_CAP, WRITE_EPOCH_HISTORY_CAP};
use std::sync::Arc;

fn seed_stable_recurrence(vp: &Viewport, t_us: u64) {
    {
        let mut reasoning = vp.global_reasoning.write().unwrap();
        reasoning.global_surprise_load = 0.08;
        reasoning.routing_focus_slot_ids[0] = 1;
        reasoning.last_update_t_us = t_us;
    }
    {
        let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
        memory.pattern_recurrence_score = 0.82;
        memory.routing_stability_score = 0.88;
        memory.dominant_focus_persistence = 0.90;
        memory.persistence_score = 0.76;
        memory.history_fill_ratio = 0.80;
        memory.last_update_t_us = t_us;
    }
}

#[test]
fn stable_recurrence_sediments_without_external_curriculum_gate() {
    let vp = Viewport::new();
    for step in 0..12 {
        let t_us = 1_000_000 + step * 33_000;
        seed_stable_recurrence(&vp, t_us);
        vp.tick_self_development_from_substrate(t_us);
    }

    let state = vp.read_self_development();
    assert!(state.stable_recurrence > 0.40);
    assert!(state.familiarity_pressure > 0.25);
    assert!(state.repetition_pressure < 0.20);
    assert!(state.output_suppression < 0.20);
}

fn seed_saturated_low_similarity_loop(vp: &Viewport) {
    {
        let mut output = vp.action_output.write().unwrap();
        *output = ActionOutputState::default();
        for idx in 0..WRITE_EPOCH_HISTORY_CAP {
            output.write_epoch_history.push(WriteEpochRecord {
                write_seq: (idx + 1) as u64,
                channel_kind: OutputChannelKind::GenericWrite,
                start_magnitude: 1.0,
                start_t_us: 2_000_000 + idx as u64,
                last_update_t_us: 2_000_100 + idx as u64,
                last_phase: ActionOutputPhase::Active,
            });
        }
        output.n_write_epochs = output.write_epoch_history.len() as u32;
    }

    {
        let mut consequence = vp.action_consequence.write().unwrap();
        *consequence = ActionConsequenceState::default();
        for idx in 0..CONSEQUENCE_WINDOW_CAP {
            consequence.windows.push(ActionConsequenceWindow {
                window_id: (idx + 1) as u64,
                modality_kind: ConsequenceModalityKind::GenericIngress,
                start_t_us: 2_010_000 + idx as u64,
                end_t_us: 2_010_500 + idx as u64,
                magnitude: 0.0,
                phase: ConsequenceWindowPhase::Closed,
                last_update_t_us: 2_010_600 + idx as u64,
            });
        }
        for idx in 0..PAIR_HISTORY_CAP {
            consequence.pairs.push(ActionConsequencePair {
                pair_seq: (idx + 1) as u64,
                write_seq: ((idx % WRITE_EPOCH_HISTORY_CAP) + 1) as u64,
                window_id: ((idx % CONSEQUENCE_WINDOW_CAP) + 1) as u64,
                delay_us: 20_000,
                status: ConsequencePairStatus::Paired,
                pair_t_us: 2_020_000 + idx as u64,
                curve_similarity: 0.0,
            });
        }
        consequence.n_windows = consequence.windows.len() as u32;
        consequence.n_pairs = consequence.pairs.len() as u32;
        consequence.total_pairs = consequence.pairs.len() as u64;
    }
}

fn seed_probe_pressure(vp: &Viewport, t_us: u64, pressure: f64) {
    {
        let mut reasoning = vp.global_reasoning.write().unwrap();
        reasoning.global_surprise_load = pressure;
        reasoning.network_instability_score = pressure * 0.5;
        reasoning.routing_focus_slot_ids[0] = 2;
        reasoning.routing_weights[0] = pressure;
        reasoning.last_update_t_us = t_us;
    }
    {
        let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
        memory.novelty_score = pressure * 0.5;
        memory.pattern_recurrence_score = 0.10;
        memory.routing_stability_score = 0.10;
        memory.dominant_focus_persistence = 0.10;
        memory.last_update_t_us = t_us;
    }
}

#[test]
fn repetitive_write_return_loop_raises_suppression_when_prediction_does_not_improve() {
    let vp = Viewport::new();
    seed_saturated_low_similarity_loop(&vp);

    for step in 0..8 {
        let t_us = 2_000_000 + step * 33_000;
        vp.tick_self_development_from_substrate(t_us);
    }

    let state = vp.read_self_development();
    assert!(state.repetition_pressure > 0.45);
    assert!(state.output_suppression > 0.25);
    assert!(state.exploratory_pressure < state.prediction_gap.max(0.01));
}

#[test]
fn self_development_suppression_dampens_probe_output_without_disabling_probe_law() {
    let baseline = Viewport::new();
    seed_probe_pressure(&baseline, 3_000_000, 0.90);
    baseline.refresh_probe_law(3_000_100);
    baseline.tick_probe_output_from_law(3_000_100);
    let baseline_probe = baseline.read_probe_law();
    let baseline_epochs = baseline.read_action_output().n_write_epochs;

    let damped = Viewport::new();
    seed_probe_pressure(&damped, 3_000_000, 0.90);
    seed_saturated_low_similarity_loop(&damped);
    for step in 0..10 {
        damped.tick_self_development_from_substrate(3_000_000 + step * 33_000);
    }
    damped.refresh_probe_law(3_400_000);
    damped.tick_probe_output_from_law(3_400_000);
    let damped_probe = damped.read_probe_law();

    assert!(baseline_epochs > 0);
    assert!(damped.read_self_development().output_suppression > 0.25);
    assert!(damped_probe.intensity < baseline_probe.intensity * 0.75);
}

#[test]
fn body_runtime_orders_self_development_before_probe_law() {
    let body_source = include_str!("../src/bin/body.rs");
    let self_development_pos = body_source
        .find("tick_self_development_from_substrate")
        .expect("self-development tick is present");
    let probe_pos = body_source
        .find("refresh_probe_law")
        .expect("probe law refresh is present");

    assert!(self_development_pos < probe_pos);
}

#[test]
fn snapshot_roundtrip_preserves_self_development_evidence() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_self_development_snapshot_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let paths = bionic_v0::snapshot::SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());
    seed_stable_recurrence(&vp, 4_000_000);
    vp.tick_self_development_from_substrate(4_000_000);

    bionic_v0::snapshot::write_snapshot(&vp, &paths).unwrap();
    let snap = bionic_v0::snapshot::read_snapshot(&paths.final_path).unwrap();

    assert!(snap.self_development.familiarity_pressure > 0.0);
    assert_eq!(snap.self_development.total_updates, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn self_development_sources_stay_numeric_and_guarded() {
    let sources = [
        include_str!("../src/self_development.rs"),
        include_str!("../src/probe_law.rs"),
        include_str!("../src/bin/body.rs"),
        include_str!("../src/snapshot.rs"),
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
                "self-development source contains rejected term: {forbidden}"
            );
        }
    }
}
