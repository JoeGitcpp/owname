use std::process::Command;

#[test]
fn curriculum_report_binary_emits_repeatable_episode_metrics() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .args(["--episodes", "3"])
        .output()
        .expect("run curriculum_report");

    assert!(
        output.status.success(),
        "curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("episodes_run=3"));
    assert!(stdout.contains("courses_run=9"));
    assert!(stdout.contains("total_steps_run="));
    assert!(stdout.contains("mean_worldline_err="));
    assert!(stdout.contains("total_pairs_formed=3"));
    assert!(stdout.contains("total_learned_events="));
    assert!(stdout.contains("max_learned_abs_delta="));
}

#[test]
fn persistent_curriculum_report_emits_same_body_metrics() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .args(["--episodes", "4", "--persistent"])
        .output()
        .expect("run curriculum_report persistent");

    assert!(
        output.status.success(),
        "persistent curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=persistent"));
    assert!(stdout.contains("episodes_run=4"));
    assert!(stdout.contains("courses_run=12"));
    assert!(stdout.contains("final_pairs=4"));
    assert!(stdout.contains("final_learned_events="));
    assert!(stdout.contains("max_learned_abs_delta="));
}

#[test]
fn long_retention_curriculum_report_emits_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--long-retention")
        .output()
        .expect("run curriculum_report long retention");

    assert!(
        output.status.success(),
        "long retention curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=long_retention"));
    assert!(stdout.contains("train_episodes=6"));
    assert!(stdout.contains("hold_learned_delta_ratio="));
    assert!(stdout.contains("hold_adaptive_route_ratio="));
    assert!(stdout.contains("after_persistent_learned_delta="));
    assert!(stdout.contains("after_cold_learned_delta="));
    assert!(stdout.contains("reentry_persistent_learned_delta="));
    assert!(stdout.contains("reentry_cold_learned_delta="));
}

#[test]
fn mark_binding_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--mark-binding")
        .output()
        .expect("run curriculum_report mark binding");

    assert!(
        output.status.success(),
        "mark binding curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=mark_binding"));
    assert!(stdout.contains("train_episodes=4"));
    assert!(stdout.contains("train_recurrence="));
    assert!(stdout.contains("train_binding_strength="));
    assert!(stdout.contains("hold_binding_strength_ratio="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
    assert!(stdout.contains("final_ambiguity="));
}

#[test]
fn tri_modal_primitive_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--tri-modal-primitive")
        .output()
        .expect("run curriculum_report tri-modal primitive");

    assert!(
        output.status.success(),
        "tri-modal primitive curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=tri_modal_primitive"));
    assert!(stdout.contains("train_episodes=4"));
    assert!(stdout.contains("vib_observations="));
    assert!(stdout.contains("phot_worldline_updates="));
    assert!(stdout.contains("mark_recurrence="));
    assert!(stdout.contains("hold_binding_strength_ratio="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
    assert!(stdout.contains("final_ambiguity="));
}

#[test]
fn factory_dna_checkpoint_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--factory-dna-checkpoint")
        .output()
        .expect("run curriculum_report factory DNA checkpoint");

    assert!(
        output.status.success(),
        "factory DNA checkpoint curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=factory_dna_checkpoint"));
    assert!(stdout.contains("digital_structure_events="));
    assert!(stdout.contains("character_digit_mark_traces="));
    assert!(stdout.contains("visual_marker_events="));
    assert!(stdout.contains("acoustic_low_band_events="));
    assert!(stdout.contains("acoustic_high_band_events="));
    assert!(stdout.contains("pitch_band_separation_margin="));
    assert!(stdout.contains("cross_channel_pairs="));
    assert!(stdout.contains("cross_channel_coupling_strength="));
    assert!(stdout.contains("hold_retention_ratio="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
    assert!(stdout.contains("dna_checkpoint_pass="));
}

#[test]
fn product_cognition_baseline_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--product-cognition-baseline")
        .output()
        .expect("run curriculum_report product cognition baseline");

    assert!(
        output.status.success(),
        "product cognition baseline curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=product_cognition_baseline"));
    assert!(stdout.contains("factory_dna_checkpoint_pass="));
    assert!(stdout.contains("course_count="));
    assert!(stdout.contains("passing_courses="));
    assert!(stdout.contains("object_relation_readiness="));
    assert!(stdout.contains("text_grounding_readiness="));
    assert!(stdout.contains("open_stream_readiness="));
    assert!(stdout.contains("cross_channel_readiness="));
    assert!(stdout.contains("active_probe_readiness="));
    assert!(stdout.contains("heldout_generalization_readiness="));
    assert!(stdout.contains("chain_memory_readiness="));
    assert!(stdout.contains("multi_step_readiness="));
    assert!(stdout.contains("long_horizon_readiness="));
    assert!(stdout.contains("min_reentry_margin="));
    assert!(stdout.contains("min_hold_ratio="));
    assert!(stdout.contains("max_wrong_binding_strength="));
    assert!(stdout.contains("product_baseline_pass="));
}

