use bionic_v0::curriculum::{run_product_cognition_baseline_curriculum, CurriculumConfig};

#[test]
fn product_cognition_baseline_requires_factory_dna_and_post_training_readiness() {
    let evidence = run_product_cognition_baseline_curriculum(CurriculumConfig::default())
        .expect("product baseline runs");

    assert_eq!(evidence.product_baseline_pass, 1);
    assert_eq!(evidence.factory_dna_checkpoint_pass, 1);
    assert!(evidence.course_count >= 10);
    assert_eq!(evidence.passing_courses, evidence.course_count);
    assert!(evidence.object_relation_readiness >= 1);
    assert!(evidence.text_grounding_readiness >= 1);
    assert!(evidence.open_stream_readiness >= 1);
    assert!(evidence.cross_channel_readiness >= 1);
    assert!(evidence.active_probe_readiness >= 1);
    assert!(evidence.heldout_generalization_readiness >= 1);
    assert!(evidence.chain_memory_readiness >= 1);
    assert!(evidence.multi_step_readiness >= 1);
    assert!(evidence.long_horizon_readiness >= 1);
    assert!(evidence.min_reentry_margin > 0.0);
    assert!(evidence.min_hold_ratio >= 0.70);
    assert!(evidence.max_wrong_binding_strength <= 0.01);
    assert_eq!(evidence.no_child_ontology_guard, 1);
}

#[test]
fn product_cognition_baseline_sources_do_not_introduce_child_or_semantic_ontology() {
    let sources = [
        include_str!("../src/curriculum.rs"),
        include_str!("../src/curriculum_governor.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/mark_disturbance.rs"),
    ];

    for src in sources {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "meaning_db",
            "dictionary",
            "parser",
            "tokenizer",
            "child_model",
            "seven_year_old",
            "human_child",
            "personality",
            "ethics",
            "identity",
            "owner",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "product cognition baseline path must not contain `{}`",
                forbidden
            );
        }
    }
}
