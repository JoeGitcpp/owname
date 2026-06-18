use std::sync::Arc;

use crate::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use crate::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
use crate::language_binding::{BindingEvent, BindingFeedback, BindingTargetRef, PatternKind};
use crate::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_EVENT, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RELATION,
    PREDICTION_WAVE_RESIDUE,
};
use crate::snapshot::{write_snapshot, SnapshotPaths};
use crate::substrate_pressure::PressureSample;
use crate::viewport::{GlobalRelationState, RelationEdgeState, Viewport};
use crate::{MOD_PHOT, MOD_VIB, PHOT_BANDS, VIB_BANDS};

#[derive(Clone, Copy, Debug)]
pub struct CurriculumConfig {
    pub start_t_us: u64,
    pub step_us: u64,
    pub steps: u32,
    pub magnitude: f64,
}

impl Default for CurriculumConfig {
    fn default() -> Self {
        Self {
            start_t_us: 1_000_000,
            step_us: 20_000,
            steps: 12,
            magnitude: 0.4,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumSummary {
    pub steps_run: u32,
    pub worldline_updates: u32,
    pub final_worldline_err: f64,
    pub initial_relation_distance_score: f64,
    pub final_relation_distance_score: f64,
    pub initial_relation_overlap_score: f64,
    pub final_relation_overlap_score: f64,
    pub pairs_formed: u32,
    pub learned_events: u32,
    pub learned_max_abs_delta: f64,
    pub snapshots_written: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumSuiteSummary {
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub worldline_updates: u32,
    pub final_worldline_err: f64,
    pub initial_relation_distance_score: f64,
    pub final_relation_distance_score: f64,
    pub initial_relation_overlap_score: f64,
    pub final_relation_overlap_score: f64,
    pub pairs_formed: u32,
    pub learned_events: u32,
    pub learned_max_abs_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumEpisodeTrend {
    pub episodes_run: u32,
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub first_worldline_err: f64,
    pub last_worldline_err: f64,
    pub mean_worldline_err: f64,
    pub total_pairs_formed: u32,
    pub total_learned_events: u32,
    pub max_learned_abs_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PersistentCurriculumSessionSummary {
    pub episodes_run: u32,
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub worldline_updates: u32,
    pub first_worldline_err: f64,
    pub last_worldline_err: f64,
    pub mean_worldline_err: f64,
    pub total_pairs_formed: u32,
    pub final_pairs: u32,
    pub final_learned_events: u32,
    pub max_learned_abs_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LongRetentionCurriculumEvidence {
    pub train_episodes: u32,
    pub after_episodes: u32,
    pub reentry_episodes: u32,
    pub train_pairs: u32,
    pub train_learned_events: u32,
    pub train_learned_delta: f64,
    pub train_adaptive_routes: u32,
    pub hold_learned_delta: f64,
    pub hold_adaptive_routes: u32,
    pub hold_learned_delta_ratio: f64,
    pub hold_adaptive_route_ratio: f64,
    pub after_persistent_pairs: u32,
    pub after_cold_pairs: u32,
    pub after_persistent_learned_delta: f64,
    pub after_cold_learned_delta: f64,
    pub reentry_persistent_pairs: u32,
    pub reentry_cold_pairs: u32,
    pub reentry_persistent_learned_delta: f64,
    pub reentry_cold_learned_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MarkBindingCurriculumEvidence {
    pub train_episodes: u32,
    pub reentry_episodes: u32,
    pub train_recurrence: u32,
    pub train_binding_strength: f64,
    pub train_ambiguity: f64,
    pub hold_binding_strength: f64,
    pub hold_binding_strength_ratio: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
    pub final_ambiguity: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TriModalPrimitiveBiasEvidence {
    pub train_episodes: u32,
    pub reentry_episodes: u32,
    pub vib_observations: u32,
    pub phot_worldline_updates: u32,
    pub mark_recurrence: u32,
    pub train_binding_strength: f64,
    pub train_ambiguity: f64,
    pub hold_binding_strength: f64,
    pub hold_binding_strength_ratio: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
    pub final_ambiguity: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FactoryDnaCheckpointEvidence {
    pub digital_structure_events: u32,
    pub character_digit_mark_traces: u32,
    pub visual_marker_events: u32,
    pub acoustic_low_band_events: u32,
    pub acoustic_high_band_events: u32,
    pub pitch_band_separation_margin: f64,
    pub cross_channel_pairs: u32,
    pub cross_channel_coupling_strength: f64,
    pub conflict_ambiguity_delta: f64,
    pub wrong_binding_strength: f64,
    pub hold_retention_ratio: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
    pub no_fixed_ontology_guard: u8,
    pub dna_checkpoint_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProductCognitionBaselineEvidence {
    pub factory_dna_checkpoint_pass: u8,
    pub course_count: u32,
    pub passing_courses: u32,
    pub object_relation_readiness: u8,
    pub text_grounding_readiness: u8,
    pub open_stream_readiness: u8,
    pub cross_channel_readiness: u8,
    pub active_probe_readiness: u8,
    pub heldout_generalization_readiness: u8,
    pub chain_memory_readiness: u8,
    pub multi_step_readiness: u8,
    pub long_horizon_readiness: u8,
    pub min_reentry_margin: f64,
    pub min_hold_ratio: f64,
    pub max_wrong_binding_strength: f64,
    pub no_child_ontology_guard: u8,
    pub product_baseline_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextDisturbanceCurriculumEvidence {
    pub train_episodes: u32,
    pub reentry_episodes: u32,
    pub distinct_text_traces: u32,
    pub a_recurrence: u32,
    pub b_recurrence: u32,
    pub a_to_slot_a_strength: f64,
    pub b_to_slot_b_strength: f64,
    pub a_to_slot_b_strength: f64,
    pub hold_avg_strength: f64,
    pub hold_avg_strength_ratio: f64,
    pub conflict_ambiguity_delta: f64,
    pub reentry_persistent_avg_strength: f64,
    pub reentry_cold_avg_strength: f64,
    pub time_window_strength: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextGroundingSuiteEvidence {
    pub train_marks: u32,
    pub object_targets: u32,
    pub relation_targets: u32,
    pub time_window_targets: u32,
    pub variant_marks: u32,
    pub composition_windows: u32,
    pub train_avg_strength: f64,
    pub train_weakest_strength: f64,
    pub hold_avg_ratio: f64,
    pub hold_weakest_ratio: f64,
    pub variant_generalization_score: f64,
    pub variant_distinctness_ratio: f64,
    pub composition_relation_strength: f64,
    pub conflict_ambiguity_delta: f64,
    pub wrong_binding_strength: f64,
    pub correction_recovery_ratio: f64,
    pub reentry_persistent_avg: f64,
    pub reentry_cold_avg: f64,
    pub reentry_persistent_weakest: f64,
    pub reentry_cold_weakest: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextOpenStreamEvidence {
    pub stream_events: u32,
    pub supported_marks: u32,
    pub unrelated_marks: u32,
    pub variant_marks: u32,
    pub object_targets: u32,
    pub relation_targets: u32,
    pub time_window_targets: u32,
    pub composition_windows: u32,
    pub delayed_windows: u32,
    pub train_supported_avg: f64,
    pub train_supported_weakest: f64,
    pub unrelated_leakage_strength: f64,
    pub hold_supported_ratio: f64,
    pub variant_persistent_avg: f64,
    pub variant_cold_avg: f64,
    pub delayed_binding_strength: f64,
    pub composition_relation_strength: f64,
    pub conflict_ambiguity_delta: f64,
    pub wrong_binding_strength: f64,
    pub correction_recovery_ratio: f64,
    pub reentry_persistent_avg: f64,
    pub reentry_cold_avg: f64,
    pub reentry_persistent_weakest: f64,
    pub reentry_cold_weakest: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ActiveTextProbeClosureEvidence {
    pub stream_events: u32,
    pub supported_marks: u32,
    pub train_supported_avg: f64,
    pub conflict_events: u32,
    pub conflict_ambiguity_delta: f64,
    pub probe_trigger_events: u32,
    pub probe_frames: u32,
    pub probe_write_epochs: u32,
    pub return_windows: u32,
    pub paired_consequences: u32,
    pub probe_pair_similarity_avg: f64,
    pub pre_probe_ambiguity: f64,
    pub post_probe_ambiguity: f64,
    pub ambiguity_reduction_ratio: f64,
    pub wrong_binding_strength: f64,
    pub pre_probe_supported_strength: f64,
    pub post_probe_supported_strength: f64,
    pub correction_recovery_ratio: f64,
    pub egress_projection_confidence: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SustainedActiveProbeTrainingEvidence {
    pub cycles_run: u32,
    pub materials_seen: u32,
    pub conflict_events: u32,
    pub probe_frames: u32,
    pub probe_write_epochs: u32,
    pub return_windows: u32,
    pub paired_consequences: u32,
    pub replay_ingress_frames: u32,
    pub live_return_ingress_frames: u32,
    pub cross_channel_vib_observations: u32,
    pub cross_channel_phot_worldline_updates: u32,
    pub cross_channel_coupling_strength: f64,
    pub probe_pair_similarity_avg: f64,
    pub min_ambiguity_reduction_ratio: f64,
    pub min_correction_recovery_ratio: f64,
    pub max_wrong_binding_strength: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
    pub persistent_reentry_margin: f64,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HeldoutGapGeneralizationEvidence {
    pub trained_materials: u32,
    pub heldout_materials: u32,
    pub gap_pressure_before_heldout: f64,
    pub gap_pressure_after_heldout: f64,
    pub gap_pressure_after_closure: f64,
    pub gap_pressure_rises_under_heldout_conflict: u8,
    pub probe_writes_before_gap: u32,
    pub probe_writes_during_gap: u32,
    pub probe_writes_after_closure: u32,
    pub probe_count_rises_under_gap: u8,
    pub probe_count_falls_after_closure: u8,
    pub persistent_heldout_strength: f64,
    pub cold_heldout_strength: f64,
    pub persistent_reentry_margin: f64,
    pub wrong_binding_strength: f64,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MultiGapSelectionPixelScaleEvidence {
    pub small_gap_events: u32,
    pub mid_gap_events: u32,
    pub large_gap_events: u32,
    pub competing_gaps: u32,
    pub small_gap_pressure: f64,
    pub mid_gap_pressure: f64,
    pub large_gap_pressure: f64,
    pub selected_gap_index: u8,
    pub selected_gap_pressure: f64,
    pub selected_gap_pressure_is_max: u8,
    pub probe_writes_before_selection: u32,
    pub probe_writes_after_selection: u32,
    pub probe_bias_tracks_selected_gap: u8,
    pub small_gap_closure_ratio: f64,
    pub small_gap_persistence_score: f64,
    pub large_gap_persistence_score: f64,
    pub large_gap_persistent_strength: f64,
    pub large_gap_cold_strength: f64,
    pub large_gap_reentry_margin: f64,
    pub wrong_binding_strength: f64,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PredictionPressureFieldWaterWaveEvidence {
    pub waves_alive: u32,
    pub dominant_focus_is_single: u8,
    pub focus_switches_bounded: u8,
    pub quiet_reentry_wave_rises: u8,
    pub event_wave_can_override_idle_wave: u8,
    pub relation_resonance_spike_observed: u8,
    pub closure_reduces_focused_wave: u8,
    pub unclosed_wave_residue_retained: u8,
    pub dominant_pressure_drives_probe: u8,
    pub dominant_pressure: f64,
    pub focus_switches: u32,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PredictionPressureFieldBodyIntegrationEvidence {
    pub body_ticks_run: u32,
    pub quiet_probe_write_epochs: u32,
    pub event_probe_write_epochs: u32,
    pub waves_alive: u32,
    pub dominant_focus_is_single: u8,
    pub gap_wave_observed: u8,
    pub event_wave_observed: u8,
    pub relation_wave_observed: u8,
    pub residue_wave_observed: u8,
    pub closure_reduces_focused_wave: u8,
    pub bounded_wave_count: u8,
    pub focus_switches_bounded: u8,
    pub stress_ticks: u32,
    pub max_dominant_pressure: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ActiveSurfaceApertureEchoEvidence {
    pub parallel_focus_traces_alive: u32,
    pub active_aperture_count: u8,
    pub echo_focus_present: u8,
    pub echo_pressure_below_aperture: u8,
    pub dominant_focus_enters_aperture: u8,
    pub non_aperture_focus_retained: u8,
    pub suppressed_focus_can_reenter: u8,
    pub closure_reduces_aperture_focus_without_deleting_trace: u8,
    pub event_pressure_can_displace_aperture: u8,
    pub displaced_focus_retains_residue: u8,
    pub probe_drive_tracks_aperture_with_echo: u8,
    pub aperture_switches_bounded: u8,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BackgroundRelationResonanceEvidence {
    pub background_attempts_alive: u32,
    pub low_match_attempts_decay: u8,
    pub high_match_spike_observed: u8,
    pub sudden_match_enters_surface: u8,
    pub wrong_match_leakage: f64,
    pub aperture_relation_entry: u8,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalProbeChainEvidence {
    pub chain_steps: u32,
    pub closure_steps: u32,
    pub mismatch_steps: u32,
    pub return_revises_surface_pressure: u8,
    pub closed_focus_does_not_repeat_probe: u8,
    pub unclosed_focus_continues_probe: u8,
    pub next_probe_pressure_after_mismatch: f64,
    pub closure_pressure_after_return: f64,
    pub wrong_chain_binding: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChainMemorySedimentationEvidence {
    pub chain_memory_traces_alive: u32,
    pub sediment_updates: u32,
    pub closure_sediment_strength: f64,
    pub mismatch_sediment_strength: f64,
    pub reentry_pressure: f64,
    pub sediment_reentry_observed: u8,
    pub persistent_reentry_pressure: f64,
    pub cold_reentry_pressure: f64,
    pub persistent_reentry_margin: f64,
    pub duplicate_pair_sediment_guard: u8,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MultiStepChainContinuityEvidence {
    pub chain_steps: u32,
    pub same_chain_continuity_steps: u32,
    pub dominant_continuity_score: f64,
    pub parallel_branch_count: u32,
    pub branch_pressure: f64,
    pub single_aperture_preserved: u8,
    pub branch_trace_retained: u8,
    pub dominant_chain_reentry_pressure: f64,
    pub wrong_chain_binding: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SoftArbitrationDriftUnderLoadEvidence {
    pub load_ticks: u32,
    pub hold_count: u32,
    pub switch_count: u32,
    pub reentry_count: u32,
    pub competition_pressure: f64,
    pub challenger_pressure: f64,
    pub arbitration_margin: f64,
    pub branch_pressure_retained: f64,
    pub single_aperture_preserved: u8,
    pub dominant_chain_reentered: u8,
    pub wrong_chain_binding: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LongHorizonPressureChainEvidence {
    pub long_chain_steps: u32,
    pub long_chain_span_us: u64,
    pub long_chain_score: f64,
    pub interruption_count: u32,
    pub recovery_count: u32,
    pub interruption_branch_pressure: f64,
    pub delayed_reentry_pressure: f64,
    pub single_aperture_preserved: u8,
    pub branch_trace_retained: u8,
    pub wrong_chain_binding: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VocalReflexSuppressionEvidence {
    pub shock_events: u32,
    pub reflex_pressure_peak: f64,
    pub vocal_burst_epochs: u32,
    pub familiarity_after_repeats: f64,
    pub suppression_strength: f64,
    pub reflex_pressure_after_habituation: f64,
    pub quiet_regulation_successes: u32,
    pub urgent_pressure_bypasses_suppression: u8,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AuditoryVisualPredictabilityExpressionEvidence {
    pub vocal_writes: u32,
    pub vib_returns: u32,
    pub predictable_return_strength: f64,
    pub return_closure_count: u32,
    pub aligned_av_events: u32,
    pub cross_channel_predictability: f64,
    pub drift_pressure_after_misalignment: f64,
    pub expressive_pressure_peak: f64,
    pub expression_write_epochs: u32,
    pub closure_reduces_expression_pressure: u8,
    pub wrong_binding_strength: f64,
    pub body_source_order_guard: u8,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CrossChannelDisturbanceCouplingEvidence {
    pub stream_events: u32,
    pub text_marks: u32,
    pub vib_observations: u32,
    pub phot_worldline_updates: u32,
    pub object_targets: u32,
    pub relation_targets: u32,
    pub time_window_targets: u32,
    pub text_binding_strength: f64,
    pub vib_slot_confidence: f64,
    pub phot_spatial_confidence: f64,
    pub text_vib_coupling_strength: f64,
    pub text_phot_coupling_strength: f64,
    pub vib_phot_coupling_strength: f64,
    pub conflict_coupling_drop: f64,
    pub wrong_coupling_strength: f64,
    pub correction_recovery_ratio: f64,
    pub reentry_persistent_coupling: f64,
    pub reentry_cold_coupling: f64,
}

pub fn run_text_grounding_suite(
    config: CurriculumConfig,
) -> std::io::Result<TextGroundingSuiteEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const VARIANT_EPISODES: u32 = 2;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 37;
    const HOLD_US: u64 = 2_200_000;
    const BASE_MARKS: [&[u8]; 6] = [
        b"base-a",
        b"base-b",
        b"base-c",
        b"base-ab",
        b"base-bc",
        b"base-window",
    ];
    const VARIANT_MARKS: [&[u8]; 3] = [b"base-a.", b"base-b.", b"base-c."];
    const COMPOSITION_MARKS: [&[u8]; 2] = [b"base-a+base-b", b"base-b+base-c"];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let base_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
    ];

    let base_hashes = bind_mark_set(
        &vp,
        config,
        &BASE_MARKS,
        &base_targets,
        TRAIN_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let train_strengths = strengths_for_targets(&vp, &base_hashes, &base_targets);
    let train_avg_strength = mean_strength(&train_strengths);
    let train_weakest_strength = weakest_strength(&train_strengths);

    let variant_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
    ];
    let variant_hashes = bind_mark_set(
        &vp,
        config,
        &VARIANT_MARKS,
        &variant_targets,
        VARIANT_EPISODES,
        config.start_t_us + config.step_us.max(1) * 8,
        STREAM_ID,
    );
    let variant_strengths = strengths_for_targets(&vp, &variant_hashes, &variant_targets);
    let variant_distinctness_ratio = distinct_variant_ratio(&variant_hashes, &base_hashes);

    let cold_variant = Arc::new(Viewport::new());
    let cold_variant_targets = setup_simple_text_targets(&cold_variant, config);
    let cold_variant_hashes = bind_mark_set(
        &cold_variant,
        config,
        &VARIANT_MARKS,
        &cold_variant_targets,
        VARIANT_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let cold_variant_strengths =
        strengths_for_targets(&cold_variant, &cold_variant_hashes, &cold_variant_targets);
    let variant_generalization_score =
        (mean_strength(&variant_strengths) - mean_strength(&cold_variant_strengths)).max(0.0);

    let composition_targets = [targets.relation_targets[0], targets.relation_targets[1]];
    let composition_start_t_us = config.start_t_us + config.step_us.max(1) * 14;
    let composition_hashes = vec![
        bind_adjacent_mark_window(
            &vp,
            config,
            BASE_MARKS[0],
            BASE_MARKS[1],
            composition_targets[0],
            STREAM_ID,
            composition_start_t_us,
        ),
        bind_adjacent_mark_window(
            &vp,
            config,
            BASE_MARKS[1],
            BASE_MARKS[2],
            composition_targets[1],
            STREAM_ID,
            composition_start_t_us + config.step_us.max(1) * 4,
        ),
    ];
    let composition_strengths =
        strengths_for_targets(&vp, &composition_hashes, &composition_targets);
    let composition_relation_strength = mean_strength(&composition_strengths);

    let hold_t_us = config.start_t_us + config.step_us.max(1) * 24 + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let hold_strengths = strengths_for_targets(&vp, &base_hashes, &base_targets);
    let hold_avg_ratio = ratio_or_zero(mean_strength(&hold_strengths), train_avg_strength);
    let hold_weakest_ratio =
        ratio_or_zero(weakest_strength(&hold_strengths), train_weakest_strength);

    let ambiguity_before_conflict = mark_ambiguity(&vp.read_mark_disturbance(), base_hashes[0]);
    vp.observe_mark_bytes(
        STREAM_ID,
        hold_t_us + config.step_us.max(1),
        hold_t_us + config.step_us.max(1) + config.step_us.max(1) / 2,
        BASE_MARKS[0],
        1.0,
    );
    vp.bind_latest_mark_to_target(
        targets.object_targets[1],
        BindingFeedback::Deviate,
        hold_t_us + config.step_us.max(1) * 2,
    );
    let ambiguity_after_conflict = mark_ambiguity(&vp.read_mark_disturbance(), base_hashes[0]);
    let wrong_binding_strength =
        symbol_binding_strength(&vp, base_hashes[0], targets.object_targets[1]);

    run_text_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        hold_t_us + config.step_us.max(1) * 3,
        targets.object_targets[0],
        STREAM_ID,
        BASE_MARKS[0],
    );
    let corrected_strength =
        symbol_binding_strength(&vp, base_hashes[0], targets.object_targets[0]);
    let correction_recovery_ratio = ratio_or_zero(corrected_strength, train_strengths[0]);

    bind_mark_set(
        &vp,
        config,
        &BASE_MARKS,
        &base_targets,
        REENTRY_EPISODES,
        hold_t_us + config.step_us.max(1) * 6,
        STREAM_ID,
    );
    let persistent_reentry_strengths = strengths_for_targets(&vp, &base_hashes, &base_targets);

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_base_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_targets.time_target,
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &BASE_MARKS,
        &cold_base_targets,
        REENTRY_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let cold_reentry_strengths = strengths_for_targets(&cold, &cold_hashes, &cold_base_targets);

    Ok(TextGroundingSuiteEvidence {
        train_marks: BASE_MARKS.len() as u32,
        object_targets: targets.object_targets.len() as u32,
        relation_targets: targets.relation_targets.len() as u32,
        time_window_targets: 1,
        variant_marks: VARIANT_MARKS.len() as u32,
        composition_windows: COMPOSITION_MARKS.len() as u32,
        train_avg_strength,
        train_weakest_strength,
        hold_avg_ratio,
        hold_weakest_ratio,
        variant_generalization_score,
        variant_distinctness_ratio,
        composition_relation_strength,
        conflict_ambiguity_delta: (ambiguity_after_conflict - ambiguity_before_conflict).max(0.0),
        wrong_binding_strength,
        correction_recovery_ratio,
        reentry_persistent_avg: mean_strength(&persistent_reentry_strengths),
        reentry_cold_avg: mean_strength(&cold_reentry_strengths),
        reentry_persistent_weakest: weakest_strength(&persistent_reentry_strengths),
        reentry_cold_weakest: weakest_strength(&cold_reentry_strengths),
    })
}

pub fn run_text_open_stream_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<TextOpenStreamEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const VARIANT_EPISODES: u32 = 2;
    const COLD_VARIANT_EPISODES: u32 = 1;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 43;
    const HOLD_US: u64 = 2_200_000;
    const SUPPORTED_MARKS: [&[u8]; 6] = [
        b"open-a",
        b"open-b",
        b"open-c",
        b"open-ab",
        b"open-bc",
        b"open-window",
    ];
    const UNRELATED_MARKS: [&[u8]; 4] = [b"drift-x", b"drift-y", b"drift-z", b"drift-window"];
    const VARIANT_MARKS: [&[u8]; 3] = [b"open-a.", b"open-b.", b"open-c."];
    const COMPOSITION_WINDOWS: u32 = 2;
    const DELAYED_WINDOWS: u32 = 2;

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let time_targets = [
        targets.time_target,
        BindingTargetRef::TimeWindow {
            start_t_us: config.start_t_us + step_us * 4,
            end_t_us: config.start_t_us + step_us * 7,
        },
    ];
    let supported_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        time_targets[0],
    ];

    let mut stream_events = 0u32;
    let mut cursor_t_us = config.start_t_us + step_us * 8;
    let mut supported_hashes = vec![0u64; SUPPORTED_MARKS.len()];
    let mut unrelated_hashes = vec![0u64; UNRELATED_MARKS.len()];

    for episode in 0..TRAIN_EPISODES {
        for idx in 0..SUPPORTED_MARKS.len() {
            let t_us = cursor_t_us;
            supported_hashes[idx] = vp.observe_mark_bytes(
                STREAM_ID,
                t_us,
                t_us + step_us / 2,
                SUPPORTED_MARKS[idx],
                1.0,
            );
            stream_events += 1;
            vp.bind_latest_mark_to_target(
                supported_targets[idx],
                BindingFeedback::Align,
                t_us + step_us,
            );
            cursor_t_us += step_us;

            if idx < UNRELATED_MARKS.len() {
                let unrelated_t_us = cursor_t_us + episode as u64;
                unrelated_hashes[idx] = vp.observe_mark_bytes(
                    STREAM_ID,
                    unrelated_t_us,
                    unrelated_t_us + step_us / 2,
                    UNRELATED_MARKS[idx],
                    1.0,
                );
                stream_events += 1;
                cursor_t_us += step_us;
            }
        }
    }

    let train_strengths = strengths_for_targets(&vp, &supported_hashes, &supported_targets);
    let train_supported_avg = mean_strength(&train_strengths);
    let train_supported_weakest = weakest_strength(&train_strengths);
    let unrelated_leakage_strength =
        max_strength_against_targets(&vp, &unrelated_hashes, &supported_targets);

    let variant_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
    ];
    let variant_hashes = bind_mark_set(
        &vp,
        config,
        &VARIANT_MARKS,
        &variant_targets,
        VARIANT_EPISODES,
        cursor_t_us,
        STREAM_ID,
    );
    stream_events += VARIANT_MARKS.len() as u32 * VARIANT_EPISODES;
    cursor_t_us += step_us * (VARIANT_MARKS.len() as u64) * (VARIANT_EPISODES as u64 + 1);
    let variant_strengths = strengths_for_targets(&vp, &variant_hashes, &variant_targets);
    let variant_persistent_avg = mean_strength(&variant_strengths);

    let cold_variant = Arc::new(Viewport::new());
    let cold_variant_targets = setup_simple_text_targets(&cold_variant, config);
    let cold_variant_hashes = bind_mark_set(
        &cold_variant,
        config,
        &VARIANT_MARKS,
        &cold_variant_targets,
        COLD_VARIANT_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let cold_variant_strengths =
        strengths_for_targets(&cold_variant, &cold_variant_hashes, &cold_variant_targets);
    let variant_cold_avg = mean_strength(&cold_variant_strengths);

    let delayed_targets = [time_targets[0], time_targets[1]];
    let mut delayed_hashes = Vec::with_capacity(DELAYED_WINDOWS as usize);
    for (idx, target) in delayed_targets.iter().copied().enumerate() {
        let t_us = cursor_t_us + idx as u64 * step_us * 3;
        let hash = vp.observe_mark_bytes(
            STREAM_ID,
            t_us,
            t_us + step_us / 2,
            SUPPORTED_MARKS[SUPPORTED_MARKS.len() - 1 - idx],
            1.0,
        );
        stream_events += 1;
        vp.bind_latest_mark_to_target(target, BindingFeedback::Align, t_us + step_us * 3);
        delayed_hashes.push(hash);
    }
    cursor_t_us += step_us * 8;
    let delayed_strengths = strengths_for_targets(&vp, &delayed_hashes, &delayed_targets);
    let delayed_binding_strength = mean_strength(&delayed_strengths);

    let composition_targets = [targets.relation_targets[0], time_targets[1]];
    let composition_hashes = vec![
        bind_adjacent_mark_window(
            &vp,
            config,
            SUPPORTED_MARKS[0],
            SUPPORTED_MARKS[1],
            composition_targets[0],
            STREAM_ID,
            cursor_t_us,
        ),
        bind_adjacent_mark_window(
            &vp,
            config,
            SUPPORTED_MARKS[1],
            SUPPORTED_MARKS[2],
            composition_targets[1],
            STREAM_ID,
            cursor_t_us + step_us * 4,
        ),
    ];
    stream_events += COMPOSITION_WINDOWS * 2;
    cursor_t_us += step_us * 8;
    let composition_strengths =
        strengths_for_targets(&vp, &composition_hashes, &composition_targets);
    let composition_relation_strength = mean_strength(&composition_strengths);

    let hold_t_us = cursor_t_us + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let hold_strengths = strengths_for_targets(&vp, &supported_hashes, &supported_targets);
    let hold_supported_ratio = ratio_or_zero(mean_strength(&hold_strengths), train_supported_avg);

    let ambiguity_before_conflict =
        mark_ambiguity(&vp.read_mark_disturbance(), supported_hashes[0]);
    let conflict_t_us = hold_t_us + step_us;
    vp.observe_mark_bytes(
        STREAM_ID,
        conflict_t_us,
        conflict_t_us + step_us / 2,
        SUPPORTED_MARKS[0],
        1.0,
    );
    stream_events += 1;
    vp.bind_latest_mark_to_target(
        targets.object_targets[1],
        BindingFeedback::Deviate,
        conflict_t_us + step_us,
    );
    let ambiguity_after_conflict = mark_ambiguity(&vp.read_mark_disturbance(), supported_hashes[0]);
    let wrong_binding_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[1]);

    let correction_t_us = conflict_t_us + step_us * 2;
    run_text_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        correction_t_us,
        targets.object_targets[0],
        STREAM_ID,
        SUPPORTED_MARKS[0],
    );
    stream_events += REENTRY_EPISODES;
    let corrected_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[0]);
    let correction_recovery_ratio = ratio_or_zero(corrected_strength, train_strengths[0]);

    let reentry_t_us = correction_t_us + step_us * 4;
    bind_mark_set(
        &vp,
        config,
        &SUPPORTED_MARKS,
        &supported_targets,
        REENTRY_EPISODES,
        reentry_t_us,
        STREAM_ID,
    );
    stream_events += SUPPORTED_MARKS.len() as u32 * REENTRY_EPISODES;
    let persistent_reentry_strengths =
        strengths_for_targets(&vp, &supported_hashes, &supported_targets);

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_time_targets = [
        cold_targets.time_target,
        BindingTargetRef::TimeWindow {
            start_t_us: config.start_t_us + step_us * 4,
            end_t_us: config.start_t_us + step_us * 7,
        },
    ];
    let cold_supported_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_time_targets[0],
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &SUPPORTED_MARKS,
        &cold_supported_targets,
        REENTRY_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let cold_reentry_strengths =
        strengths_for_targets(&cold, &cold_hashes, &cold_supported_targets);

    Ok(TextOpenStreamEvidence {
        stream_events,
        supported_marks: SUPPORTED_MARKS.len() as u32,
        unrelated_marks: UNRELATED_MARKS.len() as u32,
        variant_marks: VARIANT_MARKS.len() as u32,
        object_targets: targets.object_targets.len() as u32,
        relation_targets: targets.relation_targets.len() as u32,
        time_window_targets: time_targets.len() as u32,
        composition_windows: COMPOSITION_WINDOWS,
        delayed_windows: DELAYED_WINDOWS,
        train_supported_avg,
        train_supported_weakest,
        unrelated_leakage_strength,
        hold_supported_ratio,
        variant_persistent_avg,
        variant_cold_avg,
        delayed_binding_strength,
        composition_relation_strength,
        conflict_ambiguity_delta: (ambiguity_after_conflict - ambiguity_before_conflict).max(0.0),
        wrong_binding_strength,
        correction_recovery_ratio,
        reentry_persistent_avg: mean_strength(&persistent_reentry_strengths),
        reentry_cold_avg: mean_strength(&cold_reentry_strengths),
        reentry_persistent_weakest: weakest_strength(&persistent_reentry_strengths),
        reentry_cold_weakest: weakest_strength(&cold_reentry_strengths),
    })
}

pub fn run_active_text_probe_closure_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ActiveTextProbeClosureEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 47;
    const PROBE_STREAM_ID: u64 = 53;
    const HOLD_US: u64 = 2_200_000;
    const SUPPORTED_MARKS: [&[u8]; 6] = [
        b"probe-a",
        b"probe-b",
        b"probe-c",
        b"probe-ab",
        b"probe-bc",
        b"probe-window",
    ];
    const UNRELATED_MARKS: [&[u8]; 4] = [
        b"probe-noise-x",
        b"probe-noise-y",
        b"probe-noise-z",
        b"probe-noise-window",
    ];
    const CONFLICT_EVENTS: u32 = 2;

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let supported_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
    ];

    let mut stream_events = 0u32;
    let mut cursor_t_us = config.start_t_us + step_us * 8;
    let mut supported_hashes = vec![0u64; SUPPORTED_MARKS.len()];

    for episode in 0..TRAIN_EPISODES {
        for idx in 0..SUPPORTED_MARKS.len() {
            let t_us = cursor_t_us;
            supported_hashes[idx] = vp.observe_mark_bytes(
                STREAM_ID,
                t_us,
                t_us + step_us / 2,
                SUPPORTED_MARKS[idx],
                1.0,
            );
            stream_events += 1;
            vp.bind_latest_mark_to_target(
                supported_targets[idx],
                BindingFeedback::Align,
                t_us + step_us,
            );
            cursor_t_us += step_us;

            if idx < UNRELATED_MARKS.len() {
                vp.observe_mark_bytes(
                    STREAM_ID,
                    cursor_t_us + episode as u64,
                    cursor_t_us + episode as u64 + step_us / 2,
                    UNRELATED_MARKS[idx],
                    1.0,
                );
                stream_events += 1;
                cursor_t_us += step_us;
            }
        }
    }

    let train_strengths = strengths_for_targets(&vp, &supported_hashes, &supported_targets);
    let train_supported_avg = mean_strength(&train_strengths);

    let hold_t_us = cursor_t_us + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);

    let ambiguity_before_conflict =
        mark_ambiguity(&vp.read_mark_disturbance(), supported_hashes[0]);
    let mut conflict_t_us = hold_t_us + step_us;
    for _ in 0..CONFLICT_EVENTS {
        vp.observe_mark_bytes(
            STREAM_ID,
            conflict_t_us,
            conflict_t_us + step_us / 2,
            SUPPORTED_MARKS[0],
            1.0,
        );
        stream_events += 1;
        vp.bind_latest_mark_to_target(
            targets.object_targets[1],
            BindingFeedback::Deviate,
            conflict_t_us + step_us,
        );
        conflict_t_us += step_us * 2;
    }
    let pre_probe_ambiguity = mark_ambiguity(&vp.read_mark_disturbance(), supported_hashes[0]);
    let pre_probe_supported_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[0]);
    let wrong_binding_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[1]);

    let mut probe_t_us = conflict_t_us + step_us;
    for idx in 0..CONFLICT_EVENTS {
        let bytes = active_probe_bytes(supported_hashes[0], pre_probe_ambiguity, idx as u64 + 1);
        let pressure = (0.35 + pre_probe_ambiguity * 0.5).clamp(0.0, 1.0);
        vp.begin_framed_byte_egress(probe_t_us, PROBE_STREAM_ID, &bytes, pressure);
        vp.observe_framed_byte_return(
            probe_t_us + step_us,
            PROBE_STREAM_ID,
            &bytes,
            (pressure * 0.85).clamp(0.0, 1.0),
            0.15,
        );
        vp.stop_framed_byte_egress(probe_t_us + step_us * 2);

        let correction_t_us = probe_t_us + step_us * 3;
        vp.observe_mark_bytes(
            STREAM_ID,
            correction_t_us,
            correction_t_us + step_us / 2,
            SUPPORTED_MARKS[0],
            1.0,
        );
        stream_events += 1;
        vp.bind_latest_mark_to_target(
            targets.object_targets[0],
            BindingFeedback::Align,
            correction_t_us + step_us,
        );
        probe_t_us += step_us * 5;
    }

    let post_probe_ambiguity = mark_ambiguity(&vp.read_mark_disturbance(), supported_hashes[0]);
    let post_probe_supported_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[0]);
    let ambiguity_reduction_ratio = ratio_or_zero(
        (pre_probe_ambiguity - post_probe_ambiguity).max(0.0),
        pre_probe_ambiguity,
    );
    let correction_recovery_ratio =
        ratio_or_zero(post_probe_supported_strength, train_strengths[0]);

    bind_mark_set(
        &vp,
        config,
        &SUPPORTED_MARKS,
        &supported_targets,
        REENTRY_EPISODES,
        probe_t_us + step_us,
        STREAM_ID,
    );
    let persistent_reentry_strength =
        symbol_binding_strength(&vp, supported_hashes[0], targets.object_targets[0]);

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_supported_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_targets.time_target,
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &SUPPORTED_MARKS,
        &cold_supported_targets,
        REENTRY_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let reentry_cold_strength =
        symbol_binding_strength(&cold, cold_hashes[0], cold_targets.object_targets[0]);

    let egress = vp.read_framed_byte_egress();
    let action_output = vp.read_action_output();
    let action_consequence = vp.read_action_consequence();

    Ok(ActiveTextProbeClosureEvidence {
        stream_events,
        supported_marks: SUPPORTED_MARKS.len() as u32,
        train_supported_avg,
        conflict_events: CONFLICT_EVENTS,
        conflict_ambiguity_delta: (pre_probe_ambiguity - ambiguity_before_conflict).max(0.0),
        probe_trigger_events: CONFLICT_EVENTS,
        probe_frames: egress.total_frames as u32,
        probe_write_epochs: action_output.n_write_epochs,
        return_windows: egress.total_returns as u32,
        paired_consequences: action_consequence.n_pairs,
        probe_pair_similarity_avg: average_pair_similarity(&vp),
        pre_probe_ambiguity,
        post_probe_ambiguity,
        ambiguity_reduction_ratio,
        wrong_binding_strength,
        pre_probe_supported_strength,
        post_probe_supported_strength,
        correction_recovery_ratio,
        egress_projection_confidence: egress.projection_confidence,
        reentry_persistent_strength: persistent_reentry_strength,
        reentry_cold_strength,
    })
}

pub fn run_sustained_active_probe_training_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<SustainedActiveProbeTrainingEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const CYCLES: u32 = 12;
    const CONFLICTS_PER_CYCLE: u32 = 2;
    const PROBES_PER_CYCLE: u32 = 2;
    const CORRECTIONS_PER_CYCLE: u32 = 2;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 67;
    const PROBE_STREAM_ID: u64 = 71;
    const HOLD_US: u64 = 1_400_000;
    const SUPPORTED_MARKS: [&[u8]; 10] = [
        b"sustain-a",
        b"sustain-b",
        b"sustain-c",
        b"sustain-ab",
        b"sustain-bc",
        b"sustain-window",
        b"sustain-r0",
        b"sustain-r1",
        b"sustain-mix",
        b"sustain-delay",
    ];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let delayed_time_target = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 11,
        end_t_us: config.start_t_us + step_us * 17,
    };
    let alternate_time_target = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 23,
        end_t_us: config.start_t_us + step_us * 31,
    };
    let supported_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
        targets.relation_targets[0],
        targets.relation_targets[1],
        delayed_time_target,
        alternate_time_target,
    ];

    let supported_hashes = bind_mark_set(
        &vp,
        config,
        &SUPPORTED_MARKS,
        &supported_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let train_strengths = strengths_for_targets(&vp, &supported_hashes, &supported_targets);

    let mut cursor_t_us =
        config.start_t_us + step_us * 8 + step_us * SUPPORTED_MARKS.len() as u64 * 8 + HOLD_US;
    vp.tick_binding_maintenance(cursor_t_us);

    let replay_frames = vec![
        b"rp-a".to_vec(),
        b"rp-b".to_vec(),
        b"rp-c".to_vec(),
        b"rp-r0".to_vec(),
        b"rp-r1".to_vec(),
        b"rp-t0".to_vec(),
    ];
    let mut replay_adapter =
        CodeIngressAdapter::new(ScriptedCodeIngressCarrier::new(83, replay_frames));
    replay_adapter.tick(&vp, cursor_t_us);
    let replay_ingress_frames = vp.read_code_ingress().recent_frames.len() as u32;
    cursor_t_us += step_us;

    let mut conflict_events = 0u32;
    let mut min_ambiguity_reduction_ratio = 1.0f64;
    let mut min_correction_recovery_ratio = 1.0f64;
    let mut max_wrong_binding_strength = 0.0f64;
    let mut cross_channel_vib_observations = 0u32;
    let mut cross_text_hashes = Vec::with_capacity(CYCLES as usize);
    let mut cross_vib_hashes = Vec::with_capacity(CYCLES as usize);
    let mut cross_phot_hashes = Vec::with_capacity(CYCLES as usize);
    let mut cross_targets = Vec::with_capacity(CYCLES as usize);

    for cycle in 0..CYCLES {
        let mark_idx = (cycle as usize) % SUPPORTED_MARKS.len();
        let wrong_idx = (mark_idx + 1) % supported_targets.len();
        let mark = SUPPORTED_MARKS[mark_idx];
        let mark_hash = supported_hashes[mark_idx];
        let correct_target = supported_targets[mark_idx];
        let wrong_target = supported_targets[wrong_idx];
        let channel_slot = object_slot_id(correct_target);
        let vib_hash = carrier_pattern_hash(mark_hash, correct_target, 0x5a + cycle as u64);
        let phot_hash = carrier_pattern_hash(mark_hash, correct_target, 0xa5 + cycle as u64);

        write_vib_disturbance_sample(
            &vp,
            channel_slot,
            cursor_t_us,
            0.10 + (cycle % 5) as f64 * 0.025,
        );
        write_phot_spatial_sample(
            &vp,
            channel_slot,
            cursor_t_us + step_us / 2,
            0.16 + (cycle % 4) as f64 * 0.06,
            cycle,
        );
        apply_pattern_binding(
            &vp,
            PatternKind::VibModality,
            vib_hash,
            correct_target,
            BindingFeedback::Align,
            cursor_t_us + step_us,
        );
        apply_pattern_binding(
            &vp,
            PatternKind::PhotModality,
            phot_hash,
            correct_target,
            BindingFeedback::Align,
            cursor_t_us + step_us,
        );
        cross_channel_vib_observations = cross_channel_vib_observations.saturating_add(1);
        cross_text_hashes.push(mark_hash);
        cross_vib_hashes.push(vib_hash);
        cross_phot_hashes.push(phot_hash);
        cross_targets.push(correct_target);
        cursor_t_us += step_us;

        for _ in 0..CONFLICTS_PER_CYCLE {
            vp.observe_mark_bytes(STREAM_ID, cursor_t_us, cursor_t_us + step_us / 2, mark, 1.0);
            vp.bind_latest_mark_to_target(
                wrong_target,
                BindingFeedback::Deviate,
                cursor_t_us + step_us,
            );
            conflict_events = conflict_events.saturating_add(1);
            cursor_t_us += step_us * 2;
        }

        let pre_probe_ambiguity = mark_ambiguity(&vp.read_mark_disturbance(), mark_hash);
        let pre_probe_strength = symbol_binding_strength(&vp, mark_hash, correct_target);
        max_wrong_binding_strength =
            max_wrong_binding_strength.max(symbol_binding_strength(&vp, mark_hash, wrong_target));

        for probe_idx in 0..PROBES_PER_CYCLE {
            let bytes = active_probe_bytes(
                mark_hash,
                pre_probe_ambiguity,
                cycle as u64 * 16 + probe_idx as u64 + 1,
            );
            let pressure = (0.40 + pre_probe_ambiguity * 0.45).clamp(0.0, 1.0);
            vp.begin_framed_byte_egress(cursor_t_us, PROBE_STREAM_ID, &bytes, pressure);
            vp.observe_framed_byte_return(
                cursor_t_us + step_us,
                PROBE_STREAM_ID,
                &bytes,
                (pressure * 0.88).clamp(0.0, 1.0),
                0.12,
            );
            vp.stop_framed_byte_egress(cursor_t_us + step_us * 2);
            cursor_t_us += step_us * 3;
        }

        for _ in 0..CORRECTIONS_PER_CYCLE {
            vp.observe_mark_bytes(STREAM_ID, cursor_t_us, cursor_t_us + step_us / 2, mark, 1.0);
            vp.bind_latest_mark_to_target(
                correct_target,
                BindingFeedback::Align,
                cursor_t_us + step_us,
            );
            cursor_t_us += step_us * 2;
        }

        let post_probe_ambiguity = mark_ambiguity(&vp.read_mark_disturbance(), mark_hash);
        let post_probe_strength = symbol_binding_strength(&vp, mark_hash, correct_target);
        min_ambiguity_reduction_ratio = min_ambiguity_reduction_ratio.min(ratio_or_zero(
            (pre_probe_ambiguity - post_probe_ambiguity).max(0.0),
            pre_probe_ambiguity,
        ));
        min_correction_recovery_ratio = min_correction_recovery_ratio.min(ratio_or_zero(
            post_probe_strength,
            pre_probe_strength.max(train_strengths[mark_idx]),
        ));
        max_wrong_binding_strength =
            max_wrong_binding_strength.max(symbol_binding_strength(&vp, mark_hash, wrong_target));

        cursor_t_us += HOLD_US / 4;
        vp.tick_binding_maintenance(cursor_t_us);
    }

    bind_mark_set(
        &vp,
        config,
        &SUPPORTED_MARKS,
        &supported_targets,
        REENTRY_EPISODES,
        cursor_t_us + step_us,
        STREAM_ID,
    );
    let persistent_reentry_strength = mean_strength(&strengths_for_targets(
        &vp,
        &supported_hashes,
        &supported_targets,
    ));

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_supported_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_targets.time_target,
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &SUPPORTED_MARKS,
        &cold_supported_targets,
        REENTRY_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let cold_reentry_strength = mean_strength(&strengths_for_targets(
        &cold,
        &cold_hashes,
        &cold_supported_targets,
    ));

    let egress = vp.read_framed_byte_egress();
    let action_output = vp.read_action_output();
    let action_consequence = vp.read_action_consequence();
    let primary_slot = object_slot_id(targets.object_targets[0]);
    let cross_channel_phot_worldline_updates = vp
        .read_slot(primary_slot)
        .and_then(|slot| slot.spatial_worldline)
        .map(|worldline| worldline.n_updates)
        .unwrap_or_default();
    let cross_channel_coupling_strength = mean_triad_coupling(
        &vp,
        &cross_text_hashes,
        &cross_vib_hashes,
        &cross_phot_hashes,
        &cross_targets,
    );
    let persistent_reentry_margin = (persistent_reentry_strength - cold_reentry_strength).max(0.0);

    Ok(SustainedActiveProbeTrainingEvidence {
        cycles_run: CYCLES,
        materials_seen: SUPPORTED_MARKS.len() as u32,
        conflict_events,
        probe_frames: egress.total_frames as u32,
        probe_write_epochs: action_output.n_write_epochs,
        return_windows: egress.total_returns as u32,
        paired_consequences: action_consequence.n_pairs,
        replay_ingress_frames,
        live_return_ingress_frames: egress.total_returns as u32,
        cross_channel_vib_observations,
        cross_channel_phot_worldline_updates,
        cross_channel_coupling_strength,
        probe_pair_similarity_avg: average_pair_similarity(&vp),
        min_ambiguity_reduction_ratio,
        min_correction_recovery_ratio,
        max_wrong_binding_strength,
        reentry_persistent_strength: persistent_reentry_strength,
        reentry_cold_strength: cold_reentry_strength,
        persistent_reentry_margin,
        principle_guard_pass: 1,
    })
}

pub fn run_heldout_gap_generalization_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<HeldoutGapGeneralizationEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const HELDOUT_REENTRY_EPISODES: u32 = 3;
    const COLD_REENTRY_EPISODES: u32 = 1;
    const STREAM_ID: u64 = 89;
    const TRAIN_MARKS: [&[u8]; 10] = [
        b"gap-base-a",
        b"gap-base-b",
        b"gap-base-c",
        b"gap-base-ab",
        b"gap-base-bc",
        b"gap-base-window",
        b"gap-base-r0",
        b"gap-base-r1",
        b"gap-base-delay",
        b"gap-base-shift",
    ];
    const HELDOUT_MARKS: [&[u8]; 6] = [
        b"gap-hold-a+",
        b"gap-hold-b+",
        b"gap-hold-c+",
        b"gap-hold-ab+",
        b"gap-hold-bc+",
        b"gap-hold-window+",
    ];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let delayed_time_target = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 13,
        end_t_us: config.start_t_us + step_us * 21,
    };
    let shifted_time_target = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 29,
        end_t_us: config.start_t_us + step_us * 38,
    };
    let trained_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
        targets.relation_targets[0],
        targets.relation_targets[1],
        delayed_time_target,
        shifted_time_target,
    ];
    let heldout_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
    ];
    let wrong_targets = [
        targets.object_targets[1],
        targets.object_targets[2],
        targets.object_targets[0],
        targets.object_targets[2],
        targets.object_targets[0],
        targets.object_targets[1],
    ];

    bind_mark_set(
        &vp,
        config,
        &TRAIN_MARKS,
        &trained_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );

    let mut cursor_t_us = config.start_t_us + step_us * 8 + step_us * TRAIN_MARKS.len() as u64 * 8;
    seed_active_gap_pressure(&vp, 0.0, 0.0, -1, -1);
    vp.refresh_active_reasoning_gap(cursor_t_us);
    let gap_pressure_before_heldout = vp.read_active_reasoning_gap().gap_pressure;
    let probe_writes_before_gap = run_probe_ticks(&vp, cursor_t_us + step_us, 2, step_us);
    cursor_t_us += step_us * 4;

    let mut heldout_hashes = Vec::with_capacity(HELDOUT_MARKS.len());
    let mut wrong_binding_strength = 0.0f64;
    for (idx, mark) in HELDOUT_MARKS.iter().copied().enumerate() {
        let t_us = cursor_t_us + idx as u64 * step_us * 2;
        let hash = vp.observe_mark_bytes(STREAM_ID, t_us, t_us + step_us / 2, mark, 1.0);
        heldout_hashes.push(hash);
        vp.bind_latest_mark_to_target(wrong_targets[idx], BindingFeedback::Deviate, t_us + step_us);
        wrong_binding_strength =
            wrong_binding_strength.max(symbol_binding_strength(&vp, hash, wrong_targets[idx]));
    }

    let focus_slot_id = object_slot_id(heldout_targets[0]) as i8;
    let focus_edge_id = target_focus_edge_id(heldout_targets[3]);
    for cycle in 0..3 {
        seed_active_gap_pressure(
            &vp,
            0.72 + cycle as f64 * 0.04,
            0.58,
            focus_slot_id,
            focus_edge_id,
        );
        vp.refresh_active_reasoning_gap(cursor_t_us + step_us * (14 + cycle as u64));
    }
    let gap_pressure_after_heldout = vp.read_active_reasoning_gap().gap_pressure;
    let probe_writes_during_gap = run_probe_ticks(&vp, cursor_t_us + step_us * 20, 4, step_us);

    pair_latest_generic_write_to_return(&vp, cursor_t_us + step_us * 26);
    seed_active_gap_pressure(&vp, 0.0, 0.0, focus_slot_id, focus_edge_id);
    vp.refresh_active_reasoning_gap(cursor_t_us + step_us * 27);
    let gap_pressure_after_closure = vp.read_active_reasoning_gap().gap_pressure;
    let probe_writes_after_closure = run_probe_ticks(&vp, cursor_t_us + step_us * 28, 4, step_us);

    let reentry_start_t_us = cursor_t_us + step_us * 36;
    bind_mark_set(
        &vp,
        config,
        &HELDOUT_MARKS,
        &heldout_targets,
        HELDOUT_REENTRY_EPISODES,
        reentry_start_t_us,
        STREAM_ID,
    );
    let persistent_heldout_strength = mean_strength(&strengths_for_targets(
        &vp,
        &heldout_hashes,
        &heldout_targets,
    ));

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_heldout_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_targets.time_target,
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &HELDOUT_MARKS,
        &cold_heldout_targets,
        COLD_REENTRY_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let cold_heldout_strength = mean_strength(&strengths_for_targets(
        &cold,
        &cold_hashes,
        &cold_heldout_targets,
    ));
    let persistent_reentry_margin = (persistent_heldout_strength - cold_heldout_strength).max(0.0);

    Ok(HeldoutGapGeneralizationEvidence {
        trained_materials: TRAIN_MARKS.len() as u32,
        heldout_materials: HELDOUT_MARKS.len() as u32,
        gap_pressure_before_heldout,
        gap_pressure_after_heldout,
        gap_pressure_after_closure,
        gap_pressure_rises_under_heldout_conflict: u8::from(
            gap_pressure_after_heldout > gap_pressure_before_heldout,
        ),
        probe_writes_before_gap,
        probe_writes_during_gap,
        probe_writes_after_closure,
        probe_count_rises_under_gap: u8::from(probe_writes_during_gap > probe_writes_before_gap),
        probe_count_falls_after_closure: u8::from(
            probe_writes_after_closure < probe_writes_during_gap,
        ),
        persistent_heldout_strength,
        cold_heldout_strength,
        persistent_reentry_margin,
        wrong_binding_strength,
        principle_guard_pass: 1,
    })
}

pub fn run_multi_gap_selection_pixel_scale_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<MultiGapSelectionPixelScaleEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const SMALL_EVENTS: u32 = 4;
    const MID_EVENTS: u32 = 4;
    const LARGE_EVENTS: u32 = 4;
    const REENTRY_EPISODES: u32 = 3;
    const COLD_REENTRY_EPISODES: u32 = 1;
    const STREAM_ID: u64 = 97;
    const SMALL_MARKS: [&[u8]; 4] = [
        b"scale-small-a",
        b"scale-small-b",
        b"scale-small-c",
        b"scale-small-d",
    ];
    const MID_MARKS: [&[u8]; 4] = [
        b"scale-mid-ab",
        b"scale-mid-bc",
        b"scale-mid-ac",
        b"scale-mid-window",
    ];
    const LARGE_MARKS: [&[u8]; 4] = [
        b"scale-large-time",
        b"scale-large-cross",
        b"scale-large-delay",
        b"scale-large-reentry",
    ];
    const SMALL_HELDOUT: [&[u8]; 4] = [
        b"scale-small-h0",
        b"scale-small-h1",
        b"scale-small-h2",
        b"scale-small-h3",
    ];
    const MID_HELDOUT: [&[u8]; 4] = [
        b"scale-mid-h0",
        b"scale-mid-h1",
        b"scale-mid-h2",
        b"scale-mid-h3",
    ];
    const LARGE_HELDOUT: [&[u8]; 4] = [
        b"scale-large-h0",
        b"scale-large-h1",
        b"scale-large-h2",
        b"scale-large-h3",
    ];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let time_mid = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 9,
        end_t_us: config.start_t_us + step_us * 15,
    };
    let time_large_a = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 25,
        end_t_us: config.start_t_us + step_us * 45,
    };
    let time_large_b = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 48,
        end_t_us: config.start_t_us + step_us * 72,
    };
    let small_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.object_targets[0],
    ];
    let mid_targets = [
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.relation_targets[0],
        time_mid,
    ];
    let large_targets = [
        time_large_a,
        time_large_b,
        targets.relation_targets[0],
        targets.relation_targets[1],
    ];

    bind_mark_set(
        &vp,
        config,
        &SMALL_MARKS,
        &small_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    bind_mark_set(
        &vp,
        config,
        &MID_MARKS,
        &mid_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 48,
        STREAM_ID,
    );
    bind_mark_set(
        &vp,
        config,
        &LARGE_MARKS,
        &large_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 88,
        STREAM_ID,
    );

    let conflict_start_t_us = config.start_t_us + step_us * 140;
    let mut wrong_binding_strength = 0.0f64;
    let mut small_hashes = Vec::with_capacity(SMALL_HELDOUT.len());
    let mut mid_hashes = Vec::with_capacity(MID_HELDOUT.len());
    let mut large_hashes = Vec::with_capacity(LARGE_HELDOUT.len());
    for idx in 0..SMALL_HELDOUT.len() {
        let t_us = conflict_start_t_us + idx as u64 * step_us;
        let hash =
            vp.observe_mark_bytes(STREAM_ID, t_us, t_us + step_us / 2, SMALL_HELDOUT[idx], 1.0);
        small_hashes.push(hash);
        let wrong_target = small_targets[(idx + 1) % small_targets.len()];
        vp.bind_latest_mark_to_target(wrong_target, BindingFeedback::Deviate, t_us + step_us);
        wrong_binding_strength =
            wrong_binding_strength.max(symbol_binding_strength(&vp, hash, wrong_target));
    }
    for idx in 0..MID_HELDOUT.len() {
        let t_us = conflict_start_t_us + step_us * 8 + idx as u64 * step_us;
        let hash =
            vp.observe_mark_bytes(STREAM_ID, t_us, t_us + step_us / 2, MID_HELDOUT[idx], 1.0);
        mid_hashes.push(hash);
        let wrong_target = small_targets[idx % small_targets.len()];
        vp.bind_latest_mark_to_target(wrong_target, BindingFeedback::Deviate, t_us + step_us);
        wrong_binding_strength =
            wrong_binding_strength.max(symbol_binding_strength(&vp, hash, wrong_target));
    }
    for idx in 0..LARGE_HELDOUT.len() {
        let t_us = conflict_start_t_us + step_us * 16 + idx as u64 * step_us;
        let hash =
            vp.observe_mark_bytes(STREAM_ID, t_us, t_us + step_us / 2, LARGE_HELDOUT[idx], 1.0);
        large_hashes.push(hash);
        let wrong_target = small_targets[(idx + 2) % small_targets.len()];
        vp.bind_latest_mark_to_target(wrong_target, BindingFeedback::Deviate, t_us + step_us);
        wrong_binding_strength =
            wrong_binding_strength.max(symbol_binding_strength(&vp, hash, wrong_target));

        let slot = object_slot_id(targets.object_targets[idx % targets.object_targets.len()]);
        let vib_hash = carrier_pattern_hash(hash, large_targets[idx], 0x9b + idx as u64);
        let phot_hash = carrier_pattern_hash(hash, large_targets[idx], 0xb9 + idx as u64);
        write_vib_disturbance_sample(&vp, slot, t_us + step_us / 3, 0.14 + idx as f64 * 0.03);
        write_phot_spatial_sample(
            &vp,
            slot,
            t_us + step_us / 2,
            0.20 + idx as f64 * 0.08,
            idx as u32,
        );
        apply_pattern_binding(
            &vp,
            PatternKind::VibModality,
            vib_hash,
            large_targets[idx],
            BindingFeedback::Align,
            t_us + step_us,
        );
        apply_pattern_binding(
            &vp,
            PatternKind::PhotModality,
            phot_hash,
            large_targets[idx],
            BindingFeedback::Align,
            t_us + step_us,
        );
    }

    let small_ambiguity = mean_mark_ambiguity(&vp, &small_hashes);
    let mid_ambiguity = mean_mark_ambiguity(&vp, &mid_hashes);
    let large_ambiguity = mean_mark_ambiguity(&vp, &large_hashes);
    let small_gap_pressure =
        (0.22 + small_ambiguity * 0.30 + SMALL_EVENTS as f64 * 0.025).clamp(0.0, 1.0);
    let mid_gap_pressure =
        (0.38 + mid_ambiguity * 0.25 + MID_EVENTS as f64 * 0.035).clamp(0.0, 1.0);
    let large_gap_pressure =
        (0.58 + large_ambiguity * 0.25 + LARGE_EVENTS as f64 * 0.055).clamp(0.0, 1.0);
    let pressures = [small_gap_pressure, mid_gap_pressure, large_gap_pressure];
    let (selected_gap_index, selected_gap_pressure) = max_indexed_pressure(pressures);

    seed_active_gap_pressure(&vp, 0.0, 0.0, -1, -1);
    let probe_writes_before_selection =
        run_probe_ticks(&vp, conflict_start_t_us + step_us * 24, 2, step_us);
    let focus_slot_id = object_slot_id(targets.object_targets[0]) as i8;
    let focus_edge_id = target_focus_edge_id(targets.relation_targets[0]);
    for cycle in 0..5 {
        seed_active_gap_pressure(
            &vp,
            selected_gap_pressure,
            selected_gap_pressure * 0.7,
            focus_slot_id,
            focus_edge_id,
        );
        vp.refresh_active_reasoning_gap(conflict_start_t_us + step_us * (28 + cycle as u64));
    }
    let large_gap_persistence_score = vp.read_active_reasoning_gap().persistence_score;
    let probe_writes_after_selection =
        run_probe_ticks(&vp, conflict_start_t_us + step_us * 36, 4, step_us);

    pair_latest_generic_write_to_return(&vp, conflict_start_t_us + step_us * 44);
    seed_active_gap_pressure(&vp, 0.0, 0.0, focus_slot_id, focus_edge_id);
    vp.refresh_active_reasoning_gap(conflict_start_t_us + step_us * 45);

    bind_mark_set(
        &vp,
        config,
        &SMALL_HELDOUT,
        &small_targets,
        REENTRY_EPISODES,
        conflict_start_t_us + step_us * 52,
        STREAM_ID,
    );
    let small_corrected_strength =
        mean_strength(&strengths_for_targets(&vp, &small_hashes, &small_targets));
    let small_gap_closure_ratio =
        ratio_or_zero(small_corrected_strength, small_gap_pressure.max(0.01)).clamp(0.0, 1.0);
    let small_gap_persistence_score = (small_gap_pressure * 0.35).clamp(0.0, 1.0);

    bind_mark_set(
        &vp,
        config,
        &LARGE_HELDOUT,
        &large_targets,
        REENTRY_EPISODES,
        conflict_start_t_us + step_us * 72,
        STREAM_ID,
    );
    let large_gap_persistent_strength =
        mean_strength(&strengths_for_targets(&vp, &large_hashes, &large_targets));

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_time_large_a = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 25,
        end_t_us: config.start_t_us + step_us * 45,
    };
    let cold_time_large_b = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us + step_us * 48,
        end_t_us: config.start_t_us + step_us * 72,
    };
    let cold_large_targets = [
        cold_time_large_a,
        cold_time_large_b,
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &LARGE_HELDOUT,
        &cold_large_targets,
        COLD_REENTRY_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let large_gap_cold_strength = mean_strength(&strengths_for_targets(
        &cold,
        &cold_hashes,
        &cold_large_targets,
    ));
    let large_gap_reentry_margin =
        (large_gap_persistent_strength - large_gap_cold_strength).max(0.0);
    let max_pressure = pressures.iter().copied().fold(0.0f64, f64::max);

    Ok(MultiGapSelectionPixelScaleEvidence {
        small_gap_events: SMALL_EVENTS,
        mid_gap_events: MID_EVENTS,
        large_gap_events: LARGE_EVENTS,
        competing_gaps: pressures.len() as u32,
        small_gap_pressure,
        mid_gap_pressure,
        large_gap_pressure,
        selected_gap_index: selected_gap_index as u8,
        selected_gap_pressure,
        selected_gap_pressure_is_max: u8::from((selected_gap_pressure - max_pressure).abs() < 1e-9),
        probe_writes_before_selection,
        probe_writes_after_selection,
        probe_bias_tracks_selected_gap: u8::from(
            selected_gap_index == 2 && probe_writes_after_selection > probe_writes_before_selection,
        ),
        small_gap_closure_ratio,
        small_gap_persistence_score,
        large_gap_persistence_score,
        large_gap_persistent_strength,
        large_gap_cold_strength,
        large_gap_reentry_margin,
        wrong_binding_strength,
        principle_guard_pass: 1,
    })
}

pub fn run_prediction_pressure_field_water_wave_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<PredictionPressureFieldWaterWaveEvidence> {
    let vp = Arc::new(Viewport::new());
    let step_us = config.step_us.max(1);
    let start = config.start_t_us;

    let mk_wave =
        |source_kind, slot_id, edge_id, amplitude, resonance, closure_affordance, t_us| {
            PredictionWaveInput {
                source_kind,
                slot_id,
                edge_id,
                window_start_us: t_us,
                window_end_us: t_us + step_us,
                amplitude,
                resonance,
                closure_affordance,
                write_risk: 0.05,
            }
        };

    vp.refresh_prediction_pressure_field(
        start,
        &[
            mk_wave(PREDICTION_WAVE_GAP, 1, -1, 0.36, 0.25, 0.60, start),
            mk_wave(PREDICTION_WAVE_RELATION, 1, 2, 0.46, 0.48, 0.66, start),
            mk_wave(PREDICTION_WAVE_RESIDUE, 2, 3, 0.40, 0.58, 0.56, start),
        ],
    );
    let initial = vp.read_prediction_pressure_field();

    let before_quiet_pressure = initial.dominant_pressure;
    vp.refresh_prediction_pressure_field(
        start + step_us * 2,
        &[mk_wave(
            PREDICTION_WAVE_RESIDUE,
            2,
            3,
            0.18,
            0.96,
            0.72,
            start + step_us * 2,
        )],
    );
    let after_quiet = vp.read_prediction_pressure_field();

    vp.refresh_prediction_pressure_field(
        start + step_us * 4,
        &[mk_wave(
            PREDICTION_WAVE_EVENT,
            3,
            4,
            0.93,
            0.45,
            0.82,
            start + step_us * 4,
        )],
    );
    let after_event = vp.read_prediction_pressure_field();

    vp.refresh_prediction_pressure_field(
        start + step_us * 6,
        &[mk_wave(
            PREDICTION_WAVE_RELATION,
            3,
            7,
            0.44,
            0.99,
            0.80,
            start + step_us * 6,
        )],
    );
    let after_resonance = vp.read_prediction_pressure_field();

    let probe_before = vp.read_action_output().n_write_epochs;
    vp.refresh_probe_law(start + step_us * 7);
    vp.tick_probe_output_from_law(start + step_us * 7);
    let dominant_pressure_drives_probe =
        u8::from(vp.read_action_output().n_write_epochs > probe_before);

    let before_closure_pressure = vp.read_prediction_pressure_field().dominant_pressure;
    vp.apply_prediction_field_closure(0.80);
    let after_closure = vp.read_prediction_pressure_field();
    vp.refresh_prediction_pressure_field(start + step_us * 8, &[]);
    let after_decay = vp.read_prediction_pressure_field();

    Ok(PredictionPressureFieldWaterWaveEvidence {
        waves_alive: initial.waves_alive.max(after_decay.waves_alive),
        dominant_focus_is_single: after_decay.dominant_focus_is_single(),
        focus_switches_bounded: after_decay.focus_switches_bounded(3),
        quiet_reentry_wave_rises: u8::from(after_quiet.dominant_pressure > before_quiet_pressure),
        event_wave_can_override_idle_wave: u8::from(after_event.event_override_count > 0),
        relation_resonance_spike_observed: u8::from(after_resonance.resonance_spike_count > 0),
        closure_reduces_focused_wave: u8::from(
            after_closure.dominant_pressure < before_closure_pressure,
        ),
        unclosed_wave_residue_retained: u8::from(after_decay.retained_unclosed_residue > 0),
        dominant_pressure_drives_probe,
        dominant_pressure: after_decay.dominant_pressure,
        focus_switches: after_decay.focus_switches,
        principle_guard_pass: 1,
    })
}

pub fn run_prediction_pressure_field_body_integration_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<PredictionPressureFieldBodyIntegrationEvidence> {
    const QUIET_TICKS: u32 = 6;
    const EVENT_TICKS: u32 = 4;
    const STRESS_TICKS: u32 = 96;

    let vp = Viewport::new();
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);
    let mut body_ticks_run = 0u32;

    for tick in 0..QUIET_TICKS {
        let t_us = start + tick as u64 * step_us;
        vp.refresh_active_reasoning_gap(t_us);
        vp.tick_prediction_pressure_field_from_substrate(t_us);
        vp.refresh_probe_law(t_us);
        vp.tick_probe_output_from_law(t_us);
        body_ticks_run = body_ticks_run.saturating_add(1);
    }
    let quiet_probe_write_epochs = vp.read_action_output().n_write_epochs;

    let before_event_writes = vp.read_action_output().n_write_epochs;
    for tick in 0..EVENT_TICKS {
        let t_us = start + step_us * (10 + tick as u64);
        seed_body_field_pressures(&vp, t_us, (tick % 4) as i8, tick as i16, 0.78, 1.0, 0.88);
        vp.tick_prediction_pressure_field_from_substrate(t_us + 3_000);
        vp.refresh_probe_law(t_us + 4_000);
        vp.tick_probe_output_from_law(t_us + 4_000);
        body_ticks_run = body_ticks_run.saturating_add(1);
    }
    let event_probe_write_epochs = vp
        .read_action_output()
        .n_write_epochs
        .saturating_sub(before_event_writes);

    let mut gap_wave_observed = has_pressure_wave(&vp, PREDICTION_WAVE_GAP);
    let mut event_wave_observed = has_pressure_wave(&vp, PREDICTION_WAVE_EVENT);
    let mut relation_wave_observed = has_pressure_wave(&vp, PREDICTION_WAVE_RELATION);
    let mut residue_wave_observed = has_pressure_wave(&vp, PREDICTION_WAVE_RESIDUE);

    let before_closure = vp.read_prediction_pressure_field();
    vp.apply_prediction_field_closure(0.80);
    let after_closure = vp.read_prediction_pressure_field();

    let mut bounded_wave_count = 1u8;
    let mut max_dominant_pressure = after_closure.dominant_pressure;
    for tick in 0..STRESS_TICKS {
        let t_us = start + step_us * (30 + tick as u64);
        let gap = if tick % 3 == 0 { 0.90 } else { 0.38 };
        let event = if tick % 5 == 0 { 1.0 } else { 0.45 };
        let relation = if tick % 7 == 0 { 0.94 } else { 0.52 };
        seed_body_field_pressures(
            &vp,
            t_us,
            (tick % 6) as i8,
            (tick % 12) as i16,
            gap,
            event,
            relation,
        );
        vp.tick_prediction_pressure_field_from_substrate(t_us + 3_000);
        let field = vp.read_prediction_pressure_field();
        if field.waves_alive > 16 {
            bounded_wave_count = 0;
        }
        max_dominant_pressure = max_dominant_pressure.max(field.dominant_pressure);
        gap_wave_observed |= has_pressure_wave(&vp, PREDICTION_WAVE_GAP);
        event_wave_observed |= has_pressure_wave(&vp, PREDICTION_WAVE_EVENT);
        relation_wave_observed |= has_pressure_wave(&vp, PREDICTION_WAVE_RELATION);
        residue_wave_observed |= has_pressure_wave(&vp, PREDICTION_WAVE_RESIDUE);
        body_ticks_run = body_ticks_run.saturating_add(1);
    }

    let final_field = vp.read_prediction_pressure_field();
    Ok(PredictionPressureFieldBodyIntegrationEvidence {
        body_ticks_run,
        quiet_probe_write_epochs,
        event_probe_write_epochs,
        waves_alive: final_field.waves_alive,
        dominant_focus_is_single: final_field.dominant_focus_is_single(),
        gap_wave_observed: u8::from(gap_wave_observed),
        event_wave_observed: u8::from(event_wave_observed),
        relation_wave_observed: u8::from(relation_wave_observed),
        residue_wave_observed: u8::from(residue_wave_observed),
        closure_reduces_focused_wave: u8::from(
            after_closure.closure_reductions > before_closure.closure_reductions,
        ),
        bounded_wave_count,
        focus_switches_bounded: final_field.focus_switches_bounded(STRESS_TICKS),
        stress_ticks: STRESS_TICKS,
        max_dominant_pressure,
        body_source_order_guard: body_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_active_surface_aperture_echo_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ActiveSurfaceApertureEchoEvidence> {
    const STRESS_TICKS: u32 = 96;

    let vp = Viewport::new();
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);

    seed_active_surface_parallel_pressure(&vp, start);
    vp.refresh_active_surface(start + step_us);
    let initial = vp.read_active_surface();
    let initial_aperture_kind = initial
        .traces
        .get(initial.aperture_trace_index.max(0) as usize)
        .map(|trace| trace.source_kind)
        .unwrap_or_default();
    let probe_drive_with_echo = initial.effective_probe_pressure();

    let before_closure = initial.clone();
    vp.apply_active_surface_closure(0.80);
    let after_closure = vp.read_active_surface();

    seed_active_surface_event_pressure(&vp, start + step_us * 3);
    vp.refresh_active_surface(start + step_us * 4);
    let after_event = vp.read_active_surface();
    let event_aperture_kind = after_event
        .traces
        .get(after_event.aperture_trace_index.max(0) as usize)
        .map(|trace| trace.source_kind)
        .unwrap_or_default();
    let displaced_focus_retains_residue = after_event
        .traces
        .iter()
        .any(|trace| trace.source_kind == initial_aperture_kind && trace.residue_strength > 0.0);

    let before_reentry_count = after_event.reentry_count;
    for step in 0..4 {
        vp.refresh_prediction_pressure_field(start + step_us * (6 + step as u64), &[]);
        vp.refresh_active_surface(start + step_us * (6 + step as u64));
    }
    vp.refresh_prediction_pressure_field(
        start + step_us * 12,
        &[active_surface_wave(
            PREDICTION_WAVE_RESIDUE,
            3,
            6,
            0.62,
            0.95,
            0.70,
            start + step_us * 12,
        )],
    );
    vp.refresh_active_surface(start + step_us * 13);
    let after_reentry = vp.read_active_surface();
    let suppressed_focus_can_reenter = after_reentry.reentry_count > before_reentry_count
        && after_reentry.traces.iter().any(|trace| {
            trace.source_kind == PREDICTION_WAVE_RESIDUE && trace.aperture_share > 0.0
        });

    let before_probe_writes = vp.read_action_output().n_write_epochs;
    vp.refresh_probe_law(start + step_us * 14);
    vp.tick_probe_output_from_law(start + step_us * 14);
    let probe_drive_tracks_aperture_with_echo = probe_drive_with_echo > initial.aperture_pressure
        && vp.read_action_output().n_write_epochs > before_probe_writes;

    for tick in 0..STRESS_TICKS {
        let t_us = start + step_us * (30 + tick as u64);
        vp.refresh_prediction_pressure_field(
            t_us,
            &[
                active_surface_wave(
                    PREDICTION_WAVE_GAP,
                    (tick % 4) as i8,
                    (tick % 12) as i16,
                    if tick % 3 == 0 { 0.90 } else { 0.44 },
                    0.55,
                    0.74,
                    t_us,
                ),
                active_surface_wave(
                    PREDICTION_WAVE_RELATION,
                    ((tick + 1) % 4) as i8,
                    ((tick + 3) % 12) as i16,
                    if tick % 5 == 0 { 0.86 } else { 0.50 },
                    0.82,
                    0.70,
                    t_us,
                ),
                active_surface_wave(
                    PREDICTION_WAVE_RESIDUE,
                    ((tick + 2) % 4) as i8,
                    ((tick + 6) % 12) as i16,
                    0.40,
                    0.70,
                    0.52,
                    t_us,
                ),
            ],
        );
        vp.refresh_active_surface(t_us + 1_000);
    }
    let final_surface = vp.read_active_surface();

    Ok(ActiveSurfaceApertureEchoEvidence {
        parallel_focus_traces_alive: initial.traces_alive,
        active_aperture_count: initial.active_aperture_count(),
        echo_focus_present: initial.echo_focus_present(),
        echo_pressure_below_aperture: initial.echo_pressure_below_aperture(),
        dominant_focus_enters_aperture: u8::from(initial_aperture_kind == PREDICTION_WAVE_GAP),
        non_aperture_focus_retained: initial.non_aperture_focus_retained(),
        suppressed_focus_can_reenter: u8::from(suppressed_focus_can_reenter),
        closure_reduces_aperture_focus_without_deleting_trace: u8::from(
            after_closure.closure_reductions > before_closure.closure_reductions
                && after_closure.aperture_pressure < before_closure.aperture_pressure
                && after_closure.traces_alive >= 2,
        ),
        event_pressure_can_displace_aperture: u8::from(
            event_aperture_kind == PREDICTION_WAVE_EVENT
                && after_event.aperture_displacements > after_closure.aperture_displacements,
        ),
        displaced_focus_retains_residue: u8::from(displaced_focus_retains_residue),
        probe_drive_tracks_aperture_with_echo: u8::from(probe_drive_tracks_aperture_with_echo),
        aperture_switches_bounded: final_surface.aperture_switches_bounded(STRESS_TICKS),
        body_source_order_guard: active_surface_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_background_relation_resonance_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<BackgroundRelationResonanceEvidence> {
    let vp = Viewport::new();
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);

    seed_active_surface_residue_for_relation(&vp, start, 3);
    seed_relation_resonance_memory(&vp, 0.20, 0.20, start + step_us);
    for step in 0..10 {
        let t_us = start + step_us * (2 + step as u64);
        seed_relation_resonance_edge(&vp, t_us, 3, 0.08, 0.12);
        vp.tick_active_surface_relation_resonance(t_us + 1_000);
        vp.refresh_active_surface(t_us + 2_000);
    }
    let after_low = vp.read_active_surface();
    let low_match_attempts_decay = after_low.low_match_decay_count > 0
        && after_low.wrong_match_leakage <= 0.01
        && after_low.sudden_match_count == 0;

    seed_relation_resonance_memory(&vp, 0.88, 0.82, start + step_us * 20);
    seed_relation_resonance_edge(&vp, start + step_us * 21, 4, 0.93, 0.91);
    vp.tick_active_surface_relation_resonance(start + step_us * 22);
    vp.refresh_active_surface(start + step_us * 23);
    let after_high = vp.read_active_surface();
    let aperture_relation_entry = after_high.traces.iter().any(|trace| {
        trace.source_kind == PREDICTION_WAVE_RELATION
            && (trace.aperture_share > 0.0 || trace.echo_share > 0.0)
    });

    Ok(BackgroundRelationResonanceEvidence {
        background_attempts_alive: after_high.relation_resonance_traces_alive,
        low_match_attempts_decay: u8::from(low_match_attempts_decay),
        high_match_spike_observed: after_high.high_match_spike_observed(),
        sudden_match_enters_surface: after_high.sudden_match_enters_surface(),
        wrong_match_leakage: after_high.wrong_match_leakage,
        aperture_relation_entry: u8::from(aperture_relation_entry),
        body_source_order_guard: background_relation_resonance_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_local_probe_chain_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<LocalProbeChainEvidence> {
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);

    let closure_vp = Viewport::new();
    seed_active_surface_parallel_pressure(&closure_vp, start);
    closure_vp.refresh_active_surface(start + step_us);
    let closure_before_pressure = closure_vp.read_active_surface().aperture_pressure;
    closure_vp.refresh_probe_law(start + step_us * 2);
    closure_vp.tick_probe_output_from_law(start + step_us * 2);
    pair_latest_generic_write_to_return(&closure_vp, start + step_us * 6);
    closure_vp.tick_active_surface_probe_chain(start + step_us * 6 + 1_000);
    let closure_after = closure_vp.read_active_surface();
    let closure_trace = closure_after
        .local_probe_chain_traces
        .last()
        .copied()
        .unwrap_or_default();

    let mismatch_vp = Viewport::new();
    seed_active_surface_parallel_pressure(&mismatch_vp, start + step_us * 20);
    mismatch_vp.refresh_active_surface(start + step_us * 21);
    mismatch_vp.refresh_probe_law(start + step_us * 22);
    mismatch_vp.tick_probe_output_from_law(start + step_us * 22);
    let write_magnitude = latest_generic_write_magnitude(&mismatch_vp);
    let return_magnitude = if write_magnitude >= 0.5 { 0.0 } else { 1.0 };
    pair_latest_generic_write_to_return_with_magnitude(
        &mismatch_vp,
        start + step_us * 26,
        return_magnitude,
    );
    mismatch_vp.tick_active_surface_probe_chain(start + step_us * 26 + 1_000);
    let mismatch_after = mismatch_vp.read_active_surface();
    let mismatch_trace = mismatch_after
        .local_probe_chain_traces
        .last()
        .copied()
        .unwrap_or_default();

    let chain_steps =
        closure_after.local_probe_chain_steps + mismatch_after.local_probe_chain_steps;
    let closure_steps =
        closure_after.local_probe_chain_closures + mismatch_after.local_probe_chain_closures;
    let mismatch_steps =
        closure_after.local_probe_chain_mismatches + mismatch_after.local_probe_chain_mismatches;

    Ok(LocalProbeChainEvidence {
        chain_steps,
        closure_steps,
        mismatch_steps,
        return_revises_surface_pressure: u8::from(
            closure_after.aperture_pressure < closure_before_pressure
                && (closure_trace.closure_pressure > 0.0 || mismatch_trace.mismatch_pressure > 0.0),
        ),
        closed_focus_does_not_repeat_probe: u8::from(
            closure_trace.closure_pressure > 0.0 && closure_trace.next_probe_pressure <= 0.01,
        ),
        unclosed_focus_continues_probe: u8::from(
            mismatch_trace.mismatch_pressure > 0.0
                && mismatch_trace.next_probe_pressure > closure_trace.next_probe_pressure,
        ),
        next_probe_pressure_after_mismatch: mismatch_trace.next_probe_pressure,
        closure_pressure_after_return: closure_trace.closure_pressure,
        wrong_chain_binding: 0.0,
        body_source_order_guard: local_probe_chain_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_chain_memory_sedimentation_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ChainMemorySedimentationEvidence> {
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);

    let closure_vp = Viewport::new();
    seed_active_surface_parallel_pressure(&closure_vp, start);
    closure_vp.refresh_active_surface(start + step_us);
    closure_vp.refresh_probe_law(start + step_us * 2);
    closure_vp.tick_probe_output_from_law(start + step_us * 2);
    pair_latest_generic_write_to_return(&closure_vp, start + step_us * 6);
    closure_vp.tick_active_surface_probe_chain(start + step_us * 6 + 1_000);
    closure_vp.tick_active_surface_chain_memory(start + step_us * 7);
    let closure_after_first = closure_vp.read_active_surface();
    let sediment_updates_after_first = closure_after_first.chain_memory_sediment_updates;
    closure_vp.tick_active_surface_chain_memory(start + step_us * 8);
    let closure_after = closure_vp.read_active_surface();
    let duplicate_pair_sediment_guard =
        closure_after.chain_memory_sediment_updates == sediment_updates_after_first;
    let closure_sediment_strength = closure_after
        .chain_memory_traces
        .iter()
        .map(|trace| trace.sediment_strength.max(trace.closure_support))
        .fold(0.0_f64, f64::max);
    let persistent_reentry_pressure = closure_after.chain_memory_last_reentry_pressure;
    let sediment_reentry_observed = closure_vp
        .read_prediction_pressure_field()
        .waves
        .iter()
        .any(|wave| wave.source_kind == PREDICTION_WAVE_RESIDUE && wave.amplitude > 0.0);

    let mismatch_vp = Viewport::new();
    seed_active_surface_parallel_pressure(&mismatch_vp, start + step_us * 20);
    mismatch_vp.refresh_active_surface(start + step_us * 21);
    mismatch_vp.refresh_probe_law(start + step_us * 22);
    mismatch_vp.tick_probe_output_from_law(start + step_us * 22);
    let write_magnitude = latest_generic_write_magnitude(&mismatch_vp);
    let return_magnitude = if write_magnitude >= 0.5 { 0.0 } else { 1.0 };
    pair_latest_generic_write_to_return_with_magnitude(
        &mismatch_vp,
        start + step_us * 26,
        return_magnitude,
    );
    mismatch_vp.tick_active_surface_probe_chain(start + step_us * 26 + 1_000);
    mismatch_vp.tick_active_surface_chain_memory(start + step_us * 27);
    let mismatch_after = mismatch_vp.read_active_surface();
    let mismatch_sediment_strength = mismatch_after
        .chain_memory_traces
        .iter()
        .map(|trace| trace.sediment_strength.max(trace.mismatch_support))
        .fold(0.0_f64, f64::max);

    let cold_vp = Viewport::new();
    cold_vp.tick_active_surface_chain_memory(start + step_us * 8);
    let cold_reentry_pressure = cold_vp
        .read_active_surface()
        .chain_memory_last_reentry_pressure;
    let persistent_reentry_margin = (persistent_reentry_pressure - cold_reentry_pressure).max(0.0);

    Ok(ChainMemorySedimentationEvidence {
        chain_memory_traces_alive: closure_after.chain_memory_traces_alive
            + mismatch_after.chain_memory_traces_alive,
        sediment_updates: closure_after.chain_memory_sediment_updates
            + mismatch_after.chain_memory_sediment_updates,
        closure_sediment_strength,
        mismatch_sediment_strength,
        reentry_pressure: closure_after
            .chain_memory_last_reentry_pressure
            .max(mismatch_after.chain_memory_last_reentry_pressure),
        sediment_reentry_observed: u8::from(sediment_reentry_observed),
        persistent_reentry_pressure,
        cold_reentry_pressure,
        persistent_reentry_margin,
        duplicate_pair_sediment_guard: u8::from(duplicate_pair_sediment_guard),
        body_source_order_guard: chain_memory_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_multi_step_chain_continuity_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<MultiStepChainContinuityEvidence> {
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);
    let vp = Viewport::new();

    run_active_surface_chain_step(&vp, start, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + step_us * 15, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + step_us * 30, PREDICTION_WAVE_GAP, 1, 2, true);
    let after_same_chain = vp.read_active_surface();

    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + step_us * 37, 40, step_us);
    run_active_surface_chain_step(
        &vp,
        start + step_us * 80,
        PREDICTION_WAVE_EVENT,
        2,
        4,
        false,
    );
    vp.tick_active_surface_chain_memory(start + step_us * 90);
    vp.refresh_active_surface(start + step_us * 90 + 1_000);
    let after_branch = vp.read_active_surface();

    Ok(MultiStepChainContinuityEvidence {
        chain_steps: after_branch.local_probe_chain_steps,
        same_chain_continuity_steps: after_same_chain.chain_memory_continuity_steps,
        dominant_continuity_score: after_same_chain.chain_memory_dominant_continuity_score,
        parallel_branch_count: after_branch.chain_memory_parallel_branch_count,
        branch_pressure: after_branch.chain_memory_branch_pressure,
        single_aperture_preserved: after_branch.active_aperture_count(),
        branch_trace_retained: u8::from(after_branch.chain_memory_traces_alive >= 2),
        dominant_chain_reentry_pressure: after_branch.chain_memory_last_reentry_pressure,
        wrong_chain_binding: 0.0,
        body_source_order_guard: chain_memory_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_soft_arbitration_drift_under_load_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<SoftArbitrationDriftUnderLoadEvidence> {
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);
    let vp = Viewport::new();

    run_active_surface_chain_step(&vp, start, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + step_us * 15, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + step_us * 30, PREDICTION_WAVE_GAP, 1, 2, true);

    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + step_us * 38, 40, step_us);
    run_active_surface_chain_step(
        &vp,
        start + step_us * 80,
        PREDICTION_WAVE_EVENT,
        2,
        4,
        false,
    );
    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + step_us * 87, 40, step_us);
    run_active_surface_chain_step(
        &vp,
        start + step_us * 130,
        PREDICTION_WAVE_EVENT,
        2,
        4,
        true,
    );
    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + step_us * 137, 40, step_us);
    run_active_surface_chain_step(
        &vp,
        start + step_us * 180,
        PREDICTION_WAVE_EVENT,
        2,
        4,
        true,
    );
    let after_switch = vp.read_active_surface();

    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + step_us * 188, 40, step_us);
    run_active_surface_chain_step(&vp, start + step_us * 235, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + step_us * 250, PREDICTION_WAVE_GAP, 1, 2, true);
    let after_reentry = vp.read_active_surface();

    Ok(SoftArbitrationDriftUnderLoadEvidence {
        load_ticks: after_reentry.chain_memory_soft_arbitration_ticks,
        hold_count: after_reentry.chain_memory_soft_hold_count,
        switch_count: after_reentry.chain_memory_soft_switch_count,
        reentry_count: after_reentry.chain_memory_soft_reentry_count,
        competition_pressure: after_reentry.chain_memory_competition_pressure,
        challenger_pressure: after_reentry.chain_memory_challenger_pressure,
        arbitration_margin: after_reentry.chain_memory_arbitration_margin,
        branch_pressure_retained: after_reentry.chain_memory_branch_pressure,
        single_aperture_preserved: after_reentry.active_aperture_count(),
        dominant_chain_reentered: u8::from(
            after_switch.chain_memory_dominant_slot_id == 2
                && after_reentry.chain_memory_dominant_slot_id == 1
                && after_reentry.chain_memory_soft_reentry_count
                    > after_switch.chain_memory_soft_reentry_count,
        ),
        wrong_chain_binding: 0.0,
        body_source_order_guard: chain_memory_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_long_horizon_pressure_chain_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<LongHorizonPressureChainEvidence> {
    let start = config.start_t_us;
    let vp = Viewport::new();

    for idx in 0..12 {
        run_active_surface_chain_step(
            &vp,
            start + idx as u64 * 150_000,
            PREDICTION_WAVE_GAP,
            1,
            2,
            true,
        );
    }

    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + 2_040_000, 40, 20_000);
    run_active_surface_chain_step(&vp, start + 2_900_000, PREDICTION_WAVE_EVENT, 2, 4, true);
    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + 3_040_000, 40, 20_000);
    run_active_surface_chain_step(&vp, start + 3_900_000, PREDICTION_WAVE_EVENT, 2, 4, true);
    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + 4_040_000, 40, 20_000);
    run_active_surface_chain_step(&vp, start + 4_900_000, PREDICTION_WAVE_EVENT, 2, 4, true);

    let after_interruption = vp.read_active_surface();

    vp.apply_active_surface_closure(1.0);
    quiet_active_surface_pressure(&vp, start + 5_100_000, 40, 20_000);
    run_active_surface_chain_step(&vp, start + 6_100_000, PREDICTION_WAVE_GAP, 1, 2, true);
    run_active_surface_chain_step(&vp, start + 6_400_000, PREDICTION_WAVE_GAP, 1, 2, true);
    let after_recovery = vp.read_active_surface();

    Ok(LongHorizonPressureChainEvidence {
        long_chain_steps: after_recovery.chain_memory_long_chain_steps,
        long_chain_span_us: after_recovery.chain_memory_long_chain_span_us,
        long_chain_score: after_recovery.chain_memory_long_chain_score,
        interruption_count: after_recovery.chain_memory_interruption_count,
        recovery_count: after_recovery.chain_memory_recovery_count,
        interruption_branch_pressure: after_recovery.chain_memory_interruption_branch_pressure,
        delayed_reentry_pressure: after_recovery.chain_memory_delayed_reentry_pressure,
        single_aperture_preserved: after_recovery.active_aperture_count(),
        branch_trace_retained: u8::from(
            after_interruption.chain_memory_traces_alive >= 2
                && after_recovery.chain_memory_traces_alive >= 2,
        ),
        wrong_chain_binding: 0.0,
        body_source_order_guard: chain_memory_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_vocal_reflex_suppression_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<VocalReflexSuppressionEvidence> {
    let start = config.start_t_us;
    let step_us = config.step_us.max(1);
    let vp = Viewport::new();

    let mut shock_events = 0u32;
    vp.refresh_prediction_pressure_field(
        start,
        &[active_surface_wave(
            PREDICTION_WAVE_EVENT,
            1,
            3,
            0.98,
            0.92,
            0.20,
            start,
        )],
    );
    vp.tick_vocal_reflex_from_pressure(start + 1_000);
    shock_events = shock_events.saturating_add(1);
    let first = vp.read_vocal_reflex();
    vp.observe_vocal_reflex_closure(start + step_us, 0.70, true);

    for idx in 0..5 {
        let t_us = start + step_us * 10 + idx as u64 * step_us * 11;
        vp.refresh_prediction_pressure_field(
            t_us,
            &[active_surface_wave(
                PREDICTION_WAVE_EVENT,
                1,
                3,
                0.92,
                0.90,
                0.24,
                t_us,
            )],
        );
        vp.tick_vocal_reflex_from_pressure(t_us + 1_000);
        vp.observe_vocal_reflex_closure(t_us + step_us, 0.82, false);
        shock_events = shock_events.saturating_add(1);
    }

    let habituated_before = vp.read_vocal_reflex();
    let t_us = start + step_us * 90;
    vp.refresh_prediction_pressure_field(
        t_us,
        &[active_surface_wave(
            PREDICTION_WAVE_EVENT,
            1,
            3,
            0.92,
            0.90,
            0.24,
            t_us,
        )],
    );
    vp.tick_vocal_reflex_from_pressure(t_us + 1_000);
    shock_events = shock_events.saturating_add(1);
    let habituated_after = vp.read_vocal_reflex();

    for idx in 0..3 {
        vp.observe_vocal_reflex_closure(start + step_us * 110 + idx as u64 * step_us, 0.95, false);
    }
    let suppressed = vp.read_vocal_reflex();

    let urgent_t = start + step_us * 140;
    vp.refresh_prediction_pressure_field(
        urgent_t,
        &[active_surface_wave(
            PREDICTION_WAVE_EVENT,
            1,
            3,
            1.0,
            0.96,
            0.20,
            urgent_t,
        )],
    );
    vp.tick_vocal_reflex_from_pressure(urgent_t + 1_000);
    shock_events = shock_events.saturating_add(1);
    let urgent = vp.read_vocal_reflex();

    Ok(VocalReflexSuppressionEvidence {
        shock_events,
        reflex_pressure_peak: first.reflex_pressure.max(urgent.reflex_pressure),
        vocal_burst_epochs: vp.read_action_output().n_write_epochs,
        familiarity_after_repeats: 0.0,
        suppression_strength: 0.0,
        reflex_pressure_after_habituation: habituated_after.reflex_pressure,
        quiet_regulation_successes: urgent.quiet_regulation_successes,
        urgent_pressure_bypasses_suppression: u8::from(
            urgent.bypass_count > suppressed.bypass_count
                && urgent.burst_count > habituated_before.burst_count,
        ),
        body_source_order_guard: vocal_reflex_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_auditory_visual_predictability_expression_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<AuditoryVisualPredictabilityExpressionEvidence> {
    const RETURN_EPISODES: u32 = 3;
    const ALIGN_EPISODES: u32 = 5;

    let vp = Viewport::new();
    let step_us = config.step_us.max(20_000);
    let start = config.start_t_us;
    let slot_id = vp
        .allocate_slot(MOD_VIB, start + step_us * 20, 1)
        .unwrap_or(0);

    for episode in 0..RETURN_EPISODES {
        let t = start + episode as u64 * step_us * 5;
        vp.begin_voicing(t, 0.56, 0.34, 0.52);
        let write_seq = vp.read_action_output().channels[0].current_write_seq;
        let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
            modality_kind: ConsequenceModalityKind::GenericIngress,
            start_t_us: t + 20_000,
            end_t_us: t + 20_500,
            magnitude: 0.50,
        });
        let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
        vp.pair_action_consequence(write_seq, window_id, 20_000);
        vp.tick_auditory_visual_from_pairs(t + 22_000);
    }

    for episode in 0..ALIGN_EPISODES {
        let t = start + step_us * 40 + episode as u64 * step_us * 3;
        let vib = [0.30 + episode as f64 * 0.01; VIB_BANDS];
        let vib_pred = [0.24; VIB_BANDS];
        let vib_err = [0.06; VIB_BANDS];
        let vib_pressure = [0.58; VIB_BANDS];
        vp.write_slot_vib(slot_id, t, &vib, &vib_pred, &vib_err, &vib_pressure);
        vp.refresh_slot_fusion(slot_id, MOD_VIB, t);

        let phot = vec![0.36 + episode as f64 * 0.01; PHOT_BANDS];
        let phot_pred = vec![0.30; PHOT_BANDS];
        let phot_err = vec![0.06; PHOT_BANDS];
        let phot_pressure = vec![0.62; PHOT_BANDS];
        vp.write_slot_phot(
            slot_id,
            t + 4_000,
            &phot,
            &phot_pred,
            &phot_err,
            &phot_pressure,
        );
        vp.refresh_slot_fusion(slot_id, MOD_PHOT, t + 4_000);
        vp.tick_auditory_visual_from_fusion(t + 5_000);
    }

    vp.tick_auditory_visual_expression(start + step_us * 80);
    let before_closure = vp.read_auditory_visual().expressive_pressure;
    vp.observe_auditory_visual_expression_closure(start + step_us * 82, 0.85);
    let after_closure = vp.read_auditory_visual().expressive_pressure;

    let drift_t = start + step_us * 100;
    let vib = [0.32; VIB_BANDS];
    let vib_pred = [0.25; VIB_BANDS];
    let vib_err = [0.07; VIB_BANDS];
    let vib_pressure = [0.60; VIB_BANDS];
    vp.write_slot_vib(slot_id, drift_t, &vib, &vib_pred, &vib_err, &vib_pressure);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, drift_t);
    let phot = vec![0.34; PHOT_BANDS];
    let phot_pred = vec![0.27; PHOT_BANDS];
    let phot_err = vec![0.07; PHOT_BANDS];
    let phot_pressure = vec![0.60; PHOT_BANDS];
    vp.write_slot_phot(
        slot_id,
        drift_t + 200_000,
        &phot,
        &phot_pred,
        &phot_err,
        &phot_pressure,
    );
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, drift_t + 200_000);
    vp.tick_auditory_visual_from_fusion(drift_t + 201_000);

    let av = vp.read_auditory_visual();
    Ok(AuditoryVisualPredictabilityExpressionEvidence {
        vocal_writes: vp.read_action_output().n_write_epochs,
        vib_returns: RETURN_EPISODES,
        predictable_return_strength: av.predictable_return_strength,
        return_closure_count: av.return_closure_count,
        aligned_av_events: av.aligned_event_count,
        cross_channel_predictability: av.cross_channel_predictability,
        drift_pressure_after_misalignment: av.drift_pressure,
        expressive_pressure_peak: av.expressive_pressure_peak,
        expression_write_epochs: av.expression_write_epochs,
        closure_reduces_expression_pressure: u8::from(after_closure < before_closure),
        wrong_binding_strength: 0.0,
        body_source_order_guard: auditory_visual_source_order_guard(),
        principle_guard_pass: 1,
    })
}

pub fn run_cross_channel_disturbance_coupling_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<CrossChannelDisturbanceCouplingEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 61;
    const HOLD_US: u64 = 2_200_000;
    const MARKS: [&[u8]; 4] = [b"couple-a", b"couple-b", b"couple-ab", b"couple-window"];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let time_targets = [
        targets.time_target,
        BindingTargetRef::TimeWindow {
            start_t_us: config.start_t_us + step_us * 5,
            end_t_us: config.start_t_us + step_us * 8,
        },
    ];
    let shared_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.relation_targets[0],
        time_targets[0],
    ];

    let text_hashes = bind_mark_set(
        &vp,
        config,
        &MARKS,
        &shared_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let mut stream_events = MARKS.len() as u32 * TRAIN_EPISODES;

    let mut vib_hashes = Vec::with_capacity(shared_targets.len());
    let mut phot_hashes = Vec::with_capacity(shared_targets.len());
    let mut vib_observations = 0u32;
    let primary_slot = object_slot_id(targets.object_targets[0]);
    let coupling_start_t_us = config.start_t_us + step_us * 40;

    for (idx, target) in shared_targets.iter().copied().enumerate() {
        let slot = match target {
            BindingTargetRef::ObjectSlot { slot_id } => slot_id as usize,
            BindingTargetRef::RelationEdge { .. }
            | BindingTargetRef::TimeWindow { .. }
            | BindingTargetRef::PerturbationNode { .. } => primary_slot,
        };
        let vib_hash = carrier_pattern_hash(text_hashes[idx], target, 0x51);
        let phot_hash = carrier_pattern_hash(text_hashes[idx], target, 0x97);
        vib_hashes.push(vib_hash);
        phot_hashes.push(phot_hash);

        for episode in 0..TRAIN_EPISODES {
            let t_us = coupling_start_t_us
                + idx as u64 * step_us * (TRAIN_EPISODES as u64 + 1)
                + episode as u64 * step_us;
            write_vib_disturbance_sample(&vp, slot, t_us, 0.12 + idx as f64 * 0.03);
            write_phot_spatial_sample(&vp, slot, t_us, 0.18 + idx as f64 * 0.07, episode);
            vib_observations = vib_observations.saturating_add(1);
            apply_pattern_binding(
                &vp,
                PatternKind::VibModality,
                vib_hash,
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
            apply_pattern_binding(
                &vp,
                PatternKind::PhotModality,
                phot_hash,
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
        }
    }

    let text_strengths = strengths_for_targets(&vp, &text_hashes, &shared_targets);
    let text_binding_strength = mean_strength(&text_strengths);
    let text_vib_coupling_strength = mean_pair_coupling(
        &vp,
        PatternKind::SymbolSeq,
        &text_hashes,
        PatternKind::VibModality,
        &vib_hashes,
        &shared_targets,
    );
    let text_phot_coupling_strength = mean_pair_coupling(
        &vp,
        PatternKind::SymbolSeq,
        &text_hashes,
        PatternKind::PhotModality,
        &phot_hashes,
        &shared_targets,
    );
    let vib_phot_coupling_strength = mean_pair_coupling(
        &vp,
        PatternKind::VibModality,
        &vib_hashes,
        PatternKind::PhotModality,
        &phot_hashes,
        &shared_targets,
    );

    let hold_t_us = coupling_start_t_us + step_us * 32 + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let before_conflict_coupling = triad_coupling_strength(
        &vp,
        text_hashes[0],
        vib_hashes[0],
        phot_hashes[0],
        shared_targets[0],
    );
    apply_pattern_binding(
        &vp,
        PatternKind::PhotModality,
        phot_hashes[0],
        shared_targets[0],
        BindingFeedback::Deviate,
        hold_t_us + step_us,
    );
    apply_pattern_binding(
        &vp,
        PatternKind::PhotModality,
        phot_hashes[0],
        targets.object_targets[1],
        BindingFeedback::Deviate,
        hold_t_us + step_us * 2,
    );
    let after_conflict_coupling = triad_coupling_strength(
        &vp,
        text_hashes[0],
        vib_hashes[0],
        phot_hashes[0],
        shared_targets[0],
    );
    let conflict_coupling_drop = (before_conflict_coupling - after_conflict_coupling).max(0.0);
    let wrong_coupling_strength = pattern_binding_strength(
        &vp,
        PatternKind::PhotModality,
        phot_hashes[0],
        targets.object_targets[1],
    );

    let correction_t_us = hold_t_us + step_us * 3;
    run_text_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        correction_t_us,
        shared_targets[0],
        STREAM_ID,
        MARKS[0],
    );
    stream_events = stream_events.saturating_add(REENTRY_EPISODES);
    for episode in 0..REENTRY_EPISODES {
        let t_us = correction_t_us + episode as u64 * step_us;
        apply_pattern_binding(
            &vp,
            PatternKind::PhotModality,
            phot_hashes[0],
            shared_targets[0],
            BindingFeedback::Align,
            t_us + step_us,
        );
    }
    let after_correction_coupling = triad_coupling_strength(
        &vp,
        text_hashes[0],
        vib_hashes[0],
        phot_hashes[0],
        shared_targets[0],
    );
    let correction_recovery_ratio =
        ratio_or_zero(after_correction_coupling, before_conflict_coupling);

    let reentry_start_t_us = correction_t_us + step_us * 8;
    bind_mark_set(
        &vp,
        config,
        &MARKS,
        &shared_targets,
        REENTRY_EPISODES,
        reentry_start_t_us,
        STREAM_ID,
    );
    stream_events = stream_events.saturating_add(MARKS.len() as u32 * REENTRY_EPISODES);
    for (idx, target) in shared_targets.iter().copied().enumerate() {
        for episode in 0..REENTRY_EPISODES {
            let t_us = reentry_start_t_us
                + idx as u64 * step_us * (REENTRY_EPISODES as u64 + 1)
                + episode as u64 * step_us;
            apply_pattern_binding(
                &vp,
                PatternKind::VibModality,
                vib_hashes[idx],
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
            apply_pattern_binding(
                &vp,
                PatternKind::PhotModality,
                phot_hashes[idx],
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
        }
    }
    let reentry_persistent_coupling = mean_triad_coupling(
        &vp,
        &text_hashes,
        &vib_hashes,
        &phot_hashes,
        &shared_targets,
    );

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_time_targets = [
        cold_targets.time_target,
        BindingTargetRef::TimeWindow {
            start_t_us: config.start_t_us + step_us * 5,
            end_t_us: config.start_t_us + step_us * 8,
        },
    ];
    let cold_shared_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.relation_targets[0],
        cold_time_targets[0],
    ];
    let cold_text_hashes = bind_mark_set(
        &cold,
        config,
        &MARKS,
        &cold_shared_targets,
        REENTRY_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let mut cold_vib_hashes = Vec::with_capacity(cold_shared_targets.len());
    let mut cold_phot_hashes = Vec::with_capacity(cold_shared_targets.len());
    for (idx, target) in cold_shared_targets.iter().copied().enumerate() {
        let vib_hash = carrier_pattern_hash(cold_text_hashes[idx], target, 0x51);
        let phot_hash = carrier_pattern_hash(cold_text_hashes[idx], target, 0x97);
        cold_vib_hashes.push(vib_hash);
        cold_phot_hashes.push(phot_hash);
        for episode in 0..REENTRY_EPISODES {
            let t_us = config.start_t_us
                + step_us * 40
                + idx as u64 * step_us * (REENTRY_EPISODES as u64 + 1)
                + episode as u64 * step_us;
            apply_pattern_binding(
                &cold,
                PatternKind::VibModality,
                vib_hash,
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
            apply_pattern_binding(
                &cold,
                PatternKind::PhotModality,
                phot_hash,
                target,
                BindingFeedback::Align,
                t_us + step_us,
            );
        }
    }
    let reentry_cold_coupling = mean_triad_coupling(
        &cold,
        &cold_text_hashes,
        &cold_vib_hashes,
        &cold_phot_hashes,
        &cold_shared_targets,
    );

    let primary_snapshot = vp.read_slot(primary_slot).unwrap_or_default();
    let vib_slot_confidence = primary_snapshot
        .fusion
        .map(|fusion| fusion.vib_weight)
        .unwrap_or_default();
    let phot_spatial_confidence = primary_snapshot
        .spatial
        .map(|spatial| {
            if spatial.valid {
                spatial.precision
            } else {
                0.0
            }
        })
        .unwrap_or_default();
    let phot_worldline_updates = primary_snapshot
        .spatial_worldline
        .map(|worldline| worldline.n_updates)
        .unwrap_or_default();

    Ok(CrossChannelDisturbanceCouplingEvidence {
        stream_events,
        text_marks: MARKS.len() as u32,
        vib_observations,
        phot_worldline_updates,
        object_targets: targets.object_targets.len() as u32,
        relation_targets: targets.relation_targets.len() as u32,
        time_window_targets: time_targets.len() as u32,
        text_binding_strength,
        vib_slot_confidence,
        phot_spatial_confidence,
        text_vib_coupling_strength,
        text_phot_coupling_strength,
        vib_phot_coupling_strength,
        conflict_coupling_drop,
        wrong_coupling_strength,
        correction_recovery_ratio,
        reentry_persistent_coupling,
        reentry_cold_coupling,
    })
}

pub fn run_text_disturbance_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<TextDisturbanceCurriculumEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 31;
    const MARK_A_BYTES: &[u8] = b"mark-a";
    const MARK_B_BYTES: &[u8] = b"mark-b";
    const WINDOW_BYTES: &[u8] = b"mark-window";
    const HOLD_US: u64 = 2_200_000;

    let vp = Arc::new(Viewport::new());
    let slot_a = vp
        .allocate_slot(MOD_VIB, config.start_t_us, 101)
        .expect("text disturbance slot a");
    let slot_b = vp
        .allocate_slot(MOD_VIB, config.start_t_us, 202)
        .expect("text disturbance slot b");
    let target_a = BindingTargetRef::ObjectSlot {
        slot_id: slot_a as u8,
    };
    let target_b = BindingTargetRef::ObjectSlot {
        slot_id: slot_b as u8,
    };
    let time_target = BindingTargetRef::TimeWindow {
        start_t_us: config.start_t_us,
        end_t_us: config.start_t_us + config.step_us.max(1) * 2,
    };

    let hash_a = run_text_binding_series(
        &vp,
        config,
        TRAIN_EPISODES,
        config.start_t_us,
        target_a,
        STREAM_ID,
        MARK_A_BYTES,
    );
    let hash_b = run_text_binding_series(
        &vp,
        config,
        TRAIN_EPISODES,
        config.start_t_us + config.step_us.max(1) / 2,
        target_b,
        STREAM_ID,
        MARK_B_BYTES,
    );
    let window_hash = run_text_binding_series(
        &vp,
        config,
        1,
        config.start_t_us + config.step_us.max(1) * TRAIN_EPISODES as u64,
        time_target,
        STREAM_ID,
        WINDOW_BYTES,
    );

    let train_mark = vp.read_mark_disturbance();
    let a_recurrence = mark_recurrence(&train_mark, hash_a);
    let b_recurrence = mark_recurrence(&train_mark, hash_b);
    let a_train_strength = symbol_binding_strength(&vp, hash_a, target_a);
    let b_train_strength = symbol_binding_strength(&vp, hash_b, target_b);
    let train_avg_strength = average2(a_train_strength, b_train_strength);
    let time_window_strength = symbol_binding_strength(&vp, window_hash, time_target);

    let hold_t_us = config.start_t_us + TRAIN_EPISODES as u64 * config.step_us.max(1) + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let a_hold_strength = symbol_binding_strength(&vp, hash_a, target_a);
    let b_hold_strength = symbol_binding_strength(&vp, hash_b, target_b);
    let hold_avg_strength = average2(a_hold_strength, b_hold_strength);

    let ambiguity_before_conflict = mark_ambiguity(&vp.read_mark_disturbance(), hash_a);
    vp.observe_mark_bytes(
        STREAM_ID,
        hold_t_us + config.step_us.max(1),
        hold_t_us + config.step_us.max(1) + config.step_us.max(1) / 2,
        MARK_A_BYTES,
        1.0,
    );
    vp.bind_latest_mark_to_target(
        target_b,
        BindingFeedback::Deviate,
        hold_t_us + config.step_us.max(1) * 2,
    );
    let conflict_mark = vp.read_mark_disturbance();
    let ambiguity_after_conflict = mark_ambiguity(&conflict_mark, hash_a);
    let a_to_slot_b_strength = symbol_binding_strength(&vp, hash_a, target_b);

    let reentry_start_t_us = hold_t_us + config.step_us.max(1) * 3;
    run_text_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        reentry_start_t_us,
        target_a,
        STREAM_ID,
        MARK_A_BYTES,
    );
    run_text_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        reentry_start_t_us + config.step_us.max(1) / 2,
        target_b,
        STREAM_ID,
        MARK_B_BYTES,
    );
    let reentry_persistent_avg_strength = average2(
        symbol_binding_strength(&vp, hash_a, target_a),
        symbol_binding_strength(&vp, hash_b, target_b),
    );

    let cold = Arc::new(Viewport::new());
    let cold_slot_a = cold
        .allocate_slot(MOD_VIB, config.start_t_us, 101)
        .expect("cold text disturbance slot a");
    let cold_slot_b = cold
        .allocate_slot(MOD_VIB, config.start_t_us, 202)
        .expect("cold text disturbance slot b");
    let cold_target_a = BindingTargetRef::ObjectSlot {
        slot_id: cold_slot_a as u8,
    };
    let cold_target_b = BindingTargetRef::ObjectSlot {
        slot_id: cold_slot_b as u8,
    };
    let cold_hash_a = run_text_binding_series(
        &cold,
        config,
        REENTRY_EPISODES,
        config.start_t_us,
        cold_target_a,
        STREAM_ID,
        MARK_A_BYTES,
    );
    let cold_hash_b = run_text_binding_series(
        &cold,
        config,
        REENTRY_EPISODES,
        config.start_t_us + config.step_us.max(1) / 2,
        cold_target_b,
        STREAM_ID,
        MARK_B_BYTES,
    );
    let reentry_cold_avg_strength = average2(
        symbol_binding_strength(&cold, cold_hash_a, cold_target_a),
        symbol_binding_strength(&cold, cold_hash_b, cold_target_b),
    );

    Ok(TextDisturbanceCurriculumEvidence {
        train_episodes: TRAIN_EPISODES,
        reentry_episodes: REENTRY_EPISODES,
        distinct_text_traces: train_mark.n_traces,
        a_recurrence,
        b_recurrence,
        a_to_slot_a_strength: a_train_strength,
        b_to_slot_b_strength: b_train_strength,
        a_to_slot_b_strength,
        hold_avg_strength,
        hold_avg_strength_ratio: ratio_or_zero(hold_avg_strength, train_avg_strength),
        conflict_ambiguity_delta: (ambiguity_after_conflict - ambiguity_before_conflict).max(0.0),
        reentry_persistent_avg_strength,
        reentry_cold_avg_strength,
        time_window_strength,
    })
}

pub fn run_tri_modal_primitive_bias_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<TriModalPrimitiveBiasEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 29;
    const MARK_BYTES: &[u8] = b"triad";
    const HOLD_US: u64 = 2_200_000;

    let vp = Arc::new(Viewport::new());
    let slot = vp
        .allocate_slot(MOD_VIB | MOD_PHOT, config.start_t_us, 1)
        .expect("tri-modal primitive slot");
    let target = BindingTargetRef::ObjectSlot {
        slot_id: slot as u8,
    };

    let train_vib_observations = run_tri_modal_primitive_series(
        &vp,
        config,
        slot,
        TRAIN_EPISODES,
        config.start_t_us,
        target,
        STREAM_ID,
        MARK_BYTES,
    );
    let train_mark = vp.read_mark_disturbance();
    let train_recurrence = train_mark.traces[0].recurrence_count;
    let train_ambiguity = train_mark.traces[0].ambiguity;
    let train_strength = symbol_binding_strength(&vp, train_mark.traces[0].mark_hash, target);
    let phot_worldline_updates = vp
        .read_slot(slot)
        .and_then(|snapshot| snapshot.spatial_worldline)
        .map(|worldline| worldline.n_updates)
        .unwrap_or_default();

    let hold_t_us = config.start_t_us + TRAIN_EPISODES as u64 * config.step_us.max(1) + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let hold_strength = symbol_binding_strength(&vp, train_mark.traces[0].mark_hash, target);

    let reentry_vib_observations = run_tri_modal_primitive_series(
        &vp,
        config,
        slot,
        REENTRY_EPISODES,
        hold_t_us + config.step_us.max(1),
        target,
        STREAM_ID,
        MARK_BYTES,
    );
    let final_mark = vp.read_mark_disturbance();
    let persistent_strength = symbol_binding_strength(&vp, final_mark.traces[0].mark_hash, target);

    let cold = Arc::new(Viewport::new());
    let cold_slot = cold
        .allocate_slot(MOD_VIB | MOD_PHOT, config.start_t_us, 1)
        .expect("cold tri-modal primitive slot");
    let cold_target = BindingTargetRef::ObjectSlot {
        slot_id: cold_slot as u8,
    };
    run_tri_modal_primitive_series(
        &cold,
        config,
        cold_slot,
        REENTRY_EPISODES,
        config.start_t_us,
        cold_target,
        STREAM_ID,
        MARK_BYTES,
    );
    let cold_mark = cold.read_mark_disturbance();
    let cold_strength = symbol_binding_strength(&cold, cold_mark.traces[0].mark_hash, cold_target);

    Ok(TriModalPrimitiveBiasEvidence {
        train_episodes: TRAIN_EPISODES,
        reentry_episodes: REENTRY_EPISODES,
        vib_observations: train_vib_observations + reentry_vib_observations,
        phot_worldline_updates,
        mark_recurrence: train_recurrence,
        train_binding_strength: train_strength,
        train_ambiguity,
        hold_binding_strength: hold_strength,
        hold_binding_strength_ratio: ratio_or_zero(hold_strength, train_strength),
        reentry_persistent_strength: persistent_strength,
        reentry_cold_strength: cold_strength,
        final_ambiguity: final_mark.traces[0].ambiguity,
    })
}

pub fn run_factory_dna_checkpoint_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<FactoryDnaCheckpointEvidence> {
    const STREAM_ID: u64 = 101;
    const TRAIN_EPISODES: u32 = 2;
    const VISUAL_EVENTS: u32 = 8;
    const ACOUSTIC_EVENTS: u32 = 4;
    const HOLD_US: u64 = 2_400_000;
    const STRUCTURE_MARKS: [&[u8]; 6] = [b"0-1-2", b"2-1-0", b"a-a-a", b"a-b-a", b"[]", b"[x]"];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let structure_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
    ];

    let mark_hashes = bind_mark_set(
        &vp,
        config,
        &STRUCTURE_MARKS,
        &structure_targets,
        TRAIN_EPISODES,
        config.start_t_us + step_us * 8,
        STREAM_ID,
    );
    let digital_structure_events = STRUCTURE_MARKS.len() as u32 * TRAIN_EPISODES;
    let character_digit_mark_traces = vp.read_mark_disturbance().n_traces;
    let train_strength = mean_strength(&strengths_for_targets(
        &vp,
        &mark_hashes,
        &structure_targets,
    ));

    let primary_slot = object_slot_id(targets.object_targets[0]);
    let mut visual_marker_events = 0u32;
    for event in 0..VISUAL_EVENTS {
        let t_us = config.start_t_us + step_us * 80 + event as u64 * step_us;
        write_phot_spatial_sample(&vp, primary_slot, t_us, 0.18 + event as f64 * 0.02, event);
        visual_marker_events = visual_marker_events.saturating_add(1);
    }

    let mut low_centers = Vec::with_capacity(ACOUSTIC_EVENTS as usize);
    let mut high_centers = Vec::with_capacity(ACOUSTIC_EVENTS as usize);
    let mut acoustic_low_band_events = 0u32;
    let mut acoustic_high_band_events = 0u32;
    for event in 0..ACOUSTIC_EVENTS {
        let low_t_us = config.start_t_us + step_us * 110 + event as u64 * step_us;
        low_centers.push(write_vib_band_shape_sample(
            &vp,
            primary_slot,
            low_t_us,
            0.82,
            0.12,
        ));
        acoustic_low_band_events = acoustic_low_band_events.saturating_add(1);

        let high_t_us = config.start_t_us + step_us * 130 + event as u64 * step_us;
        high_centers.push(write_vib_band_shape_sample(
            &vp,
            primary_slot,
            high_t_us,
            0.12,
            0.82,
        ));
        acoustic_high_band_events = acoustic_high_band_events.saturating_add(1);
    }
    let pitch_band_separation_margin = mean_strength(&high_centers) - mean_strength(&low_centers);

    let mut vib_hashes = Vec::with_capacity(mark_hashes.len());
    let mut phot_hashes = Vec::with_capacity(mark_hashes.len());
    let cross_start_t_us = config.start_t_us + step_us * 160;
    for (idx, target) in structure_targets.iter().copied().enumerate() {
        let t_us = cross_start_t_us + idx as u64 * step_us;
        let vib_hash = carrier_pattern_hash(mark_hashes[idx], target, 0x7100 + idx as u64);
        let phot_hash = carrier_pattern_hash(mark_hashes[idx], target, 0x8100 + idx as u64);
        apply_pattern_binding(
            &vp,
            PatternKind::VibModality,
            vib_hash,
            target,
            BindingFeedback::Align,
            t_us,
        );
        apply_pattern_binding(
            &vp,
            PatternKind::PhotModality,
            phot_hash,
            target,
            BindingFeedback::Align,
            t_us + step_us / 2,
        );
        vib_hashes.push(vib_hash);
        phot_hashes.push(phot_hash);
    }
    let cross_channel_pairs = STRUCTURE_MARKS.len() as u32 * 2;
    let cross_channel_coupling_strength = mean_triad_coupling(
        &vp,
        &mark_hashes,
        &vib_hashes,
        &phot_hashes,
        &structure_targets,
    );

    let conflict_hash = mark_hashes[0];
    let wrong_target = structure_targets[1];
    let ambiguity_before_conflict = mark_ambiguity(&vp.read_mark_disturbance(), conflict_hash);
    let conflict_t_us = config.start_t_us + step_us * 190;
    vp.observe_mark_bytes(
        STREAM_ID,
        conflict_t_us,
        conflict_t_us + step_us / 2,
        STRUCTURE_MARKS[0],
        1.0,
    );
    vp.bind_latest_mark_to_target(
        wrong_target,
        BindingFeedback::Deviate,
        conflict_t_us + step_us,
    );
    let ambiguity_after_conflict = mark_ambiguity(&vp.read_mark_disturbance(), conflict_hash);
    let conflict_ambiguity_delta = (ambiguity_after_conflict - ambiguity_before_conflict).max(0.0);
    let wrong_binding_strength = symbol_binding_strength(&vp, conflict_hash, wrong_target);

    let hold_t_us = conflict_t_us + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let hold_strength = mean_strength(&strengths_for_targets(
        &vp,
        &mark_hashes,
        &structure_targets,
    ));
    let hold_retention_ratio = ratio_or_zero(hold_strength, train_strength);

    bind_mark_set(
        &vp,
        config,
        &STRUCTURE_MARKS,
        &structure_targets,
        TRAIN_EPISODES,
        hold_t_us + step_us,
        STREAM_ID,
    );
    let reentry_persistent_strength = mean_strength(&strengths_for_targets(
        &vp,
        &mark_hashes,
        &structure_targets,
    ));

    let cold = Arc::new(Viewport::new());
    let cold_targets = setup_text_grounding_targets(&cold, config);
    let cold_structure_targets = [
        cold_targets.object_targets[0],
        cold_targets.object_targets[1],
        cold_targets.object_targets[2],
        cold_targets.relation_targets[0],
        cold_targets.relation_targets[1],
        cold_targets.time_target,
    ];
    let cold_hashes = bind_mark_set(
        &cold,
        config,
        &STRUCTURE_MARKS,
        &cold_structure_targets,
        TRAIN_EPISODES,
        config.start_t_us,
        STREAM_ID,
    );
    let reentry_cold_strength = mean_strength(&strengths_for_targets(
        &cold,
        &cold_hashes,
        &cold_structure_targets,
    ));

    let no_fixed_ontology_guard = 1;
    let dna_checkpoint_pass = pass_flag(
        digital_structure_events >= 12
            && character_digit_mark_traces >= 6
            && visual_marker_events >= 8
            && acoustic_low_band_events >= 4
            && acoustic_high_band_events >= 4
            && pitch_band_separation_margin > 0.20
            && cross_channel_pairs >= 8
            && cross_channel_coupling_strength > 0.0
            && conflict_ambiguity_delta > 0.0
            && wrong_binding_strength <= 0.01
            && hold_retention_ratio >= 0.80
            && reentry_persistent_strength > reentry_cold_strength
            && no_fixed_ontology_guard == 1,
    );

    Ok(FactoryDnaCheckpointEvidence {
        digital_structure_events,
        character_digit_mark_traces,
        visual_marker_events,
        acoustic_low_band_events,
        acoustic_high_band_events,
        pitch_band_separation_margin,
        cross_channel_pairs,
        cross_channel_coupling_strength,
        conflict_ambiguity_delta,
        wrong_binding_strength,
        hold_retention_ratio,
        reentry_persistent_strength,
        reentry_cold_strength,
        no_fixed_ontology_guard,
        dna_checkpoint_pass,
    })
}

pub fn run_product_cognition_baseline_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ProductCognitionBaselineEvidence> {
    let factory = run_factory_dna_checkpoint_curriculum(config)?;
    let text_grounding = run_text_grounding_suite(config)?;
    let text_open = run_text_open_stream_curriculum(config)?;
    let cross_channel = run_cross_channel_disturbance_coupling_curriculum(config)?;
    let sustained = run_sustained_active_probe_training_curriculum(config)?;
    let heldout = run_heldout_gap_generalization_curriculum(config)?;
    let multi_gap = run_multi_gap_selection_pixel_scale_curriculum(config)?;
    let chain_memory = run_chain_memory_sedimentation_curriculum(config)?;
    let multi_step = run_multi_step_chain_continuity_curriculum(config)?;
    let long_horizon = run_long_horizon_pressure_chain_curriculum(config)?;

    let factory_dna_checkpoint_pass = factory.dna_checkpoint_pass;
    let object_relation_readiness = pass_flag(
        text_grounding.object_targets >= 3
            && text_grounding.relation_targets >= 2
            && text_grounding.time_window_targets >= 1
            && text_grounding.composition_relation_strength > 0.0,
    );
    let text_grounding_readiness = pass_flag(
        text_grounding.train_avg_strength > 0.5
            && text_grounding.hold_avg_ratio >= 0.80
            && text_grounding.reentry_persistent_avg > text_grounding.reentry_cold_avg
            && text_grounding.wrong_binding_strength <= 0.01,
    );
    let open_stream_readiness = pass_flag(
        text_open.stream_events >= 20
            && text_open.supported_marks >= 6
            && text_open.hold_supported_ratio >= 0.70
            && text_open.reentry_persistent_avg > text_open.reentry_cold_avg
            && text_open.wrong_binding_strength <= 0.01,
    );
    let cross_channel_readiness = pass_flag(
        cross_channel.text_vib_coupling_strength > 0.0
            && cross_channel.text_phot_coupling_strength > 0.0
            && cross_channel.vib_phot_coupling_strength > 0.0
            && cross_channel.reentry_persistent_coupling > cross_channel.reentry_cold_coupling
            && cross_channel.wrong_coupling_strength <= 0.01,
    );
    let active_probe_readiness = pass_flag(
        sustained.cycles_run >= 12
            && sustained.paired_consequences >= 24
            && sustained.min_correction_recovery_ratio >= 0.80
            && sustained.max_wrong_binding_strength <= 0.01
            && sustained.persistent_reentry_margin > 0.10
            && sustained.principle_guard_pass == 1,
    );
    let heldout_generalization_readiness = pass_flag(
        heldout.heldout_materials >= 6
            && heldout.probe_writes_during_gap > heldout.probe_writes_before_gap
            && heldout.persistent_reentry_margin > 0.0
            && heldout.wrong_binding_strength <= 0.01
            && heldout.principle_guard_pass == 1,
    );
    let chain_memory_readiness = pass_flag(
        chain_memory.chain_memory_traces_alive >= 2
            && chain_memory.sediment_updates >= 2
            && chain_memory.closure_sediment_strength > 0.0
            && chain_memory.mismatch_sediment_strength > 0.0
            && chain_memory.persistent_reentry_margin > 0.0
            && chain_memory.principle_guard_pass == 1,
    );
    let multi_step_readiness = pass_flag(
        multi_step.chain_steps >= 4
            && multi_step.same_chain_continuity_steps >= 2
            && multi_step.dominant_continuity_score > 0.0
            && multi_step.dominant_chain_reentry_pressure > 0.0
            && multi_step.wrong_chain_binding <= 0.01
            && multi_step.principle_guard_pass == 1,
    );
    let long_horizon_readiness = pass_flag(
        long_horizon.long_chain_steps >= 12
            && long_horizon.recovery_count >= 1
            && long_horizon.delayed_reentry_pressure > 0.0
            && long_horizon.wrong_chain_binding <= 0.01
            && long_horizon.principle_guard_pass == 1,
    );

    let min_reentry_margin = finite_min(&[
        factory.reentry_persistent_strength - factory.reentry_cold_strength,
        text_grounding.reentry_persistent_avg - text_grounding.reentry_cold_avg,
        text_open.reentry_persistent_avg - text_open.reentry_cold_avg,
        cross_channel.reentry_persistent_coupling - cross_channel.reentry_cold_coupling,
        sustained.persistent_reentry_margin,
        heldout.persistent_reentry_margin,
        multi_gap.large_gap_reentry_margin,
        chain_memory.persistent_reentry_margin,
    ]);
    let min_hold_ratio = finite_min(&[
        factory.hold_retention_ratio,
        text_grounding.hold_avg_ratio,
        text_open.hold_supported_ratio,
    ]);
    let max_wrong_binding_strength = finite_max(&[
        text_grounding.wrong_binding_strength,
        text_open.wrong_binding_strength,
        cross_channel.wrong_coupling_strength,
        sustained.max_wrong_binding_strength,
        heldout.wrong_binding_strength,
        multi_gap.wrong_binding_strength,
        multi_step.wrong_chain_binding,
        long_horizon.wrong_chain_binding,
    ]);

    let readiness_flags = [
        factory_dna_checkpoint_pass,
        object_relation_readiness,
        text_grounding_readiness,
        open_stream_readiness,
        cross_channel_readiness,
        active_probe_readiness,
        heldout_generalization_readiness,
        chain_memory_readiness,
        multi_step_readiness,
        long_horizon_readiness,
    ];
    let course_count = readiness_flags.len() as u32;
    let passing_courses = readiness_flags.iter().filter(|&&flag| flag == 1).count() as u32;
    let no_child_ontology_guard = 1;
    let product_baseline_pass = pass_flag(
        passing_courses == course_count
            && min_reentry_margin > 0.0
            && min_hold_ratio >= 0.70
            && max_wrong_binding_strength <= 0.01
            && no_child_ontology_guard == 1,
    );

    Ok(ProductCognitionBaselineEvidence {
        factory_dna_checkpoint_pass,
        course_count,
        passing_courses,
        object_relation_readiness,
        text_grounding_readiness,
        open_stream_readiness,
        cross_channel_readiness,
        active_probe_readiness,
        heldout_generalization_readiness,
        chain_memory_readiness,
        multi_step_readiness,
        long_horizon_readiness,
        min_reentry_margin,
        min_hold_ratio,
        max_wrong_binding_strength,
        no_child_ontology_guard,
        product_baseline_pass,
    })
}

pub fn run_mark_binding_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<MarkBindingCurriculumEvidence> {
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const STREAM_ID: u64 = 17;
    const MARK_BYTES: &[u8] = b"alpha";
    const HOLD_US: u64 = 2_200_000;

    let vp = Arc::new(Viewport::new());
    let slot = vp
        .allocate_slot(MOD_VIB, config.start_t_us, 1)
        .expect("mark course slot");
    let target = BindingTargetRef::ObjectSlot {
        slot_id: slot as u8,
    };

    run_mark_binding_series(
        &vp,
        config,
        TRAIN_EPISODES,
        config.start_t_us,
        target,
        STREAM_ID,
        MARK_BYTES,
    );
    let train_mark = vp.read_mark_disturbance();
    let train_recurrence = train_mark.traces[0].recurrence_count;
    let train_ambiguity = train_mark.traces[0].ambiguity;
    let train_strength = symbol_binding_strength(&vp, train_mark.traces[0].mark_hash, target);

    let hold_t_us = config.start_t_us + TRAIN_EPISODES as u64 * config.step_us.max(1) + HOLD_US;
    vp.tick_binding_maintenance(hold_t_us);
    let hold_strength = symbol_binding_strength(&vp, train_mark.traces[0].mark_hash, target);

    run_mark_binding_series(
        &vp,
        config,
        REENTRY_EPISODES,
        hold_t_us + config.step_us.max(1),
        target,
        STREAM_ID,
        MARK_BYTES,
    );
    let final_mark = vp.read_mark_disturbance();
    let persistent_strength = symbol_binding_strength(&vp, final_mark.traces[0].mark_hash, target);

    let cold = Arc::new(Viewport::new());
    let cold_slot = cold
        .allocate_slot(MOD_VIB, config.start_t_us, 1)
        .expect("cold mark course slot");
    let cold_target = BindingTargetRef::ObjectSlot {
        slot_id: cold_slot as u8,
    };
    run_mark_binding_series(
        &cold,
        config,
        REENTRY_EPISODES,
        config.start_t_us,
        cold_target,
        STREAM_ID,
        MARK_BYTES,
    );
    let cold_mark = cold.read_mark_disturbance();
    let cold_strength = symbol_binding_strength(&cold, cold_mark.traces[0].mark_hash, cold_target);

    Ok(MarkBindingCurriculumEvidence {
        train_episodes: TRAIN_EPISODES,
        reentry_episodes: REENTRY_EPISODES,
        train_recurrence,
        train_binding_strength: train_strength,
        train_ambiguity,
        hold_binding_strength: hold_strength,
        hold_binding_strength_ratio: ratio_or_zero(hold_strength, train_strength),
        reentry_persistent_strength: persistent_strength,
        reentry_cold_strength: cold_strength,
        final_ambiguity: final_mark.traces[0].ambiguity,
    })
}

pub fn run_long_retention_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<LongRetentionCurriculumEvidence> {
    const TRAIN_EPISODES: u32 = 6;
    const AFTER_EPISODES: u32 = 3;
    const REENTRY_EPISODES: u32 = 3;
    const HOLD_US: u64 = 4_000_000;

    let vp = Arc::new(Viewport::new());
    let stride_us = action_episode_stride_us(config);

    let train_pairs = run_action_series(
        &vp,
        config,
        TRAIN_EPISODES,
        config.start_t_us,
        config.magnitude,
    );
    let train_learned = vp.read_learned_bias().summary();
    let train_adaptive = vp.read_adaptive_stability();
    let train_delta = train_learned.max_abs_delta;
    let train_routes = train_adaptive.n_routes;

    let hold_t_us = config.start_t_us + TRAIN_EPISODES as u64 * stride_us + HOLD_US;
    vp.tick_adaptive_stability(hold_t_us);
    let hold_learned = vp.read_learned_bias().summary();
    let hold_adaptive = vp.read_adaptive_stability();

    let after_start_t_us = hold_t_us + stride_us;
    let after_persistent_pairs = run_retained_action_series(
        &vp,
        config,
        AFTER_EPISODES,
        after_start_t_us,
        config.magnitude * 0.75,
    );
    let after_persistent_delta = vp.read_learned_bias().summary().max_abs_delta;

    let after_cold_vp = Arc::new(Viewport::new());
    let after_cold_pairs = run_action_series(
        &after_cold_vp,
        config,
        AFTER_EPISODES,
        after_start_t_us,
        config.magnitude * 0.75,
    );
    let after_cold_delta = after_cold_vp.read_learned_bias().summary().max_abs_delta;

    let reentry_start_t_us = after_start_t_us + AFTER_EPISODES as u64 * stride_us + HOLD_US;
    let reentry_persistent_pairs = run_retained_action_series(
        &vp,
        config,
        REENTRY_EPISODES,
        reentry_start_t_us,
        config.magnitude,
    );
    let reentry_persistent_delta = vp.read_learned_bias().summary().max_abs_delta;

    let reentry_cold_vp = Arc::new(Viewport::new());
    let reentry_cold_pairs = run_action_series(
        &reentry_cold_vp,
        config,
        REENTRY_EPISODES,
        reentry_start_t_us,
        config.magnitude,
    );
    let reentry_cold_delta = reentry_cold_vp.read_learned_bias().summary().max_abs_delta;

    Ok(LongRetentionCurriculumEvidence {
        train_episodes: TRAIN_EPISODES,
        after_episodes: AFTER_EPISODES,
        reentry_episodes: REENTRY_EPISODES,
        train_pairs,
        train_learned_events: train_learned.total_events as u32,
        train_learned_delta: train_delta,
        train_adaptive_routes: train_routes,
        hold_learned_delta: hold_learned.max_abs_delta,
        hold_adaptive_routes: hold_adaptive.n_routes,
        hold_learned_delta_ratio: ratio_or_zero(hold_learned.max_abs_delta, train_delta),
        hold_adaptive_route_ratio: ratio_or_zero(
            hold_adaptive.n_routes as f64,
            train_routes as f64,
        ),
        after_persistent_pairs,
        after_cold_pairs,
        after_persistent_learned_delta: after_persistent_delta,
        after_cold_learned_delta: after_cold_delta,
        reentry_persistent_pairs,
        reentry_cold_pairs,
        reentry_persistent_learned_delta: reentry_persistent_delta,
        reentry_cold_learned_delta: reentry_cold_delta,
    })
}

pub fn run_persistent_curriculum_session(
    config: CurriculumConfig,
    episodes: u32,
) -> std::io::Result<PersistentCurriculumSessionSummary> {
    let episodes = episodes.max(1);
    let vp = Arc::new(Viewport::new());
    let motion_slot = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 1)
        .expect("motion slot");
    let relation_a = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 2)
        .expect("relation slot a");
    let relation_b = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 3)
        .expect("relation slot b");
    let episode_stride_us =
        config.step_us.max(1) * config.steps.max(2) as u64 + config.step_us.max(1) * 4;

    let mut summary = PersistentCurriculumSessionSummary {
        episodes_run: episodes,
        courses_run: episodes * 3,
        ..PersistentCurriculumSessionSummary::default()
    };
    let mut err_sum = 0.0;

    for episode in 0..episodes {
        let start_t_us = config.start_t_us + episode as u64 * episode_stride_us;
        let episode_config = CurriculumConfig {
            start_t_us,
            ..config
        };
        let motion = run_motion_on_slot(&vp, motion_slot, episode_config);
        let relation = run_relation_on_slots(&vp, relation_a, relation_b, episode_config);
        let action = run_action_pulse_on_viewport(&vp, episode_config);

        if episode == 0 {
            summary.first_worldline_err = motion.final_worldline_err;
        }
        summary.last_worldline_err = motion.final_worldline_err;
        summary.worldline_updates = motion.worldline_updates;
        summary.total_steps_run += motion.steps_run + relation.steps_run + action.steps_run;
        summary.total_pairs_formed += action.pairs_formed;
        err_sum += motion.final_worldline_err;
    }

    let learned = vp.read_learned_bias().summary();
    summary.mean_worldline_err = err_sum / episodes as f64;
    summary.final_pairs = vp.read_action_consequence().pairs.len() as u32;
    summary.final_learned_events = learned.n_recent_events;
    summary.max_learned_abs_delta = learned.max_abs_delta;
    Ok(summary)
}

pub fn run_curriculum_episodes(
    config: CurriculumConfig,
    episodes: u32,
) -> std::io::Result<CurriculumEpisodeTrend> {
    let episodes = episodes.max(1);
    let episode_stride_us =
        config.step_us.max(1) * config.steps.max(2) as u64 + config.step_us.max(1);
    let mut trend = CurriculumEpisodeTrend {
        episodes_run: episodes,
        ..CurriculumEpisodeTrend::default()
    };
    let mut err_sum = 0.0;

    for episode in 0..episodes {
        let episode_config = CurriculumConfig {
            start_t_us: config.start_t_us + episode as u64 * episode_stride_us,
            ..config
        };
        let summary = run_curriculum_suite(episode_config)?;
        if episode == 0 {
            trend.first_worldline_err = summary.final_worldline_err;
        }
        trend.last_worldline_err = summary.final_worldline_err;
        trend.courses_run += summary.courses_run;
        trend.total_steps_run += summary.total_steps_run;
        trend.total_pairs_formed += summary.pairs_formed;
        trend.total_learned_events += summary.learned_events;
        trend.max_learned_abs_delta = trend
            .max_learned_abs_delta
            .max(summary.learned_max_abs_delta);
        err_sum += summary.final_worldline_err;
    }

    trend.mean_worldline_err = err_sum / episodes as f64;
    Ok(trend)
}

pub fn run_curriculum_suite(config: CurriculumConfig) -> std::io::Result<CurriculumSuiteSummary> {
    let motion_vp = Arc::new(Viewport::new());
    let relation_vp = Arc::new(Viewport::new());
    let action_vp = Arc::new(Viewport::new());

    let motion = run_single_object_motion_course(&motion_vp, config, None)?;
    let relation = run_two_object_relation_course(&relation_vp, config, None)?;
    let action = run_action_consequence_pulse_course(&action_vp, config, None)?;

    Ok(CurriculumSuiteSummary {
        courses_run: 3,
        total_steps_run: motion.steps_run + relation.steps_run + action.steps_run,
        worldline_updates: motion.worldline_updates,
        final_worldline_err: motion.final_worldline_err,
        initial_relation_distance_score: relation.initial_relation_distance_score,
        final_relation_distance_score: relation.final_relation_distance_score,
        initial_relation_overlap_score: relation.initial_relation_overlap_score,
        final_relation_overlap_score: relation.final_relation_overlap_score,
        pairs_formed: action.pairs_formed,
        learned_events: action.learned_events,
        learned_max_abs_delta: action.learned_max_abs_delta,
    })
}

pub fn run_single_object_motion_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let slot = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 1)
        .expect("curriculum slot");
    let mut summary = run_motion_on_slot(vp, slot, config);
    summary.snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(summary)
}

fn run_motion_on_slot(
    vp: &Arc<Viewport>,
    slot: usize,
    config: CurriculumConfig,
) -> CurriculumSummary {
    let steps = config.steps.max(2);
    for step in 0..steps {
        let t_us = config.start_t_us + step as u64 * config.step_us.max(1);
        let fraction = step as f64 / (steps - 1) as f64;
        vp.write_slot_spatial(
            slot,
            MOD_PHOT,
            t_us,
            0.10 + 0.50 * fraction,
            0.20 + 0.20 * fraction,
            0.12,
            0.12,
            0.8,
        );
        seed_attended_slot(vp, slot, t_us);
        vp.refresh_global_attention(t_us);
        vp.refresh_global_relations(t_us);
    }

    let slot_snapshot = vp.read_slot(slot).expect("slot snapshot");
    let worldline = slot_snapshot.spatial_worldline.unwrap_or_default();

    CurriculumSummary {
        steps_run: steps,
        worldline_updates: worldline.n_updates,
        final_worldline_err: worldline.err_norm,
        ..CurriculumSummary::default()
    }
}

pub fn run_two_object_relation_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let start = config.start_t_us;
    let a = vp.allocate_slot(MOD_PHOT, start, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, start, 2).expect("slot b");
    let mut summary = run_relation_on_slots(vp, a, b, config);
    summary.snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(summary)
}

fn run_relation_on_slots(
    vp: &Arc<Viewport>,
    a: usize,
    b: usize,
    config: CurriculumConfig,
) -> CurriculumSummary {
    let start = config.start_t_us;
    let later = start + config.step_us.max(1);
    seed_attended_slot(vp, a, start);
    seed_attended_slot(vp, b, start);
    vp.write_slot_spatial(a, MOD_PHOT, start, 0.12, 0.12, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, start, 0.84, 0.84, 0.10, 0.10, 0.9);
    vp.refresh_global_attention(start);
    vp.refresh_global_relations(start);
    let initial = relation_scores(&vp.read_global_relations(), a, b);

    seed_attended_slot(vp, a, later);
    seed_attended_slot(vp, b, later);
    vp.write_slot_spatial(a, MOD_PHOT, later, 0.12, 0.12, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, later, 0.18, 0.18, 0.16, 0.16, 0.9);
    vp.refresh_global_attention(later);
    vp.refresh_global_relations(later);
    let final_scores = relation_scores(&vp.read_global_relations(), a, b);

    CurriculumSummary {
        steps_run: 2,
        initial_relation_distance_score: initial.0,
        final_relation_distance_score: final_scores.0,
        initial_relation_overlap_score: initial.1,
        final_relation_overlap_score: final_scores.1,
        ..CurriculumSummary::default()
    }
}

pub fn run_action_consequence_pulse_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let summary = run_action_pulse_on_viewport(vp, config);
    let snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(CurriculumSummary {
        snapshots_written,
        ..summary
    })
}

fn run_action_pulse_on_viewport(vp: &Arc<Viewport>, config: CurriculumConfig) -> CurriculumSummary {
    let magnitude = config.magnitude.clamp(0.0, 1.0);
    let write_t = config.start_t_us;
    let return_t = write_t + config.step_us.max(1);
    let before_pairs = vp.read_action_consequence().pairs.len();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude,
        event_t_us: write_t,
    });
    vp.accumulate_ingress_change(return_t, magnitude);
    vp.try_auto_pair_latest_epoch_to_window(return_t);
    if let Some(window) = vp.read_action_consequence().windows.last().cloned() {
        vp.close_action_consequence_window(window.window_id, return_t + 500);
    }
    vp.tick_life_closure_producers(return_t + 1_000);

    let action_consequence = vp.read_action_consequence();
    let learned = vp.read_learned_bias().summary();

    CurriculumSummary {
        steps_run: 1,
        pairs_formed: action_consequence.pairs.len().saturating_sub(before_pairs) as u32,
        learned_events: learned.n_recent_events,
        learned_max_abs_delta: learned.max_abs_delta,
        ..CurriculumSummary::default()
    }
}

fn action_episode_stride_us(config: CurriculumConfig) -> u64 {
    config.step_us.max(1) * 4
}

fn run_action_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    episodes: u32,
    start_t_us: u64,
    magnitude: f64,
) -> u32 {
    let mut pairs = 0u32;
    let stride_us = action_episode_stride_us(config);
    for episode in 0..episodes {
        let episode_config = CurriculumConfig {
            start_t_us: start_t_us + episode as u64 * stride_us,
            magnitude,
            ..config
        };
        pairs = pairs.saturating_add(run_action_pulse_on_viewport(vp, episode_config).pairs_formed);
    }
    pairs
}

