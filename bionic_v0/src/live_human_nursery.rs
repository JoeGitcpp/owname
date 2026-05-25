use crate::active_reasoning::ActiveReasoningGapState;
use crate::adaptive_stability::AdaptiveStabilityState;
use crate::code_ingress::CodeIngressState;
use crate::evidence_tension::EvidenceTensionState;
use crate::language_binding::LanguageBindingState;
use crate::learned_bias::LearnedBiasSummary;
use crate::live_io_evidence::summarize_live_io_evidence;
use crate::self_development::SelfDevelopmentState;
use crate::snapshot::{
    SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub, SnapshotConsequencePair,
    SnapshotConsequenceWindow, SnapshotData, SnapshotFramedByteEgress, SnapshotLastClaim,
    SnapshotProbeLaw, SnapshotSlot, SnapshotSpatial, SnapshotVocalMemory, SnapshotWriteEpochRecord,
};
use crate::surface_projection::project_surface_frame;
use crate::viewport::{
    GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState, GlobalRelationState,
};
use crate::vocal_imitation::VocalImitationState;
use crate::vocal_organ::{VocalOrganPhase, VocalOrganState};
use crate::vocal_target::VocalTargetState;
use crate::vpc::VpcContractState;
use crate::{HEALTH_FAULT, HEALTH_OK, MOD_PHOT};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveHumanNurseryEvidence {
    pub snapshots_seen: u32,
    pub phot_health_observed: u8,
    pub phot_fault_observed: u8,
    pub vib_activity_observed: u8,
    pub phot_structure_observed: u8,
    pub vocal_activity_observed: u8,
    pub body_write_out_observed: u8,
    pub vocal_write_epochs: u32,
    pub return_windows: u32,
    pub temporal_pairing_observed: u8,
    pub pairing_or_unpaired_boundary_observed: u8,
    pub closure_or_reentry_observed: u8,
    pub organic_surface_nonblank: u8,
    pub teacher_phase_count: u8,
    pub communication_candidate: u8,
    pub principle_guard_pass: u8,
}

pub fn summarize_live_human_nursery<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
) -> LiveHumanNurseryEvidence {
    let mut evidence = LiveHumanNurseryEvidence::default();
    let mut latest_pair_similarity = 0.0_f64;
    let mut pressure_visible = false;

    for snap in snapshots {
        evidence.snapshots_seen = evidence.snapshots_seen.saturating_add(1);

        evidence.phot_health_observed = 1;
        if snap.phot_health == HEALTH_FAULT {
            evidence.phot_fault_observed = 1;
        }

        let vib_seen = snap.ambient_vib.event_energy > 0.0
            || snap.ambient_vib.candidate_flag
            || snap.last_obs_t_us > 0;
        if vib_seen {
            evidence.vib_activity_observed = 1;
        }

        let phot_structure_seen = snap.ambient_phot.event_energy > 0.0
            || snap.slots.iter().any(|slot| {
                slot.active
                    && (slot.phot.is_some()
                        || slot
                            .spatial
                            .map(|sp| sp.valid && sp.precision > 0.0)
                            .unwrap_or(false))
            });
        if phot_structure_seen {
            evidence.phot_structure_observed = 1;
        }

        let vocal_seen = snap.vocal_target.active_energy > 0.0
            || snap.vocal_organ.breath_drive > 0.0
            || snap.vocal_organ.phase == VocalOrganPhase::Active;
        if vocal_seen {
            evidence.vocal_activity_observed = 1;
        }

        let live_io = summarize_live_io_evidence(snap);
        if live_io.body_write_out_exists {
            evidence.body_write_out_observed = 1;
        }
        evidence.vocal_write_epochs = evidence
            .vocal_write_epochs
            .max(snap.action_output.n_write_epochs);
        evidence.return_windows = evidence
            .return_windows
            .max(snap.action_consequence.n_windows);
        if live_io.temporal_pairing_exists {
            evidence.temporal_pairing_observed = 1;
        }
        latest_pair_similarity = latest_pair_similarity.max(live_io.latest_curve_similarity);

        let closure_or_reentry = snap.active_reasoning_gap.closure_score > 0.0
            || snap.active_reasoning_gap.reentry_score > 0.0;
        if closure_or_reentry {
            evidence.closure_or_reentry_observed = 1;
        }

        let surface = project_surface_frame(snap);
        let organ = surface.organ_drive;
        if !surface.objects.is_empty()
            || !surface.relations.is_empty()
            || organ.regulated_activation > 0.0
            || organ.residue_drive > 0.0
            || organ.reentry_drive > 0.0
        {
            evidence.organic_surface_nonblank = 1;
        }

        if snap.evidence_tension.unresolved_pressure > 0.0
            || snap.global_reasoning.network_instability_score > 0.0
            || latest_pair_similarity < 0.999 && latest_pair_similarity > 0.0
        {
            pressure_visible = true;
        }
    }

    let any_activity = evidence.vib_activity_observed == 1
        || evidence.phot_structure_observed == 1
        || evidence.vocal_activity_observed == 1
        || evidence.body_write_out_observed == 1
        || evidence.return_windows > 0;
    if evidence.temporal_pairing_observed == 1 || any_activity {
        evidence.pairing_or_unpaired_boundary_observed = 1;
    }

    evidence.teacher_phase_count = observed_phase_count(&evidence);

    let candidate_gates_pass = evidence.body_write_out_observed == 1
        && evidence.vocal_write_epochs > 0
        && evidence.return_windows > 0
        && evidence.temporal_pairing_observed == 1
        && evidence.closure_or_reentry_observed == 1
        && evidence.pairing_or_unpaired_boundary_observed == 1
        && (pressure_visible || latest_pair_similarity > 0.0);
    if candidate_gates_pass {
        evidence.communication_candidate = 1;
    }

    if evidence.snapshots_seen > 0
        && (evidence.communication_candidate == 0 || candidate_gates_pass)
        && evidence.pairing_or_unpaired_boundary_observed == 1
    {
        evidence.principle_guard_pass = 1;
    }

    evidence
}

