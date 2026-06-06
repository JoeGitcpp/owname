use std::sync::Arc;

use bionic_v0::curriculum::{
    run_action_consequence_pulse_course, run_curriculum_episodes, run_curriculum_suite,
    run_long_retention_curriculum, run_persistent_curriculum_session,
    run_single_object_motion_course, run_two_object_relation_course, CurriculumConfig,
};
use bionic_v0::snapshot::{read_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn single_object_motion_course_produces_worldline_evidence() {
    let vp = Arc::new(Viewport::new());
    let dir = std::env::temp_dir().join(format!("bionic_v0_curriculum_single_motion_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);

    let summary = run_single_object_motion_course(&vp, CurriculumConfig::default(), Some(&paths))
        .expect("course runs");

    assert!(summary.steps_run >= 8);
    assert!(summary.worldline_updates >= 8);
    assert!(summary.final_worldline_err >= 0.0);
    assert!(summary.final_worldline_err <= 1.0);
    assert_eq!(summary.snapshots_written, 1);

    let snap = read_snapshot(&paths.final_path).expect("snapshot");
    let slot = snap
        .slots
        .iter()
        .find(|slot| slot.spatial_worldline.is_some())
        .expect("worldline visible in snapshot");
    let wl = slot.spatial_worldline.expect("worldline");
    assert!(wl.valid);
    assert!(wl.n_updates >= 8);
}

#[test]
fn two_object_relation_course_tracks_relation_projection_change() {
    let vp = Arc::new(Viewport::new());

    let summary = run_two_object_relation_course(&vp, CurriculumConfig::default(), None)
        .expect("course runs");

    assert!(summary.steps_run >= 2);
    assert!(summary.final_relation_distance_score > summary.initial_relation_distance_score);
    assert!(summary.final_relation_overlap_score > summary.initial_relation_overlap_score);
}

#[test]
fn action_consequence_pulse_course_forms_pair_and_learning_evidence() {
    let vp = Arc::new(Viewport::new());

    let summary = run_action_consequence_pulse_course(&vp, CurriculumConfig::default(), None)
        .expect("course runs");

    assert_eq!(summary.pairs_formed, 1);
    assert!(summary.learned_events >= 1);
    assert!(summary.learned_max_abs_delta > 0.0);
}

#[test]
fn curriculum_suite_runs_all_v0_courses_and_aggregates_metrics() {
    let summary = run_curriculum_suite(CurriculumConfig::default()).expect("suite runs");

    assert_eq!(summary.courses_run, 3);
    assert!(summary.total_steps_run >= 11);
    assert!(summary.worldline_updates >= 8);
    assert!(summary.final_worldline_err >= 0.0);
    assert!(summary.final_worldline_err <= 1.0);
    assert!(summary.final_relation_distance_score > summary.initial_relation_distance_score);
    assert!(summary.final_relation_overlap_score > summary.initial_relation_overlap_score);
    assert_eq!(summary.pairs_formed, 1);
    assert!(summary.learned_events >= 1);
    assert!(summary.learned_max_abs_delta > 0.0);
}

#[test]
fn curriculum_episode_sequence_reports_trend_metrics_across_repeated_suites() {
    let trend = run_curriculum_episodes(CurriculumConfig::default(), 4).expect("episodes run");

    assert_eq!(trend.episodes_run, 4);
    assert_eq!(trend.courses_run, 12);
    assert!(trend.total_steps_run >= 44);
    assert!(trend.mean_worldline_err >= 0.0);
    assert!(trend.mean_worldline_err <= 1.0);
    assert!(trend.first_worldline_err >= 0.0);
    assert!(trend.last_worldline_err >= 0.0);
    assert_eq!(trend.total_pairs_formed, 4);
    assert!(trend.total_learned_events >= 4);
    assert!(trend.max_learned_abs_delta > 0.0);
}

#[test]
fn persistent_curriculum_session_accumulates_evidence_in_one_body() {
    let summary =
        run_persistent_curriculum_session(CurriculumConfig::default(), 4).expect("session runs");

    assert_eq!(summary.episodes_run, 4);
    assert_eq!(summary.courses_run, 12);
    assert!(summary.total_steps_run >= 44);
    assert!(summary.worldline_updates >= 32);
    assert!(summary.mean_worldline_err >= 0.0);
    assert!(summary.mean_worldline_err <= 1.0);
    assert_eq!(summary.total_pairs_formed, 4);
    assert_eq!(summary.final_pairs, 4);
    assert!(summary.final_learned_events >= 4);
    assert!(summary.max_learned_abs_delta > 0.0);
}

#[test]
fn long_retention_curriculum_proves_hold_generalization_and_reentry_evidence() {
    let evidence =
        run_long_retention_curriculum(CurriculumConfig::default()).expect("long run completes");

    assert_eq!(evidence.train_episodes, 6);
    assert_eq!(evidence.after_episodes, 3);
    assert_eq!(evidence.reentry_episodes, 3);
    assert!(evidence.train_pairs >= 6);
    assert!(evidence.train_learned_events >= 6);
    assert!(evidence.train_adaptive_routes > 0);

    assert!(
        evidence.hold_learned_delta_ratio >= 0.80,
        "hold should preserve learned-bias evidence relative to train"
    );
    assert!(
        evidence.hold_adaptive_route_ratio >= 0.50,
        "hold should preserve observer-visible route traces relative to train"
    );

    assert!(
        evidence.after_persistent_learned_delta > evidence.after_cold_learned_delta,
        "related after-stage perturbation should retain stronger evidence than cold baseline"
    );
    assert!(
        evidence.reentry_persistent_learned_delta > evidence.reentry_cold_learned_delta,
        "reentry should close stronger than cold baseline after hold"
    );
    assert!(
        evidence.after_persistent_pairs > evidence.after_cold_pairs,
        "after-stage persistent pair closure should exceed cold baseline"
    );
    assert!(
        evidence.reentry_persistent_pairs > evidence.reentry_cold_pairs,
        "reentry persistent pair closure should exceed cold baseline"
    );
}

#[test]
fn curriculum_runner_stays_external_numeric_harness() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/curriculum.rs"))
        .expect("read curriculum.rs");

    for forbidden in [
        "semantic_readiness",
        "SemanticReadinessState",
        "symbolizer",
        "SymbolizerState",
        "concept_id",
        "ObjectConceptBinding",
        "RelationConceptBinding",
        "request_response",
        "answer_model",
        "target_label",
        "result_label",
        "personality",
        "owner",
        "ethics",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "curriculum runner must stay external numeric harness, found `{}`",
            forbidden
        );
    }
}