#[test]
fn text_disturbance_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--text-disturbance")
        .output()
        .expect("run curriculum_report text disturbance");

    assert!(
        output.status.success(),
        "text disturbance curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=text_disturbance"));
    assert!(stdout.contains("train_episodes=4"));
    assert!(stdout.contains("distinct_text_traces="));
    assert!(stdout.contains("a_recurrence="));
    assert!(stdout.contains("b_recurrence="));
    assert!(stdout.contains("hold_avg_strength_ratio="));
    assert!(stdout.contains("conflict_ambiguity_delta="));
    assert!(stdout.contains("reentry_persistent_avg_strength="));
    assert!(stdout.contains("reentry_cold_avg_strength="));
    assert!(stdout.contains("time_window_strength="));
}

#[test]
fn text_grounding_suite_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--text-grounding-suite")
        .output()
        .expect("run curriculum_report text grounding suite");

    assert!(
        output.status.success(),
        "text grounding suite curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=text_grounding_suite"));
    assert!(stdout.contains("train_marks=6"));
    assert!(stdout.contains("object_targets="));
    assert!(stdout.contains("relation_targets="));
    assert!(stdout.contains("variant_generalization_score="));
    assert!(stdout.contains("variant_distinctness_ratio="));
    assert!(stdout.contains("composition_relation_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("reentry_persistent_avg="));
    assert!(stdout.contains("reentry_cold_avg="));
}

#[test]
fn text_open_stream_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--text-open-stream")
        .output()
        .expect("run curriculum_report text open stream");

    assert!(
        output.status.success(),
        "text open stream curriculum_report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=text_open_stream"));
    assert!(stdout.contains("stream_events="));
    assert!(stdout.contains("supported_marks="));
    assert!(stdout.contains("unrelated_marks="));
    assert!(stdout.contains("variant_marks="));
    assert!(stdout.contains("object_targets="));
    assert!(stdout.contains("relation_targets="));
    assert!(stdout.contains("time_window_targets="));
    assert!(stdout.contains("composition_windows="));
    assert!(stdout.contains("delayed_windows="));
    assert!(stdout.contains("train_supported_avg="));
    assert!(stdout.contains("unrelated_leakage_strength="));
    assert!(stdout.contains("hold_supported_ratio="));
    assert!(stdout.contains("variant_persistent_avg="));
    assert!(stdout.contains("variant_cold_avg="));
    assert!(stdout.contains("delayed_binding_strength="));
    assert!(stdout.contains("composition_relation_strength="));
    assert!(stdout.contains("conflict_ambiguity_delta="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("reentry_persistent_avg="));
    assert!(stdout.contains("reentry_cold_avg="));
}