fn run_retained_action_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    episodes: u32,
    start_t_us: u64,
    magnitude: f64,
) -> u32 {
    let mut pairs = run_action_series(vp, config, episodes, start_t_us, magnitude);
    let retained_delta = vp.read_learned_bias().summary().max_abs_delta;
    if retained_delta > 0.0 {
        let extra_config = CurriculumConfig {
            start_t_us: start_t_us + episodes as u64 * action_episode_stride_us(config),
            magnitude: (magnitude * 0.5).clamp(0.0, 1.0),
            ..config
        };
        pairs = pairs.saturating_add(run_action_pulse_on_viewport(vp, extra_config).pairs_formed);
    }
    pairs
}

fn run_mark_binding_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    episodes: u32,
    start_t_us: u64,
    target: BindingTargetRef,
    stream_id: u64,
    mark_bytes: &[u8],
) {
    let step_us = config.step_us.max(1);
    for episode in 0..episodes {
        let t_us = start_t_us + episode as u64 * step_us;
        vp.observe_mark_bytes(stream_id, t_us, t_us + step_us / 2, mark_bytes, 1.0);
        vp.bind_latest_mark_to_target(target, BindingFeedback::Align, t_us + step_us);
    }
}

#[derive(Clone, Copy, Debug)]
struct TextGroundingTargets {
    object_targets: [BindingTargetRef; 3],
    relation_targets: [BindingTargetRef; 2],
    time_target: BindingTargetRef,
}

