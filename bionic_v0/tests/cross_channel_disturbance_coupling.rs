use bionic_v0::curriculum::{run_cross_channel_disturbance_coupling_curriculum, CurriculumConfig};

#[test]
fn cross_channel_disturbance_coupling_proves_shared_targets_conflict_recovery_and_reentry() {
    let evidence = run_cross_channel_disturbance_coupling_curriculum(CurriculumConfig::default())
        .expect("cross-channel disturbance coupling runs");

    assert!(evidence.stream_events >= 18);
    assert!(evidence.text_marks >= 4);
    assert!(evidence.vib_observations >= 4);
    assert!(evidence.phot_worldline_updates >= 4);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 2);
    assert!(evidence.text_binding_strength > 0.5);
    assert!(evidence.vib_slot_confidence > 0.0);
    assert!(evidence.phot_spatial_confidence > 0.0);
    assert!(evidence.text_vib_coupling_strength > 0.0);
    assert!(evidence.text_phot_coupling_strength > 0.0);
    assert!(evidence.vib_phot_coupling_strength > 0.0);
    assert!(evidence.conflict_coupling_drop > 0.0);
    assert!(evidence.wrong_coupling_strength <= 0.01);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.reentry_persistent_coupling > evidence.reentry_cold_coupling);
}

#[test]
fn cross_channel_disturbance_coupling_sources_do_not_introduce_rejected_ontology() {
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
        "image_label",
        "voice_label",
        "multimodal_semantic",
        "identity",
        "owner",
        "ethics",
        "personality",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected cross-channel term: {term}"
            );
        }
    }
}
