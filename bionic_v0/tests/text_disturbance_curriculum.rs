use bionic_v0::curriculum::{run_text_disturbance_curriculum, CurriculumConfig};

#[test]
fn text_disturbance_curriculum_builds_distinct_hold_conflict_and_reentry_evidence() {
    let evidence =
        run_text_disturbance_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.train_episodes, 4);
    assert_eq!(evidence.reentry_episodes, 2);
    assert!(evidence.distinct_text_traces >= 3);
    assert!(evidence.a_recurrence >= evidence.train_episodes);
    assert!(evidence.b_recurrence >= evidence.train_episodes);
    assert!(evidence.a_to_slot_a_strength > 0.5);
    assert!(evidence.b_to_slot_b_strength > 0.5);
    assert!(evidence.a_to_slot_b_strength <= 0.01);
    assert!(
        evidence.hold_avg_strength_ratio >= 0.80,
        "hold should preserve most trained text disturbance routing strength"
    );
    assert!(
        evidence.conflict_ambiguity_delta > 0.0,
        "conflict should increase ambiguity instead of rewriting object routing"
    );
    assert!(
        evidence.reentry_persistent_avg_strength > evidence.reentry_cold_avg_strength,
        "persistent reentry should beat cold text disturbance baseline"
    );
    assert!(evidence.time_window_strength > 0.0);
}

#[test]
fn text_disturbance_sources_do_not_introduce_rejected_language_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/curriculum.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
    ];

    for src in sources {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "text_understanding",
            "name_gate",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "text disturbance path must not contain `{}`",
                forbidden
            );
        }
    }
}