fn observed_phase_count(evidence: &LiveHumanNurseryEvidence) -> u8 {
    let phase_0 = evidence.phot_health_observed == 1 || evidence.vib_activity_observed == 1;
    let phase_1 = evidence.vocal_activity_observed == 1 || evidence.vocal_write_epochs > 0;
    let phase_2 = evidence.phot_structure_observed == 1 && evidence.vib_activity_observed == 1;
    let phase_3 = evidence.body_write_out_observed == 1
        && (evidence.return_windows > 0 || evidence.closure_or_reentry_observed == 1);
    let phase_4 = evidence.communication_candidate == 1;

    [phase_0, phase_1, phase_2, phase_3, phase_4]
        .into_iter()
        .filter(|observed| *observed)
        .count() as u8
}

pub fn sample_live_human_nursery_snapshots() -> Vec<SnapshotData> {
    let mut life = empty_snapshot();
    life.seq = 1;
    life.phot_health = HEALTH_OK;
    life.phot_started_t_us = 1_000;
    life.phot_last_frame_t_us = 1_300;
    life.ambient_vib.event_energy = 0.18;
    life.ambient_vib.candidate_flag = true;
    life.ambient_phot.event_energy = 0.24;
    life.slots.push(active_phot_slot());

    let mut vocal = empty_snapshot();
    vocal.seq = 2;
    vocal.phot_health = HEALTH_OK;
    vocal.ambient_vib.event_energy = 0.22;
    vocal.vocal_target.active_energy = 0.5;
    vocal.vocal_organ = active_vocal_organ();
    vocal.action_output.n_write_epochs = 1;
    vocal.action_output.write_epoch_history = vec![write_epoch()];

    let mut closure = empty_snapshot();
    closure.seq = 3;
    closure.phot_health = HEALTH_OK;
    closure.ambient_vib.event_energy = 0.22;
    closure.vocal_target.active_energy = 0.5;
    closure.vocal_organ = active_vocal_organ();
    closure.action_output.n_write_epochs = 1;
    closure.action_output.write_epoch_history = vec![write_epoch()];
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

    vec![life, vocal, closure]
}

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

fn active_vocal_organ() -> VocalOrganState {
    VocalOrganState {
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
    }
}

fn write_epoch() -> SnapshotWriteEpochRecord {
    SnapshotWriteEpochRecord {
        write_seq: 1,
        channel_kind: 0,
        start_magnitude: 0.5,
        start_t_us: 2_000,
        last_update_t_us: 2_500,
        last_phase: 1,
    }
}
