use bionic_v0::active_reasoning::ActiveReasoningGapState;
use bionic_v0::adaptive_stability::AdaptiveStabilityState;
use bionic_v0::code_ingress::CodeIngressState;
use bionic_v0::evidence_tension::EvidenceTensionState;
use bionic_v0::language_binding::LanguageBindingState;
use bionic_v0::learned_bias::LearnedBiasSummary;
use bionic_v0::live_human_nursery::summarize_live_human_nursery;
use bionic_v0::self_development::SelfDevelopmentState;
use bionic_v0::snapshot::{
    SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub, SnapshotConsequencePair,
    SnapshotConsequenceWindow, SnapshotData, SnapshotFramedByteEgress, SnapshotLastClaim,
    SnapshotProbeLaw, SnapshotSlot, SnapshotSpatial, SnapshotVocalMemory, SnapshotWriteEpochRecord,
};
use bionic_v0::viewport::{
    GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState, GlobalRelationState,
};
use bionic_v0::vocal_imitation::VocalImitationState;
use bionic_v0::vocal_organ::{VocalOrganPhase, VocalOrganState};
use bionic_v0::vocal_target::VocalTargetState;
use bionic_v0::vpc::VpcContractState;
use bionic_v0::{HEALTH_FAULT, HEALTH_OK, MOD_PHOT};

