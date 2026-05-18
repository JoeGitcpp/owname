use std::path::PathBuf;

use bionic_v0::curriculum::{
    run_active_surface_aperture_echo_curriculum, run_active_text_probe_closure_curriculum,
    run_auditory_visual_predictability_expression_curriculum,
    run_background_relation_resonance_curriculum, run_chain_memory_sedimentation_curriculum,
    run_cross_channel_disturbance_coupling_curriculum, run_curriculum_episodes,
    run_factory_dna_checkpoint_curriculum, run_heldout_gap_generalization_curriculum,
    run_local_probe_chain_curriculum, run_long_horizon_pressure_chain_curriculum,
    run_long_retention_curriculum, run_mark_binding_curriculum,
    run_multi_gap_selection_pixel_scale_curriculum, run_multi_step_chain_continuity_curriculum,
    run_persistent_curriculum_session, run_prediction_pressure_field_body_integration_curriculum,
    run_prediction_pressure_field_water_wave_curriculum, run_product_cognition_baseline_curriculum,
    run_soft_arbitration_drift_under_load_curriculum,
    run_sustained_active_probe_training_curriculum, run_text_disturbance_curriculum,
    run_text_grounding_suite, run_text_open_stream_curriculum,
    run_tri_modal_primitive_bias_curriculum, run_vocal_reflex_suppression_curriculum,
    CurriculumConfig,
};
use bionic_v0::curriculum_governor::{
    run_persistent_curriculum_governor, PersistentCurriculumGovernorConfig,
};
use bionic_v0::curriculum_material::{
    default_external_curriculum_training_plan, review_external_curriculum_training_plan,
};
use bionic_v0::live_human_nursery::{
    sample_live_human_nursery_snapshots, summarize_live_human_nursery,
};
use bionic_v0::snapshot::read_snapshot;

fn parse_args() -> (
    u32,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
) {
    let mut args = std::env::args().skip(1);
    let mut episodes = 3;
    let mut persistent = false;
    let mut long_retention = false;
    let mut mark_binding = false;
    let mut tri_modal_primitive = false;
    let mut factory_dna_checkpoint = false;
    let mut product_cognition_baseline = false;
    let mut text_disturbance = false;
    let mut text_grounding_suite = false;
    let mut text_open_stream = false;
    let mut active_text_probe_closure = false;
    let mut sustained_active_probe_training = false;
    let mut heldout_gap_generalization = false;
    let mut multi_gap_selection_pixel_scale = false;
    let mut prediction_pressure_field_water_wave = false;
    let mut prediction_pressure_field_body_integration = false;
    let mut active_surface_aperture_echo = false;
    let mut background_relation_resonance = false;
    let mut local_probe_chain = false;
    let mut chain_memory_sedimentation = false;
    let mut multi_step_chain_continuity = false;
    let mut soft_arbitration_drift_under_load = false;
    let mut long_horizon_pressure_chain = false;
    let mut vocal_reflex_suppression = false;
    let mut auditory_visual_predictability_expression = false;
    let mut cross_channel_disturbance_coupling = false;
    let mut external_curriculum_plan = false;
    let mut persistent_curriculum_governor = false;
    let mut live_human_nursery = false;
    while let Some(arg) = args.next() {
        if arg == "--episodes" {
            if let Some(value) = args.next().and_then(|s| s.parse::<u32>().ok()) {
                episodes = value.max(1);
            }
        } else if arg == "--persistent" {
            persistent = true;
        } else if arg == "--long-retention" {
            long_retention = true;
        } else if arg == "--mark-binding" {
            mark_binding = true;
        } else if arg == "--tri-modal-primitive" {
            tri_modal_primitive = true;
        } else if arg == "--factory-dna-checkpoint" {
            factory_dna_checkpoint = true;
        } else if arg == "--product-cognition-baseline" {
            product_cognition_baseline = true;
        } else if arg == "--text-disturbance" {
            text_disturbance = true;
        } else if arg == "--text-grounding-suite" {
            text_grounding_suite = true;
        } else if arg == "--text-open-stream" {
            text_open_stream = true;
        } else if arg == "--active-text-probe-closure" {
            active_text_probe_closure = true;
        } else if arg == "--sustained-active-probe-training" {
            sustained_active_probe_training = true;
        } else if arg == "--heldout-gap-generalization" {
            heldout_gap_generalization = true;
        } else if arg == "--multi-gap-selection-pixel-scale" {
            multi_gap_selection_pixel_scale = true;
        } else if arg == "--prediction-pressure-field-water-wave" {
            prediction_pressure_field_water_wave = true;
        } else if arg == "--prediction-pressure-field-body-integration" {
            prediction_pressure_field_body_integration = true;
        } else if arg == "--active-surface-aperture-echo" {
            active_surface_aperture_echo = true;
        } else if arg == "--background-relation-resonance-sudden-match" {
            background_relation_resonance = true;
        } else if arg == "--local-probe-chain" {
            local_probe_chain = true;
        } else if arg == "--chain-memory-sedimentation" {
            chain_memory_sedimentation = true;
        } else if arg == "--multi-step-chain-continuity" {
            multi_step_chain_continuity = true;
        } else if arg == "--soft-arbitration-drift-under-load" {
            soft_arbitration_drift_under_load = true;
        } else if arg == "--long-horizon-pressure-chain" {
            long_horizon_pressure_chain = true;
        } else if arg == "--vocal-reflex-suppression" {
            vocal_reflex_suppression = true;
        } else if arg == "--auditory-visual-predictability-expression" {
            auditory_visual_predictability_expression = true;
        } else if arg == "--cross-channel-disturbance-coupling" {
            cross_channel_disturbance_coupling = true;
        } else if arg == "--external-curriculum-plan" {
            external_curriculum_plan = true;
        } else if arg == "--persistent-curriculum-governor" {
            persistent_curriculum_governor = true;
        } else if arg == "--live-human-nursery" {
            live_human_nursery = true;
        }
    }
    (
        episodes,
        persistent,
        long_retention,
        mark_binding,
        tri_modal_primitive,
        factory_dna_checkpoint,
        product_cognition_baseline,
        text_disturbance,
        text_grounding_suite,
        text_open_stream,
        active_text_probe_closure,
        sustained_active_probe_training,
        heldout_gap_generalization,
        multi_gap_selection_pixel_scale,
        prediction_pressure_field_water_wave,
        prediction_pressure_field_body_integration,
        active_surface_aperture_echo,
        background_relation_resonance,
        local_probe_chain,
        chain_memory_sedimentation,
        multi_step_chain_continuity,
        soft_arbitration_drift_under_load,
        long_horizon_pressure_chain,
        vocal_reflex_suppression,
        auditory_visual_predictability_expression,
        cross_channel_disturbance_coupling,
        external_curriculum_plan,
        persistent_curriculum_governor,
        live_human_nursery,
    )
}

