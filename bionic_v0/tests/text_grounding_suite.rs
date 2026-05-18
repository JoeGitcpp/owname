use bionic_v0::curriculum::{run_text_grounding_suite, CurriculumConfig};

#[test]
fn text_grounding_suite_proves_multi_target_variant_composition_correction_and_reentry() {
    let evidence = run_text_grounding_suite(CurriculumConfig::default()).expect("suite runs");

    assert_eq!(evidence.train_marks, 6);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 1);
    assert!(evidence.variant_marks >= 3);
    assert!(evidence.composition_windows >= 2);
    assert!(evidence.train_avg_strength > 0.5);
    assert!(evidence.train_weakest_strength > 0.0);
    assert!(evidence.hold_avg_ratio >= 0.80);
    assert!(evidence.hold_weakest_ratio >= 0.60);
    assert!(evidence.variant_generalization_score > 0.0);
    assert!(evidence.variant_distinctness_ratio > 0.0);
    assert!(evidence.composition_relation_strength > 0.0);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.reentry_persistent_avg > evidence.reentry_cold_avg);
    assert!(evidence.reentry_persistent_weakest > evidence.reentry_cold_weakest);
}

#[test]
fn text_grounding_suite_sources_do_not_introduce_rejected_language_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
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
            "meaning_db",
            "dictionary",
            "parser",
            "tokenizer",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "text grounding suite path must not contain `{}`",
                forbidden
            );
        }
    }
}