fn make_empty_snapshot() -> SnapshotData {
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

fn active_phot_slot() -> SnapshotSlot {
    SnapshotSlot {
        active: true,
        last_update_t_us: 1_000,
        source_modality: MOD_PHOT,
        nomination_seq: 1,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(SnapshotSpatial {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: 0.2,
            span_y: 0.2,
            precision: 0.7,
            last_update_t_us: 1_000,
            valid: true,
        }),
        spatial_worldline: None,
        fusion: None,
        high_layer: None,
        attention_share: 0.4,
        precision_bias: 0.0,
    }
}

#[test]
fn live_human_nursery_summarizes_life_sign_vocal_visual_and_reentry_evidence() {
    let mut life = make_empty_snapshot();
    life.seq = 1;
    life.phot_health = HEALTH_OK;
    life.ambient_vib.event_energy = 0.18;
    life.ambient_vib.candidate_flag = true;
    life.slots.push(active_phot_slot());

    let mut vocal = make_empty_snapshot();
    vocal.seq = 2;
    vocal.phot_health = HEALTH_OK;
    vocal.ambient_vib.event_energy = 0.22;
    vocal.vocal_target.active_energy = 0.5;
    vocal.vocal_organ = VocalOrganState {
        breath_drive: 0.45,
        pitch_tension: 0.35,
        resonance_position: 0.4,
        noise_mix: 0.0,
        formant_f1: 500.0,
        formant_f2: 1500.0,
        formant_f3: 2500.0,
        voiced_ratio: 0.0,
        breath_noise_ratio: 0.0,
        envelope_attack: 0.0,
        envelope_decay: 0.0,
        envelope_sustain: 0.0,
        phase: VocalOrganPhase::Active,
        last_update_t_us: 2_000,
    };
    vocal.action_output.n_write_epochs = 1;
    vocal.action_output.write_epoch_history = vec![SnapshotWriteEpochRecord {
        write_seq: 1,
        channel_kind: 0,
        start_magnitude: 0.5,
        start_t_us: 2_000,
        last_update_t_us: 2_500,
        last_phase: 1,
    }];

    let mut closure = make_empty_snapshot();
    closure.seq = 3;
    closure.phot_health = HEALTH_OK;
    closure.ambient_vib.event_energy = 0.22;
    closure.vocal_target.active_energy = 0.5;
    closure.vocal_organ = VocalOrganState {
        breath_drive: 0.45,
        pitch_tension: 0.35,
        resonance_position: 0.4,
        noise_mix: 0.0,
        formant_f1: 500.0,
        formant_f2: 1500.0,
        formant_f3: 2500.0,
        voiced_ratio: 0.0,
        breath_noise_ratio: 0.0,
        envelope_attack: 0.0,
        envelope_decay: 0.0,
        envelope_sustain: 0.0,
        phase: VocalOrganPhase::Active,
        last_update_t_us: 2_000,
    };
    closure.action_output.n_write_epochs = 1;
    closure.action_output.write_epoch_history = vec![SnapshotWriteEpochRecord {
        write_seq: 1,
        channel_kind: 0,
        start_magnitude: 0.5,
        start_t_us: 2_000,
        last_update_t_us: 2_500,
        last_phase: 1,
    }];
    closure.action_consequence.n_windows = 1;
    closure.action_consequence.windows = vec![SnapshotConsequenceWindow {
        window_id: 7,
        modality_kind: 0,
        start_t_us: 2_800,
        end_t_us: 3_100,
        magnitude: 0.48,
        phase: 1,
        last_update_t_us: 3_100,
    }];
    closure.action_consequence.n_pairs = 1;
    closure.action_consequence.total_pairs = 1;
    closure.action_consequence.pairs = vec![SnapshotConsequencePair {
        pair_seq: 1,
        write_seq: 1,
        window_id: 7,
        delay_us: 800,
        status: 1,
        pair_t_us: 3_200,
        curve_similarity: 0.92,
    }];
    closure.active_reasoning_gap.closure_score = 0.6;
    closure.active_reasoning_gap.reentry_score = 0.4;
    closure.evidence_tension.unresolved_pressure = 0.2;

    let snapshots = [life, vocal, closure];
    let evidence = summarize_live_human_nursery(snapshots.iter());

    assert_eq!(evidence.phot_health_observed, 1);
    assert_eq!(evidence.phot_fault_observed, 0);
    assert_eq!(evidence.vib_activity_observed, 1);
    assert!(evidence.vocal_write_epochs >= 1);
    assert!(evidence.return_windows >= 1);
    assert_eq!(evidence.organic_surface_nonblank, 1);
    assert!(evidence.teacher_phase_count >= 3);
    assert_eq!(evidence.communication_candidate, 1);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn nursery_refuses_communication_candidate_without_write_return_closure() {
    let mut snap = make_empty_snapshot();
    snap.seq = 1;
    snap.phot_health = HEALTH_OK;
    snap.ambient_vib.event_energy = 0.3;
    snap.ambient_vib.candidate_flag = true;
    snap.slots.push(active_phot_slot());

    let snapshots = [snap];
    let evidence = summarize_live_human_nursery(snapshots.iter());

    assert_eq!(evidence.communication_candidate, 0);
    assert_eq!(evidence.pairing_or_unpaired_boundary_observed, 1);
    assert_eq!(evidence.principle_guard_pass, 1);
}

#[test]
fn nursery_treats_phot_fault_as_observable_health_not_silent_gap() {
    let mut snap = make_empty_snapshot();
    snap.seq = 1;
    snap.phot_health = HEALTH_FAULT;

    let snapshots = [snap];
    let evidence = summarize_live_human_nursery(snapshots.iter());

    assert_eq!(evidence.phot_health_observed, 1);
    assert_eq!(evidence.phot_fault_observed, 1);
    assert_eq!(evidence.pairing_or_unpaired_boundary_observed, 0);
    assert_eq!(evidence.communication_candidate, 0);
}

#[test]
fn live_human_nursery_sources_do_not_introduce_fixed_runtime_ontology() {
    let sources = [
        include_str!("../src/live_human_nursery.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
    ];

    for src in sources {
        for forbidden in [
            "teacher_module",
            "chat",
            "semantic",
            "concept",
            "token",
            "label",
            "word_meaning",
            "identity",
            "personality",
            "owner",
            "human_voice",
            "human_face",
            "speech_recognition",
            "image_recognition",
        ] {
            assert!(
                !src.contains(forbidden),
                "live human nursery runtime source must not contain `{}`",
                forbidden
            );
        }
    }
}
