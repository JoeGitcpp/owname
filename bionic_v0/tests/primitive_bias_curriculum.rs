use bionic_v0::curriculum::{run_tri_modal_primitive_bias_curriculum, CurriculumConfig};

#[test]
fn tri_modal_primitive_curriculum_holds_and_reenters_against_cold_baseline() {
    let evidence =
        run_tri_modal_primitive_bias_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.train_episodes, 4);
    assert_eq!(evidence.reentry_episodes, 2);
    assert!(evidence.vib_observations >= evidence.train_episodes + evidence.reentry_episodes);
    assert!(evidence.phot_worldline_updates >= evidence.train_episodes);
    assert!(evidence.mark_recurrence >= evidence.train_episodes);
    assert!(evidence.train_binding_strength > 0.5);
    assert!(
        evidence.hold_binding_strength_ratio >= 0.80,
        "hold should preserve most tri-modal mark/object routing strength"
    );
    assert!(
        evidence.reentry_persistent_strength > evidence.reentry_cold_strength,
        "reentry should recover stronger than a fresh cold baseline"
    );
    assert!(evidence.final_ambiguity <= evidence.train_ambiguity + 0.25);
}

#[test]
fn primitive_bias_sources_do_not_introduce_rejected_ontology() {
    let sources = [
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
                "primitive bias path must not contain `{}`",
                forbidden
            );
        }
    }
}
