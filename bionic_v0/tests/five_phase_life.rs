use bionic_v0::active_reasoning::ActiveReasoningGapState;
use bionic_v0::adaptive_stability::AdaptiveStabilityState;
use bionic_v0::code_ingress::CodeIngressState;
use bionic_v0::evidence_tension::EvidenceTensionState;
use bionic_v0::five_phase_life::summarize_five_phase_life;
use bionic_v0::language_binding::LanguageBindingState;
use bionic_v0::learned_bias::LearnedBiasSummary;
use bionic_v0::live_human_nursery::sample_live_human_nursery_snapshots;
use bionic_v0::self_development::SelfDevelopmentState;
use bionic_v0::snapshot::{
    SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub, SnapshotData,
    SnapshotFramedByteEgress, SnapshotLastClaim, SnapshotProbeLaw, SnapshotVocalMemory,
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
        audio_raw_total_samples: 0,
        audio_raw_last_clock_us: 0,
        slot_xlayers: Vec::new(),
        slot_towers: Vec::new(),
        thread_fault: false,
        thread_fault_count: 0,
        dual_drive: Default::default(),
        axis_xlayers: Vec::new(),
        developmental_maturity: 0.0,
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

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert!((0.0..=1.0).contains(&evidence.wood_balance));
    assert!((0.0..=1.0).contains(&evidence.fire_balance));
    assert!((0.0..=1.0).contains(&evidence.earth_balance));
    assert!((0.0..=1.0).contains(&evidence.metal_balance));
    assert!((0.0..=1.0).contains(&evidence.water_balance));
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn fire_is_insufficient_when_pressure_has_no_manifestation() {
    let mut snap = empty_snapshot();
    snap.active_reasoning_gap.gap_pressure = 0.80;
    snap.global_reasoning_memory.novelty_score = 0.50;
    snap.self_development.exploratory_pressure = 0.52;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert!(evidence.wood_balance > 0.40);
    assert_eq!(evidence.fire_insufficient, 1);
    assert_eq!(evidence.fire_live_closure_gate_pass, 0);
}

#[test]
fn metal_excess_flags_when_suppression_blocks_wood_and_fire() {
    let mut snap = empty_snapshot();
    snap.probe_law.suppression = 0.96;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert_eq!(evidence.metal_excess, 1);
    assert_eq!(evidence.wood_fire_non_suppression, 0);
}

#[test]
fn five_phase_life_sources_do_not_introduce_body_ontology() {
    let src = include_str!("../src/five_phase_life.rs");
    for forbidden in [
        "semantic_readiness",
        "concept_id",
        "token_id",
        "object_name",
        "meaning",
        "personality",
        "identity",
        "owner",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "five phase life evidence must not contain `{forbidden}`"
        );
    }
}

#[test]
fn fire_live_closure_refuses_deterministic_fixture_even_when_closure_exists() {
    let snapshots = sample_live_human_nursery_snapshots();
    let evidence = summarize_five_phase_life(snapshots.iter(), 1, 0);

    assert!(evidence.fire_balance > 0.0);
    assert_eq!(evidence.fire_live_closure_gate_pass, 0);
}

#[test]
fn fire_live_closure_accepts_runtime_snapshot_with_write_return_and_closure() {
    let snapshots = sample_live_human_nursery_snapshots();
    let evidence = summarize_five_phase_life(snapshots.iter(), 0, 1);

    assert!(evidence.fire_balance > 0.0);
    assert_eq!(evidence.fire_live_closure_gate_pass, 1);
    assert_eq!(evidence.principle_guard_pass, 1);
}