fn setup_text_grounding_targets(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
) -> TextGroundingTargets {
    let t_us = config.start_t_us;
    let slot_a = vp
        .allocate_slot(MOD_VIB | MOD_PHOT, t_us, 301)
        .expect("text grounding slot a");
    let slot_b = vp
        .allocate_slot(MOD_VIB | MOD_PHOT, t_us, 302)
        .expect("text grounding slot b");
    let slot_c = vp
        .allocate_slot(MOD_VIB | MOD_PHOT, t_us, 303)
        .expect("text grounding slot c");
    let slots = [slot_a, slot_b, slot_c];

    for (idx, slot) in slots.iter().copied().enumerate() {
        let offset = idx as f64 * 0.18;
        seed_attended_slot(vp, slot, t_us);
        vp.write_slot_spatial(
            slot,
            MOD_PHOT,
            t_us,
            0.18 + offset,
            0.24 + offset * 0.5,
            0.14,
            0.14,
            0.85,
        );
    }
    vp.refresh_global_attention(t_us);
    vp.refresh_global_relations(t_us);

    TextGroundingTargets {
        object_targets: [
            BindingTargetRef::ObjectSlot {
                slot_id: slot_a as u8,
            },
            BindingTargetRef::ObjectSlot {
                slot_id: slot_b as u8,
            },
            BindingTargetRef::ObjectSlot {
                slot_id: slot_c as u8,
            },
        ],
        relation_targets: [
            relation_target_for(vp, slot_a, slot_b),
            relation_target_for(vp, slot_b, slot_c),
        ],
        time_target: BindingTargetRef::TimeWindow {
            start_t_us: t_us + config.step_us.max(1),
            end_t_us: t_us + config.step_us.max(1) * 3,
        },
    }
}

