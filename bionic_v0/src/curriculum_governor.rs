use crate::curriculum::{
    run_active_text_probe_closure_curriculum, run_cross_channel_disturbance_coupling_curriculum,
    run_long_retention_curriculum, run_mark_binding_curriculum, run_text_grounding_suite,
    run_text_open_stream_curriculum, CurriculumConfig,
};
use crate::curriculum_material::{
    default_external_curriculum_training_plan, review_external_curriculum_training_plan,
};

#[derive(Clone, Copy, Debug)]
pub struct PersistentCurriculumGovernorConfig {
    pub max_rounds: u32,
    pub foundation_margin_floor: f64,
}

impl Default for PersistentCurriculumGovernorConfig {
    fn default() -> Self {
        Self {
            max_rounds: 3,
            foundation_margin_floor: 0.10,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GovernorCourseKind {
    MaterialPlan = 1,
    LongRetention = 2,
    MarkBinding = 3,
    TextGrounding = 4,
    TextOpenStream = 5,
    ActiveTextProbeClosure = 6,
    CrossChannelCoupling = 7,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GovernorCourseReview {
    pub course_kind: u8,
    pub score: f64,
    pub margin_score: f64,
    pub threshold: f64,
    pub passed: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GovernorMarginPoint {
    pub course_kind: u8,
    pub point_index: u8,
    pub point_kind: u16,
    pub margin_score: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct GovernorCourseEvaluation {
    pub course_reviews: Vec<GovernorCourseReview>,
    pub margin_points: Vec<GovernorMarginPoint>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GovernorDecisionTrace {
    pub round_index: u32,
    pub selected_course_kind: u8,
    pub selected_score: f64,
    pub selected_margin_score: f64,
    pub weakest_margin_course_kind: u8,
    pub weakest_margin_score: f64,
    pub selected_threshold: f64,
    pub passing_courses: u32,
    pub failing_courses: u32,
    pub near_threshold_courses: u32,
    pub foundation_margin_floor: f64,
    pub foundation_min_margin_ready: u8,
    pub foundation_text_grounding_ready: u8,
    pub foundation_live_io_ready: u8,
    pub foundation_cross_channel_ready: u8,
    pub foundation_stage_ready: u8,
    pub stopped_after_pass: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PersistentCurriculumGovernorEvidence {
    pub rounds_run: u32,
    pub courses_evaluated: u32,
    pub material_groups: u32,
    pub material_principle_pass: u8,
    pub body_touch_points: u32,
    pub decision_trace_len: u32,
    pub selected_course_kind: u8,
    pub selected_score: f64,
    pub selected_margin_score: f64,
    pub weakest_margin_course_kind: u8,
    pub weakest_margin_score: f64,
    pub selected_threshold: f64,
    pub passing_courses: u32,
    pub failing_courses: u32,
    pub near_threshold_courses: u32,
    pub foundation_margin_floor: f64,
    pub foundation_min_margin_ready: u8,
    pub foundation_text_grounding_ready: u8,
    pub foundation_live_io_ready: u8,
    pub foundation_cross_channel_ready: u8,
    pub foundation_stage_ready: u8,
    pub stopped_after_pass: u8,
    pub min_score: f64,
    pub max_score: f64,
    pub min_margin_score: f64,
    pub max_margin_score: f64,
    pub selected_margin_points_len: u32,
    pub selected_weakest_margin_point_kind: u16,
    pub selected_weakest_margin_point_score: f64,
    pub course_reviews: Vec<GovernorCourseReview>,
    pub selected_margin_points: Vec<GovernorMarginPoint>,
    pub decision_trace: Vec<GovernorDecisionTrace>,
}

pub fn run_persistent_curriculum_governor(
    config: PersistentCurriculumGovernorConfig,
) -> std::io::Result<PersistentCurriculumGovernorEvidence> {
    let max_rounds = config.max_rounds.max(1);
    let foundation_margin_floor = unit(config.foundation_margin_floor);
    let material_plan = default_external_curriculum_training_plan();
    let material_review = review_external_curriculum_training_plan(&material_plan);
    let mut evidence = PersistentCurriculumGovernorEvidence {
        material_groups: material_review.material_groups,
        material_principle_pass: material_review.principle_review_pass,
        body_touch_points: material_review.body_touch_points,
        ..PersistentCurriculumGovernorEvidence::default()
    };

    for round_index in 0..max_rounds {
        let evaluation = evaluate_governor_courses(CurriculumConfig::default())?;
        let course_reviews = evaluation.course_reviews;
        let decision = decide_next_course(round_index, &course_reviews, foundation_margin_floor);
        let selected_margin_points =
            select_margin_points(&evaluation.margin_points, decision.selected_course_kind);
        let selected_weakest_margin_point = weakest_margin_point(&selected_margin_points);

        evidence.rounds_run = round_index + 1;
        evidence.courses_evaluated = course_reviews.len() as u32;
        evidence.decision_trace.push(decision);
        evidence.decision_trace_len = evidence.decision_trace.len() as u32;
        evidence.selected_course_kind = decision.selected_course_kind;
        evidence.selected_score = decision.selected_score;
        evidence.selected_margin_score = decision.selected_margin_score;
        evidence.weakest_margin_course_kind = decision.weakest_margin_course_kind;
        evidence.weakest_margin_score = decision.weakest_margin_score;
        evidence.selected_threshold = decision.selected_threshold;
        evidence.passing_courses = decision.passing_courses;
        evidence.failing_courses = decision.failing_courses;
        evidence.near_threshold_courses = decision.near_threshold_courses;
        evidence.foundation_margin_floor = decision.foundation_margin_floor;
        evidence.foundation_min_margin_ready = decision.foundation_min_margin_ready;
        evidence.foundation_text_grounding_ready = decision.foundation_text_grounding_ready;
        evidence.foundation_live_io_ready = decision.foundation_live_io_ready;
        evidence.foundation_cross_channel_ready = decision.foundation_cross_channel_ready;
        evidence.foundation_stage_ready = decision.foundation_stage_ready;
        evidence.stopped_after_pass = decision.stopped_after_pass;
        evidence.min_score = course_reviews
            .iter()
            .map(|review| review.score)
            .fold(1.0, f64::min);
        evidence.max_score = course_reviews
            .iter()
            .map(|review| review.score)
            .fold(0.0, f64::max);
        evidence.min_margin_score = course_reviews
            .iter()
            .map(|review| review.margin_score)
            .fold(1.0, f64::min);
        evidence.max_margin_score = course_reviews
            .iter()
            .map(|review| review.margin_score)
            .fold(0.0, f64::max);
        evidence.selected_margin_points_len = selected_margin_points.len() as u32;
        evidence.selected_weakest_margin_point_kind = selected_weakest_margin_point.point_kind;
        evidence.selected_weakest_margin_point_score = selected_weakest_margin_point.margin_score;
        evidence.course_reviews = course_reviews;
        evidence.selected_margin_points = selected_margin_points;

        if decision.stopped_after_pass == 1 {
            break;
        }
    }

    Ok(evidence)
}

fn evaluate_governor_courses(
    config: CurriculumConfig,
) -> std::io::Result<GovernorCourseEvaluation> {
    let material_plan = default_external_curriculum_training_plan();
    let material = review_external_curriculum_training_plan(&material_plan);
    let long_retention = run_long_retention_curriculum(config)?;
    let mark_binding = run_mark_binding_curriculum(config)?;
    let text_grounding = run_text_grounding_suite(config)?;
    let text_open = run_text_open_stream_curriculum(config)?;
    let active_probe = run_active_text_probe_closure_curriculum(config)?;
    let cross_channel = run_cross_channel_disturbance_coupling_curriculum(config)?;

    let mut course_reviews = Vec::new();
    let mut margin_points = Vec::new();

    let mut push_review = |review: GovernorCourseReview, points: Vec<GovernorMarginPoint>| {
        course_reviews.push(review);
        margin_points.extend(points);
    };

    let (review, points) = review_with_margins(
        GovernorCourseKind::MaterialPlan,
        gate_score(&[
            material.material_groups >= 8,
            material.principle_review_pass == 1,
            material.body_touch_points == 0,
            material.missing_required_roles == 0,
        ]),
        &[
            min_margin(material.material_groups as f64, 8.0),
            min_margin(material.carrier_kinds as f64, 4.0),
            min_margin(material.stage_kinds as f64, 6.0),
            flag_margin(material.principle_review_pass == 1),
            zero_guard_margin(material.body_touch_points),
            zero_guard_margin(material.missing_required_roles),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::LongRetention,
        gate_score(&[
            long_retention.train_pairs > 0,
            long_retention.train_learned_events > 0,
            long_retention.hold_learned_delta_ratio >= 0.50,
            long_retention.hold_adaptive_route_ratio >= 0.50,
            long_retention.after_persistent_pairs >= long_retention.after_cold_pairs,
            long_retention.reentry_persistent_pairs >= long_retention.reentry_cold_pairs,
        ]),
        &[
            min_margin(long_retention.train_learned_delta, 0.0),
            min_margin(long_retention.hold_learned_delta_ratio, 0.50),
            min_margin(long_retention.hold_adaptive_route_ratio, 0.50),
            separation_margin(
                long_retention.after_persistent_pairs as f64,
                long_retention.after_cold_pairs as f64,
            ),
            separation_margin(
                long_retention.reentry_persistent_pairs as f64,
                long_retention.reentry_cold_pairs as f64,
            ),
            separation_margin(
                long_retention.after_persistent_learned_delta,
                long_retention.after_cold_learned_delta,
            ),
            separation_margin(
                long_retention.reentry_persistent_learned_delta,
                long_retention.reentry_cold_learned_delta,
            ),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::MarkBinding,
        gate_score(&[
            mark_binding.train_recurrence >= 4,
            mark_binding.train_binding_strength > 0.5,
            mark_binding.hold_binding_strength_ratio >= 0.80,
            mark_binding.reentry_persistent_strength > mark_binding.reentry_cold_strength,
            mark_binding.final_ambiguity <= mark_binding.train_ambiguity + 0.25,
        ]),
        &[
            min_margin(mark_binding.train_binding_strength, 0.50),
            min_margin(mark_binding.hold_binding_strength_ratio, 0.80),
            separation_margin(
                mark_binding.reentry_persistent_strength,
                mark_binding.reentry_cold_strength,
            ),
            max_margin(
                mark_binding.final_ambiguity,
                mark_binding.train_ambiguity + 0.25,
            ),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::TextGrounding,
        gate_score(&[
            text_grounding.train_marks >= 6,
            text_grounding.object_targets >= 3,
            text_grounding.relation_targets >= 2,
            text_grounding.time_window_targets >= 1,
            text_grounding.train_avg_strength > 0.5,
            text_grounding.correction_recovery_ratio >= 0.50,
            text_grounding.wrong_binding_strength <= 0.01,
            text_grounding.reentry_persistent_avg > text_grounding.reentry_cold_avg,
        ]),
        &[
            min_margin(text_grounding.train_avg_strength, 0.50),
            min_margin(text_grounding.train_weakest_strength, 0.25),
            min_margin(text_grounding.correction_recovery_ratio, 0.50),
            max_margin(text_grounding.wrong_binding_strength, 0.01),
            separation_margin(
                text_grounding.reentry_persistent_avg,
                text_grounding.reentry_cold_avg,
            ),
            separation_margin(
                text_grounding.reentry_persistent_weakest,
                text_grounding.reentry_cold_weakest,
            ),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::TextOpenStream,
        gate_score(&[
            text_open.stream_events >= 20,
            text_open.supported_marks >= 6,
            text_open.unrelated_marks >= 4,
            text_open.train_supported_avg > 0.5,
            text_open.unrelated_leakage_strength <= 0.01,
            text_open.correction_recovery_ratio >= 0.50,
            text_open.reentry_persistent_avg > text_open.reentry_cold_avg,
        ]),
        &[
            min_margin(text_open.train_supported_avg, 0.50),
            min_margin(text_open.train_supported_weakest, 0.25),
            max_margin(text_open.unrelated_leakage_strength, 0.01),
            min_margin(text_open.hold_supported_ratio, 0.50),
            separation_margin(text_open.variant_persistent_avg, text_open.variant_cold_avg),
            min_margin(text_open.delayed_binding_strength, 0.25),
            min_margin(text_open.composition_relation_strength, 0.25),
            min_margin(text_open.correction_recovery_ratio, 0.50),
            separation_margin(text_open.reentry_persistent_avg, text_open.reentry_cold_avg),
            separation_margin(
                text_open.reentry_persistent_weakest,
                text_open.reentry_cold_weakest,
            ),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::ActiveTextProbeClosure,
        gate_score(&[
            active_probe.stream_events >= 20,
            active_probe.probe_trigger_events >= 2,
            active_probe.probe_frames >= 2,
            active_probe.paired_consequences >= 2,
            active_probe.ambiguity_reduction_ratio > 0.10,
            active_probe.correction_recovery_ratio >= 0.50,
            active_probe.reentry_persistent_strength > active_probe.reentry_cold_strength,
        ]),
        &[
            min_margin(active_probe.train_supported_avg, 0.50),
            min_margin(active_probe.probe_pair_similarity_avg, 0.25),
            min_margin(active_probe.ambiguity_reduction_ratio, 0.10),
            max_margin(active_probe.wrong_binding_strength, 0.01),
            min_margin(active_probe.correction_recovery_ratio, 0.50),
            min_margin(active_probe.egress_projection_confidence, 0.25),
            separation_margin(
                active_probe.reentry_persistent_strength,
                active_probe.reentry_cold_strength,
            ),
        ],
    );
    push_review(review, points);

    let (review, points) = review_with_margins(
        GovernorCourseKind::CrossChannelCoupling,
        gate_score(&[
            cross_channel.stream_events >= 18,
            cross_channel.text_marks >= 4,
            cross_channel.vib_observations >= 4,
            cross_channel.phot_worldline_updates >= 4,
            cross_channel.text_vib_coupling_strength > 0.0,
            cross_channel.text_phot_coupling_strength > 0.0,
            cross_channel.vib_phot_coupling_strength > 0.0,
            cross_channel.conflict_coupling_drop > 0.0,
            cross_channel.wrong_coupling_strength <= 0.01,
            cross_channel.correction_recovery_ratio >= 0.50,
            cross_channel.reentry_persistent_coupling > cross_channel.reentry_cold_coupling,
        ]),
        &[
            min_margin(cross_channel.text_binding_strength, 0.25),
            min_margin(cross_channel.vib_slot_confidence, 0.25),
            min_margin(cross_channel.phot_spatial_confidence, 0.25),
            min_margin(cross_channel.text_vib_coupling_strength, 0.0),
            min_margin(cross_channel.text_phot_coupling_strength, 0.0),
            min_margin(cross_channel.vib_phot_coupling_strength, 0.0),
            min_margin(cross_channel.conflict_coupling_drop, 0.0),
            max_margin(cross_channel.wrong_coupling_strength, 0.01),
            min_margin(cross_channel.correction_recovery_ratio, 0.50),
            separation_margin(
                cross_channel.reentry_persistent_coupling,
                cross_channel.reentry_cold_coupling,
            ),
        ],
    );
    push_review(review, points);

    Ok(GovernorCourseEvaluation {
        course_reviews,
        margin_points,
    })
}

fn decide_next_course(
    round_index: u32,
    course_reviews: &[GovernorCourseReview],
    foundation_margin_floor: f64,
) -> GovernorDecisionTrace {
    let weakest_margin = weakest_margin_course(course_reviews);
    let passing_courses = course_reviews
        .iter()
        .filter(|review| review.passed == 1)
        .count() as u32;
    let failing_courses = course_reviews.len() as u32 - passing_courses;
    let selected = if failing_courses == 0 {
        weakest_margin
    } else {
        course_reviews
            .iter()
            .copied()
            .min_by(|lhs, rhs| {
                compare_unit(lhs.score, rhs.score)
                    .then_with(|| compare_unit(lhs.margin_score, rhs.margin_score))
                    .then_with(|| lhs.course_kind.cmp(&rhs.course_kind))
            })
            .unwrap_or_default()
    };
    let near_threshold_courses = course_reviews
        .iter()
        .filter(|review| review.margin_score <= foundation_margin_floor)
        .count() as u32;
    let foundation_min_margin_ready =
        readiness_flag(weakest_margin.margin_score > foundation_margin_floor);
    let foundation_text_grounding_ready =
        course_passed(course_reviews, GovernorCourseKind::TextGrounding);
    let foundation_live_io_ready =
        course_passed(course_reviews, GovernorCourseKind::ActiveTextProbeClosure);
    let foundation_cross_channel_ready =
        course_passed(course_reviews, GovernorCourseKind::CrossChannelCoupling);
    let foundation_stage_ready = readiness_flag(
        failing_courses == 0
            && foundation_min_margin_ready == 1
            && foundation_text_grounding_ready == 1
            && foundation_live_io_ready == 1
            && foundation_cross_channel_ready == 1,
    );

    GovernorDecisionTrace {
        round_index,
        selected_course_kind: selected.course_kind,
        selected_score: selected.score,
        selected_margin_score: selected.margin_score,
        weakest_margin_course_kind: weakest_margin.course_kind,
        weakest_margin_score: weakest_margin.margin_score,
        selected_threshold: selected.threshold,
        passing_courses,
        failing_courses,
        near_threshold_courses,
        foundation_margin_floor,
        foundation_min_margin_ready,
        foundation_text_grounding_ready,
        foundation_live_io_ready,
        foundation_cross_channel_ready,
        foundation_stage_ready,
        stopped_after_pass: foundation_stage_ready,
    }
}

fn review(course_kind: GovernorCourseKind, score: f64, margin_score: f64) -> GovernorCourseReview {
    let score = unit(score);
    GovernorCourseReview {
        course_kind: course_kind as u8,
        score,
        margin_score: unit(margin_score),
        threshold: 1.0,
        passed: if score >= 1.0 { 1 } else { 0 },
    }
}

fn review_with_margins(
    course_kind: GovernorCourseKind,
    score: f64,
    margins: &[f64],
) -> (GovernorCourseReview, Vec<GovernorMarginPoint>) {
    let points = margin_points(course_kind, margins);
    let review = review(course_kind, score, course_margin(margins));
    (review, points)
}

fn margin_points(course_kind: GovernorCourseKind, margins: &[f64]) -> Vec<GovernorMarginPoint> {
    let course_code = course_kind as u8;
    margins
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, value)| {
            let point_index = (idx + 1) as u8;
            GovernorMarginPoint {
                course_kind: course_code,
                point_index,
                point_kind: course_code as u16 * 16 + point_index as u16,
                margin_score: unit(value),
            }
        })
        .collect()
}

fn gate_score(gates: &[bool]) -> f64 {
    if gates.is_empty() {
        return 0.0;
    }
    gates.iter().filter(|gate| **gate).count() as f64 / gates.len() as f64
}

fn unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn weakest_margin_course(course_reviews: &[GovernorCourseReview]) -> GovernorCourseReview {
    course_reviews
        .iter()
        .copied()
        .min_by(|lhs, rhs| {
            compare_unit(lhs.margin_score, rhs.margin_score)
                .then_with(|| lhs.course_kind.cmp(&rhs.course_kind))
        })
        .unwrap_or_default()
}

fn select_margin_points(
    margin_points: &[GovernorMarginPoint],
    course_kind: u8,
) -> Vec<GovernorMarginPoint> {
    margin_points
        .iter()
        .copied()
        .filter(|point| point.course_kind == course_kind)
        .collect()
}

fn weakest_margin_point(margin_points: &[GovernorMarginPoint]) -> GovernorMarginPoint {
    margin_points
        .iter()
        .copied()
        .min_by(|lhs, rhs| {
            compare_unit(lhs.margin_score, rhs.margin_score)
                .then_with(|| lhs.point_kind.cmp(&rhs.point_kind))
        })
        .unwrap_or_default()
}

fn course_passed(course_reviews: &[GovernorCourseReview], kind: GovernorCourseKind) -> u8 {
    course_reviews
        .iter()
        .find(|review| review.course_kind == kind as u8)
        .map(|review| review.passed)
        .unwrap_or(0)
}

fn readiness_flag(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

fn compare_unit(lhs: f64, rhs: f64) -> std::cmp::Ordering {
    lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
}

fn course_margin(points: &[f64]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    points.iter().copied().map(unit).fold(1.0, f64::min)
}

fn min_margin(value: f64, floor: f64) -> f64 {
    if !value.is_finite() || !floor.is_finite() {
        return 0.0;
    }
    if floor <= 0.0 {
        unit(value)
    } else {
        unit((value - floor) / floor.abs().max(f64::EPSILON))
    }
}

fn max_margin(value: f64, ceiling: f64) -> f64 {
    if !value.is_finite() || !ceiling.is_finite() {
        return 0.0;
    }
    if ceiling <= 0.0 {
        if value <= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        unit((ceiling - value) / ceiling.abs().max(f64::EPSILON))
    }
}

fn separation_margin(left: f64, right: f64) -> f64 {
    if !left.is_finite() || !right.is_finite() {
        return 0.0;
    }
    let scale = left.abs().max(right.abs()).max(1.0);
    unit((left - right) / scale)
}

fn flag_margin(passed: bool) -> f64 {
    if passed {
        1.0
    } else {
        0.0
    }
}

fn zero_guard_margin(value: u32) -> f64 {
    if value == 0 {
        1.0
    } else {
        0.0
    }
}
