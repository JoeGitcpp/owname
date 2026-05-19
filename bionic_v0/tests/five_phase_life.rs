use bionic_v0::active_reasoning::ActiveReasoningGapState;
use bionic_v0::adaptive_stability::AdaptiveStabilityState;
use bionic_v0::code_ingress::CodeIngressState;
use bionic_v0::evidence_tension::EvidenceTensionState;
use bionic_v0::five_phase_life::summarize_five_phase_life;
use bionic_v0::language_binding::LanguageBindingState;
use bionic_v0::learned_bias::LearnedBiasSummary;
use bionic_v0::self_development::SelfDevelopmentState;
use bionic_v0::snapshot::{
    SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub,
    SnapshotFramedByteEgress, SnapshotLastClaim, SnapshotProbeLaw, SnapshotVocalMemory,
    SnapshotData,
};
use bionic_v0::viewport::{
    GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState, GlobalRelationState,
};
use bionic_v0::vocal_imitation::VocalImitationState;
use bionic_v0::vocal_organ::VocalOrganState;
use bionic_v0::vocal_target::VocalTargetState;
use bionic_v0::vpc::VpcContractState;

fn empty_snapshot() -> SnapshotData {
    SnapshotData {
        seq: 0,
        master_tick: 0,
        clock_us: 0,
        start_us: 0,
        source_writes: 0,
        predictor_ticks: 0,
        snapshot_fail_count: 0,
        history_cap: 0,
        axis_count: 0,
        hearing_fault: false,
        hearing_fault_count: 0,
        hl_a: 0.0,
        hl_avg_mse: 0.0,
        hl_pred_mse: 0.0,
        hl_meta_err: 0.0,
        hl_modulation: 0.0,
        last_obs_t_us: 0,
        health_state: 0,
        phot_health: 0,
        phot_started_t_us: 0,
        phot_last_frame_t_us: 0,
        axes: Vec::new(),
        ambient_nomination_seq: 0,
        ambient_vib: SnapshotAmbientSub::default(),
        ambient_phot: SnapshotAmbientSub::default(),
        ambient_mech: SnapshotAmbientSub::default(),
        slots: Vec::new(),
        last_claim: SnapshotLastClaim::default(),
        global_attention: GlobalAttentionState::default(),
        global_relations: GlobalRelationState::default(),
        global_reasoning: GlobalReasoningState::default(),
        global_reasoning_memory: GlobalReasoningMemorySummary::default(),
        language_binding: LanguageBindingState::default(),
        vpc_contract: VpcContractState::default(),
        action_output: SnapshotActionOutput::default(),
        action_consequence: SnapshotActionConsequence::default(),
        probe_law: SnapshotProbeLaw::default(),
        vocal_target: VocalTargetState::default(),
        vocal_organ: VocalOrganState::default(),
        vocal_imitation: VocalImitationState::default(),
        vocal_memory: SnapshotVocalMemory::default(),
        code_ingress: CodeIngressState::default(),
        learned_bias: LearnedBiasSummary::default(),
        adaptive_stability: AdaptiveStabilityState::default(),
        evidence_tension: EvidenceTensionState::default(),
        framed_byte_egress: SnapshotFramedByteEgress::default(),
        active_reasoning_gap: ActiveReasoningGapState::default(),
        self_development: SelfDevelopmentState::default(),
    }
}

#[test]
fn five_phase_life_scores_are_bounded_and_observer_side() {
    let mut snap = empty_snapshot();
    snap.active_reasoning_gap.gap_pressure = 0.72;
    snap.active_reasoning_gap.closure_score = 0.44;
    snap.active_reasoning_gap.reentry_score = 0.38;
    snap.global_reasoning_memory.novelty_score = 0.61;
    snap.global_reasoning_memory.persistence_score = 0.54;
    snap.self_development.exploratory_pressure = 0.58;
    snap.self_development.stable_recurrence = 0.63;
    snap.self_development.familiarity_pressure = 0.57;
    snap.self_development.output_suppression = 0.20;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert!((0.0..=1.0).contains(&evidence.wood_balance));
    assert!((0.0..=1.0).contains(&evidence.fire_balance));
    assert!((0.0..=1.0).contains(&evidence.earth_balance));
    assert!((0.0..=1.0).contains(&evidence.metal_balance));
    assert!((0.0..=1.0).contains(&evidence.water_balance));
    assert_eq!(evidence.principle_guard_pass, 1);
}