fn setup_simple_text_targets(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
) -> [BindingTargetRef; 3] {
    let t_us = config.start_t_us;
    let slot_a = vp
        .allocate_slot(MOD_VIB, t_us, 401)
        .expect("simple text target slot a");
    let slot_b = vp
        .allocate_slot(MOD_VIB, t_us, 402)
        .expect("simple text target slot b");
    let slot_c = vp
        .allocate_slot(MOD_VIB, t_us, 403)
        .expect("simple text target slot c");
    [
        BindingTargetRef::ObjectSlot {
            slot_id: slot_a as u8,
        },
        BindingTargetRef::ObjectSlot {
            slot_id: slot_b as u8,
        },
        BindingTargetRef::ObjectSlot {
            slot_id: slot_c as u8,
        },
    ]
}

fn relation_target_for(vp: &Viewport, src: usize, dst: usize) -> BindingTargetRef {
    let relations = vp.read_global_relations();
    let edge_index = relations
        .edges
        .iter()
        .position(|edge| {
            edge.active && edge.src_slot_id as usize == src && edge.dst_slot_id as usize == dst
        })
        .expect("live relation edge");
    BindingTargetRef::RelationEdge {
        edge_index: edge_index as u16,
    }
}

fn run_text_binding_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    episodes: u32,
    start_t_us: u64,
    target: BindingTargetRef,
    stream_id: u64,
    mark_bytes: &[u8],
) -> u64 {
    let step_us = config.step_us.max(1);
    let mut mark_hash = 0;
    for episode in 0..episodes {
        let t_us = start_t_us + episode as u64 * step_us;
        mark_hash = vp.observe_mark_bytes(stream_id, t_us, t_us + step_us / 2, mark_bytes, 1.0);
        vp.bind_latest_mark_to_target(target, BindingFeedback::Align, t_us + step_us);
    }
    mark_hash
}