fn live_human_nursery_runtime_dir_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg == "--live-human-nursery" {
            if let Some(next) = args.peek() {
                if !next.starts_with("--") {
                    return args.next().map(PathBuf::from);
                }
            }
            return None;
        }
    }
    None
}

fn main() -> std::io::Result<()> {
    let (
        episodes,
        persistent,
        long_retention,
        mark_binding,
        tri_modal_primitive,
        factory_dna_checkpoint,
        product_cognition_baseline,
        text_disturbance,
        text_grounding_suite,
        text_open_stream,
        active_text_probe_closure,
        sustained_active_probe_training,
        heldout_gap_generalization,
        multi_gap_selection_pixel_scale,
        prediction_pressure_field_water_wave,
        prediction_pressure_field_body_integration,
        active_surface_aperture_echo,
        background_relation_resonance,
        local_probe_chain,
        chain_memory_sedimentation,
        multi_step_chain_continuity,
        soft_arbitration_drift_under_load,
        long_horizon_pressure_chain,
        vocal_reflex_suppression,
        auditory_visual_predictability_expression,
        cross_channel_disturbance_coupling,
        external_curriculum_plan,
        persistent_curriculum_governor,
        live_human_nursery,
    ) = parse_args();

    println!("curriculum_report_v0");
    if persistent_curriculum_governor {
        let evidence =
            run_persistent_curriculum_governor(PersistentCurriculumGovernorConfig::default())?;
        println!("mode=persistent_curriculum_governor");
        println!("rounds_run={}", evidence.rounds_run);
        println!("courses_evaluated={}", evidence.courses_evaluated);
        println!("material_groups={}", evidence.material_groups);
        println!(
            "material_principle_pass={}",
            evidence.material_principle_pass
        );
        println!("decision_trace_len={}", evidence.decision_trace_len);
        println!("selected_course_kind={}", evidence.selected_course_kind);
        println!("selected_score={:.6}", evidence.selected_score);
        println!(
            "selected_margin_score={:.6}",
            evidence.selected_margin_score
        );
        println!(
            "weakest_margin_course_kind={}",
            evidence.weakest_margin_course_kind
        );
        println!("weakest_margin_score={:.6}", evidence.weakest_margin_score);
        println!("selected_threshold={:.6}", evidence.selected_threshold);
        println!("passing_courses={}", evidence.passing_courses);
        println!("failing_courses={}", evidence.failing_courses);
        println!("near_threshold_courses={}", evidence.near_threshold_courses);
        println!(
            "foundation_margin_floor={:.6}",
            evidence.foundation_margin_floor
        );
        println!(
            "foundation_min_margin_ready={}",
            evidence.foundation_min_margin_ready
        );
        println!(
            "foundation_text_grounding_ready={}",
            evidence.foundation_text_grounding_ready
        );
        println!(
            "foundation_live_io_ready={}",
            evidence.foundation_live_io_ready
        );
        println!(
            "foundation_cross_channel_ready={}",
            evidence.foundation_cross_channel_ready
        );
        println!("foundation_stage_ready={}", evidence.foundation_stage_ready);
        println!(
            "selected_margin_points_len={}",
            evidence.selected_margin_points_len
        );
        println!(
            "selected_weakest_margin_point_kind={}",
            evidence.selected_weakest_margin_point_kind
        );
        println!(
            "selected_weakest_margin_point_score={:.6}",
            evidence.selected_weakest_margin_point_score
        );
        for (idx, point) in evidence.selected_margin_points.iter().enumerate() {
            println!("selected_margin_point_{idx:02}_kind={}", point.point_kind);
            println!(
                "selected_margin_point_{idx:02}_score={:.6}",
                point.margin_score
            );
        }
        println!("stopped_after_pass={}", evidence.stopped_after_pass);
        println!("{}={}", "body_touch_points", evidence.body_touch_points);
        println!("min_score={:.6}", evidence.min_score);
        println!("max_score={:.6}", evidence.max_score);
        println!("min_margin_score={:.6}", evidence.min_margin_score);
        println!("max_margin_score={:.6}", evidence.max_margin_score);
    } else if external_curriculum_plan {
        let plan = default_external_curriculum_training_plan();
        let review = review_external_curriculum_training_plan(&plan);
        println!("mode=external_curriculum_plan");
        println!("material_groups={}", review.material_groups);
        println!("synthetic_groups={}", review.synthetic_groups);
        println!("replay_groups={}", review.replay_groups);
        println!("probe_return_groups={}", review.probe_return_groups);
        println!("live_ingress_groups={}", review.live_ingress_groups);
        println!("carrier_kinds={}", review.carrier_kinds);
        println!("stage_kinds={}", review.stage_kinds);
        println!("reproducible_groups={}", review.reproducible_groups);
        println!("observer_only_groups={}", review.observer_only_groups);
        println!("{}={}", "body_touch_points", review.body_touch_points);
        println!("missing_required_roles={}", review.missing_required_roles);
        println!("principle_review_pass={}", review.principle_review_pass);
    } else if cross_channel_disturbance_coupling {
        let evidence =
            run_cross_channel_disturbance_coupling_curriculum(CurriculumConfig::default())?;
        println!("mode=cross_channel_disturbance_coupling");
        println!("stream_events={}", evidence.stream_events);
        println!("text_marks={}", evidence.text_marks);
        println!("vib_observations={}", evidence.vib_observations);
        println!("phot_worldline_updates={}", evidence.phot_worldline_updates);
        println!("object_targets={}", evidence.object_targets);
        println!("relation_targets={}", evidence.relation_targets);
        println!("time_window_targets={}", evidence.time_window_targets);
        println!(
            "text_binding_strength={:.6}",
            evidence.text_binding_strength
        );
        println!("vib_slot_confidence={:.6}", evidence.vib_slot_confidence);
        println!(
            "phot_spatial_confidence={:.6}",
            evidence.phot_spatial_confidence
        );
        println!(
            "text_vib_coupling_strength={:.6}",
            evidence.text_vib_coupling_strength
        );
        println!(
            "text_phot_coupling_strength={:.6}",
            evidence.text_phot_coupling_strength
        );
        println!(
            "vib_phot_coupling_strength={:.6}",
            evidence.vib_phot_coupling_strength
        );
        println!(
            "conflict_coupling_drop={:.6}",
            evidence.conflict_coupling_drop
        );
        println!(
            "wrong_coupling_strength={:.6}",
            evidence.wrong_coupling_strength
        );
        println!(
            "correction_recovery_ratio={:.6}",
            evidence.correction_recovery_ratio
        );
        println!(
            "reentry_persistent_coupling={:.6}",
            evidence.reentry_persistent_coupling
        );
        println!(
            "reentry_cold_coupling={:.6}",
            evidence.reentry_cold_coupling
        );
    } else if active_text_probe_closure {
        let evidence = run_active_text_probe_closure_curriculum(CurriculumConfig::default())?;
        println!("mode=active_text_probe_closure");
        println!("stream_events={}", evidence.stream_events);
        println!("supported_marks={}", evidence.supported_marks);
        println!("train_supported_avg={:.6}", evidence.train_supported_avg);
        println!("conflict_events={}", evidence.conflict_events);
        println!(
            "conflict_ambiguity_delta={:.6}",
            evidence.conflict_ambiguity_delta
        );
        println!("probe_trigger_events={}", evidence.probe_trigger_events);
        println!("probe_frames={}", evidence.probe_frames);
        println!("probe_write_epochs={}", evidence.probe_write_epochs);
        println!("return_windows={}", evidence.return_windows);
        println!("paired_consequences={}", evidence.paired_consequences);
        println!(
            "probe_pair_similarity_avg={:.6}",
            evidence.probe_pair_similarity_avg
        );
        println!("pre_probe_ambiguity={:.6}", evidence.pre_probe_ambiguity);
        println!("post_probe_ambiguity={:.6}", evidence.post_probe_ambiguity);
        println!(
            "ambiguity_reduction_ratio={:.6}",
            evidence.ambiguity_reduction_ratio
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!(
            "pre_probe_supported_strength={:.6}",
            evidence.pre_probe_supported_strength
        );
        println!(
            "post_probe_supported_strength={:.6}",
            evidence.post_probe_supported_strength
        );
        println!(
            "correction_recovery_ratio={:.6}",
            evidence.correction_recovery_ratio
        );
        println!(
            "egress_projection_confidence={:.6}",
            evidence.egress_projection_confidence
        );
        println!(
            "reentry_persistent_strength={:.6}",
            evidence.reentry_persistent_strength
        );
        println!(
            "reentry_cold_strength={:.6}",
            evidence.reentry_cold_strength
        );
    } else if background_relation_resonance {
        let evidence = run_background_relation_resonance_curriculum(CurriculumConfig::default())?;
        println!("mode=background_relation_resonance_sudden_match");
        println!(
            "background_attempts_alive={}",
            evidence.background_attempts_alive
        );
        println!(
            "low_match_attempts_decay={}",
            evidence.low_match_attempts_decay
        );
        println!(
            "high_match_spike_observed={}",
            evidence.high_match_spike_observed
        );
        println!(
            "sudden_match_enters_surface={}",
            evidence.sudden_match_enters_surface
        );
        println!("wrong_match_leakage={:.6}", evidence.wrong_match_leakage);
        println!(
            "aperture_relation_entry={}",
            evidence.aperture_relation_entry
        );
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if local_probe_chain {
        let evidence = run_local_probe_chain_curriculum(CurriculumConfig::default())?;
        println!("mode=local_probe_chain");
        println!("chain_steps={}", evidence.chain_steps);
        println!("closure_steps={}", evidence.closure_steps);
        println!("mismatch_steps={}", evidence.mismatch_steps);
        println!(
            "return_revises_surface_pressure={}",
            evidence.return_revises_surface_pressure
        );
        println!(
            "closed_focus_does_not_repeat_probe={}",
            evidence.closed_focus_does_not_repeat_probe
        );
        println!(
            "unclosed_focus_continues_probe={}",
            evidence.unclosed_focus_continues_probe
        );
        println!(
            "next_probe_pressure_after_mismatch={:.6}",
            evidence.next_probe_pressure_after_mismatch
        );
        println!(
            "closure_pressure_after_return={:.6}",
            evidence.closure_pressure_after_return
        );
        println!("wrong_chain_binding={:.6}", evidence.wrong_chain_binding);
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if chain_memory_sedimentation {
        let evidence = run_chain_memory_sedimentation_curriculum(CurriculumConfig::default())?;
        println!("mode=chain_memory_sedimentation");
        println!(
            "chain_memory_traces_alive={}",
            evidence.chain_memory_traces_alive
        );
        println!("sediment_updates={}", evidence.sediment_updates);
        println!(
            "closure_sediment_strength={:.6}",
            evidence.closure_sediment_strength
        );
        println!(
            "mismatch_sediment_strength={:.6}",
            evidence.mismatch_sediment_strength
        );
        println!("reentry_pressure={:.6}", evidence.reentry_pressure);
        println!(
            "sediment_reentry_observed={}",
            evidence.sediment_reentry_observed
        );
        println!(
            "persistent_reentry_pressure={:.6}",
            evidence.persistent_reentry_pressure
        );
        println!(
            "cold_reentry_pressure={:.6}",
            evidence.cold_reentry_pressure
        );
        println!(
            "persistent_reentry_margin={:.6}",
            evidence.persistent_reentry_margin
        );
        println!(
            "duplicate_pair_sediment_guard={}",
            evidence.duplicate_pair_sediment_guard
        );
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if multi_step_chain_continuity {
        let evidence = run_multi_step_chain_continuity_curriculum(CurriculumConfig::default())?;
        println!("mode=multi_step_chain_continuity");
        println!("chain_steps={}", evidence.chain_steps);
        println!(
            "same_chain_continuity_steps={}",
            evidence.same_chain_continuity_steps
        );
        println!(
            "dominant_continuity_score={:.6}",
            evidence.dominant_continuity_score
        );
        println!("parallel_branch_count={}", evidence.parallel_branch_count);
        println!("branch_pressure={:.6}", evidence.branch_pressure);
        println!(
            "single_aperture_preserved={}",
            evidence.single_aperture_preserved
        );
        println!("branch_trace_retained={}", evidence.branch_trace_retained);
        println!(
            "dominant_chain_reentry_pressure={:.6}",
            evidence.dominant_chain_reentry_pressure
        );
        println!("wrong_chain_binding={:.6}", evidence.wrong_chain_binding);
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if soft_arbitration_drift_under_load {
        let evidence =
            run_soft_arbitration_drift_under_load_curriculum(CurriculumConfig::default())?;
        println!("mode=soft_arbitration_drift_under_load");
        println!("load_ticks={}", evidence.load_ticks);
        println!("hold_count={}", evidence.hold_count);
        println!("switch_count={}", evidence.switch_count);
        println!("reentry_count={}", evidence.reentry_count);
        println!("competition_pressure={:.6}", evidence.competition_pressure);
        println!("challenger_pressure={:.6}", evidence.challenger_pressure);
        println!("arbitration_margin={:.6}", evidence.arbitration_margin);
        println!(
            "branch_pressure_retained={:.6}",
            evidence.branch_pressure_retained
        );
        println!(
            "single_aperture_preserved={}",
            evidence.single_aperture_preserved
        );
        println!(
            "dominant_chain_reentered={}",
            evidence.dominant_chain_reentered
        );
        println!("wrong_chain_binding={:.6}", evidence.wrong_chain_binding);
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if long_horizon_pressure_chain {
        let evidence = run_long_horizon_pressure_chain_curriculum(CurriculumConfig::default())?;
        println!("mode=long_horizon_pressure_chain");
        println!("long_chain_steps={}", evidence.long_chain_steps);
        println!("long_chain_span_us={}", evidence.long_chain_span_us);
        println!("long_chain_score={:.6}", evidence.long_chain_score);
        println!("interruption_count={}", evidence.interruption_count);
        println!("recovery_count={}", evidence.recovery_count);
        println!(
            "interruption_branch_pressure={:.6}",
            evidence.interruption_branch_pressure
        );
        println!(
            "delayed_reentry_pressure={:.6}",
            evidence.delayed_reentry_pressure
        );
        println!(
            "single_aperture_preserved={}",
            evidence.single_aperture_preserved
        );
        println!("branch_trace_retained={}", evidence.branch_trace_retained);
        println!("wrong_chain_binding={:.6}", evidence.wrong_chain_binding);
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if vocal_reflex_suppression {
        let evidence = run_vocal_reflex_suppression_curriculum(CurriculumConfig::default())?;
        println!("mode=vocal_reflex_suppression");
        println!("shock_events={}", evidence.shock_events);
        println!("reflex_pressure_peak={:.6}", evidence.reflex_pressure_peak);
        println!("vocal_burst_epochs={}", evidence.vocal_burst_epochs);
        println!(
            "familiarity_after_repeats={:.6}",
            evidence.familiarity_after_repeats
        );
        println!("suppression_strength={:.6}", evidence.suppression_strength);
        println!(
            "reflex_pressure_after_habituation={:.6}",
            evidence.reflex_pressure_after_habituation
        );
        println!(
            "quiet_regulation_successes={}",
            evidence.quiet_regulation_successes
        );
        println!(
            "urgent_pressure_bypasses_suppression={}",
            evidence.urgent_pressure_bypasses_suppression
        );
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if auditory_visual_predictability_expression {
        let evidence =
            run_auditory_visual_predictability_expression_curriculum(CurriculumConfig::default())?;
        println!("mode=auditory_visual_predictability_expression");
        println!("vocal_writes={}", evidence.vocal_writes);
        println!("vib_returns={}", evidence.vib_returns);
        println!(
            "predictable_return_strength={:.6}",
            evidence.predictable_return_strength
        );
        println!("return_closure_count={}", evidence.return_closure_count);
        println!("aligned_av_events={}", evidence.aligned_av_events);
        println!(
            "cross_channel_predictability={:.6}",
            evidence.cross_channel_predictability
        );
        println!(
            "drift_pressure_after_misalignment={:.6}",
            evidence.drift_pressure_after_misalignment
        );
        println!(
            "expressive_pressure_peak={:.6}",
            evidence.expressive_pressure_peak
        );
        println!(
            "expression_write_epochs={}",
            evidence.expression_write_epochs
        );
        println!(
            "closure_reduces_expression_pressure={}",
            evidence.closure_reduces_expression_pressure
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if active_surface_aperture_echo {
        let evidence = run_active_surface_aperture_echo_curriculum(CurriculumConfig::default())?;
        println!("mode=active_surface_aperture_echo");
        println!(
            "parallel_focus_traces_alive={}",
            evidence.parallel_focus_traces_alive
        );
        println!("active_aperture_count={}", evidence.active_aperture_count);
        println!("echo_focus_present={}", evidence.echo_focus_present);
        println!(
            "echo_pressure_below_aperture={}",
            evidence.echo_pressure_below_aperture
        );
        println!(
            "dominant_focus_enters_aperture={}",
            evidence.dominant_focus_enters_aperture
        );
        println!(
            "non_aperture_focus_retained={}",
            evidence.non_aperture_focus_retained
        );
        println!(
            "suppressed_focus_can_reenter={}",
            evidence.suppressed_focus_can_reenter
        );
        println!(
            "closure_reduces_aperture_focus_without_deleting_trace={}",
            evidence.closure_reduces_aperture_focus_without_deleting_trace
        );
        println!(
            "event_pressure_can_displace_aperture={}",
            evidence.event_pressure_can_displace_aperture
        );
        println!(
            "displaced_focus_retains_residue={}",
            evidence.displaced_focus_retains_residue
        );
        println!(
            "probe_drive_tracks_aperture_with_echo={}",
            evidence.probe_drive_tracks_aperture_with_echo
        );
        println!(
            "aperture_switches_bounded={}",
            evidence.aperture_switches_bounded
        );
        let body_order_key = "body_source_order_guard";
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if prediction_pressure_field_body_integration {
        let evidence =
            run_prediction_pressure_field_body_integration_curriculum(CurriculumConfig::default())?;
        println!("mode=prediction_pressure_field_body_integration");
        let body_ticks_key = "body_ticks_run";
        let body_order_key = "body_source_order_guard";
        println!("{body_ticks_key}={}", evidence.body_ticks_run);
        println!(
            "quiet_probe_write_epochs={}",
            evidence.quiet_probe_write_epochs
        );
        println!(
            "event_probe_write_epochs={}",
            evidence.event_probe_write_epochs
        );
        println!("waves_alive={}", evidence.waves_alive);
        println!(
            "dominant_focus_is_single={}",
            evidence.dominant_focus_is_single
        );
        println!("gap_wave_observed={}", evidence.gap_wave_observed);
        println!("event_wave_observed={}", evidence.event_wave_observed);
        println!("relation_wave_observed={}", evidence.relation_wave_observed);
        println!("residue_wave_observed={}", evidence.residue_wave_observed);
        println!(
            "closure_reduces_focused_wave={}",
            evidence.closure_reduces_focused_wave
        );
        println!("bounded_wave_count={}", evidence.bounded_wave_count);
        println!("focus_switches_bounded={}", evidence.focus_switches_bounded);
        println!("stress_ticks={}", evidence.stress_ticks);
        println!(
            "max_dominant_pressure={:.6}",
            evidence.max_dominant_pressure
        );
        println!("{body_order_key}={}", evidence.body_source_order_guard);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if prediction_pressure_field_water_wave {
        let evidence =
            run_prediction_pressure_field_water_wave_curriculum(CurriculumConfig::default())?;
        println!("mode=prediction_pressure_field_water_wave");
        println!("waves_alive={}", evidence.waves_alive);
        println!(
            "dominant_focus_is_single={}",
            evidence.dominant_focus_is_single
        );
        println!("focus_switches_bounded={}", evidence.focus_switches_bounded);
        println!(
            "quiet_reentry_wave_rises={}",
            evidence.quiet_reentry_wave_rises
        );
        println!(
            "event_wave_can_override_idle_wave={}",
            evidence.event_wave_can_override_idle_wave
        );
        println!(
            "relation_resonance_spike_observed={}",
            evidence.relation_resonance_spike_observed
        );
        println!(
            "closure_reduces_focused_wave={}",
            evidence.closure_reduces_focused_wave
        );
        println!(
            "unclosed_wave_residue_retained={}",
            evidence.unclosed_wave_residue_retained
        );
        println!(
            "dominant_pressure_drives_probe={}",
            evidence.dominant_pressure_drives_probe
        );
        println!("dominant_pressure={:.6}", evidence.dominant_pressure);
        println!("focus_switches={}", evidence.focus_switches);
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if multi_gap_selection_pixel_scale {
        let evidence = run_multi_gap_selection_pixel_scale_curriculum(CurriculumConfig::default())?;
        println!("mode=multi_gap_selection_pixel_scale");
        println!("small_gap_events={}", evidence.small_gap_events);
        println!("mid_gap_events={}", evidence.mid_gap_events);
        println!("large_gap_events={}", evidence.large_gap_events);
        println!("competing_gaps={}", evidence.competing_gaps);
        println!("small_gap_pressure={:.6}", evidence.small_gap_pressure);
        println!("mid_gap_pressure={:.6}", evidence.mid_gap_pressure);
        println!("large_gap_pressure={:.6}", evidence.large_gap_pressure);
        println!("selected_gap_index={}", evidence.selected_gap_index);
        println!(
            "selected_gap_pressure={:.6}",
            evidence.selected_gap_pressure
        );
        println!(
            "selected_gap_pressure_is_max={}",
            evidence.selected_gap_pressure_is_max
        );
        println!(
            "probe_writes_before_selection={}",
            evidence.probe_writes_before_selection
        );
        println!(
            "probe_writes_after_selection={}",
            evidence.probe_writes_after_selection
        );
        println!(
            "probe_bias_tracks_selected_gap={}",
            evidence.probe_bias_tracks_selected_gap
        );
        println!(
            "small_gap_closure_ratio={:.6}",
            evidence.small_gap_closure_ratio
        );
        println!(
            "small_gap_persistence_score={:.6}",
            evidence.small_gap_persistence_score
        );
        println!(
            "large_gap_persistence_score={:.6}",
            evidence.large_gap_persistence_score
        );
        println!(
            "large_gap_persistent_strength={:.6}",
            evidence.large_gap_persistent_strength
        );
        println!(
            "large_gap_cold_strength={:.6}",
            evidence.large_gap_cold_strength
        );
        println!(
            "large_gap_reentry_margin={:.6}",
            evidence.large_gap_reentry_margin
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if heldout_gap_generalization {
        let evidence = run_heldout_gap_generalization_curriculum(CurriculumConfig::default())?;
        println!("mode=heldout_gap_generalization");
        println!("trained_materials={}", evidence.trained_materials);
        println!("heldout_materials={}", evidence.heldout_materials);
        println!(
            "gap_pressure_before_heldout={:.6}",
            evidence.gap_pressure_before_heldout
        );
        println!(
            "gap_pressure_after_heldout={:.6}",
            evidence.gap_pressure_after_heldout
        );
        println!(
            "gap_pressure_after_closure={:.6}",
            evidence.gap_pressure_after_closure
        );
        println!(
            "gap_pressure_rises_under_heldout_conflict={}",
            evidence.gap_pressure_rises_under_heldout_conflict
        );
        println!(
            "probe_writes_before_gap={}",
            evidence.probe_writes_before_gap
        );
        println!(
            "probe_writes_during_gap={}",
            evidence.probe_writes_during_gap
        );
        println!(
            "probe_writes_after_closure={}",
            evidence.probe_writes_after_closure
        );
        println!(
            "probe_count_rises_under_gap={}",
            evidence.probe_count_rises_under_gap
        );
        println!(
            "probe_count_falls_after_closure={}",
            evidence.probe_count_falls_after_closure
        );
        println!(
            "persistent_heldout_strength={:.6}",
            evidence.persistent_heldout_strength
        );
        println!(
            "cold_heldout_strength={:.6}",
            evidence.cold_heldout_strength
        );
        println!(
            "persistent_reentry_margin={:.6}",
            evidence.persistent_reentry_margin
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if sustained_active_probe_training {
        let evidence = run_sustained_active_probe_training_curriculum(CurriculumConfig::default())?;
        println!("mode=sustained_active_probe_training");
        println!("cycles_run={}", evidence.cycles_run);
        println!("materials_seen={}", evidence.materials_seen);
        println!("conflict_events={}", evidence.conflict_events);
        println!("probe_frames={}", evidence.probe_frames);
        println!("probe_write_epochs={}", evidence.probe_write_epochs);
        println!("return_windows={}", evidence.return_windows);
        println!("paired_consequences={}", evidence.paired_consequences);
        println!("replay_ingress_frames={}", evidence.replay_ingress_frames);
        println!(
            "live_return_ingress_frames={}",
            evidence.live_return_ingress_frames
        );
        println!(
            "cross_channel_vib_observations={}",
            evidence.cross_channel_vib_observations
        );
        println!(
            "cross_channel_phot_worldline_updates={}",
            evidence.cross_channel_phot_worldline_updates
        );
        println!(
            "cross_channel_coupling_strength={:.6}",
            evidence.cross_channel_coupling_strength
        );
        println!(
            "probe_pair_similarity_avg={:.6}",
            evidence.probe_pair_similarity_avg
        );
        println!(
            "min_ambiguity_reduction_ratio={:.6}",
            evidence.min_ambiguity_reduction_ratio
        );
        println!(
            "min_correction_recovery_ratio={:.6}",
            evidence.min_correction_recovery_ratio
        );
        println!(
            "max_wrong_binding_strength={:.6}",
            evidence.max_wrong_binding_strength
        );
        println!(
            "reentry_persistent_strength={:.6}",
            evidence.reentry_persistent_strength
        );
        println!(
            "reentry_cold_strength={:.6}",
            evidence.reentry_cold_strength
        );
        println!(
            "persistent_reentry_margin={:.6}",
            evidence.persistent_reentry_margin
        );
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if text_open_stream {
        let evidence = run_text_open_stream_curriculum(CurriculumConfig::default())?;
        println!("mode=text_open_stream");
        println!("stream_events={}", evidence.stream_events);
        println!("supported_marks={}", evidence.supported_marks);
        println!("unrelated_marks={}", evidence.unrelated_marks);
        println!("variant_marks={}", evidence.variant_marks);
        println!("object_targets={}", evidence.object_targets);
        println!("relation_targets={}", evidence.relation_targets);
        println!("time_window_targets={}", evidence.time_window_targets);
        println!("composition_windows={}", evidence.composition_windows);
        println!("delayed_windows={}", evidence.delayed_windows);
        println!("train_supported_avg={:.6}", evidence.train_supported_avg);
        println!(
            "train_supported_weakest={:.6}",
            evidence.train_supported_weakest
        );
        println!(
            "unrelated_leakage_strength={:.6}",
            evidence.unrelated_leakage_strength
        );
        println!("hold_supported_ratio={:.6}", evidence.hold_supported_ratio);
        println!(
            "variant_persistent_avg={:.6}",
            evidence.variant_persistent_avg
        );
        println!("variant_cold_avg={:.6}", evidence.variant_cold_avg);
        println!(
            "delayed_binding_strength={:.6}",
            evidence.delayed_binding_strength
        );
        println!(
            "composition_relation_strength={:.6}",
            evidence.composition_relation_strength
        );
        println!(
            "conflict_ambiguity_delta={:.6}",
            evidence.conflict_ambiguity_delta
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!(
            "correction_recovery_ratio={:.6}",
            evidence.correction_recovery_ratio
        );
        println!(
            "reentry_persistent_avg={:.6}",
            evidence.reentry_persistent_avg
        );
        println!("reentry_cold_avg={:.6}", evidence.reentry_cold_avg);
        println!(
            "reentry_persistent_weakest={:.6}",
            evidence.reentry_persistent_weakest
        );
        println!("reentry_cold_weakest={:.6}", evidence.reentry_cold_weakest);
    } else if text_grounding_suite {
        let evidence = run_text_grounding_suite(CurriculumConfig::default())?;
        println!("mode=text_grounding_suite");
        println!("train_marks={}", evidence.train_marks);
        println!("object_targets={}", evidence.object_targets);
        println!("relation_targets={}", evidence.relation_targets);
        println!("time_window_targets={}", evidence.time_window_targets);
        println!("variant_marks={}", evidence.variant_marks);
        println!("composition_windows={}", evidence.composition_windows);
        println!("train_avg_strength={:.6}", evidence.train_avg_strength);
        println!(
            "train_weakest_strength={:.6}",
            evidence.train_weakest_strength
        );
        println!("hold_avg_ratio={:.6}", evidence.hold_avg_ratio);
        println!("hold_weakest_ratio={:.6}", evidence.hold_weakest_ratio);
        println!(
            "variant_generalization_score={:.6}",
            evidence.variant_generalization_score
        );
        println!(
            "variant_distinctness_ratio={:.6}",
            evidence.variant_distinctness_ratio
        );
        println!(
            "composition_relation_strength={:.6}",
            evidence.composition_relation_strength
        );
        println!(
            "conflict_ambiguity_delta={:.6}",
            evidence.conflict_ambiguity_delta
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!(
            "correction_recovery_ratio={:.6}",
            evidence.correction_recovery_ratio
        );
        println!(
            "reentry_persistent_avg={:.6}",
            evidence.reentry_persistent_avg
        );
        println!("reentry_cold_avg={:.6}", evidence.reentry_cold_avg);
        println!(
            "reentry_persistent_weakest={:.6}",
            evidence.reentry_persistent_weakest
        );
        println!("reentry_cold_weakest={:.6}", evidence.reentry_cold_weakest);
    } else if text_disturbance {
        let evidence = run_text_disturbance_curriculum(CurriculumConfig::default())?;
        println!("mode=text_disturbance");
        println!("train_episodes={}", evidence.train_episodes);
        println!("reentry_episodes={}", evidence.reentry_episodes);
        println!("distinct_text_traces={}", evidence.distinct_text_traces);
        println!("a_recurrence={}", evidence.a_recurrence);
        println!("b_recurrence={}", evidence.b_recurrence);
        println!("a_to_slot_a_strength={:.6}", evidence.a_to_slot_a_strength);
        println!("b_to_slot_b_strength={:.6}", evidence.b_to_slot_b_strength);
        println!("a_to_slot_b_strength={:.6}", evidence.a_to_slot_b_strength);
        println!("hold_avg_strength={:.6}", evidence.hold_avg_strength);
        println!(
            "hold_avg_strength_ratio={:.6}",
            evidence.hold_avg_strength_ratio
        );
        println!(
            "conflict_ambiguity_delta={:.6}",
            evidence.conflict_ambiguity_delta
        );
        println!(
            "reentry_persistent_avg_strength={:.6}",
            evidence.reentry_persistent_avg_strength
        );
        println!(
            "reentry_cold_avg_strength={:.6}",
            evidence.reentry_cold_avg_strength
        );
        println!("time_window_strength={:.6}", evidence.time_window_strength);
    } else if live_human_nursery {
        let runtime_dir_arg = live_human_nursery_runtime_dir_arg();
        let deterministic_fixture = if runtime_dir_arg.is_some() { 0 } else { 1 };
        let runtime_snapshot_read = if runtime_dir_arg.is_some() { 1 } else { 0 };
        let snapshots = if let Some(runtime_dir) = runtime_dir_arg {
            vec![read_snapshot(&runtime_dir.join("snapshot.bin"))?]
        } else {
            sample_live_human_nursery_snapshots()
        };
        let evidence = summarize_live_human_nursery(snapshots.iter());
        println!("mode=live_human_nursery");
        println!("deterministic_fixture={deterministic_fixture}");
        println!("runtime_snapshot_read={runtime_snapshot_read}");
        println!("snapshots_seen={}", evidence.snapshots_seen);
        println!("phot_health_observed={}", evidence.phot_health_observed);
        println!("phot_fault_observed={}", evidence.phot_fault_observed);
        println!("vib_activity_observed={}", evidence.vib_activity_observed);
        println!(
            "phot_structure_observed={}",
            evidence.phot_structure_observed
        );
        println!(
            "vocal_activity_observed={}",
            evidence.vocal_activity_observed
        );
        println!(
            "body_write_out_observed={}",
            evidence.body_write_out_observed
        );
        println!("vocal_write_epochs={}", evidence.vocal_write_epochs);
        println!("return_windows={}", evidence.return_windows);
        println!(
            "temporal_pairing_observed={}",
            evidence.temporal_pairing_observed
        );
        println!(
            "pairing_or_unpaired_boundary_observed={}",
            evidence.pairing_or_unpaired_boundary_observed
        );
        println!(
            "closure_or_reentry_observed={}",
            evidence.closure_or_reentry_observed
        );
        println!(
            "organic_surface_nonblank={}",
            evidence.organic_surface_nonblank
        );
        println!("teacher_phase_count={}", evidence.teacher_phase_count);
        println!(
            "communication_candidate={}",
            evidence.communication_candidate
        );
        println!("principle_guard_pass={}", evidence.principle_guard_pass);
    } else if product_cognition_baseline {
        let evidence = run_product_cognition_baseline_curriculum(CurriculumConfig::default())?;
        println!("mode=product_cognition_baseline");
        println!(
            "factory_dna_checkpoint_pass={}",
            evidence.factory_dna_checkpoint_pass
        );
        println!("course_count={}", evidence.course_count);
        println!("passing_courses={}", evidence.passing_courses);
        println!(
            "object_relation_readiness={}",
            evidence.object_relation_readiness
        );
        println!(
            "text_grounding_readiness={}",
            evidence.text_grounding_readiness
        );
        println!("open_stream_readiness={}", evidence.open_stream_readiness);
        println!(
            "cross_channel_readiness={}",
            evidence.cross_channel_readiness
        );
        println!("active_probe_readiness={}", evidence.active_probe_readiness);
        println!(
            "heldout_generalization_readiness={}",
            evidence.heldout_generalization_readiness
        );
        println!("chain_memory_readiness={}", evidence.chain_memory_readiness);
        println!("multi_step_readiness={}", evidence.multi_step_readiness);
        println!("long_horizon_readiness={}", evidence.long_horizon_readiness);
        println!("min_reentry_margin={:.6}", evidence.min_reentry_margin);
        println!("min_hold_ratio={:.6}", evidence.min_hold_ratio);
        println!(
            "max_wrong_binding_strength={:.6}",
            evidence.max_wrong_binding_strength
        );
        println!(
            "no_child_ontology_guard={}",
            evidence.no_child_ontology_guard
        );
        println!("product_baseline_pass={}", evidence.product_baseline_pass);
    } else if factory_dna_checkpoint {
        let evidence = run_factory_dna_checkpoint_curriculum(CurriculumConfig::default())?;
        println!("mode=factory_dna_checkpoint");
        println!(
            "digital_structure_events={}",
            evidence.digital_structure_events
        );
        println!(
            "character_digit_mark_traces={}",
            evidence.character_digit_mark_traces
        );
        println!("visual_marker_events={}", evidence.visual_marker_events);
        println!(
            "acoustic_low_band_events={}",
            evidence.acoustic_low_band_events
        );
        println!(
            "acoustic_high_band_events={}",
            evidence.acoustic_high_band_events
        );
        println!(
            "pitch_band_separation_margin={:.6}",
            evidence.pitch_band_separation_margin
        );
        println!("cross_channel_pairs={}", evidence.cross_channel_pairs);
        println!(
            "cross_channel_coupling_strength={:.6}",
            evidence.cross_channel_coupling_strength
        );
        println!(
            "conflict_ambiguity_delta={:.6}",
            evidence.conflict_ambiguity_delta
        );
        println!(
            "wrong_binding_strength={:.6}",
            evidence.wrong_binding_strength
        );
        println!("hold_retention_ratio={:.6}", evidence.hold_retention_ratio);
        println!(
            "reentry_persistent_strength={:.6}",
            evidence.reentry_persistent_strength
        );
        println!(
            "reentry_cold_strength={:.6}",
            evidence.reentry_cold_strength
        );
        println!(
            "no_fixed_ontology_guard={}",
            evidence.no_fixed_ontology_guard
        );
        println!("dna_checkpoint_pass={}", evidence.dna_checkpoint_pass);
    } else if tri_modal_primitive {
        let evidence = run_tri_modal_primitive_bias_curriculum(CurriculumConfig::default())?;
        println!("mode=tri_modal_primitive");
        println!("train_episodes={}", evidence.train_episodes);
        println!("reentry_episodes={}", evidence.reentry_episodes);
        println!("vib_observations={}", evidence.vib_observations);
        println!("phot_worldline_updates={}", evidence.phot_worldline_updates);
        println!("mark_recurrence={}", evidence.mark_recurrence);
        println!(
            "train_binding_strength={:.6}",
            evidence.train_binding_strength
        );
        println!("train_ambiguity={:.6}", evidence.train_ambiguity);
        println!(
            "hold_binding_strength={:.6}",
            evidence.hold_binding_strength
        );
        println!(
            "hold_binding_strength_ratio={:.6}",
            evidence.hold_binding_strength_ratio
        );
        println!(
            "reentry_persistent_strength={:.6}",
            evidence.reentry_persistent_strength
        );
        println!(
            "reentry_cold_strength={:.6}",
            evidence.reentry_cold_strength
        );
        println!("final_ambiguity={:.6}", evidence.final_ambiguity);
    } else if mark_binding {
        let evidence = run_mark_binding_curriculum(CurriculumConfig::default())?;
        println!("mode=mark_binding");
        println!("train_episodes={}", evidence.train_episodes);
        println!("reentry_episodes={}", evidence.reentry_episodes);
        println!("train_recurrence={}", evidence.train_recurrence);
        println!(
            "train_binding_strength={:.6}",
            evidence.train_binding_strength
        );
        println!("train_ambiguity={:.6}", evidence.train_ambiguity);
        println!(
            "hold_binding_strength={:.6}",
            evidence.hold_binding_strength
        );
        println!(
            "hold_binding_strength_ratio={:.6}",
            evidence.hold_binding_strength_ratio
        );
        println!(
            "reentry_persistent_strength={:.6}",
            evidence.reentry_persistent_strength
        );
        println!(
            "reentry_cold_strength={:.6}",
            evidence.reentry_cold_strength
        );
        println!("final_ambiguity={:.6}", evidence.final_ambiguity);
    } else if long_retention {
        let evidence = run_long_retention_curriculum(CurriculumConfig::default())?;
        println!("mode=long_retention");
        println!("train_episodes={}", evidence.train_episodes);
        println!("after_episodes={}", evidence.after_episodes);
        println!("reentry_episodes={}", evidence.reentry_episodes);
        println!("train_pairs={}", evidence.train_pairs);
        println!("train_learned_events={}", evidence.train_learned_events);
        println!("train_learned_delta={:.6}", evidence.train_learned_delta);
        println!("train_adaptive_routes={}", evidence.train_adaptive_routes);
        println!("hold_learned_delta={:.6}", evidence.hold_learned_delta);
        println!("hold_adaptive_routes={}", evidence.hold_adaptive_routes);
        println!(
            "hold_learned_delta_ratio={:.6}",
            evidence.hold_learned_delta_ratio
        );
        println!(
            "hold_adaptive_route_ratio={:.6}",
            evidence.hold_adaptive_route_ratio
        );
        println!("after_persistent_pairs={}", evidence.after_persistent_pairs);
        println!("after_cold_pairs={}", evidence.after_cold_pairs);
        println!(
            "after_persistent_learned_delta={:.6}",
            evidence.after_persistent_learned_delta
        );
        println!(
            "after_cold_learned_delta={:.6}",
            evidence.after_cold_learned_delta
        );
        println!(
            "reentry_persistent_pairs={}",
            evidence.reentry_persistent_pairs
        );
        println!("reentry_cold_pairs={}", evidence.reentry_cold_pairs);
        println!(
            "reentry_persistent_learned_delta={:.6}",
            evidence.reentry_persistent_learned_delta
        );
        println!(
            "reentry_cold_learned_delta={:.6}",
            evidence.reentry_cold_learned_delta
        );
    } else if persistent {
        let trend = run_persistent_curriculum_session(CurriculumConfig::default(), episodes)?;
        println!("mode=persistent");
        println!("episodes_run={}", trend.episodes_run);
        println!("courses_run={}", trend.courses_run);
        println!("total_steps_run={}", trend.total_steps_run);
        println!("first_worldline_err={:.6}", trend.first_worldline_err);
        println!("last_worldline_err={:.6}", trend.last_worldline_err);
        println!("mean_worldline_err={:.6}", trend.mean_worldline_err);
        println!("total_pairs_formed={}", trend.total_pairs_formed);
        println!("final_pairs={}", trend.final_pairs);
        println!("final_learned_events={}", trend.final_learned_events);
        println!("max_learned_abs_delta={:.6}", trend.max_learned_abs_delta);
    } else {
        let trend = run_curriculum_episodes(CurriculumConfig::default(), episodes)?;
        println!("mode=fresh");
        println!("episodes_run={}", trend.episodes_run);
        println!("courses_run={}", trend.courses_run);
        println!("total_steps_run={}", trend.total_steps_run);
        println!("first_worldline_err={:.6}", trend.first_worldline_err);
        println!("last_worldline_err={:.6}", trend.last_worldline_err);
        println!("mean_worldline_err={:.6}", trend.mean_worldline_err);
        println!("total_pairs_formed={}", trend.total_pairs_formed);
        println!("total_learned_events={}", trend.total_learned_events);
        println!("max_learned_abs_delta={:.6}", trend.max_learned_abs_delta);
    }

    Ok(())
}
