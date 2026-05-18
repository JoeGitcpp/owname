use bionic_v0::curriculum::{run_multi_gap_selection_pixel_scale_curriculum, CurriculumConfig};

#[test]
fn multi_gap_selection_pixel_scale_proves_pressure_selection_and_scale_persistence() {
    let evidence = run_multi_gap_selection_pixel_scale_curriculum(CurriculumConfig::default())
        .expect("multi-gap selection pixel-scale curriculum");

    assert!(evidence.small_gap_events >= 4);
    assert!(evidence.mid_gap_events >= 4);
    assert!(evidence.large_gap_events >= 4);
    assert!(evidence.competing_gaps >= 3);
    assert_eq!(evidence.selected_gap_pressure_is_max, 1);
    assert_eq!(evidence.probe_bias_tracks_selected_gap, 1);
    assert!(evidence.probe_writes_after_selection > evidence.probe_writes_before_selection);
    assert!(evidence.small_gap_closure_ratio > 0.40);
    assert!(evidence.large_gap_persistence_score > evidence.small_gap_persistence_score);
    assert!(evidence.large_gap_persistent_strength > evidence.large_gap_cold_strength);
    assert!(evidence.large_gap_reentry_margin > 0.10);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn multi_gap_selection_pixel_scale_source_stays_numeric_and_guarded() {
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