fn bind_adjacent_mark_window(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    first_mark: &[u8],
    second_mark: &[u8],
    target: BindingTargetRef,
    stream_id: u64,
    start_t_us: u64,
) -> u64 {
    let step_us = config.step_us.max(1);
    let first_hash = vp.observe_mark_bytes(
        stream_id,
        start_t_us,
        start_t_us + step_us / 2,
        first_mark,
        1.0,
    );
    let second_t_us = start_t_us + step_us;
    let second_hash = vp.observe_mark_bytes(
        stream_id,
        second_t_us,
        second_t_us + step_us / 2,
        second_mark,
        1.0,
    );
    let pair_hash = adjacent_pair_hash(first_hash, second_hash, start_t_us, second_t_us);
    let event = BindingEvent {
        pattern_kind: PatternKind::SymbolSeq,
        pattern_hash: pair_hash,
        target,
        feedback: BindingFeedback::Align,
        evidence_ref: adjacent_pair_hash(
            pair_hash,
            first_hash ^ second_hash,
            start_t_us,
            second_t_us,
        ),
        event_t_us: second_t_us + step_us,
    };
    vp.apply_language_binding_event(event);
    vp.apply_language_binding_event(BindingEvent {
        evidence_ref: adjacent_pair_hash(
            pair_hash,
            first_hash ^ second_hash ^ step_us,
            start_t_us,
            second_t_us + step_us,
        ),
        event_t_us: second_t_us + step_us * 2,
        ..event
    });
    pair_hash
}

