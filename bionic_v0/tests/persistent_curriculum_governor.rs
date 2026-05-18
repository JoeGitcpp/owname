use bionic_v0::curriculum_governor::{
    run_persistent_curriculum_governor, PersistentCurriculumGovernorConfig,
};

#[test]
fn persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops() {
    let evidence =
        run_persistent_curriculum_governor(PersistentCurriculumGovernorConfig::default())
            .expect("governor runs");
    let default_config = PersistentCurriculumGovernorConfig::default();

    assert!(evidence.rounds_run >= 1);
    assert!(evidence.courses_evaluated >= 7);
    assert!(evidence.material_groups >= 8);
    assert_eq!(evidence.material_principle_pass, 1);
    assert_eq!(evidence.body_touch_points, 0);
    assert!(evidence.decision_trace_len >= 1);
    assert!(evidence.selected_course_kind > 0);
    assert!(evidence.selected_score >= 0.0 && evidence.selected_score <= 1.0);
    assert!(evidence.selected_threshold > 0.0 && evidence.selected_threshold <= 1.0);
    assert_eq!(
        evidence.passing_courses + evidence.failing_courses,
        evidence.courses_evaluated
    );
    assert_eq!(evidence.failing_courses, 0);
    assert_eq!(evidence.foundation_margin_floor, 0.10);
    assert_eq!(evidence.foundation_min_margin_ready, 1);
    assert_eq!(evidence.foundation_stage_ready, 1);
    assert_eq!(evidence.stopped_after_pass, 1);
    assert!(evidence.rounds_run <= default_config.max_rounds);
    assert!(evidence.min_margin_score > evidence.foundation_margin_floor);
}

#[test]
fn persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass() {
    let evidence =
        run_persistent_curriculum_governor(PersistentCurriculumGovernorConfig::default())
            .expect("governor runs");

    assert_eq!(evidence.failing_courses, 0);
    assert_eq!(evidence.selected_score, 1.0);
    assert!(evidence.selected_margin_score >= 0.0);
    assert!(evidence.selected_margin_score <= 1.0);
    assert_eq!(
        evidence.selected_course_kind,
        evidence.weakest_margin_course_kind
    );
    assert_eq!(
        evidence.selected_margin_score,
        evidence.weakest_margin_score
    );
    assert!(evidence.min_margin_score < evidence.max_margin_score);
    assert!(evidence.near_threshold_courses <= evidence.courses_evaluated);
    assert!(evidence.foundation_margin_floor > 0.0);
    assert!(evidence.foundation_margin_floor <= 1.0);
    assert!(evidence.foundation_text_grounding_ready <= 1);
    assert!(evidence.foundation_live_io_ready <= 1);
    assert!(evidence.foundation_cross_channel_ready <= 1);
    assert!(evidence.foundation_stage_ready <= 1);
    assert!(evidence.selected_margin_points_len > 0);
    assert!(evidence.selected_weakest_margin_point_kind > 0);
    assert_eq!(
        evidence.selected_weakest_margin_point_score,
        evidence.selected_margin_score
    );
    assert!(evidence
        .selected_margin_points
        .iter()
        .all(|point| point.course_kind == evidence.selected_course_kind));
}

#[test]
fn persistent_curriculum_governor_source_stays_external_numeric_and_guarded() {
    let sources = [
        include_str!("../src/curriculum_governor.rs"),
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
        "prompt_answer",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "println!(\"body",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "governor source contains rejected term: {term}"
            );
        }
    }
}
