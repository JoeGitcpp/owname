use bionic_v0::curriculum::{run_factory_dna_checkpoint_curriculum, CurriculumConfig};

#[test]
fn factory_dna_checkpoint_proves_birth_structure_pitch_visual_and_cross_channel_readiness() {
    let evidence =
        run_factory_dna_checkpoint_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.dna_checkpoint_pass, 1);
    assert!(evidence.digital_structure_events >= 12);
    assert!(evidence.character_digit_mark_traces >= 6);
    assert!(evidence.visual_marker_events >= 8);
    assert!(evidence.acoustic_low_band_events >= 4);
    assert!(evidence.acoustic_high_band_events >= 4);
    assert!(evidence.pitch_band_separation_margin > 0.20);
    assert!(evidence.cross_channel_pairs >= 8);
    assert!(evidence.cross_channel_coupling_strength > 0.0);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.hold_retention_ratio >= 0.80);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
    assert_eq!(evidence.no_fixed_ontology_guard, 1);
}

#[test]
fn factory_dna_checkpoint_sources_do_not_introduce_fixed_ontology() {
    let sources = [
        include_str!("../src/curriculum.rs"),
        include_str!("../src/evolution_priors.rs"),
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
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
            "phoneme_inventory",
            "visual_label",
            "speech_recognizer",
            "digit_meaning",
            "character_meaning",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "factory DNA checkpoint path must not contain `{}`",
                forbidden
            );
        }
    }
}