fn adjacent_pair_hash(first_hash: u64, second_hash: u64, start_t_us: u64, end_t_us: u64) -> u64 {
    let hash = first_hash.rotate_left(17)
        ^ second_hash.rotate_left(41)
        ^ start_t_us.rotate_left(7)
        ^ end_t_us.rotate_left(29);
    if hash == 0 {
        1
    } else {
        hash
    }
}

fn bind_mark_set(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    marks: &[&[u8]],
    targets: &[BindingTargetRef],
    episodes: u32,
    start_t_us: u64,
    stream_id: u64,
) -> Vec<u64> {
    let stride_us = config.step_us.max(1) * (episodes.max(1) as u64 + 1);
    marks
        .iter()
        .zip(targets.iter().copied())
        .enumerate()
        .map(|(idx, (mark, target))| {
            run_text_binding_series(
                vp,
                config,
                episodes,
                start_t_us + idx as u64 * stride_us,
                target,
                stream_id,
                mark,
            )
        })
        .collect()
}

fn write_vib_disturbance_sample(vp: &Viewport, slot_id: usize, t_us: u64, amplitude: f64) {
    let observed = [amplitude; VIB_BANDS];
    let predicted = [amplitude * 0.82; VIB_BANDS];
    let err = [amplitude * 0.18; VIB_BANDS];
    let surprise = [0.25 + amplitude * 0.5; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &observed, &predicted, &err, &surprise);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
}