#[test]
fn active_text_probe_closure_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--active-text-probe-closure")
        .output()
        .expect("run curriculum_report active text probe closure");

    assert!(
        output.status.success(),
        "active text probe closure curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=active_text_probe_closure"));
    assert!(stdout.contains("stream_events="));
    assert!(stdout.contains("supported_marks="));
    assert!(stdout.contains("train_supported_avg="));
    assert!(stdout.contains("conflict_events="));
    assert!(stdout.contains("conflict_ambiguity_delta="));
    assert!(stdout.contains("probe_trigger_events="));
    assert!(stdout.contains("probe_frames="));
    assert!(stdout.contains("probe_write_epochs="));
    assert!(stdout.contains("return_windows="));
    assert!(stdout.contains("paired_consequences="));
    assert!(stdout.contains("probe_pair_similarity_avg="));
    assert!(stdout.contains("pre_probe_ambiguity="));
    assert!(stdout.contains("post_probe_ambiguity="));
    assert!(stdout.contains("ambiguity_reduction_ratio="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("post_probe_supported_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("egress_projection_confidence="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
}

#[test]
fn sustained_active_probe_training_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--sustained-active-probe-training")
        .output()
        .expect("run curriculum_report sustained active probe training");

    assert!(
        output.status.success(),
        "sustained active probe training curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=sustained_active_probe_training"));
    assert!(stdout.contains("cycles_run="));
    assert!(stdout.contains("materials_seen="));
    assert!(stdout.contains("conflict_events="));
    assert!(stdout.contains("probe_frames="));
    assert!(stdout.contains("probe_write_epochs="));
    assert!(stdout.contains("return_windows="));
    assert!(stdout.contains("paired_consequences="));
    assert!(stdout.contains("replay_ingress_frames="));
    assert!(stdout.contains("live_return_ingress_frames="));
    assert!(stdout.contains("cross_channel_vib_observations="));
    assert!(stdout.contains("cross_channel_phot_worldline_updates="));
    assert!(stdout.contains("cross_channel_coupling_strength="));
    assert!(stdout.contains("probe_pair_similarity_avg="));
    assert!(stdout.contains("min_ambiguity_reduction_ratio="));
    assert!(stdout.contains("min_correction_recovery_ratio="));
    assert!(stdout.contains("max_wrong_binding_strength="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
    assert!(stdout.contains("persistent_reentry_margin="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn heldout_gap_generalization_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--heldout-gap-generalization")
        .output()
        .expect("run curriculum_report held-out gap generalization");

    assert!(
        output.status.success(),
        "held-out gap generalization report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=heldout_gap_generalization"));
    assert!(stdout.contains("trained_materials="));
    assert!(stdout.contains("heldout_materials="));
    assert!(stdout.contains("gap_pressure_before_heldout="));
    assert!(stdout.contains("gap_pressure_after_heldout="));
    assert!(stdout.contains("gap_pressure_after_closure="));
    assert!(stdout.contains("gap_pressure_rises_under_heldout_conflict="));
    assert!(stdout.contains("probe_writes_before_gap="));
    assert!(stdout.contains("probe_writes_during_gap="));
    assert!(stdout.contains("probe_writes_after_closure="));
    assert!(stdout.contains("probe_count_rises_under_gap="));
    assert!(stdout.contains("probe_count_falls_after_closure="));
    assert!(stdout.contains("persistent_heldout_strength="));
    assert!(stdout.contains("cold_heldout_strength="));
    assert!(stdout.contains("persistent_reentry_margin="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn multi_gap_selection_pixel_scale_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--multi-gap-selection-pixel-scale")
        .output()
        .expect("run curriculum_report multi-gap selection pixel scale");

    assert!(
        output.status.success(),
        "multi-gap selection pixel scale report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=multi_gap_selection_pixel_scale"));
    assert!(stdout.contains("small_gap_events="));
    assert!(stdout.contains("mid_gap_events="));
    assert!(stdout.contains("large_gap_events="));
    assert!(stdout.contains("competing_gaps="));
    assert!(stdout.contains("small_gap_pressure="));
    assert!(stdout.contains("mid_gap_pressure="));
    assert!(stdout.contains("large_gap_pressure="));
    assert!(stdout.contains("selected_gap_index="));
    assert!(stdout.contains("selected_gap_pressure="));
    assert!(stdout.contains("selected_gap_pressure_is_max="));
    assert!(stdout.contains("probe_writes_before_selection="));
    assert!(stdout.contains("probe_writes_after_selection="));
    assert!(stdout.contains("probe_bias_tracks_selected_gap="));
    assert!(stdout.contains("small_gap_closure_ratio="));
    assert!(stdout.contains("small_gap_persistence_score="));
    assert!(stdout.contains("large_gap_persistence_score="));
    assert!(stdout.contains("large_gap_persistent_strength="));
    assert!(stdout.contains("large_gap_cold_strength="));
    assert!(stdout.contains("large_gap_reentry_margin="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn prediction_pressure_field_water_wave_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--prediction-pressure-field-water-wave")
        .output()
        .expect("run curriculum_report prediction pressure field water wave");

    assert!(
        output.status.success(),
        "prediction pressure field water wave report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=prediction_pressure_field_water_wave"));
    assert!(stdout.contains("waves_alive="));
    assert!(stdout.contains("dominant_focus_is_single="));
    assert!(stdout.contains("focus_switches_bounded="));
    assert!(stdout.contains("quiet_reentry_wave_rises="));
    assert!(stdout.contains("event_wave_can_override_idle_wave="));
    assert!(stdout.contains("relation_resonance_spike_observed="));
    assert!(stdout.contains("closure_reduces_focused_wave="));
    assert!(stdout.contains("unclosed_wave_residue_retained="));
    assert!(stdout.contains("dominant_pressure_drives_probe="));
    assert!(stdout.contains("dominant_pressure="));
    assert!(stdout.contains("focus_switches="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn prediction_pressure_field_body_integration_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--prediction-pressure-field-body-integration")
        .output()
        .expect("run curriculum_report prediction pressure field body integration");

    assert!(
        output.status.success(),
        "prediction pressure field body integration report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=prediction_pressure_field_body_integration"));
    assert!(stdout.contains("body_ticks_run="));
    assert!(stdout.contains("quiet_probe_write_epochs="));
    assert!(stdout.contains("event_probe_write_epochs="));
    assert!(stdout.contains("waves_alive="));
    assert!(stdout.contains("dominant_focus_is_single="));
    assert!(stdout.contains("gap_wave_observed="));
    assert!(stdout.contains("event_wave_observed="));
    assert!(stdout.contains("relation_wave_observed="));
    assert!(stdout.contains("residue_wave_observed="));
    assert!(stdout.contains("closure_reduces_focused_wave="));
    assert!(stdout.contains("bounded_wave_count="));
    assert!(stdout.contains("focus_switches_bounded="));
    assert!(stdout.contains("stress_ticks="));
    assert!(stdout.contains("max_dominant_pressure="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn active_surface_aperture_echo_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--active-surface-aperture-echo")
        .output()
        .expect("run curriculum_report active surface aperture echo");

    assert!(
        output.status.success(),
        "active surface aperture echo report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=active_surface_aperture_echo"));
    assert!(stdout.contains("parallel_focus_traces_alive="));
    assert!(stdout.contains("active_aperture_count="));
    assert!(stdout.contains("echo_focus_present="));
    assert!(stdout.contains("echo_pressure_below_aperture="));
    assert!(stdout.contains("dominant_focus_enters_aperture="));
    assert!(stdout.contains("non_aperture_focus_retained="));
    assert!(stdout.contains("suppressed_focus_can_reenter="));
    assert!(stdout.contains("closure_reduces_aperture_focus_without_deleting_trace="));
    assert!(stdout.contains("event_pressure_can_displace_aperture="));
    assert!(stdout.contains("displaced_focus_retains_residue="));
    assert!(stdout.contains("probe_drive_tracks_aperture_with_echo="));
    assert!(stdout.contains("aperture_switches_bounded="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn background_relation_resonance_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--background-relation-resonance-sudden-match")
        .output()
        .expect("run curriculum_report background relation resonance");

    assert!(
        output.status.success(),
        "background relation resonance report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=background_relation_resonance_sudden_match"));
    assert!(stdout.contains("background_attempts_alive="));
    assert!(stdout.contains("low_match_attempts_decay="));
    assert!(stdout.contains("high_match_spike_observed="));
    assert!(stdout.contains("sudden_match_enters_surface="));
    assert!(stdout.contains("wrong_match_leakage="));
    assert!(stdout.contains("aperture_relation_entry="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn local_probe_chain_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--local-probe-chain")
        .output()
        .expect("run curriculum_report local probe chain");

    assert!(
        output.status.success(),
        "local probe chain report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=local_probe_chain"));
    assert!(stdout.contains("chain_steps="));
    assert!(stdout.contains("closure_steps="));
    assert!(stdout.contains("mismatch_steps="));
    assert!(stdout.contains("return_revises_surface_pressure="));
    assert!(stdout.contains("closed_focus_does_not_repeat_probe="));
    assert!(stdout.contains("unclosed_focus_continues_probe="));
    assert!(stdout.contains("next_probe_pressure_after_mismatch="));
    assert!(stdout.contains("closure_pressure_after_return="));
    assert!(stdout.contains("wrong_chain_binding="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn chain_memory_sedimentation_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--chain-memory-sedimentation")
        .output()
        .expect("run curriculum_report chain memory sedimentation");

    assert!(
        output.status.success(),
        "chain memory sedimentation report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=chain_memory_sedimentation"));
    assert!(stdout.contains("chain_memory_traces_alive="));
    assert!(stdout.contains("sediment_updates="));
    assert!(stdout.contains("closure_sediment_strength="));
    assert!(stdout.contains("mismatch_sediment_strength="));
    assert!(stdout.contains("reentry_pressure="));
    assert!(stdout.contains("sediment_reentry_observed="));
    assert!(stdout.contains("persistent_reentry_pressure="));
    assert!(stdout.contains("cold_reentry_pressure="));
    assert!(stdout.contains("persistent_reentry_margin="));
    assert!(stdout.contains("duplicate_pair_sediment_guard="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn multi_step_chain_continuity_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--multi-step-chain-continuity")
        .output()
        .expect("run curriculum_report multi-step chain continuity");

    assert!(
        output.status.success(),
        "multi-step chain continuity report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=multi_step_chain_continuity"));
    assert!(stdout.contains("chain_steps="));
    assert!(stdout.contains("same_chain_continuity_steps="));
    assert!(stdout.contains("dominant_continuity_score="));
    assert!(stdout.contains("parallel_branch_count="));
    assert!(stdout.contains("branch_pressure="));
    assert!(stdout.contains("single_aperture_preserved="));
    assert!(stdout.contains("branch_trace_retained="));
    assert!(stdout.contains("dominant_chain_reentry_pressure="));
    assert!(stdout.contains("wrong_chain_binding="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn soft_arbitration_drift_under_load_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--soft-arbitration-drift-under-load")
        .output()
        .expect("run curriculum_report soft arbitration drift under load");

    assert!(
        output.status.success(),
        "soft arbitration drift under load report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=soft_arbitration_drift_under_load"));
    assert!(stdout.contains("load_ticks="));
    assert!(stdout.contains("hold_count="));
    assert!(stdout.contains("switch_count="));
    assert!(stdout.contains("reentry_count="));
    assert!(stdout.contains("competition_pressure="));
    assert!(stdout.contains("challenger_pressure="));
    assert!(stdout.contains("arbitration_margin="));
    assert!(stdout.contains("branch_pressure_retained="));
    assert!(stdout.contains("single_aperture_preserved="));
    assert!(stdout.contains("dominant_chain_reentered="));
    assert!(stdout.contains("wrong_chain_binding="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn long_horizon_pressure_chain_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--long-horizon-pressure-chain")
        .output()
        .expect("run curriculum_report long horizon pressure chain");

    assert!(
        output.status.success(),
        "long horizon pressure chain report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=long_horizon_pressure_chain"));
    assert!(stdout.contains("long_chain_steps="));
    assert!(stdout.contains("long_chain_span_us="));
    assert!(stdout.contains("long_chain_score="));
    assert!(stdout.contains("interruption_count="));
    assert!(stdout.contains("recovery_count="));
    assert!(stdout.contains("interruption_branch_pressure="));
    assert!(stdout.contains("delayed_reentry_pressure="));
    assert!(stdout.contains("single_aperture_preserved="));
    assert!(stdout.contains("branch_trace_retained="));
    assert!(stdout.contains("wrong_chain_binding="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn vocal_reflex_suppression_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--vocal-reflex-suppression")
        .output()
        .expect("run curriculum_report vocal reflex suppression");

    assert!(
        output.status.success(),
        "vocal reflex suppression report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=vocal_reflex_suppression"));
    assert!(stdout.contains("shock_events="));
    assert!(stdout.contains("reflex_pressure_peak="));
    assert!(stdout.contains("vocal_burst_epochs="));
    assert!(stdout.contains("familiarity_after_repeats="));
    assert!(stdout.contains("suppression_strength="));
    assert!(stdout.contains("reflex_pressure_after_habituation="));
    assert!(stdout.contains("quiet_regulation_successes="));
    assert!(stdout.contains("urgent_pressure_bypasses_suppression="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn auditory_visual_predictability_expression_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--auditory-visual-predictability-expression")
        .output()
        .expect("run curriculum_report auditory visual predictability expression");

    assert!(
        output.status.success(),
        "auditory visual curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=auditory_visual_predictability_expression"));
    assert!(stdout.contains("vocal_writes="));
    assert!(stdout.contains("vib_returns="));
    assert!(stdout.contains("predictable_return_strength="));
    assert!(stdout.contains("return_closure_count="));
    assert!(stdout.contains("aligned_av_events="));
    assert!(stdout.contains("cross_channel_predictability="));
    assert!(stdout.contains("drift_pressure_after_misalignment="));
    assert!(stdout.contains("expressive_pressure_peak="));
    assert!(stdout.contains("expression_write_epochs="));
    assert!(stdout.contains("closure_reduces_expression_pressure="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("body_source_order_guard="));
    assert!(stdout.contains("principle_guard_pass="));
}

#[test]
fn cross_channel_disturbance_coupling_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--cross-channel-disturbance-coupling")
        .output()
        .expect("run curriculum_report cross-channel disturbance coupling");

    assert!(
        output.status.success(),
        "cross-channel disturbance coupling curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=cross_channel_disturbance_coupling"));
    assert!(stdout.contains("stream_events="));
    assert!(stdout.contains("text_marks="));
    assert!(stdout.contains("vib_observations="));
    assert!(stdout.contains("phot_worldline_updates="));
    assert!(stdout.contains("object_targets="));
    assert!(stdout.contains("relation_targets="));
    assert!(stdout.contains("time_window_targets="));
    assert!(stdout.contains("text_binding_strength="));
    assert!(stdout.contains("text_vib_coupling_strength="));
    assert!(stdout.contains("text_phot_coupling_strength="));
    assert!(stdout.contains("vib_phot_coupling_strength="));
    assert!(stdout.contains("conflict_coupling_drop="));
    assert!(stdout.contains("wrong_coupling_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("reentry_persistent_coupling="));
    assert!(stdout.contains("reentry_cold_coupling="));
}

#[test]
fn external_curriculum_plan_report_emits_material_and_principle_review() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--external-curriculum-plan")
        .output()
        .expect("run curriculum_report external curriculum plan");

    assert!(
        output.status.success(),
        "external curriculum plan report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=external_curriculum_plan"));
    assert!(stdout.contains("material_groups="));
    assert!(stdout.contains("synthetic_groups="));
    assert!(stdout.contains("replay_groups="));
    assert!(stdout.contains("probe_return_groups="));
    assert!(stdout.contains("live_ingress_groups="));
    assert!(stdout.contains("carrier_kinds="));
    assert!(stdout.contains("stage_kinds="));
    assert!(stdout.contains("reproducible_groups="));
    assert!(stdout.contains("observer_only_groups="));
    assert!(stdout.contains("body_touch_points="));
    assert!(stdout.contains("missing_required_roles="));
    assert!(stdout.contains("principle_review_pass="));
}

#[test]
fn persistent_curriculum_governor_report_emits_decision_trace_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--persistent-curriculum-governor")
        .output()
        .expect("run curriculum_report persistent curriculum governor");

    assert!(
        output.status.success(),
        "persistent curriculum governor report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=persistent_curriculum_governor"));
    assert!(stdout.contains("rounds_run="));
    assert!(stdout.contains("courses_evaluated="));
    assert!(stdout.contains("material_groups="));
    assert!(stdout.contains("material_principle_pass="));
    assert!(stdout.contains("decision_trace_len="));
    assert!(stdout.contains("selected_course_kind="));
    assert!(stdout.contains("selected_score="));
    assert!(stdout.contains("selected_threshold="));
    assert!(stdout.contains("passing_courses="));
    assert!(stdout.contains("failing_courses="));
    assert!(stdout.contains("stopped_after_pass="));
    assert!(stdout.contains("body_touch_points="));
    assert!(stdout.contains("selected_margin_score="));
    assert!(stdout.contains("weakest_margin_course_kind="));
    assert!(stdout.contains("weakest_margin_score="));
    assert!(stdout.contains("near_threshold_courses="));
    assert!(stdout.contains("min_margin_score="));
    assert!(stdout.contains("max_margin_score="));
    assert!(stdout.contains("foundation_margin_floor="));
    assert!(stdout.contains("foundation_min_margin_ready="));
    assert!(stdout.contains("foundation_text_grounding_ready="));
    assert!(stdout.contains("foundation_live_io_ready="));
    assert!(stdout.contains("foundation_cross_channel_ready="));
    assert!(stdout.contains("foundation_stage_ready="));
    assert!(stdout.contains("selected_margin_points_len="));
    assert!(stdout.contains("selected_weakest_margin_point_kind="));
    assert!(stdout.contains("selected_weakest_margin_point_score="));
    assert!(stdout.contains("selected_margin_point_00_kind="));
    assert!(stdout.contains("selected_margin_point_00_score="));
}

#[test]
fn curriculum_report_source_stays_observer_side_and_numeric() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/curriculum_report.rs"
    ))
    .expect("read curriculum_report.rs");

    for forbidden in [
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "request_response",
        "answer_model",
        "target_label",
        "result_label",
        "personality",
        "owner",
        "ethics",
    ] {
        assert!(
            !src.contains(forbidden),
            "curriculum_report must not introduce rejected ontology term `{}`",
            forbidden
        );
    }
}
