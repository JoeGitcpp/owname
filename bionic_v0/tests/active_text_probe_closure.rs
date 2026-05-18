use bionic_v0::curriculum::{
    run_active_text_probe_closure_curriculum, run_sustained_active_probe_training_curriculum,
    CurriculumConfig,
};

#[test]
fn active_text_probe_closure_proves_probe_return_correction_and_reentry() {
    let evidence = run_active_text_probe_closure_curriculum(CurriculumConfig::default())
        .expect("active text probe closure runs");

    assert!(evidence.stream_events >= 20);
    assert!(evidence.supported_marks >= 6);
    assert!(evidence.train_supported_avg > 0.5);
    assert!(evidence.conflict_events >= 2);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.probe_trigger_events >= 2);
    assert!(evidence.probe_frames >= 2);
    assert!(evidence.probe_write_epochs >= 2);
    assert!(evidence.return_windows >= 2);
    assert!(evidence.paired_consequences >= 2);
    assert!(evidence.probe_pair_similarity_avg > 0.0);
    assert!(evidence.pre_probe_ambiguity > evidence.post_probe_ambiguity);
    assert!(evidence.ambiguity_reduction_ratio > 0.10);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.post_probe_supported_strength >= evidence.pre_probe_supported_strength);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.egress_projection_confidence > 0.0);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
}

#[test]
fn sustained_active_probe_training_proves_repeated_probe_return_correction_and_reentry() {
    let evidence = run_sustained_active_probe_training_curriculum(CurriculumConfig::default())
        .expect("sustained active probe training runs");

    assert!(evidence.cycles_run >= 12);
    assert!(evidence.materials_seen >= 10);
    assert!(evidence.conflict_events >= 24);
    assert!(evidence.probe_frames >= 24);
    assert!(evidence.probe_write_epochs >= 24);
    assert!(evidence.return_windows >= 24);
    assert!(evidence.paired_consequences >= 24);
    assert!(evidence.replay_ingress_frames >= 6);
    assert!(evidence.live_return_ingress_frames >= 24);
    assert!(evidence.cross_channel_vib_observations >= 12);
    assert!(evidence.cross_channel_phot_worldline_updates >= 3);
    assert!(evidence.cross_channel_coupling_strength > 0.0);
    assert!(evidence.probe_pair_similarity_avg > 0.50);
    assert!(evidence.min_ambiguity_reduction_ratio > 0.10);
    assert!(evidence.min_correction_recovery_ratio >= 0.80);
    assert!(evidence.max_wrong_binding_strength <= 0.01);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
    assert!(evidence.persistent_reentry_margin > 0.10);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn active_text_probe_closure_sources_do_not_introduce_rejected_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/framed_byte_egress.rs"),
        include_str!("../src/action_output.rs"),
        include_str!("../src/action_consequence.rs"),
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
        "request_response",
        "answer_model",
        "target_label",
        "result_label",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected active text probe term: {term}"
            );
        }
    }
}