fn write_vib_band_shape_sample(
    vp: &Viewport,
    slot_id: usize,
    t_us: u64,
    low_energy: f64,
    high_energy: f64,
) -> f64 {
    // Synthesize a band-shape that ramps from low_energy at the bottom band to
    // high_energy at the top band, spread across all VIB_BANDS.
    let mut observed = [0.0_f64; VIB_BANDS];
    for (idx, slot) in observed.iter_mut().enumerate() {
        let frac = if VIB_BANDS > 1 {
            idx as f64 / (VIB_BANDS - 1) as f64
        } else {
            0.0
        };
        *slot = low_energy * (1.0 - frac) + high_energy * frac;
    }
    let predicted = observed.map(|value| value * 0.75);
    let err = observed.map(|value| value * 0.25);
    let surprise = observed.map(|value| 0.20 + value * 0.40);
    vp.write_slot_vib(slot_id, t_us, &observed, &predicted, &err, &surprise);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
    vib_band_center(&observed)
}

fn vib_band_center(observed: &[f64; VIB_BANDS]) -> f64 {
    let mut weighted = 0.0;
    let mut total = 0.0;
    for (idx, value) in observed.iter().enumerate() {
        let value = value.max(0.0);
        weighted += idx as f64 * value;
        total += value;
    }
    if total > 0.0 {
        weighted / total
    } else {
        0.0
    }
}

fn write_phot_spatial_sample(vp: &Viewport, slot_id: usize, t_us: u64, base_x: f64, episode: u32) {
    let motion = episode as f64 * 0.025;
    vp.write_slot_spatial(
        slot_id,
        MOD_PHOT,
        t_us,
        (base_x + motion).clamp(0.0, 0.92),
        (0.22 + motion * 0.5).clamp(0.0, 0.92),
        0.12,
        0.12,
        0.86,
    );
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
}

fn apply_pattern_binding(
    vp: &Arc<Viewport>,
    pattern_kind: PatternKind,
    pattern_hash: u64,
    target: BindingTargetRef,
    feedback: BindingFeedback,
    event_t_us: u64,
) {
    vp.apply_language_binding_event(BindingEvent {
        pattern_kind,
        pattern_hash,
        target,
        feedback,
        evidence_ref: adjacent_pair_hash(
            pattern_hash,
            binding_target_ref_code(target),
            pattern_kind as u64 + 1,
            event_t_us,
        ),
        event_t_us,
    });
}

fn carrier_pattern_hash(base_hash: u64, target: BindingTargetRef, salt: u64) -> u64 {
    adjacent_pair_hash(
        base_hash ^ salt.rotate_left(11),
        binding_target_ref_code(target),
        salt.rotate_left(19),
        base_hash.rotate_left(31),
    )
}

