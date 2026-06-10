use bionic_v0::allostatic_stability::summarize_allostatic_stability;
use bionic_v0::live_human_nursery::sample_live_human_nursery_snapshots;

#[test]
fn allostatic_stability_scores_are_bounded_and_observer_side() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let snap = snapshots.last_mut().expect("sample snapshot");
    snap.evidence_tension.unresolved_pressure = 0.46;
    snap.active_reasoning_gap.gap_pressure = 0.52;
    snap.active_reasoning_gap.closure_score = 0.40;
    snap.self_development.familiarity_pressure = 0.48;
    snap.self_development.exploratory_pressure = 0.42;
    snap.global_reasoning_memory.routing_stability_score = 0.44;
    snap.probe_law.intensity = 0.50;

    let evidence = summarize_allostatic_stability(snapshots.iter(), 1, 0);

    for value in [
        evidence.arousal_load,
        evidence.reserve_level,
        evidence.recovery_debt,
        evidence.bearing_capacity,
        evidence.exploration_allowance,
        evidence.expression_gain,
        evidence.protective_limit,
        evidence.overdrive_window,
        evidence.downshift_pressure,
    ] {
        assert!(value.is_finite());
        assert!((0.0..=1.0).contains(&value));
    }
    assert_eq!(evidence.deterministic_fixture, 1);
    assert_eq!(evidence.runtime_snapshot_read, 0);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn moderate_demand_with_support_opens_overdrive_and_expression_gain() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let mut snap = snapshots.pop().expect("sample snapshot");
    snap.evidence_tension.unresolved_pressure = 0.58;
    snap.active_reasoning_gap.gap_pressure = 0.56;
    snap.active_reasoning_gap.closure_score = 0.62;
    snap.global_reasoning_memory.routing_stability_score = 0.66;
    snap.global_reasoning_memory.pattern_recurrence_score = 0.61;
    snap.self_development.familiarity_pressure = 0.60;
    snap.self_development.stable_recurrence = 0.64;
    snap.probe_law.output_intensity = 0.42;

    let evidence = summarize_allostatic_stability([snap].iter(), 1, 0);

    assert!(evidence.reserve_level > 0.35);
    assert!(evidence.expression_gain > evidence.protective_limit);
    assert!(evidence.exploration_allowance > 0.10);
    assert!(evidence.overdrive_window > 0.0);
}

#[test]
fn low_support_and_recovery_debt_raise_downshift() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let mut snap = snapshots.pop().expect("sample snapshot");
    snap.action_consequence.n_pairs = 0;
    snap.action_consequence.total_pairs = 0;
    snap.action_consequence.pairs.clear();
    snap.evidence_tension.unresolved_pressure = 0.88;
    snap.active_reasoning_gap.gap_pressure = 0.82;
    snap.active_reasoning_gap.closure_score = 0.02;
    snap.active_reasoning_gap.reentry_score = 0.0;
    snap.framed_byte_egress.correction_deviation = 0.80;
    snap.self_development.repetition_pressure = 0.78;
    snap.self_development.familiarity_pressure = 0.03;
    snap.self_development.stable_recurrence = 0.0;
    snap.self_development.room_familiarity = 0.0;
    snap.global_reasoning_memory.routing_stability_score = 0.02;
    snap.global_reasoning_memory.pattern_recurrence_score = 0.0;
    snap.probe_law.suppression = 0.76;

    let evidence = summarize_allostatic_stability([snap].iter(), 1, 0);

    assert!(evidence.recovery_debt > 0.50);
    assert!(evidence.reserve_level < 0.50);
    assert!(evidence.protective_limit > evidence.expression_gain);
    assert!(evidence.downshift_pressure > 0.40);
}

#[test]
fn allostatic_stability_sources_do_not_introduce_emotion_or_personality_state() {
    let src = include_str!("../src/allostatic_stability.rs");
    for forbidden in [
        "emotion_state",
        "happy_state",
        "sad_state",
        "anger_module",
        "fear_module",
        "personality_emotion",
        "semantic_readiness",
        "concept_id",
        "token_id",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "allostatic evidence must not contain `{forbidden}`"
        );
    }
}
