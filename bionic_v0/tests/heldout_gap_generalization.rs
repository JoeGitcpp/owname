use bionic_v0::curriculum::{run_heldout_gap_generalization_curriculum, CurriculumConfig};

#[test]
fn heldout_gap_generalization_proves_pressure_probe_closure_and_reentry() {
    let evidence = run_heldout_gap_generalization_curriculum(CurriculumConfig::default())
        .expect("held-out gap generalization curriculum");

    assert!(evidence.trained_materials >= 10);
    assert!(evidence.heldout_materials >= 6);
    assert_eq!(evidence.gap_pressure_rises_under_heldout_conflict, 1);
    assert!(evidence.gap_pressure_after_heldout > evidence.gap_pressure_before_heldout);
    assert!(evidence.gap_pressure_after_closure < evidence.gap_pressure_after_heldout);
    assert_eq!(evidence.probe_count_rises_under_gap, 1);
    assert_eq!(evidence.probe_count_falls_after_closure, 1);
    assert!(evidence.probe_writes_during_gap > evidence.probe_writes_before_gap);
    assert!(evidence.probe_writes_after_closure < evidence.probe_writes_during_gap);
    assert!(evidence.persistent_heldout_strength > evidence.cold_heldout_strength);
    assert!(evidence.persistent_reentry_margin > 0.10);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn heldout_gap_generalization_source_stays_numeric_and_guarded() {
    let source = include_str!("../src/curriculum.rs");
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
        "image_label",
        "voice_label",
        "prompt_answer",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "println!(\"body",
        "eprintln!",
    ] {
        assert!(
            !source.contains(forbidden),
            "curriculum source contains rejected term: {forbidden}"
        );
    }
}
