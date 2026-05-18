use bionic_v0::curriculum::{run_text_open_stream_curriculum, CurriculumConfig};

#[test]
fn text_open_stream_proves_mixed_stream_delay_composition_correction_and_reentry() {
    let evidence =
        run_text_open_stream_curriculum(CurriculumConfig::default()).expect("open stream runs");

    assert!(evidence.stream_events >= 20);
    assert!(evidence.supported_marks >= 6);
    assert!(evidence.unrelated_marks >= 4);
    assert!(evidence.variant_marks >= 3);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 2);
    assert!(evidence.composition_windows >= 2);
    assert!(evidence.delayed_windows >= 2);
    assert!(evidence.train_supported_avg > 0.5);
    assert!(evidence.train_supported_weakest > 0.0);
    assert!(evidence.unrelated_leakage_strength < evidence.train_supported_avg);
    assert!(evidence.hold_supported_ratio >= 0.70);
    assert!(evidence.variant_persistent_avg > evidence.variant_cold_avg);
    assert!(evidence.delayed_binding_strength > 0.0);
    assert!(evidence.composition_relation_strength > 0.25);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.reentry_persistent_avg > evidence.reentry_cold_avg);
    assert!(evidence.reentry_persistent_weakest > evidence.reentry_cold_weakest);
}

#[test]
fn text_open_stream_sources_do_not_introduce_rejected_language_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/curriculum.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
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
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected language ontology term: {term}"
            );
        }
    }
}