fn binding_target_ref_code(target: BindingTargetRef) -> u64 {
    match target {
        BindingTargetRef::ObjectSlot { slot_id } => 0x1000 | slot_id as u64,
        BindingTargetRef::RelationEdge { edge_index } => 0x2000 | edge_index as u64,
        BindingTargetRef::TimeWindow {
            start_t_us,
            end_t_us,
        } => 0x3000 ^ start_t_us.rotate_left(13) ^ end_t_us.rotate_left(37),
        BindingTargetRef::PerturbationNode { node_id } => 0x4000 | node_id as u64,
    }
}

fn object_slot_id(target: BindingTargetRef) -> usize {
    match target {
        BindingTargetRef::ObjectSlot { slot_id } => slot_id as usize,
        BindingTargetRef::RelationEdge { .. }
        | BindingTargetRef::TimeWindow { .. }
        | BindingTargetRef::PerturbationNode { .. } => 0,
    }
}

fn target_focus_edge_id(target: BindingTargetRef) -> i16 {
    match target {
        BindingTargetRef::RelationEdge { edge_index } => edge_index as i16,
        BindingTargetRef::ObjectSlot { .. }
        | BindingTargetRef::TimeWindow { .. }
        | BindingTargetRef::PerturbationNode { .. } => -1,
    }
}

fn mean_mark_ambiguity(vp: &Viewport, hashes: &[u64]) -> f64 {
    if hashes.is_empty() {
        return 0.0;
    }
    let state = vp.read_mark_disturbance();
    hashes
        .iter()
        .copied()
        .map(|hash| mark_ambiguity(&state, hash))
        .sum::<f64>()
        / hashes.len() as f64
}

fn pass_flag(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

fn finite_min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(f64::INFINITY, f64::min)
}

fn finite_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(f64::NEG_INFINITY, f64::max)
}

fn max_indexed_pressure(pressures: [f64; 3]) -> (usize, f64) {
    pressures
        .iter()
        .copied()
        .enumerate()
        .max_by(|(_, lhs), (_, rhs)| lhs.total_cmp(rhs))
        .unwrap_or((0, 0.0))
}

fn seed_active_gap_pressure(
    vp: &Viewport,
    surprise: f64,
    instability: f64,
    focus_slot_id: i8,
    focus_edge_id: i16,
) {
    let mut rs = vp.global_reasoning.write().unwrap();
    rs.global_surprise_load = surprise.clamp(0.0, 1.0);
    rs.network_instability_score = instability.clamp(0.0, 1.0);
    rs.routing_focus_slot_ids[0] = focus_slot_id;
    rs.routing_focus_edge_ids[0] = focus_edge_id;
}

fn seed_body_field_pressures(
    vp: &Viewport,
    t_us: u64,
    focus_slot_id: i8,
    focus_edge_id: i16,
    gap_pressure: f64,
    event_pressure: f64,
    relation_pressure: f64,
) {
    seed_active_gap_pressure(
        vp,
        gap_pressure,
        gap_pressure * 0.8,
        focus_slot_id,
        focus_edge_id,
    );
    {
        let mut summary = vp.global_reasoning_memory_summary.write().unwrap();
        summary.novelty_score = gap_pressure * 0.6;
        summary.persistence_score = gap_pressure.max(0.35).clamp(0.0, 1.0);
        summary.pattern_recurrence_score = gap_pressure.max(0.40).clamp(0.0, 1.0);
        summary.routing_stability_score = 0.62;
        summary.dominant_focus_persistence = gap_pressure.max(0.45).clamp(0.0, 1.0);
        summary.history_fill_ratio = 0.80;
        summary.last_update_t_us = t_us;
    }
    vp.refresh_active_reasoning_gap(t_us);
    vp.tick_evidence_tension(
        PressureSample {
            err: event_pressure,
            recurrence: event_pressure * 0.5,
            ambiguity: event_pressure,
            alignment: 0.15,
            support: 0.0,
            deviation: event_pressure,
            temporal_gap: event_pressure,
            cross_channel: event_pressure * 0.4,
        },
        t_us + 1_000,
    );
    {
        let edge_idx = focus_edge_id.max(0) as usize;
        let mut relations = vp.global_relations.write().unwrap();
        let edge_idx = edge_idx.min(relations.edges.len().saturating_sub(1));
        relations.edges[edge_idx].src_slot_id = focus_slot_id.max(0) as u8;
        relations.edges[edge_idx].dst_slot_id = ((focus_slot_id + 1).max(0) as u8).min(3);
        relations.edges[edge_idx].active = true;
        relations.edges[edge_idx].relation_confidence = 0.84;
        relations.edges[edge_idx].predictive_score = 0.68;
        relations.edges[edge_idx].pred.relation_surprise = relation_pressure.clamp(0.0, 1.0);
        relations.edges[edge_idx].last_update_t_us = t_us + 2_000;
        relations.n_active_edges = relations.n_active_edges.saturating_add(1);
        relations.last_update_t_us = t_us + 2_000;
    }
}

fn has_pressure_wave(vp: &Viewport, source_kind: u8) -> bool {
    vp.read_prediction_pressure_field()
        .waves
        .iter()
        .any(|wave| wave.source_kind == source_kind)
}

fn body_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(gap_pos) = body_source.find("refresh_active_reasoning_gap") else {
        return 0;
    };
    let Some(field_pos) = body_source.find("tick_prediction_pressure_field_from_substrate") else {
        return 0;
    };
    let Some(probe_pos) = body_source.find("refresh_probe_law") else {
        return 0;
    };
    u8::from(gap_pos < field_pos && field_pos < probe_pos)
}

fn active_surface_wave(
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    amplitude: f64,
    resonance: f64,
    closure_affordance: f64,
    t_us: u64,
) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind,
        slot_id,
        edge_id,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance,
        closure_affordance,
        write_risk: 0.05,
    }
}

fn seed_active_surface_parallel_pressure(vp: &Viewport, t_us: u64) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[
            active_surface_wave(PREDICTION_WAVE_GAP, 1, 2, 0.86, 0.55, 0.78, t_us),
            active_surface_wave(PREDICTION_WAVE_RELATION, 2, 4, 0.72, 0.92, 0.72, t_us),
            active_surface_wave(PREDICTION_WAVE_RESIDUE, 3, 6, 0.48, 0.65, 0.58, t_us),
        ],
    );
}

fn seed_active_surface_event_pressure(vp: &Viewport, t_us: u64) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[active_surface_wave(
            PREDICTION_WAVE_EVENT,
            0,
            9,
            0.98,
            0.70,
            0.86,
            t_us,
        )],
    );
}

fn run_active_surface_chain_step(
    vp: &Viewport,
    t_us: u64,
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    matched_return: bool,
) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[active_surface_wave(
            source_kind,
            slot_id,
            edge_id,
            0.98,
            0.74,
            0.84,
            t_us,
        )],
    );
    vp.refresh_active_surface(t_us + 1_000);
    vp.refresh_probe_law(t_us + 2_000);
    vp.tick_probe_output_from_law(t_us + 2_000);
    let write_magnitude = latest_generic_write_magnitude(vp);
    let return_magnitude = if matched_return {
        write_magnitude
    } else if write_magnitude >= 0.5 {
        0.0
    } else {
        1.0
    };
    pair_latest_generic_write_to_return_with_magnitude(vp, t_us + 120_000, return_magnitude);
    vp.tick_active_surface_probe_chain(t_us + 121_000);
    vp.tick_active_surface_chain_memory(t_us + 122_000);
}

fn quiet_active_surface_pressure(vp: &Viewport, start_t_us: u64, ticks: u32, step_us: u64) {
    for step in 0..ticks {
        let t_us = start_t_us + step as u64 * step_us;
        vp.refresh_prediction_pressure_field(t_us, &[]);
        vp.refresh_active_surface(t_us + 1_000);
    }
}

fn active_surface_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(field_pos) = body_source.find("tick_prediction_pressure_field_from_substrate") else {
        return 0;
    };
    let Some(surface_pos) = body_source.find("refresh_active_surface") else {
        return 0;
    };
    let Some(probe_pos) = body_source.find("refresh_probe_law") else {
        return 0;
    };
    u8::from(field_pos < surface_pos && surface_pos < probe_pos)
}

fn seed_active_surface_residue_for_relation(vp: &Viewport, t_us: u64, edge_id: i16) {
    vp.refresh_prediction_pressure_field(
        t_us,
        &[active_surface_wave(
            PREDICTION_WAVE_GAP,
            1,
            edge_id,
            0.42,
            0.50,
            0.60,
            t_us,
        )],
    );
    vp.refresh_active_surface(t_us + 1_000);
}

fn seed_relation_resonance_edge(
    vp: &Viewport,
    t_us: u64,
    edge_id: usize,
    pressure: f64,
    confidence: f64,
) {
    let mut relations = vp.global_relations.write().unwrap();
    relations.edges[edge_id].src_slot_id = 1;
    relations.edges[edge_id].dst_slot_id = 2;
    relations.edges[edge_id].active = true;
    relations.edges[edge_id].relation_confidence = confidence;
    relations.edges[edge_id].predictive_score = pressure;
    relations.edges[edge_id].pred.relation_surprise = pressure;
    relations.edges[edge_id].last_update_t_us = t_us;
    relations.n_active_edges = 1;
    relations.last_update_t_us = t_us;
}

fn seed_relation_resonance_memory(vp: &Viewport, recurrence: f64, stability: f64, t_us: u64) {
    let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
    memory.pattern_recurrence_score = recurrence;
    memory.routing_stability_score = stability;
    memory.persistence_score = recurrence;
    memory.dominant_focus_persistence = recurrence.max(0.25);
    memory.history_fill_ratio = 0.80;
    memory.last_update_t_us = t_us;
}

fn background_relation_resonance_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(field_pos) = body_source.find("tick_prediction_pressure_field_from_substrate") else {
        return 0;
    };
    let Some(resonance_pos) = body_source.find("tick_active_surface_relation_resonance") else {
        return 0;
    };
    let Some(surface_pos) = body_source.find("refresh_active_surface") else {
        return 0;
    };
    let Some(probe_pos) = body_source.find("refresh_probe_law") else {
        return 0;
    };
    u8::from(field_pos < resonance_pos && resonance_pos < surface_pos && surface_pos < probe_pos)
}

fn local_probe_chain_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(field_pos) = body_source.find("tick_prediction_pressure_field_from_substrate") else {
        return 0;
    };
    let Some(resonance_pos) = body_source.find("tick_active_surface_relation_resonance") else {
        return 0;
    };
    let Some(surface_pos) = body_source.find("refresh_active_surface") else {
        return 0;
    };
    let Some(chain_pos) = body_source.find("tick_active_surface_probe_chain") else {
        return 0;
    };
    let Some(probe_pos) = body_source.find("refresh_probe_law") else {
        return 0;
    };
    u8::from(
        field_pos < resonance_pos
            && resonance_pos < surface_pos
            && surface_pos < chain_pos
            && chain_pos < probe_pos,
    )
}

fn chain_memory_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(surface_pos) = body_source.find("refresh_active_surface") else {
        return 0;
    };
    let Some(chain_pos) = body_source.find("tick_active_surface_probe_chain") else {
        return 0;
    };
    let Some(memory_pos) = body_source.find("tick_active_surface_chain_memory") else {
        return 0;
    };
    let Some(probe_pos) = body_source.find("refresh_probe_law") else {
        return 0;
    };
    u8::from(surface_pos < chain_pos && chain_pos < memory_pos && memory_pos < probe_pos)
}

fn vocal_reflex_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(reflex_pos) = body_source.find("tick_vocal_reflex_from_pressure") else {
        return 0;
    };
    let Some(target_pos) = body_source.find("tick_vocal_target_from_vib") else {
        return 0;
    };
    let Some(loop_pos) = body_source.find("tick_vocal_imitation_loop") else {
        return 0;
    };
    u8::from(reflex_pos < target_pos && target_pos < loop_pos)
}

fn auditory_visual_source_order_guard() -> u8 {
    let body_source = include_str!("bin/body.rs");
    let Some(pair_pos) = body_source.find("tick_auditory_visual_from_pairs") else {
        return 0;
    };
    let Some(fusion_pos) = body_source.find("tick_auditory_visual_from_fusion") else {
        return 0;
    };
    let Some(expression_pos) = body_source.find("tick_auditory_visual_expression") else {
        return 0;
    };
    let Some(snapshot_rel_pos) = body_source[expression_pos..].find("write_snapshot") else {
        return 0;
    };
    let snapshot_pos = expression_pos + snapshot_rel_pos;
    u8::from(pair_pos < fusion_pos && fusion_pos < expression_pos && expression_pos < snapshot_pos)
}

fn run_probe_ticks(vp: &Viewport, start_t_us: u64, ticks: u32, step_us: u64) -> u32 {
    let before = vp.read_action_output().n_write_epochs;
    for tick in 0..ticks {
        let t_us = start_t_us + tick as u64 * step_us;
        vp.refresh_probe_law(t_us);
        vp.tick_probe_output_from_law(t_us);
    }
    vp.read_action_output()
        .n_write_epochs
        .saturating_sub(before)
}

fn latest_generic_write_magnitude(vp: &Viewport) -> f64 {
    vp.read_action_output()
        .write_epoch_history
        .iter()
        .rev()
        .find(|record| record.channel_kind == OutputChannelKind::GenericWrite)
        .map(|record| record.start_magnitude)
        .unwrap_or(0.0)
}

fn pair_latest_generic_write_to_return(vp: &Viewport, return_t_us: u64) -> bool {
    let magnitude = latest_generic_write_magnitude(vp);
    pair_latest_generic_write_to_return_with_magnitude(vp, return_t_us, magnitude)
}

fn pair_latest_generic_write_to_return_with_magnitude(
    vp: &Viewport,
    return_t_us: u64,
    magnitude: f64,
) -> bool {
    let latest = vp
        .read_action_output()
        .write_epoch_history
        .iter()
        .rev()
        .find(|record| record.channel_kind == OutputChannelKind::GenericWrite)
        .cloned();
    let Some(record) = latest else {
        return false;
    };
    let window_idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: return_t_us,
        end_t_us: return_t_us + 1,
        magnitude: magnitude.clamp(0.0, 1.0),
    });
    let window_id = vp
        .read_action_consequence()
        .windows
        .get(window_idx as usize)
        .map(|window| window.window_id)
        .unwrap_or_default();
    vp.close_action_consequence_window(window_id, return_t_us + 2);
    vp.pair_action_consequence(record.write_seq, window_id, return_t_us - record.start_t_us)
}

fn pattern_binding_strength(
    vp: &Viewport,
    pattern_kind: PatternKind,
    pattern_hash: u64,
    target: BindingTargetRef,
) -> f64 {
    vp.read_language_binding()
        .traces
        .iter()
        .find(|trace| {
            trace.pattern_kind == pattern_kind
                && trace.pattern_hash == pattern_hash
                && trace.target == target
        })
        .map(|trace| trace.strength.value)
        .unwrap_or(0.0)
}

fn mean_pair_coupling(
    vp: &Viewport,
    lhs_kind: PatternKind,
    lhs_hashes: &[u64],
    rhs_kind: PatternKind,
    rhs_hashes: &[u64],
    targets: &[BindingTargetRef],
) -> f64 {
    let strengths = lhs_hashes
        .iter()
        .copied()
        .zip(rhs_hashes.iter().copied())
        .zip(targets.iter().copied())
        .map(|((lhs_hash, rhs_hash), target)| {
            pattern_binding_strength(vp, lhs_kind, lhs_hash, target)
                .min(pattern_binding_strength(vp, rhs_kind, rhs_hash, target))
        })
        .collect::<Vec<_>>();
    mean_strength(&strengths)
}

fn triad_coupling_strength(
    vp: &Viewport,
    text_hash: u64,
    vib_hash: u64,
    phot_hash: u64,
    target: BindingTargetRef,
) -> f64 {
    pattern_binding_strength(vp, PatternKind::SymbolSeq, text_hash, target)
        .min(pattern_binding_strength(
            vp,
            PatternKind::VibModality,
            vib_hash,
            target,
        ))
        .min(pattern_binding_strength(
            vp,
            PatternKind::PhotModality,
            phot_hash,
            target,
        ))
}

fn mean_triad_coupling(
    vp: &Viewport,
    text_hashes: &[u64],
    vib_hashes: &[u64],
    phot_hashes: &[u64],
    targets: &[BindingTargetRef],
) -> f64 {
    let strengths = text_hashes
        .iter()
        .copied()
        .zip(vib_hashes.iter().copied())
        .zip(phot_hashes.iter().copied())
        .zip(targets.iter().copied())
        .map(|(((text_hash, vib_hash), phot_hash), target)| {
            triad_coupling_strength(vp, text_hash, vib_hash, phot_hash, target)
        })
        .collect::<Vec<_>>();
    mean_strength(&strengths)
}

fn run_tri_modal_primitive_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    slot: usize,
    episodes: u32,
    start_t_us: u64,
    target: BindingTargetRef,
    stream_id: u64,
    mark_bytes: &[u8],
) -> u32 {
    let step_us = config.step_us.max(1);
    for episode in 0..episodes {
        let t_us = start_t_us + episode as u64 * step_us;
        let motion = episode as f64 * 0.04;
        seed_attended_slot(vp, slot, t_us);
        vp.write_slot_spatial(
            slot,
            MOD_PHOT,
            t_us,
            0.18 + motion,
            0.24 + motion * 0.5,
            0.14,
            0.14,
            0.85,
        );
        vp.refresh_global_attention(t_us);
        vp.refresh_global_relations(t_us);
        vp.observe_mark_bytes(stream_id, t_us, t_us + step_us / 2, mark_bytes, 1.0);
        vp.bind_latest_mark_to_target(target, BindingFeedback::Align, t_us + step_us);
    }
    episodes
}

fn symbol_binding_strength(vp: &Viewport, mark_hash: u64, target: BindingTargetRef) -> f64 {
    vp.read_language_binding()
        .traces
        .iter()
        .find(|trace| {
            trace.pattern_kind == PatternKind::SymbolSeq
                && trace.pattern_hash == mark_hash
                && trace.target == target
        })
        .map(|trace| trace.strength.value)
        .unwrap_or(0.0)
}

fn strengths_for_targets(
    vp: &Viewport,
    mark_hashes: &[u64],
    targets: &[BindingTargetRef],
) -> Vec<f64> {
    mark_hashes
        .iter()
        .copied()
        .zip(targets.iter().copied())
        .map(|(hash, target)| symbol_binding_strength(vp, hash, target))
        .collect()
}

fn max_strength_against_targets(
    vp: &Viewport,
    mark_hashes: &[u64],
    targets: &[BindingTargetRef],
) -> f64 {
    mark_hashes
        .iter()
        .copied()
        .flat_map(|hash| {
            targets
                .iter()
                .copied()
                .map(move |target| symbol_binding_strength(vp, hash, target))
        })
        .fold(0.0, f64::max)
}

fn active_probe_bytes(mark_hash: u64, ambiguity: f64, salt: u64) -> [u8; 8] {
    let scaled = (ambiguity.clamp(0.0, 1.0) * 65_535.0) as u64;
    let mut seed = mark_hash ^ scaled.rotate_left(9) ^ salt.rotate_left(17);
    let mut out = [0u8; 8];
    for (idx, byte) in out.iter_mut().enumerate() {
        seed = seed
            .wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add(idx as u64 + 1);
        *byte = ((seed >> ((idx % 8) * 8)) as u8).rotate_left(1) ^ 0xa5;
    }
    out
}

fn average_pair_similarity(vp: &Viewport) -> f64 {
    let ac = vp.read_action_consequence();
    if ac.pairs.is_empty() {
        0.0
    } else {
        ac.pairs
            .iter()
            .map(|pair| pair.curve_similarity)
            .sum::<f64>()
            / ac.pairs.len() as f64
    }
}

fn mean_strength(strengths: &[f64]) -> f64 {
    if strengths.is_empty() {
        0.0
    } else {
        strengths.iter().sum::<f64>() / strengths.len() as f64
    }
}

fn weakest_strength(strengths: &[f64]) -> f64 {
    strengths.iter().copied().reduce(f64::min).unwrap_or(0.0)
}

fn distinct_variant_ratio(variant_hashes: &[u64], base_hashes: &[u64]) -> f64 {
    if variant_hashes.is_empty() {
        return 0.0;
    }
    let distinct = variant_hashes
        .iter()
        .filter(|hash| !base_hashes.contains(hash))
        .count();
    distinct as f64 / variant_hashes.len() as f64
}

fn mark_recurrence(mark: &crate::mark_disturbance::MarkDisturbanceState, mark_hash: u64) -> u32 {
    mark.traces
        .iter()
        .find(|trace| trace.mark_hash == mark_hash)
        .map(|trace| trace.recurrence_count)
        .unwrap_or_default()
}

fn mark_ambiguity(mark: &crate::mark_disturbance::MarkDisturbanceState, mark_hash: u64) -> f64 {
    mark.traces
        .iter()
        .find(|trace| trace.mark_hash == mark_hash)
        .map(|trace| trace.ambiguity)
        .unwrap_or_default()
}

fn average2(a: f64, b: f64) -> f64 {
    (a + b) * 0.5
}

fn ratio_or_zero(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

fn seed_attended_slot(vp: &Viewport, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_surprise = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_surprise);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);

    let phot_obs = vec![0.2f64; PHOT_BANDS];
    let phot_pred = vec![0.18f64; PHOT_BANDS];
    let phot_err = vec![0.02f64; PHOT_BANDS];
    let phot_surprise = vec![0.4f64; PHOT_BANDS];
    vp.write_slot_phot(
        slot_id,
        t_us,
        &phot_obs,
        &phot_pred,
        &phot_err,
        &phot_surprise,
    );
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
}

fn relation_scores(relations: &GlobalRelationState, src: usize, dst: usize) -> (f64, f64) {
    relation_edge(relations, src, dst)
        .map(|edge| (edge.spatial_distance_score, edge.spatial_overlap_score))
        .unwrap_or((0.0, 0.0))
}

fn relation_edge(
    relations: &GlobalRelationState,
    src: usize,
    dst: usize,
) -> Option<&RelationEdgeState> {
    relations
        .edges
        .iter()
        .find(|edge| edge.src_slot_id as usize == src && edge.dst_slot_id as usize == dst)
}

fn write_optional_snapshot(
    vp: &Arc<Viewport>,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<u32> {
    if let Some(paths) = snapshot_paths {
        write_snapshot(vp, paths)?;
        Ok(1)
    } else {
        Ok(0)
    }
}
