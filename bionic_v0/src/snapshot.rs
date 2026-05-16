use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::action_consequence::ActionConsequenceState;
use crate::action_output::ActionOutputState;
use crate::active_reasoning::{ActiveReasoningGapState, ActiveReasoningGapTrace};
use crate::adaptive_stability::{AdaptiveRouteTrace, AdaptiveStabilityState};
use crate::code_ingress::{
    CodeIngressFrameTrace, CodeIngressState, CodeProjectionCandidate, CodeSequenceWindowTrace,
};
use crate::evidence_tension::{EvidenceTensionState, EvidenceTensionTrace};
use crate::framed_byte_egress::{FramedByteEgressPhase, FramedByteEgressState};
use crate::language_binding::{
    BindingStatus, BindingTargetRef, BindingTrace, LanguageBindingState, PatternKind,
};
use crate::learned_bias::LearnedBiasSummary;
use crate::object_high_layer::ObjectHighLayerState;
use crate::probe_law::ProbeLawState;
use crate::viewport::{
    AmbientModalitySnapshot, AxisRing, BandFamilyState, CrossModalFusionState,
    GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState, GlobalRelationState,
    RelationEdgeState, SpatialState, SpatialWorldlineState, Viewport,
};
use crate::vocal_imitation::VocalImitationState;
use crate::vocal_memory::VocalMemoryState;
use crate::vocal_organ::VocalOrganState;
use crate::vocal_target::VocalTargetState;
use crate::vpc::{
    ErrAlignmentKind, HorizonKind, ObservationDomainKind, ObservationDomainRef, PredictionHorizon,
    PredictionPublicationTrace, PredictorLifecycleState, PublicationStatus, VpcContractState,
};
use crate::{
    AXIS_COUNT, CODE_INGRESS_BYTE_SAMPLE_CAP, EVIDENCE_TENSION_HISTORY_CAP, HISTORY_CAP,
    N_OBJECT_SLOTS, PROBE_HISTORY_CAP, SNAPSHOT_MAGIC, SNAPSHOT_VERSION,
};
use crate::{N_REL_EDGES, N_ROUTING_FOCUS_EDGES, N_ROUTING_FOCUS_SLOTS};

pub struct SnapshotPaths {
    pub final_path: PathBuf,
    pub tmp_path: PathBuf,
}

impl SnapshotPaths {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let d = dir.as_ref().to_path_buf();
        Self {
            final_path: d.join("snapshot.bin"),
            tmp_path: d.join("snapshot.bin.tmp"),
        }
    }
}

pub fn write_snapshot(vp: &Arc<Viewport>, paths: &SnapshotPaths) -> std::io::Result<()> {
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);

    buf.extend_from_slice(&SNAPSHOT_MAGIC.to_le_bytes());
    buf.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
    let seq = vp.snapshot_writes.fetch_add(1, Ordering::Relaxed) + 1;
    buf.extend_from_slice(&seq.to_le_bytes());
    buf.extend_from_slice(&vp.master_tick.load(Ordering::Relaxed).to_le_bytes());
    buf.extend_from_slice(&vp.clock_us.load(Ordering::Relaxed).to_le_bytes());
    buf.extend_from_slice(&vp.start_us.load(Ordering::Relaxed).to_le_bytes());
    buf.extend_from_slice(&vp.source_writes.load(Ordering::Relaxed).to_le_bytes());
    buf.extend_from_slice(&vp.predictor_ticks.load(Ordering::Relaxed).to_le_bytes());
    buf.extend_from_slice(&vp.snap_fail_count().to_le_bytes());
    buf.extend_from_slice(&(HISTORY_CAP as u32).to_le_bytes());
    buf.extend_from_slice(&(AXIS_COUNT as u32).to_le_bytes());

    // v3: hearing fault state
    let (fault, fault_count) = vp.hearing_status();
    buf.extend_from_slice(&(fault as u64).to_le_bytes());
    buf.extend_from_slice(&fault_count.to_le_bytes());

    // v3: high-level predictor state
    let (hl_a, hl_avg_mse, hl_pred_mse, hl_meta_err, hl_mod) = vp.read_hl_state();
    buf.extend_from_slice(&hl_a.to_le_bytes());
    buf.extend_from_slice(&hl_avg_mse.to_le_bytes());
    buf.extend_from_slice(&hl_pred_mse.to_le_bytes());
    buf.extend_from_slice(&hl_meta_err.to_le_bytes());
    buf.extend_from_slice(&hl_mod.to_le_bytes());

    // v4: last observation timestamp (for hearing health QUIET/OK window)
    buf.extend_from_slice(&vp.read_last_obs_t_us().to_le_bytes());

    // v9: first-class health state as a single numeric byte-wide field
    // (stored u64 for alignment with other substrate state words).
    buf.extend_from_slice(&(vp.read_health_state().as_u8() as u64).to_le_bytes());

    // v11: PHOT ingress health — independent of the VIB-biased health_state
    // above. Numeric-only; observer renders, never recomputes.
    buf.extend_from_slice(&(vp.read_phot_health().as_u8() as u64).to_le_bytes());

    // v13: PHOT startup and last-frame timestamps (Phase 4C.1).
    buf.extend_from_slice(&vp.read_phot_started_t_us().to_le_bytes());
    buf.extend_from_slice(&vp.read_phot_last_frame_t_us().to_le_bytes());

    for axis in 0..AXIS_COUNT {
        let (a, b, surprise, last_err, mse, updates) = vp.read_predictor(axis);
        buf.extend_from_slice(&a.to_le_bytes());
        buf.extend_from_slice(&b.to_le_bytes());
        buf.extend_from_slice(&surprise.to_le_bytes());
        buf.extend_from_slice(&last_err.to_le_bytes());
        buf.extend_from_slice(&mse.to_le_bytes());
        buf.extend_from_slice(&updates.to_le_bytes());

        write_ring(&mut buf, &vp.observed[axis].read().unwrap());
        write_ring(&mut buf, &vp.predicted[axis].read().unwrap());
        write_ring(&mut buf, &vp.err_stream[axis].read().unwrap());
    }

    // v7: AmbientChannel snapshot — shared nomination_seq + per-modality channels
    let amb = vp.read_ambient();
    buf.extend_from_slice(&amb.nomination_seq.to_le_bytes());
    write_ambient_sub(&mut buf, amb.vib);
    write_ambient_sub(&mut buf, amb.phot);
    write_ambient_sub(&mut buf, amb.mech);

    // v6: ObjectSlot array (fixed N_OBJECT_SLOTS, per-modality band families).
    //
    // Per-slot layout:
    //   active:u64, last_update_t_us:u64, source_modality:u64, nomination_seq:u64,
    //   vib_family_block, phot_family_block, mech_family_block, has_spatial:u8
    //
    // Family block:
    //   present:u8, [ n_bands:u16, observed*n_bands, predicted*n_bands,
    //                 err*n_bands, surprise*n_bands ] if present
    buf.extend_from_slice(&(N_OBJECT_SLOTS as u32).to_le_bytes());
    for slot_id in 0..N_OBJECT_SLOTS {
        let snap = vp.read_slot(slot_id).unwrap_or_default();
        buf.extend_from_slice(&(snap.active as u64).to_le_bytes());
        buf.extend_from_slice(&snap.last_update_t_us.to_le_bytes());
        buf.extend_from_slice(&(snap.source_modality as u64).to_le_bytes());
        buf.extend_from_slice(&snap.nomination_seq.to_le_bytes());
        write_family(&mut buf, snap.vib.as_ref());
        write_family(&mut buf, snap.phot.as_ref());
        write_family(&mut buf, snap.mech.as_ref());
        write_spatial(&mut buf, snap.spatial);
        write_spatial_worldline(&mut buf, snap.spatial_worldline);
        write_fusion(&mut buf, snap.fusion);
        write_high_layer(&mut buf, snap.high_layer);
        buf.extend_from_slice(&snap.attention_share.to_le_bytes());
        buf.extend_from_slice(&snap.precision_bias.to_le_bytes());
    }

    // v10: LastClaimDebug — numeric-only, observer-facing. Written on every
    // claim_slot_for_event. Observers render; they never recompute scores.
    let d = vp.read_last_claim_debug();
    buf.extend_from_slice(&d.seq.to_le_bytes());
    buf.extend_from_slice(&d.event_t_us.to_le_bytes());
    buf.extend_from_slice(&(d.modality_bit as u64).to_le_bytes());
    buf.extend_from_slice(&(d.kind as u64).to_le_bytes());
    buf.extend_from_slice(&(d.chosen_slot as i64).to_le_bytes());
    buf.extend_from_slice(&d.best_score.to_le_bytes());
    buf.extend_from_slice(&d.best_time_score.to_le_bytes());
    buf.extend_from_slice(&d.best_spatial_score.to_le_bytes());
    buf.extend_from_slice(&d.best_modality_score.to_le_bytes());
    buf.extend_from_slice(&d.best_energy_score.to_le_bytes());
    buf.extend_from_slice(&d.best_surprise_score.to_le_bytes());
    buf.extend_from_slice(&(d.n_candidates_considered as u64).to_le_bytes());
    buf.extend_from_slice(&d.min_reuse_score.to_le_bytes());

    // v16: GlobalAttentionState
    write_global_attention(&mut buf, &vp.read_global_attention());

    // v17: GlobalRelationState
    write_global_relations(&mut buf, &vp.read_global_relations());

    // v19: GlobalReasoningState
    write_global_reasoning(&mut buf, &vp.read_global_reasoning());

    // v20: GlobalReasoningMemorySummary (Phase 5G)
    write_global_reasoning_memory_summary(&mut buf, &vp.read_global_reasoning_memory_summary());

    // v24: language-binding trace substrate
    write_language_binding(&mut buf, &vp.read_language_binding());

    // v33: VPC timing / provenance trace substrate
    write_vpc_contract(&mut buf, &vp.read_vpc_contract());

    // v25: action-output substrate (write-out side, 2026-04-21)
    write_action_output(&mut buf, &vp.read_action_output());

    // v26: action-consequence loop substrate (2026-04-21)
    write_action_consequence(&mut buf, &vp.read_action_consequence());

    // v30: probe-law substrate
    write_probe_law(&mut buf, &vp.read_probe_law());

    // v36: vocal substrate (vocal_target, vocal_organ, vocal_imitation, vocal_memory)
    write_vocal_target(&mut buf, &vp.read_vocal_target());
    write_vocal_organ(&mut buf, &vp.read_vocal_organ());
    write_vocal_imitation(&mut buf, &vp.read_vocal_imitation());
    write_vocal_memory(&mut buf, &vp.read_vocal_memory());

    // v37: code-ingress trace substrate.
    write_code_ingress(&mut buf, &vp.read_code_ingress());

    // v38: learned-bias delta summary. Numeric-only observer evidence; base
    // priors remain separate and are never serialized as learned state here.
    write_learned_bias_summary(&mut buf, &vp.read_learned_bias().summary());

    // v39: adaptive stability route-local numeric traces.
    write_adaptive_stability(&mut buf, &vp.read_adaptive_stability());

    // v40: evidence-tension pressure trace substrate.
    write_evidence_tension(&mut buf, &vp.read_evidence_tension());

    // v41: framed-byte egress substrate.
    write_framed_byte_egress(&mut buf, &vp.read_framed_byte_egress());

    // v46: active reasoning gap observer trace.
    write_active_reasoning_gap(&mut buf, &vp.read_active_reasoning_gap());

    // Rejected v21/v22 naming sections were removed by cleanup spec
    // 2026-04-20. Snapshot terminates at the reasoning memory summary
    // plus language-binding traces.

    {
        let mut f = File::create(&paths.tmp_path)?;
        f.write_all(&buf)?;
        f.sync_data()?;
    }
    std::fs::rename(&paths.tmp_path, &paths.final_path)?;
    Ok(())
}

fn write_spatial(buf: &mut Vec<u8>, sp: Option<SpatialState>) {
    match sp {
        None => buf.push(0u8),
        Some(s) => {
            buf.push(1u8);
            buf.extend_from_slice(&s.pos_x.to_le_bytes());
            buf.extend_from_slice(&s.pos_y.to_le_bytes());
            buf.extend_from_slice(&s.span_x.to_le_bytes());
            buf.extend_from_slice(&s.span_y.to_le_bytes());
            buf.extend_from_slice(&s.precision.to_le_bytes());
            buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
            buf.push(if s.valid { 1u8 } else { 0u8 });
        }
    }
}

fn write_spatial_worldline(buf: &mut Vec<u8>, wl: Option<SpatialWorldlineState>) {
    match wl {
        None => buf.push(0u8),
        Some(s) => {
            buf.push(1u8);
            buf.extend_from_slice(&(s.n_updates as u64).to_le_bytes());
            buf.extend_from_slice(&s.prev_t_us.to_le_bytes());
            buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
            buf.extend_from_slice(&s.prev_pos_x.to_le_bytes());
            buf.extend_from_slice(&s.prev_pos_y.to_le_bytes());
            buf.extend_from_slice(&s.predicted_pos_x.to_le_bytes());
            buf.extend_from_slice(&s.predicted_pos_y.to_le_bytes());
            buf.extend_from_slice(&s.velocity_x_per_s.to_le_bytes());
            buf.extend_from_slice(&s.velocity_y_per_s.to_le_bytes());
            buf.extend_from_slice(&s.err_x.to_le_bytes());
            buf.extend_from_slice(&s.err_y.to_le_bytes());
            buf.extend_from_slice(&s.err_norm.to_le_bytes());
            buf.extend_from_slice(&s.last_dt_us.to_le_bytes());
            buf.push(if s.valid { 1u8 } else { 0u8 });
        }
    }
}

fn write_fusion(buf: &mut Vec<u8>, f: Option<CrossModalFusionState>) {
    match f {
        None => buf.push(0u8),
        Some(s) => {
            buf.push(1u8);
            buf.extend_from_slice(&(s.modality_mask as u64).to_le_bytes());
            buf.extend_from_slice(&s.last_fused_t_us.to_le_bytes());
            buf.extend_from_slice(&s.last_vib_t_us.to_le_bytes());
            buf.extend_from_slice(&s.last_phot_t_us.to_le_bytes());
            buf.extend_from_slice(&s.temporal_alignment_score.to_le_bytes());
            buf.extend_from_slice(&s.cross_modal_confidence.to_le_bytes());
            buf.extend_from_slice(&s.vib_weight.to_le_bytes());
            buf.extend_from_slice(&s.phot_weight.to_le_bytes());
            buf.extend_from_slice(&s.fused_surprise.to_le_bytes());
            buf.push(if s.is_cross_modal { 1u8 } else { 0u8 });
        }
    }
}

fn write_high_layer(buf: &mut Vec<u8>, hl: Option<ObjectHighLayerState>) {
    match hl {
        None => buf.push(0u8),
        Some(s) => {
            buf.push(1u8);
            buf.extend_from_slice(&s.predicted_confidence.to_le_bytes());
            buf.extend_from_slice(&s.predicted_alignment.to_le_bytes());
            buf.extend_from_slice(&s.predicted_surprise.to_le_bytes());
            buf.extend_from_slice(&s.object_err.to_le_bytes());
            buf.extend_from_slice(&s.attention_bias.to_le_bytes());
            buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
        }
    }
}

fn write_ambient_sub(buf: &mut Vec<u8>, sub: AmbientModalitySnapshot) {
    buf.extend_from_slice(&sub.event_energy.to_le_bytes());
    buf.extend_from_slice(&sub.surprise.to_le_bytes());
    buf.extend_from_slice(&sub.last_event_t_us.to_le_bytes());
    buf.extend_from_slice(&(sub.candidate_flag as u64).to_le_bytes());
}

fn write_family(buf: &mut Vec<u8>, fam: Option<&BandFamilyState>) {
    match fam {
        None => buf.push(0u8),
        Some(f) => {
            buf.push(1u8);
            buf.extend_from_slice(&f.n_bands.to_le_bytes());
            for v in &f.observed {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            for v in &f.predicted {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            for v in &f.err {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            for v in &f.surprise {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
}

fn write_ring(buf: &mut Vec<u8>, ring: &AxisRing) {
    buf.extend_from_slice(&(ring.write_idx as u32).to_le_bytes());
    buf.extend_from_slice(&ring.total_written.to_le_bytes());
    for s in ring.buf.iter() {
        buf.extend_from_slice(&s.t_us.to_le_bytes());
        buf.extend_from_slice(&s.value.to_le_bytes());
        buf.extend_from_slice(&(if s.valid { 1u8 } else { 0u8 }).to_le_bytes());
        buf.extend_from_slice(&[0u8; 7]);
    }
}

pub struct SnapshotAxis {
    pub a: f64,
    pub b: f64,
    pub surprise: f64,
    pub last_err: f64,
    pub mse: f64,
    pub updates: u64,
    pub observed: Vec<(u64, f64)>,
    pub predicted: Vec<(u64, f64)>,
    pub err: Vec<(u64, f64)>,
}

pub struct SnapshotBandFamily {
    pub n_bands: u16,
    pub observed: Vec<f64>,
    pub predicted: Vec<f64>,
    pub err: Vec<f64>,
    pub surprise: Vec<f64>,
}

pub struct SnapshotSlot {
    pub active: bool,
    pub last_update_t_us: u64,
    pub source_modality: u8,
    pub nomination_seq: u64,
    pub vib: Option<SnapshotBandFamily>,
    pub phot: Option<SnapshotBandFamily>,
    pub mech: Option<SnapshotBandFamily>,
    pub spatial: Option<SnapshotSpatial>,
    pub spatial_worldline: Option<SpatialWorldlineState>,
    /// Phase 5A: cross-modal fusion state. None when never refreshed.
    pub fusion: Option<CrossModalFusionState>,
    /// Phase 5B: high-layer output state. None when never refreshed.
    pub high_layer: Option<ObjectHighLayerState>,
    /// Phase 5C: normalised attention share suggestion (0.0 if not yet allocated).
    pub attention_share: f64,
    /// Phase 5C: precision bias suggestion (2 * share, clamped [0,2]).
    pub precision_bias: f64,
}

#[derive(Clone, Copy, Default)]
pub struct SnapshotSpatial {
    pub pos_x: f64,
    pub pos_y: f64,
    pub span_x: f64,
    pub span_y: f64,
    pub precision: f64,
    pub last_update_t_us: u64,
    pub valid: bool,
}

pub struct SnapshotData {
    pub seq: u64,
    pub master_tick: u64,
    pub clock_us: u64,
    pub start_us: u64,
    pub source_writes: u64,
    pub predictor_ticks: u64,
    pub snapshot_fail_count: u64,
    pub history_cap: u32,
    pub axis_count: u32,
    pub hearing_fault: bool,
    pub hearing_fault_count: u64,
    pub hl_a: f64,
    pub hl_avg_mse: f64,
    pub hl_pred_mse: f64,
    pub hl_meta_err: f64,
    pub hl_modulation: f64,
    pub last_obs_t_us: u64,
    pub health_state: u8,
    pub phot_health: u8,
    /// Phase 4C.1: PHOT provider startup time (µs); 0 = not yet started.
    pub phot_started_t_us: u64,
    /// Phase 4C.1: wall-clock time (µs) of last received frame; 0 = none.
    pub phot_last_frame_t_us: u64,
    pub axes: Vec<SnapshotAxis>,
    // v7: object layer — per-modality ambient + slots
    pub ambient_nomination_seq: u64,
    pub ambient_vib: SnapshotAmbientSub,
    pub ambient_phot: SnapshotAmbientSub,
    pub ambient_mech: SnapshotAmbientSub,
    pub slots: Vec<SnapshotSlot>,
    // v10: last claim debug (numeric-only, observer-facing)
    pub last_claim: SnapshotLastClaim,
    // v16: global attention allocation state
    pub global_attention: GlobalAttentionState,
    // v17: cross-object temporal relation substrate
    pub global_relations: GlobalRelationState,
    // v19: cross-object reasoning / routing substrate
    pub global_reasoning: GlobalReasoningState,
    // v20: temporal memory summary for reasoning
    pub global_reasoning_memory: GlobalReasoningMemorySummary,
    // v24: trace-only language binding state
    pub language_binding: LanguageBindingState,
    // v33: VPC timing / provenance trace state
    pub vpc_contract: VpcContractState,
    // Rejected v21/v22 naming sections removed by cleanup spec 2026-04-20.
    // v25: action-output substrate (write-out side)
    pub action_output: SnapshotActionOutput,
    // v26: action-consequence loop substrate
    pub action_consequence: SnapshotActionConsequence,
    // v30: probe-law substrate (numeric/time-only)
    pub probe_law: SnapshotProbeLaw,
    // v36: vocal substrate (vocal_target, vocal_organ, vocal_imitation, vocal_memory)
    pub vocal_target: VocalTargetState,
    pub vocal_organ: VocalOrganState,
    pub vocal_imitation: VocalImitationState,
    pub vocal_memory: SnapshotVocalMemory,
    // v37: code-ingress trace substrate
    pub code_ingress: CodeIngressState,
    // v38: learned-bias delta summary
    pub learned_bias: LearnedBiasSummary,
    // v39: adaptive stability route-local numeric traces.
    pub adaptive_stability: AdaptiveStabilityState,
    // v40: evidence-tension pressure trace substrate.
    pub evidence_tension: EvidenceTensionState,
    // v41: framed-byte egress substrate.
    pub framed_byte_egress: SnapshotFramedByteEgress,
    // v46: active reasoning gap observer trace.
    pub active_reasoning_gap: ActiveReasoningGapState,
}

impl SnapshotData {
    pub fn vocal_gap_trace(&self) -> (f64, f64, u32, u32) {
        let s = self.vocal_imitation;
        (
            s.imitation_pressure,
            s.last_gap,
            s.converge_count,
            s.fail_count,
        )
    }
}

/// Snapshot-visible representation of the action-output substrate (v25/v26/v27).
/// Numeric-only; no world content; no language gate.
#[derive(Clone, Debug, Default)]
pub struct SnapshotActionOutput {
    pub n_channels: u32,
    pub channels: Vec<SnapshotActionOutputChannel>,
    /// v27: minimal write-epoch ledger — one entry per Begin event.
    /// Numeric/time-only; no target, result, or meaning fields.
    pub n_write_epochs: u32,
    pub write_epoch_history: Vec<SnapshotWriteEpochRecord>,
}

/// Snapshot-visible write epoch record (v27).
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotWriteEpochRecord {
    pub write_seq: u64,
    pub channel_kind: u64,
    pub start_magnitude: f64,
    pub start_t_us: u64,
    pub last_update_t_us: u64,
    pub last_phase: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotActionOutputChannel {
    pub channel_kind: u64,
    pub phase: u64,
    pub current_magnitude: f64,
    pub peak_magnitude: f64,
    pub start_t_us: u64,
    pub last_update_t_us: u64,
    pub last_transition_t_us: u64,
    pub reinforce_count: u32,
    pub decay_count: u32,
    /// v26: monotonic write-epoch pairing handle.
    pub current_write_seq: u64,
}

/// Snapshot-visible representation of the action-consequence substrate (v26).
/// Numeric/time-only; no target, result, or imitation fields.
#[derive(Clone, Debug, Default)]
pub struct SnapshotActionConsequence {
    pub n_windows: u32,
    pub windows: Vec<SnapshotConsequenceWindow>,
    pub n_pairs: u32,
    pub total_pairs: u64,
    pub total_framed_byte_pairs: u64,
    pub total_other_pairs: u64,
    pub pairs: Vec<SnapshotConsequencePair>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotConsequenceWindow {
    /// v29: stable monotonic window identifier.
    pub window_id: u64,
    pub modality_kind: u64,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
    /// v28: lifecycle phase; 0 = Open, 1 = Closed.
    pub phase: u64,
    /// v28: substrate time of last phase or boundary update.
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotConsequencePair {
    /// v42: monotonic pair sequence for downstream observer cursors.
    pub pair_seq: u64,
    pub write_seq: u64,
    /// v29: stable window identifier (replaces window_index Vec position).
    pub window_id: u64,
    pub delay_us: u64,
    pub status: u64,
    pub pair_t_us: u64,
    /// v31: numeric similarity between write and consequence magnitudes.
    pub curve_similarity: f64,
}

/// Snapshot-visible representation of the probe-law substrate (v30).
/// Numeric/time-only; no target, result, or semantic fields.
#[derive(Clone, Debug, Default)]
pub struct SnapshotProbeLaw {
    pub intensity: f64,
    pub suppression: f64,
    pub phase: u8,
    pub output_intensity: f64,
    pub last_update_t_us: u64,
    pub history_len: u8,
    pub history: [f64; PROBE_HISTORY_CAP],
}

/// Snapshot-visible representation of vocal memory (v36).
/// Numeric-only; no text, no TTS, no token ontology.
#[derive(Clone, Debug, Default)]
pub struct SnapshotVocalMemory {
    pub n_prototypes: u32,
    pub prototypes: Vec<SnapshotVocalPrototype>,
    pub last_update_t_us: u64,
}

/// Snapshot-visible framed-byte egress trace (v37).
/// Numeric/time/byte-shape only; no meaning or controller fields.
#[derive(Clone, Debug, Default)]
pub struct SnapshotFramedByteEgress {
    pub phase: u8,
    pub active_stream_id: u64,
    pub current_write_seq: u64,
    pub byte_cursor: u64,
    pub n_frames: u32,
    pub n_returns: u32,
    pub total_frames: u64,
    pub total_returns: u64,
    pub last_frame_seq: u64,
    pub last_return_window_id: u64,
    pub output_pressure: f64,
    pub projection_confidence: f64,
    pub correction_deviation: f64,
    pub correction_count: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotVocalPrototype {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub stability: f64,
    pub reuse_success: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Default)]
pub struct SnapshotLastClaim {
    pub seq: u64,
    pub event_t_us: u64,
    pub modality_bit: u8,
    pub kind: u8,
    pub chosen_slot: i16,
    pub best_score: f64,
    pub best_time_score: f64,
    pub best_spatial_score: f64,
    pub best_modality_score: f64,
    pub best_energy_score: f64,
    pub best_surprise_score: f64,
    pub n_candidates_considered: u8,
    pub min_reuse_score: f64,
}

#[derive(Clone, Copy, Default)]
pub struct SnapshotAmbientSub {
    pub event_energy: f64,
    pub surprise: f64,
    pub last_event_t_us: u64,
    pub candidate_flag: bool,
}

pub fn read_snapshot(path: &Path) -> std::io::Result<SnapshotData> {
    let bytes = std::fs::read(path)?;
    let mut cur = 0usize;

    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_u32(b: &[u8], c: &mut usize) -> u32 {
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        u32::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    let magic = rd_u64(&bytes, &mut cur);
    if magic != SNAPSHOT_MAGIC {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let version = rd_u64(&bytes, &mut cur);
    if version != SNAPSHOT_VERSION {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let seq = rd_u64(&bytes, &mut cur);
    let master_tick = rd_u64(&bytes, &mut cur);
    let clock_us = rd_u64(&bytes, &mut cur);
    let start_us = rd_u64(&bytes, &mut cur);
    let source_writes = rd_u64(&bytes, &mut cur);
    let predictor_ticks = rd_u64(&bytes, &mut cur);
    let snapshot_fail_count = rd_u64(&bytes, &mut cur);
    let history_cap = rd_u32(&bytes, &mut cur);
    let axis_count = rd_u32(&bytes, &mut cur);

    // v3 fields
    let hearing_fault = rd_u64(&bytes, &mut cur) != 0;
    let hearing_fault_count = rd_u64(&bytes, &mut cur);
    let hl_a = rd_f64(&bytes, &mut cur);
    let hl_avg_mse = rd_f64(&bytes, &mut cur);
    let hl_pred_mse = rd_f64(&bytes, &mut cur);
    let hl_meta_err = rd_f64(&bytes, &mut cur);
    let hl_modulation = rd_f64(&bytes, &mut cur);

    // v4 fields
    let last_obs_t_us = rd_u64(&bytes, &mut cur);

    // v9: first-class health state
    let health_state = rd_u64(&bytes, &mut cur) as u8;

    // v11: PHOT ingress health
    let phot_health = rd_u64(&bytes, &mut cur) as u8;

    // v13: PHOT startup and last-frame timestamps (Phase 4C.1).
    let phot_started_t_us = rd_u64(&bytes, &mut cur);
    let phot_last_frame_t_us = rd_u64(&bytes, &mut cur);

    let mut axes = Vec::with_capacity(axis_count as usize);
    for _ in 0..axis_count {
        let a = rd_f64(&bytes, &mut cur);
        let b = rd_f64(&bytes, &mut cur);
        let surprise = rd_f64(&bytes, &mut cur);
        let last_err = rd_f64(&bytes, &mut cur);
        let mse = rd_f64(&bytes, &mut cur);
        let updates = rd_u64(&bytes, &mut cur);

        let observed = read_ring_linear(&bytes, &mut cur, history_cap as usize);
        let predicted = read_ring_linear(&bytes, &mut cur, history_cap as usize);
        let err = read_ring_linear(&bytes, &mut cur, history_cap as usize);

        axes.push(SnapshotAxis {
            a,
            b,
            surprise,
            last_err,
            mse,
            updates,
            observed,
            predicted,
            err,
        });
    }

    // v7 ambient
    let ambient_nomination_seq = rd_u64(&bytes, &mut cur);
    let read_amb_sub = |c: &mut usize| -> SnapshotAmbientSub {
        let event_energy = rd_f64(&bytes, c);
        let surprise = rd_f64(&bytes, c);
        let last_event_t_us = rd_u64(&bytes, c);
        let candidate_flag = rd_u64(&bytes, c) != 0;
        SnapshotAmbientSub {
            event_energy,
            surprise,
            last_event_t_us,
            candidate_flag,
        }
    };
    let ambient_vib = read_amb_sub(&mut cur);
    let ambient_phot = read_amb_sub(&mut cur);
    let ambient_mech = read_amb_sub(&mut cur);

    // v6 slots — per-modality band families
    let n_slots = rd_u32(&bytes, &mut cur) as usize;
    let mut slots = Vec::with_capacity(n_slots);
    for _ in 0..n_slots {
        let active = rd_u64(&bytes, &mut cur) != 0;
        let last_update_t_us = rd_u64(&bytes, &mut cur);
        let source_modality = rd_u64(&bytes, &mut cur) as u8;
        let nomination_seq = rd_u64(&bytes, &mut cur);
        let vib = read_family(&bytes, &mut cur);
        let phot = read_family(&bytes, &mut cur);
        let mech = read_family(&bytes, &mut cur);
        let spatial = read_spatial(&bytes, &mut cur);
        let spatial_worldline = read_spatial_worldline(&bytes, &mut cur);
        let fusion = read_fusion(&bytes, &mut cur);
        let high_layer = read_high_layer(&bytes, &mut cur);
        let attention_share = rd_f64(&bytes, &mut cur);
        let precision_bias = rd_f64(&bytes, &mut cur);
        slots.push(SnapshotSlot {
            active,
            last_update_t_us,
            source_modality,
            nomination_seq,
            vib,
            phot,
            mech,
            spatial,
            spatial_worldline,
            fusion,
            high_layer,
            attention_share,
            precision_bias,
        });
    }

    // v10 last-claim debug
    fn rd_i64(b: &[u8], c: &mut usize) -> i64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        i64::from_le_bytes(a)
    }
    let lc_seq = rd_u64(&bytes, &mut cur);
    let lc_event_t_us = rd_u64(&bytes, &mut cur);
    let lc_modality_bit = rd_u64(&bytes, &mut cur) as u8;
    let lc_kind = rd_u64(&bytes, &mut cur) as u8;
    let lc_chosen_slot = rd_i64(&bytes, &mut cur) as i16;
    let lc_best = rd_f64(&bytes, &mut cur);
    let lc_time = rd_f64(&bytes, &mut cur);
    let lc_spatial = rd_f64(&bytes, &mut cur);
    let lc_modality = rd_f64(&bytes, &mut cur);
    let lc_energy = rd_f64(&bytes, &mut cur);
    let lc_surprise = rd_f64(&bytes, &mut cur);
    let lc_n = rd_u64(&bytes, &mut cur) as u8;
    let lc_min = rd_f64(&bytes, &mut cur);
    let last_claim = SnapshotLastClaim {
        seq: lc_seq,
        event_t_us: lc_event_t_us,
        modality_bit: lc_modality_bit,
        kind: lc_kind,
        chosen_slot: lc_chosen_slot,
        best_score: lc_best,
        best_time_score: lc_time,
        best_spatial_score: lc_spatial,
        best_modality_score: lc_modality,
        best_energy_score: lc_energy,
        best_surprise_score: lc_surprise,
        n_candidates_considered: lc_n,
        min_reuse_score: lc_min,
    };

    // v16: global attention state
    let global_attention = read_global_attention_snapshot(&bytes, &mut cur);

    // v17: global relation state
    let global_relations = read_global_relations_snapshot(&bytes, &mut cur);

    // v19: global reasoning state
    let global_reasoning = read_global_reasoning_snapshot(&bytes, &mut cur);

    // v20: global reasoning memory summary (skip if version < 20)
    let global_reasoning_memory = if version >= 20 {
        read_global_reasoning_memory_summary(&bytes, &mut cur)
    } else {
        GlobalReasoningMemorySummary::default()
    };

    let language_binding = if version >= 24 {
        read_language_binding_snapshot(&bytes, &mut cur)
    } else {
        LanguageBindingState::default()
    };

    let vpc_contract = if version >= 33 {
        read_vpc_contract_snapshot(&bytes, &mut cur)
    } else {
        VpcContractState::default()
    };

    let action_output = if version >= 25 {
        read_action_output_snapshot(&bytes, &mut cur, version)
    } else {
        SnapshotActionOutput::default()
    };

    let action_consequence = if version >= 29 {
        read_action_consequence_snapshot(&bytes, &mut cur, version)
    } else {
        SnapshotActionConsequence::default()
    };

    let probe_law = if version >= 30 {
        read_probe_law_snapshot(&bytes, &mut cur)
    } else {
        SnapshotProbeLaw::default()
    };

    let vocal_target = if version >= 36 {
        read_vocal_target_snapshot(&bytes, &mut cur)
    } else {
        VocalTargetState::default()
    };

    let vocal_organ = if version >= 36 {
        read_vocal_organ_snapshot(&bytes, &mut cur)
    } else {
        VocalOrganState::default()
    };

    let vocal_imitation = if version >= 36 {
        read_vocal_imitation_snapshot(&bytes, &mut cur)
    } else {
        VocalImitationState::default()
    };

    let vocal_memory = if version >= 36 {
        read_vocal_memory_snapshot(&bytes, &mut cur)
    } else {
        SnapshotVocalMemory::default()
    };

    let code_ingress = if version >= 37 {
        read_code_ingress_snapshot(&bytes, &mut cur)
    } else {
        CodeIngressState::default()
    };

    let learned_bias = if version >= 38 {
        read_learned_bias_summary(&bytes, &mut cur)
    } else {
        LearnedBiasSummary::default()
    };

    let adaptive_stability = if version >= 39 {
        read_adaptive_stability_snapshot(&bytes, &mut cur)
    } else {
        AdaptiveStabilityState::default()
    };

    let evidence_tension = if version >= 40 {
        read_evidence_tension_snapshot(&bytes, &mut cur)
    } else {
        EvidenceTensionState::default()
    };

    let framed_byte_egress = if version >= 41 {
        read_framed_byte_egress_snapshot(&bytes, &mut cur)
    } else {
        SnapshotFramedByteEgress::default()
    };

    let active_reasoning_gap = if version >= 46 {
        read_active_reasoning_gap_snapshot(&bytes, &mut cur)
    } else {
        ActiveReasoningGapState::default()
    };

    // Rejected v21/v22 naming sections removed by cleanup spec 2026-04-20.
    // Older snapshot files containing those bytes are no longer readable;
    // v23+ omits them entirely.

    Ok(SnapshotData {
        seq,
        master_tick,
        clock_us,
        start_us,
        source_writes,
        predictor_ticks,
        snapshot_fail_count,
        history_cap,
        axis_count,
        hearing_fault,
        hearing_fault_count,
        hl_a,
        hl_avg_mse,
        hl_pred_mse,
        hl_meta_err,
        hl_modulation,
        last_obs_t_us,
        health_state,
        phot_health,
        phot_started_t_us,
        phot_last_frame_t_us,
        axes,
        ambient_nomination_seq,
        ambient_vib,
        ambient_phot,
        ambient_mech,
        slots,
        last_claim,
        global_attention,
        global_relations,
        global_reasoning,
        global_reasoning_memory,
        language_binding,
        vpc_contract,
        action_output,
        action_consequence,
        probe_law,
        vocal_target,
        vocal_organ,
        vocal_imitation,
        vocal_memory,
        code_ingress,
        learned_bias,
        adaptive_stability,
        evidence_tension,
        framed_byte_egress,
        active_reasoning_gap,
    })
}

fn read_spatial(bytes: &[u8], cur: &mut usize) -> Option<SnapshotSpatial> {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let present = bytes[*cur];
    *cur += 1;
    if present == 0 {
        return None;
    }
    let pos_x = rd_f64(bytes, cur);
    let pos_y = rd_f64(bytes, cur);
    let span_x = rd_f64(bytes, cur);
    let span_y = rd_f64(bytes, cur);
    let precision = rd_f64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let valid = bytes[*cur] != 0;
    *cur += 1;
    Some(SnapshotSpatial {
        pos_x,
        pos_y,
        span_x,
        span_y,
        precision,
        last_update_t_us,
        valid,
    })
}

fn read_spatial_worldline(bytes: &[u8], cur: &mut usize) -> Option<SpatialWorldlineState> {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let present = bytes[*cur];
    *cur += 1;
    if present == 0 {
        return None;
    }
    let n_updates = rd_u64(bytes, cur) as u32;
    let prev_t_us = rd_u64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let prev_pos_x = rd_f64(bytes, cur);
    let prev_pos_y = rd_f64(bytes, cur);
    let predicted_pos_x = rd_f64(bytes, cur);
    let predicted_pos_y = rd_f64(bytes, cur);
    let velocity_x_per_s = rd_f64(bytes, cur);
    let velocity_y_per_s = rd_f64(bytes, cur);
    let err_x = rd_f64(bytes, cur);
    let err_y = rd_f64(bytes, cur);
    let err_norm = rd_f64(bytes, cur);
    let last_dt_us = rd_u64(bytes, cur);
    let valid = bytes[*cur] != 0;
    *cur += 1;
    Some(SpatialWorldlineState {
        n_updates,
        prev_t_us,
        last_update_t_us,
        prev_pos_x,
        prev_pos_y,
        predicted_pos_x,
        predicted_pos_y,
        velocity_x_per_s,
        velocity_y_per_s,
        err_x,
        err_y,
        err_norm,
        last_dt_us,
        valid,
    })
}

fn read_fusion(bytes: &[u8], cur: &mut usize) -> Option<CrossModalFusionState> {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let present = bytes[*cur];
    *cur += 1;
    if present == 0 {
        return None;
    }
    let modality_mask = rd_u64(bytes, cur) as u8;
    let last_fused_t_us = rd_u64(bytes, cur);
    let last_vib_t_us = rd_u64(bytes, cur);
    let last_phot_t_us = rd_u64(bytes, cur);
    let temporal_alignment_score = rd_f64(bytes, cur);
    let cross_modal_confidence = rd_f64(bytes, cur);
    let vib_weight = rd_f64(bytes, cur);
    let phot_weight = rd_f64(bytes, cur);
    let fused_surprise = rd_f64(bytes, cur);
    let is_cross_modal = bytes[*cur] != 0;
    *cur += 1;
    Some(CrossModalFusionState {
        modality_mask,
        last_fused_t_us,
        last_vib_t_us,
        last_phot_t_us,
        temporal_alignment_score,
        cross_modal_confidence,
        vib_weight,
        phot_weight,
        fused_surprise,
        is_cross_modal,
    })
}

fn read_high_layer(bytes: &[u8], cur: &mut usize) -> Option<ObjectHighLayerState> {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let present = bytes[*cur];
    *cur += 1;
    if present == 0 {
        return None;
    }
    let predicted_confidence = rd_f64(bytes, cur);
    let predicted_alignment = rd_f64(bytes, cur);
    let predicted_surprise = rd_f64(bytes, cur);
    let object_err = rd_f64(bytes, cur);
    let attention_bias = rd_f64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    Some(ObjectHighLayerState {
        predicted_confidence,
        predicted_alignment,
        predicted_surprise,
        object_err,
        attention_bias,
        last_update_t_us,
    })
}

fn read_family(bytes: &[u8], cur: &mut usize) -> Option<SnapshotBandFamily> {
    fn rd_u16(b: &[u8], c: &mut usize) -> u16 {
        let mut a = [0u8; 2];
        a.copy_from_slice(&b[*c..*c + 2]);
        *c += 2;
        u16::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let present = bytes[*cur];
    *cur += 1;
    if present == 0 {
        return None;
    }
    let n_bands = rd_u16(bytes, cur);
    let n = n_bands as usize;
    let mut observed = Vec::with_capacity(n);
    for _ in 0..n {
        observed.push(rd_f64(bytes, cur));
    }
    let mut predicted = Vec::with_capacity(n);
    for _ in 0..n {
        predicted.push(rd_f64(bytes, cur));
    }
    let mut err = Vec::with_capacity(n);
    for _ in 0..n {
        err.push(rd_f64(bytes, cur));
    }
    let mut surprise = Vec::with_capacity(n);
    for _ in 0..n {
        surprise.push(rd_f64(bytes, cur));
    }
    Some(SnapshotBandFamily {
        n_bands,
        observed,
        predicted,
        err,
        surprise,
    })
}

fn read_ring_linear(bytes: &[u8], cur: &mut usize, cap: usize) -> Vec<(u64, f64)> {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_u32(b: &[u8], c: &mut usize) -> u32 {
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        u32::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let write_idx = rd_u32(bytes, cur) as usize;
    let total_written = rd_u64(bytes, cur);
    let mut raw = Vec::with_capacity(cap);
    for _ in 0..cap {
        let t_us = rd_u64(bytes, cur);
        let value = rd_f64(bytes, cur);
        let valid = bytes[*cur] != 0;
        *cur += 8;
        raw.push((t_us, value, valid));
    }
    let available = (total_written as usize).min(cap);
    let mut out = Vec::with_capacity(available);
    let start = if total_written as usize <= cap {
        0
    } else {
        write_idx
    };
    for i in 0..available {
        let idx = (start + i) % cap;
        let (t, v, valid) = raw[idx];
        if valid {
            out.push((t, v));
        }
    }
    out
}

fn write_global_attention(buf: &mut Vec<u8>, ga: &GlobalAttentionState) {
    buf.extend_from_slice(&(ga.focused_slot_id as i64).to_le_bytes());
    for &id in ga.top_slot_ids.iter() {
        buf.extend_from_slice(&(id as i64).to_le_bytes());
    }
    for &s in ga.attention_shares.iter() {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf.extend_from_slice(&ga.attention_entropy.to_le_bytes());
    buf.extend_from_slice(&ga.focused_score.to_le_bytes());
    buf.extend_from_slice(&(ga.n_active as u64).to_le_bytes());
    for &b in ga.slot_biases.iter() {
        buf.extend_from_slice(&b.to_le_bytes());
    }
}

fn read_global_attention_snapshot(bytes: &[u8], cur: &mut usize) -> GlobalAttentionState {
    fn rd_i64(b: &[u8], c: &mut usize) -> i64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        i64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    let focused_slot_id = rd_i64(bytes, cur) as i8;
    let mut top_slot_ids = [-1i8; N_OBJECT_SLOTS];
    for i in 0..N_OBJECT_SLOTS {
        top_slot_ids[i] = rd_i64(bytes, cur) as i8;
    }
    let mut attention_shares = [0.0f64; N_OBJECT_SLOTS];
    for i in 0..N_OBJECT_SLOTS {
        attention_shares[i] = rd_f64(bytes, cur);
    }
    let attention_entropy = rd_f64(bytes, cur);
    let focused_score = rd_f64(bytes, cur);
    let n_active = rd_u64(bytes, cur) as u8;
    let mut slot_biases = [0.0f64; N_OBJECT_SLOTS];
    for i in 0..N_OBJECT_SLOTS {
        slot_biases[i] = rd_f64(bytes, cur);
    }
    GlobalAttentionState {
        focused_slot_id,
        top_slot_ids,
        attention_shares,
        attention_entropy,
        focused_score,
        n_active,
        slot_biases,
        replay_intensity: 0.0,
        last_replay_t_us: 0,
    }
}

fn write_global_relations(buf: &mut Vec<u8>, gr: &GlobalRelationState) {
    buf.extend_from_slice(&(gr.n_active_edges as u64).to_le_bytes());
    buf.extend_from_slice(&gr.last_update_t_us.to_le_bytes());
    for edge in gr.edges.iter() {
        buf.extend_from_slice(&(edge.src_slot_id as u64).to_le_bytes());
        buf.extend_from_slice(&(edge.dst_slot_id as u64).to_le_bytes());
        buf.extend_from_slice(&(edge.active as u64).to_le_bytes());
        buf.extend_from_slice(&edge.co_change_score.to_le_bytes());
        buf.extend_from_slice(&edge.lead_lag_score.to_le_bytes());
        buf.extend_from_slice(&edge.predictive_score.to_le_bytes());
        buf.extend_from_slice(&edge.relation_confidence.to_le_bytes());
        buf.extend_from_slice(&edge.spatial_distance_score.to_le_bytes());
        buf.extend_from_slice(&edge.spatial_overlap_score.to_le_bytes());
        buf.extend_from_slice(&edge.last_update_t_us.to_le_bytes());
        // v18: Phase 5E relation predictor output
        buf.extend_from_slice(&edge.pred.predicted_co_change.to_le_bytes());
        buf.extend_from_slice(&edge.pred.predicted_lead_lag.to_le_bytes());
        buf.extend_from_slice(&edge.pred.predicted_predictive.to_le_bytes());
        buf.extend_from_slice(&edge.pred.predicted_confidence.to_le_bytes());
        buf.extend_from_slice(&edge.pred.relation_err.to_le_bytes());
        buf.extend_from_slice(&edge.pred.relation_surprise.to_le_bytes());
        buf.extend_from_slice(&edge.pred.last_predict_t_us.to_le_bytes());
    }
}

fn read_global_relations_snapshot(bytes: &[u8], cur: &mut usize) -> GlobalRelationState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_active_edges = rd_u64(bytes, cur) as u8;
    let last_update_t_us = rd_u64(bytes, cur);
    let mut edges = [RelationEdgeState::default(); N_REL_EDGES];
    for i in 0..N_REL_EDGES {
        let src_slot_id = rd_u64(bytes, cur) as u8;
        let dst_slot_id = rd_u64(bytes, cur) as u8;
        let active = rd_u64(bytes, cur) != 0;
        let co_change_score = rd_f64(bytes, cur);
        let lead_lag_score = rd_f64(bytes, cur);
        let predictive_score = rd_f64(bytes, cur);
        let relation_confidence = rd_f64(bytes, cur);
        let spatial_distance_score = rd_f64(bytes, cur);
        let spatial_overlap_score = rd_f64(bytes, cur);
        let last_update = rd_u64(bytes, cur);
        // v18: Phase 5E relation predictor output
        let predicted_co_change = rd_f64(bytes, cur);
        let predicted_lead_lag = rd_f64(bytes, cur);
        let predicted_predictive = rd_f64(bytes, cur);
        let predicted_confidence = rd_f64(bytes, cur);
        let relation_err = rd_f64(bytes, cur);
        let relation_surprise = rd_f64(bytes, cur);
        let last_predict_t_us = rd_u64(bytes, cur);
        use crate::viewport::RelationEdgePredictionState;
        edges[i] = RelationEdgeState {
            src_slot_id,
            dst_slot_id,
            active,
            co_change_score,
            lead_lag_score,
            predictive_score,
            relation_confidence,
            spatial_distance_score,
            spatial_overlap_score,
            last_update_t_us: last_update,
            pred: RelationEdgePredictionState {
                predicted_co_change,
                predicted_lead_lag,
                predicted_predictive,
                predicted_confidence,
                relation_err,
                relation_surprise,
                last_predict_t_us,
            },
        };
    }
    GlobalRelationState {
        edges,
        n_active_edges,
        last_update_t_us,
    }
}

fn write_global_reasoning(buf: &mut Vec<u8>, rs: &GlobalReasoningState) {
    buf.extend_from_slice(&rs.global_surprise_load.to_le_bytes());
    buf.extend_from_slice(&rs.coordinated_event_score.to_le_bytes());
    buf.extend_from_slice(&rs.network_instability_score.to_le_bytes());
    for &id in rs.routing_focus_slot_ids.iter() {
        buf.extend_from_slice(&(id as i64).to_le_bytes());
    }
    for &id in rs.routing_focus_edge_ids.iter() {
        buf.extend_from_slice(&(id as i64).to_le_bytes());
    }
    for &w in rs.routing_weights.iter() {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    buf.extend_from_slice(&rs.last_update_t_us.to_le_bytes());
}

fn read_global_reasoning_snapshot(bytes: &[u8], cur: &mut usize) -> GlobalReasoningState {
    fn rd_i64(b: &[u8], c: &mut usize) -> i64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        i64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let global_surprise_load = rd_f64(bytes, cur);
    let coordinated_event_score = rd_f64(bytes, cur);
    let network_instability_score = rd_f64(bytes, cur);
    let mut routing_focus_slot_ids = [-1i8; N_ROUTING_FOCUS_SLOTS];
    for k in 0..N_ROUTING_FOCUS_SLOTS {
        routing_focus_slot_ids[k] = rd_i64(bytes, cur) as i8;
    }
    let mut routing_focus_edge_ids = [-1i16; N_ROUTING_FOCUS_EDGES];
    for k in 0..N_ROUTING_FOCUS_EDGES {
        routing_focus_edge_ids[k] = rd_i64(bytes, cur) as i16;
    }
    let mut routing_weights = [0.0f64; N_ROUTING_FOCUS_EDGES];
    for k in 0..N_ROUTING_FOCUS_EDGES {
        routing_weights[k] = rd_f64(bytes, cur);
    }
    let last_update_t_us = rd_i64(bytes, cur) as u64;
    GlobalReasoningState {
        global_surprise_load,
        coordinated_event_score,
        network_instability_score,
        routing_focus_slot_ids,
        routing_focus_edge_ids,
        routing_weights,
        last_update_t_us,
    }
}

fn write_global_reasoning_memory_summary(buf: &mut Vec<u8>, ms: &GlobalReasoningMemorySummary) {
    buf.extend_from_slice(&ms.novelty_score.to_le_bytes());
    buf.extend_from_slice(&ms.persistence_score.to_le_bytes());
    buf.extend_from_slice(&ms.routing_stability_score.to_le_bytes());
    buf.extend_from_slice(&ms.pattern_recurrence_score.to_le_bytes());
    buf.extend_from_slice(&ms.history_fill_ratio.to_le_bytes());
    buf.extend_from_slice(&ms.dominant_focus_persistence.to_le_bytes());
    buf.extend_from_slice(&ms.last_update_t_us.to_le_bytes());
}

fn read_global_reasoning_memory_summary(
    bytes: &[u8],
    cur: &mut usize,
) -> GlobalReasoningMemorySummary {
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    GlobalReasoningMemorySummary {
        novelty_score: rd_f64(bytes, cur),
        persistence_score: rd_f64(bytes, cur),
        routing_stability_score: rd_f64(bytes, cur),
        pattern_recurrence_score: rd_f64(bytes, cur),
        history_fill_ratio: rd_f64(bytes, cur),
        dominant_focus_persistence: rd_f64(bytes, cur),
        last_update_t_us: rd_u64(bytes, cur),
    }
}

fn write_language_binding(buf: &mut Vec<u8>, st: &LanguageBindingState) {
    buf.extend_from_slice(&(st.traces.len() as u32).to_le_bytes());
    for trace in &st.traces {
        buf.push(pattern_kind_code(trace.pattern_kind));
        buf.extend_from_slice(&trace.pattern_hash.to_le_bytes());
        write_binding_target(buf, trace.target);
        buf.extend_from_slice(&trace.strength.value.to_le_bytes());
        buf.push(binding_status_code(trace.status));
        buf.extend_from_slice(&(trace.evidence_refs.len() as u32).to_le_bytes());
        for evidence_ref in &trace.evidence_refs {
            buf.extend_from_slice(&evidence_ref.to_le_bytes());
        }
        buf.extend_from_slice(&trace.reinforce_count.to_le_bytes());
        buf.extend_from_slice(&trace.weaken_count.to_le_bytes());
        buf.extend_from_slice(&trace.last_update_t_us.to_le_bytes());
    }
}

fn read_language_binding_snapshot(bytes: &[u8], cur: &mut usize) -> LanguageBindingState {
    fn rd_u32(b: &[u8], c: &mut usize) -> u32 {
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        u32::from_le_bytes(a)
    }
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_traces = rd_u32(bytes, cur) as usize;
    let mut traces = Vec::with_capacity(n_traces);
    for _ in 0..n_traces {
        let pattern_kind = pattern_kind_from_code(bytes[*cur]);
        *cur += 1;
        let pattern_hash = rd_u64(bytes, cur);
        let target = read_binding_target(bytes, cur);
        let strength = crate::language_binding::BindingStrength::new(rd_f64(bytes, cur));
        let status = binding_status_from_code(bytes[*cur]);
        *cur += 1;
        let n_evidence = rd_u32(bytes, cur) as usize;
        let mut evidence_refs = Vec::with_capacity(n_evidence);
        for _ in 0..n_evidence {
            evidence_refs.push(rd_u64(bytes, cur));
        }
        let reinforce_count = rd_u32(bytes, cur);
        let weaken_count = rd_u32(bytes, cur);
        let last_update_t_us = rd_u64(bytes, cur);
        traces.push(BindingTrace {
            pattern_kind,
            pattern_hash,
            target,
            strength,
            status,
            evidence_refs,
            reinforce_count,
            weaken_count,
            last_update_t_us,
        });
    }
    LanguageBindingState {
        n_bindings: traces.len(),
        traces,
    }
}

fn pattern_kind_code(kind: PatternKind) -> u8 {
    match kind {
        PatternKind::PhotModality => 0,
        PatternKind::VibModality => 1,
        PatternKind::MechModality => 2,
        PatternKind::SymbolSeq => 3,
    }
}

fn write_vpc_contract(buf: &mut Vec<u8>, st: &VpcContractState) {
    buf.extend_from_slice(&(st.predictor_traces.len() as u32).to_le_bytes());
    for trace in &st.predictor_traces {
        buf.push(trace.predictor_family);
        buf.push(observation_domain_kind_code(trace.domain.kind));
        buf.extend_from_slice(&trace.domain.primary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.secondary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.tertiary.to_le_bytes());
        buf.push(horizon_kind_code(trace.horizon.kind));
        buf.extend_from_slice(&trace.horizon.publish_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_start_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_end_t_us.to_le_bytes());
        buf.push(predictor_lifecycle_code(trace.lifecycle));
    }

    buf.extend_from_slice(&(st.publications.len() as u32).to_le_bytes());
    for trace in &st.publications {
        buf.push(trace.predictor_family);
        buf.push(observation_domain_kind_code(trace.domain.kind));
        buf.extend_from_slice(&trace.domain.primary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.secondary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.tertiary.to_le_bytes());
        buf.push(horizon_kind_code(trace.horizon.kind));
        buf.extend_from_slice(&trace.horizon.publish_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_start_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_end_t_us.to_le_bytes());
        buf.push(err_alignment_kind_code(trace.alignment));
        buf.push(publication_status_code(trace.status));
    }

    buf.extend_from_slice(&(st.open_predictions.len() as u32).to_le_bytes());
    for trace in &st.open_predictions {
        buf.push(trace.predictor_family);
        buf.push(observation_domain_kind_code(trace.domain.kind));
        buf.extend_from_slice(&trace.domain.primary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.secondary.to_le_bytes());
        buf.extend_from_slice(&trace.domain.tertiary.to_le_bytes());
        buf.push(horizon_kind_code(trace.horizon.kind));
        buf.extend_from_slice(&trace.horizon.publish_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_start_t_us.to_le_bytes());
        buf.extend_from_slice(&trace.horizon.target_end_t_us.to_le_bytes());
        buf.push(err_alignment_kind_code(trace.alignment));
        buf.extend_from_slice(&trace.publication_index.to_le_bytes());
    }
}

fn read_vpc_contract_snapshot(bytes: &[u8], cur: &mut usize) -> VpcContractState {
    fn rd_u16(b: &[u8], c: &mut usize) -> u16 {
        let mut a = [0u8; 2];
        a.copy_from_slice(&b[*c..*c + 2]);
        *c += 2;
        u16::from_le_bytes(a)
    }
    fn rd_u32(b: &[u8], c: &mut usize) -> u32 {
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        u32::from_le_bytes(a)
    }
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }

    let n_predictor_traces = rd_u32(bytes, cur) as usize;
    let mut predictor_traces = Vec::with_capacity(n_predictor_traces);
    for _ in 0..n_predictor_traces {
        let predictor_family = bytes[*cur];
        *cur += 1;
        let domain_kind = observation_domain_kind_from_code(bytes[*cur]);
        *cur += 1;
        let domain = ObservationDomainRef {
            kind: domain_kind,
            primary: rd_u16(bytes, cur),
            secondary: rd_u16(bytes, cur),
            tertiary: rd_u16(bytes, cur),
        };
        let horizon_kind = horizon_kind_from_code(bytes[*cur]);
        *cur += 1;
        let horizon = PredictionHorizon {
            kind: horizon_kind,
            publish_t_us: rd_u64(bytes, cur),
            target_start_t_us: rd_u64(bytes, cur),
            target_end_t_us: rd_u64(bytes, cur),
        };
        let lifecycle = predictor_lifecycle_from_code(bytes[*cur]);
        *cur += 1;
        predictor_traces.push(crate::vpc::PredictorFamilyTrace {
            predictor_family,
            domain,
            horizon,
            lifecycle,
        });
    }

    let n_publications = rd_u32(bytes, cur) as usize;
    let mut publications = Vec::with_capacity(n_publications);
    for _ in 0..n_publications {
        let predictor_family = bytes[*cur];
        *cur += 1;
        let domain_kind = observation_domain_kind_from_code(bytes[*cur]);
        *cur += 1;
        let domain = ObservationDomainRef {
            kind: domain_kind,
            primary: rd_u16(bytes, cur),
            secondary: rd_u16(bytes, cur),
            tertiary: rd_u16(bytes, cur),
        };
        let horizon_kind = horizon_kind_from_code(bytes[*cur]);
        *cur += 1;
        let horizon = PredictionHorizon {
            kind: horizon_kind,
            publish_t_us: rd_u64(bytes, cur),
            target_start_t_us: rd_u64(bytes, cur),
            target_end_t_us: rd_u64(bytes, cur),
        };
        let alignment = err_alignment_kind_from_code(bytes[*cur]);
        *cur += 1;
        let status = publication_status_from_code(bytes[*cur]);
        *cur += 1;
        publications.push(PredictionPublicationTrace {
            predictor_family,
            domain,
            horizon,
            alignment,
            status,
        });
    }

    let n_open = rd_u32(bytes, cur) as usize;
    let mut open_predictions = Vec::with_capacity(n_open);
    for _ in 0..n_open {
        let predictor_family = bytes[*cur];
        *cur += 1;
        let domain_kind = observation_domain_kind_from_code(bytes[*cur]);
        *cur += 1;
        let domain = ObservationDomainRef {
            kind: domain_kind,
            primary: rd_u16(bytes, cur),
            secondary: rd_u16(bytes, cur),
            tertiary: rd_u16(bytes, cur),
        };
        let horizon_kind = horizon_kind_from_code(bytes[*cur]);
        *cur += 1;
        let horizon = PredictionHorizon {
            kind: horizon_kind,
            publish_t_us: rd_u64(bytes, cur),
            target_start_t_us: rd_u64(bytes, cur),
            target_end_t_us: rd_u64(bytes, cur),
        };
        let alignment = err_alignment_kind_from_code(bytes[*cur]);
        *cur += 1;
        let publication_index = rd_u32(bytes, cur);
        open_predictions.push(crate::vpc::OpenPredictionTrace {
            predictor_family,
            domain,
            horizon,
            alignment,
            publication_index,
        });
    }

    VpcContractState {
        predictor_traces,
        publications,
        open_predictions,
    }
}

fn observation_domain_kind_code(kind: ObservationDomainKind) -> u8 {
    match kind {
        ObservationDomainKind::Axis => 0,
        ObservationDomainKind::SlotFamily => 1,
        ObservationDomainKind::RelationEdge => 2,
        ObservationDomainKind::ReasoningFragment => 3,
    }
}

fn observation_domain_kind_from_code(code: u8) -> ObservationDomainKind {
    match code {
        1 => ObservationDomainKind::SlotFamily,
        2 => ObservationDomainKind::RelationEdge,
        3 => ObservationDomainKind::ReasoningFragment,
        _ => ObservationDomainKind::Axis,
    }
}

fn horizon_kind_code(kind: HorizonKind) -> u8 {
    match kind {
        HorizonKind::Point => 0,
        HorizonKind::Window => 1,
    }
}

fn horizon_kind_from_code(code: u8) -> HorizonKind {
    match code {
        1 => HorizonKind::Window,
        _ => HorizonKind::Point,
    }
}

fn err_alignment_kind_code(kind: ErrAlignmentKind) -> u8 {
    match kind {
        ErrAlignmentKind::PointExact => 0,
        ErrAlignmentKind::WindowInclusive => 1,
    }
}

fn err_alignment_kind_from_code(code: u8) -> ErrAlignmentKind {
    match code {
        1 => ErrAlignmentKind::WindowInclusive,
        _ => ErrAlignmentKind::PointExact,
    }
}

fn publication_status_code(status: PublicationStatus) -> u8 {
    match status {
        PublicationStatus::Open => 0,
        PublicationStatus::Resolved => 1,
        PublicationStatus::Expired => 2,
    }
}

fn publication_status_from_code(code: u8) -> PublicationStatus {
    match code {
        1 => PublicationStatus::Resolved,
        2 => PublicationStatus::Expired,
        _ => PublicationStatus::Open,
    }
}

fn predictor_lifecycle_code(state: PredictorLifecycleState) -> u8 {
    match state {
        PredictorLifecycleState::InsufficientContext => 0,
        PredictorLifecycleState::Warmup => 1,
        PredictorLifecycleState::OpenPublication => 2,
        PredictorLifecycleState::ResolvedPublication => 3,
    }
}

fn predictor_lifecycle_from_code(code: u8) -> PredictorLifecycleState {
    match code {
        1 => PredictorLifecycleState::Warmup,
        2 => PredictorLifecycleState::OpenPublication,
        3 => PredictorLifecycleState::ResolvedPublication,
        _ => PredictorLifecycleState::InsufficientContext,
    }
}

fn pattern_kind_from_code(code: u8) -> PatternKind {
    match code {
        1 => PatternKind::VibModality,
        2 => PatternKind::MechModality,
        3 => PatternKind::SymbolSeq,
        _ => PatternKind::PhotModality,
    }
}

fn binding_status_code(status: BindingStatus) -> u8 {
    match status {
        BindingStatus::Candidate => 0,
        BindingStatus::Reinforced => 1,
        BindingStatus::Weakened => 2,
    }
}

fn binding_status_from_code(code: u8) -> BindingStatus {
    match code {
        1 => BindingStatus::Reinforced,
        2 => BindingStatus::Weakened,
        _ => BindingStatus::Candidate,
    }
}

fn write_binding_target(buf: &mut Vec<u8>, target: BindingTargetRef) {
    match target {
        BindingTargetRef::ObjectSlot { slot_id } => {
            buf.push(0);
            buf.extend_from_slice(&(slot_id as u64).to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
        }
        BindingTargetRef::RelationEdge { edge_index } => {
            buf.push(1);
            buf.extend_from_slice(&(edge_index as u64).to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
        }
        BindingTargetRef::TimeWindow {
            start_t_us,
            end_t_us,
        } => {
            buf.push(2);
            buf.extend_from_slice(&start_t_us.to_le_bytes());
            buf.extend_from_slice(&end_t_us.to_le_bytes());
        }
    }
}

fn read_binding_target(bytes: &[u8], cur: &mut usize) -> BindingTargetRef {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    let tag = bytes[*cur];
    *cur += 1;
    let a = rd_u64(bytes, cur);
    let b = rd_u64(bytes, cur);
    match tag {
        1 => BindingTargetRef::RelationEdge {
            edge_index: a as u16,
        },
        2 => BindingTargetRef::TimeWindow {
            start_t_us: a,
            end_t_us: b,
        },
        _ => BindingTargetRef::ObjectSlot { slot_id: a as u8 },
    }
}

// ─── v25/v27: action-output substrate ────────────────────────────────────────

fn write_action_output(buf: &mut Vec<u8>, s: &ActionOutputState) {
    buf.extend_from_slice(&(s.n_channels as u64).to_le_bytes());
    for ch in &s.channels {
        buf.extend_from_slice(&(ch.channel_kind as u64).to_le_bytes());
        buf.extend_from_slice(&(ch.phase as u64).to_le_bytes());
        buf.extend_from_slice(&ch.current_magnitude.to_le_bytes());
        buf.extend_from_slice(&ch.peak_magnitude.to_le_bytes());
        buf.extend_from_slice(&ch.start_t_us.to_le_bytes());
        buf.extend_from_slice(&ch.last_update_t_us.to_le_bytes());
        buf.extend_from_slice(&ch.last_transition_t_us.to_le_bytes());
        buf.extend_from_slice(&(ch.reinforce_count as u64).to_le_bytes());
        buf.extend_from_slice(&(ch.decay_count as u64).to_le_bytes());
        // v26: write-epoch pairing handle
        buf.extend_from_slice(&ch.current_write_seq.to_le_bytes());
    }
    // v27: minimal write-epoch ledger
    buf.extend_from_slice(&(s.n_write_epochs as u64).to_le_bytes());
    for ep in &s.write_epoch_history {
        buf.extend_from_slice(&ep.write_seq.to_le_bytes());
        buf.extend_from_slice(&(ep.channel_kind as u64).to_le_bytes());
        buf.extend_from_slice(&ep.start_magnitude.to_le_bytes());
        buf.extend_from_slice(&ep.start_t_us.to_le_bytes());
        buf.extend_from_slice(&ep.last_update_t_us.to_le_bytes());
        buf.extend_from_slice(&(ep.last_phase as u64).to_le_bytes());
    }
}

fn read_action_output_snapshot(
    bytes: &[u8],
    cur: &mut usize,
    version: u64,
) -> SnapshotActionOutput {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_channels = rd_u64(bytes, cur) as u32;
    let mut channels = Vec::with_capacity(n_channels as usize);
    for _ in 0..n_channels {
        let channel_kind = rd_u64(bytes, cur);
        let phase = rd_u64(bytes, cur);
        let current_magnitude = rd_f64(bytes, cur);
        let peak_magnitude = rd_f64(bytes, cur);
        let start_t_us = rd_u64(bytes, cur);
        let last_update_t_us = rd_u64(bytes, cur);
        let last_transition_t_us = rd_u64(bytes, cur);
        let reinforce_count = rd_u64(bytes, cur) as u32;
        let decay_count = rd_u64(bytes, cur) as u32;
        let current_write_seq = if version >= 26 { rd_u64(bytes, cur) } else { 0 };
        channels.push(SnapshotActionOutputChannel {
            channel_kind,
            phase,
            current_magnitude,
            peak_magnitude,
            start_t_us,
            last_update_t_us,
            last_transition_t_us,
            reinforce_count,
            decay_count,
            current_write_seq,
        });
    }
    // v27: write-epoch ledger
    let (n_write_epochs, write_epoch_history) = if version >= 27 {
        let n = rd_u64(bytes, cur) as u32;
        let mut history = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let write_seq = rd_u64(bytes, cur);
            let channel_kind = rd_u64(bytes, cur);
            let start_magnitude = rd_f64(bytes, cur);
            let start_t_us = rd_u64(bytes, cur);
            let last_update_t_us = rd_u64(bytes, cur);
            let last_phase = rd_u64(bytes, cur);
            history.push(SnapshotWriteEpochRecord {
                write_seq,
                channel_kind,
                start_magnitude,
                start_t_us,
                last_update_t_us,
                last_phase,
            });
        }
        (n, history)
    } else {
        (0, vec![])
    };
    SnapshotActionOutput {
        n_channels,
        channels,
        n_write_epochs,
        write_epoch_history,
    }
}

// ─── v26: action-consequence loop substrate ───────────────────────────────────

fn write_action_consequence(buf: &mut Vec<u8>, s: &ActionConsequenceState) {
    buf.extend_from_slice(&(s.n_windows as u64).to_le_bytes());
    for w in &s.windows {
        // v29: window_id is the first field written per window
        buf.extend_from_slice(&w.window_id.to_le_bytes());
        buf.extend_from_slice(&(w.modality_kind as u64).to_le_bytes());
        buf.extend_from_slice(&w.start_t_us.to_le_bytes());
        buf.extend_from_slice(&w.end_t_us.to_le_bytes());
        buf.extend_from_slice(&w.magnitude.to_le_bytes());
        // v28: lifecycle fields
        buf.extend_from_slice(&w.phase.as_u64().to_le_bytes());
        buf.extend_from_slice(&w.last_update_t_us.to_le_bytes());
    }
    buf.extend_from_slice(&(s.n_pairs as u64).to_le_bytes());
    buf.extend_from_slice(&s.total_pairs.to_le_bytes());
    buf.extend_from_slice(&s.total_framed_byte_pairs.to_le_bytes());
    buf.extend_from_slice(&s.total_other_pairs.to_le_bytes());
    for p in &s.pairs {
        // v42: pair_seq is first field per pair.
        buf.extend_from_slice(&p.pair_seq.to_le_bytes());
        buf.extend_from_slice(&p.write_seq.to_le_bytes());
        // v29: window_id replaces window_index
        buf.extend_from_slice(&p.window_id.to_le_bytes());
        buf.extend_from_slice(&p.delay_us.to_le_bytes());
        buf.extend_from_slice(&(p.status as u64).to_le_bytes());
        buf.extend_from_slice(&p.pair_t_us.to_le_bytes());
        // v31: curve_similarity
        buf.extend_from_slice(&p.curve_similarity.to_le_bytes());
    }
}

fn read_action_consequence_snapshot(
    bytes: &[u8],
    cur: &mut usize,
    version: u64,
) -> SnapshotActionConsequence {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_windows = rd_u64(bytes, cur) as u32;
    let mut windows = Vec::with_capacity(n_windows as usize);
    for _ in 0..n_windows {
        // v29: window_id is the first field read per window
        let window_id = rd_u64(bytes, cur);
        let modality = rd_u64(bytes, cur);
        let start_t_us = rd_u64(bytes, cur);
        let end_t_us = rd_u64(bytes, cur);
        let magnitude = rd_f64(bytes, cur);
        // v28: lifecycle fields
        let phase = rd_u64(bytes, cur);
        let last_update_t_us = rd_u64(bytes, cur);
        windows.push(SnapshotConsequenceWindow {
            window_id,
            modality_kind: modality,
            start_t_us,
            end_t_us,
            magnitude,
            phase,
            last_update_t_us,
        });
    }
    let n_pairs = rd_u64(bytes, cur) as u32;
    let total_pairs = if version >= 43 {
        rd_u64(bytes, cur)
    } else {
        n_pairs as u64
    };
    let (total_framed_byte_pairs, total_other_pairs) = if version >= 44 {
        (rd_u64(bytes, cur), rd_u64(bytes, cur))
    } else {
        (0, total_pairs)
    };
    let mut pairs = Vec::with_capacity(n_pairs as usize);
    for _ in 0..n_pairs {
        let pair_seq = if version >= 42 { rd_u64(bytes, cur) } else { 0 };
        let write_seq = rd_u64(bytes, cur);
        // v29: window_id replaces window_index
        let window_id = rd_u64(bytes, cur);
        let delay_us = rd_u64(bytes, cur);
        let status = rd_u64(bytes, cur);
        let pair_t_us = rd_u64(bytes, cur);
        let curve_similarity = if version >= 31 {
            rd_f64(bytes, cur)
        } else {
            0.0
        };
        pairs.push(SnapshotConsequencePair {
            pair_seq,
            write_seq,
            window_id,
            delay_us,
            status,
            pair_t_us,
            curve_similarity,
        });
    }
    SnapshotActionConsequence {
        n_windows,
        windows,
        n_pairs,
        total_pairs,
        total_framed_byte_pairs,
        total_other_pairs,
        pairs,
    }
}

// ─── v30: probe-law substrate ─────────────────────────────────────────────────

fn write_probe_law(buf: &mut Vec<u8>, s: &ProbeLawState) {
    buf.extend_from_slice(&s.intensity.to_le_bytes());
    buf.extend_from_slice(&s.suppression.to_le_bytes());
    buf.push(s.phase.as_u8());
    buf.extend_from_slice(&[0u8; 7]); // padding to 8-byte alignment
    buf.extend_from_slice(&s.output_intensity.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
    buf.push(s.history_len as u8);
    buf.extend_from_slice(&[0u8; 7]); // padding
    for v in &s.history {
        buf.extend_from_slice(&v.to_le_bytes());
    }
}

fn read_probe_law_snapshot(bytes: &[u8], cur: &mut usize) -> SnapshotProbeLaw {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let intensity = rd_f64(bytes, cur);
    let suppression = rd_f64(bytes, cur);
    let phase = bytes[*cur];
    *cur += 8; // phase byte + 7 padding bytes
    let output_intensity = rd_f64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let history_len = bytes[*cur];
    *cur += 8; // history_len byte + 7 padding bytes
    let mut history = [0.0f64; PROBE_HISTORY_CAP];
    for v in history.iter_mut() {
        *v = rd_f64(bytes, cur);
    }
    SnapshotProbeLaw {
        intensity,
        suppression,
        phase,
        output_intensity,
        last_update_t_us,
        history_len,
        history,
    }
}

// ─── v36: vocal substrate ────────────────────────────────────────────────────

fn write_vocal_target(buf: &mut Vec<u8>, s: &VocalTargetState) {
    buf.extend_from_slice(&s.active_energy.to_le_bytes());
    buf.extend_from_slice(&s.pitch_hint.to_le_bytes());
    buf.extend_from_slice(&s.resonance_hint.to_le_bytes());
    buf.extend_from_slice(&s.envelope_hint.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_vocal_target_snapshot(bytes: &[u8], cur: &mut usize) -> VocalTargetState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    VocalTargetState {
        active_energy: rd_f64(bytes, cur),
        pitch_hint: rd_f64(bytes, cur),
        resonance_hint: rd_f64(bytes, cur),
        envelope_hint: rd_f64(bytes, cur),
        last_update_t_us: rd_u64(bytes, cur),
    }
}

fn write_vocal_organ(buf: &mut Vec<u8>, s: &VocalOrganState) {
    buf.extend_from_slice(&s.breath_drive.to_le_bytes());
    buf.extend_from_slice(&s.pitch_tension.to_le_bytes());
    buf.extend_from_slice(&s.resonance_position.to_le_bytes());
    buf.extend_from_slice(&s.noise_mix.to_le_bytes());
    buf.push(s.phase.as_u8());
    buf.extend_from_slice(&[0u8; 7]);
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_vocal_organ_snapshot(bytes: &[u8], cur: &mut usize) -> VocalOrganState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    use crate::vocal_organ::VocalOrganPhase;
    let breath_drive = rd_f64(bytes, cur);
    let pitch_tension = rd_f64(bytes, cur);
    let resonance_position = rd_f64(bytes, cur);
    let noise_mix = rd_f64(bytes, cur);
    let phase_byte = bytes[*cur];
    *cur += 8;
    let phase = if phase_byte == 1 {
        VocalOrganPhase::Active
    } else {
        VocalOrganPhase::Idle
    };
    let last_update_t_us = rd_u64(bytes, cur);
    VocalOrganState {
        breath_drive,
        pitch_tension,
        resonance_position,
        noise_mix,
        phase,
        last_update_t_us,
    }
}

fn write_vocal_imitation(buf: &mut Vec<u8>, s: &VocalImitationState) {
    buf.extend_from_slice(&s.imitation_pressure.to_le_bytes());
    buf.extend_from_slice(&s.suppression.to_le_bytes());
    buf.extend_from_slice(&s.last_gap.to_le_bytes());
    buf.extend_from_slice(&(s.converge_count as u64).to_le_bytes());
    buf.extend_from_slice(&(s.fail_count as u64).to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_vocal_imitation_snapshot(bytes: &[u8], cur: &mut usize) -> VocalImitationState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    VocalImitationState {
        imitation_pressure: rd_f64(bytes, cur),
        suppression: rd_f64(bytes, cur),
        last_gap: rd_f64(bytes, cur),
        converge_count: rd_u64(bytes, cur) as u32,
        fail_count: rd_u64(bytes, cur) as u32,
        last_update_t_us: rd_u64(bytes, cur),
    }
}

fn write_vocal_memory(buf: &mut Vec<u8>, s: &VocalMemoryState) {
    buf.extend_from_slice(&(s.prototypes.len() as u64).to_le_bytes());
    for p in &s.prototypes {
        buf.extend_from_slice(&p.breath_drive.to_le_bytes());
        buf.extend_from_slice(&p.pitch_tension.to_le_bytes());
        buf.extend_from_slice(&p.resonance_position.to_le_bytes());
        buf.extend_from_slice(&p.stability.to_le_bytes());
        buf.extend_from_slice(&(p.reuse_success as u64).to_le_bytes());
        buf.extend_from_slice(&p.last_update_t_us.to_le_bytes());
    }
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_vocal_memory_snapshot(bytes: &[u8], cur: &mut usize) -> SnapshotVocalMemory {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_prototypes = rd_u64(bytes, cur) as u32;
    let mut prototypes = Vec::with_capacity(n_prototypes as usize);
    for _ in 0..n_prototypes {
        prototypes.push(SnapshotVocalPrototype {
            breath_drive: rd_f64(bytes, cur),
            pitch_tension: rd_f64(bytes, cur),
            resonance_position: rd_f64(bytes, cur),
            stability: rd_f64(bytes, cur),
            reuse_success: rd_u64(bytes, cur) as u32,
            last_update_t_us: rd_u64(bytes, cur),
        });
    }
    let last_update_t_us = rd_u64(bytes, cur);
    SnapshotVocalMemory {
        n_prototypes,
        prototypes,
        last_update_t_us,
    }
}

// ─── v39: adaptive stability substrate ───────────────────────────────────────

fn write_adaptive_stability(buf: &mut Vec<u8>, s: &AdaptiveStabilityState) {
    buf.extend_from_slice(&(s.routes.len() as u64).to_le_bytes());
    for r in &s.routes {
        buf.extend_from_slice(&r.route_id.to_le_bytes());
        buf.extend_from_slice(&(r.route_code as u64).to_le_bytes());
        buf.extend_from_slice(&(r.phase_code as u64).to_le_bytes());
        buf.extend_from_slice(&r.err_ema.to_le_bytes());
        buf.extend_from_slice(&r.resource_cost_ema.to_le_bytes());
        buf.extend_from_slice(&r.closure_gain_ema.to_le_bytes());
        buf.extend_from_slice(&r.degradation_score.to_le_bytes());
        buf.extend_from_slice(&r.bypass_weight.to_le_bytes());
        buf.extend_from_slice(&r.relearn_pressure.to_le_bytes());
        buf.extend_from_slice(&r.slow_path_weight.to_le_bytes());
        buf.extend_from_slice(&r.fast_path_strength.to_le_bytes());
        buf.extend_from_slice(&(r.stable_run as u64).to_le_bytes());
        buf.extend_from_slice(&(r.drift_run as u64).to_le_bytes());
        buf.extend_from_slice(&r.last_good_t_us.to_le_bytes());
        buf.extend_from_slice(&r.last_update_t_us.to_le_bytes());
    }
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_adaptive_stability_snapshot(bytes: &[u8], cur: &mut usize) -> AdaptiveStabilityState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }
    let n_routes = rd_u64(bytes, cur) as u32;
    let mut routes = Vec::with_capacity(n_routes as usize);
    for _ in 0..n_routes {
        routes.push(AdaptiveRouteTrace {
            route_id: rd_u64(bytes, cur),
            route_code: rd_u64(bytes, cur).min(u16::MAX as u64) as u16,
            phase_code: rd_u64(bytes, cur).min(u8::MAX as u64) as u8,
            err_ema: rd_f64(bytes, cur),
            resource_cost_ema: rd_f64(bytes, cur),
            closure_gain_ema: rd_f64(bytes, cur),
            degradation_score: rd_f64(bytes, cur),
            bypass_weight: rd_f64(bytes, cur),
            relearn_pressure: rd_f64(bytes, cur),
            slow_path_weight: rd_f64(bytes, cur),
            fast_path_strength: rd_f64(bytes, cur),
            stable_run: rd_u64(bytes, cur) as u32,
            drift_run: rd_u64(bytes, cur) as u32,
            last_good_t_us: rd_u64(bytes, cur),
            last_update_t_us: rd_u64(bytes, cur),
        });
    }
    let last_update_t_us = rd_u64(bytes, cur);
    AdaptiveStabilityState {
        n_routes,
        routes,
        last_update_t_us,
    }
}

// ─── v40: evidence-tension pressure trace substrate ──────────────────────────

fn write_evidence_tension(buf: &mut Vec<u8>, s: &EvidenceTensionState) {
    buf.extend_from_slice(&s.current_tension.to_le_bytes());
    buf.extend_from_slice(&s.unresolved_pressure.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
    buf.extend_from_slice(&s.last_pair_cursor.to_le_bytes());
    buf.extend_from_slice(&(s.history_head as u64).to_le_bytes());
    buf.extend_from_slice(&(s.history_len as u64).to_le_bytes());
    for trace in &s.history {
        buf.extend_from_slice(&trace.t_us.to_le_bytes());
        buf.extend_from_slice(&trace.tension.to_le_bytes());
        buf.extend_from_slice(&trace.unresolved_pressure.to_le_bytes());
        buf.extend_from_slice(&trace.closure_support.to_le_bytes());
        buf.extend_from_slice(&trace.pair_cursor.to_le_bytes());
    }
}

fn read_evidence_tension_snapshot(bytes: &[u8], cur: &mut usize) -> EvidenceTensionState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    let current_tension = rd_f64(bytes, cur);
    let unresolved_pressure = rd_f64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let last_pair_cursor = rd_u64(bytes, cur);
    let history_head = rd_u64(bytes, cur) as usize;
    let history_len = rd_u64(bytes, cur) as usize;
    let mut history = [EvidenceTensionTrace::default(); EVIDENCE_TENSION_HISTORY_CAP];
    for trace in history.iter_mut() {
        *trace = EvidenceTensionTrace {
            t_us: rd_u64(bytes, cur),
            tension: rd_f64(bytes, cur),
            unresolved_pressure: rd_f64(bytes, cur),
            closure_support: rd_f64(bytes, cur),
            pair_cursor: rd_u64(bytes, cur),
        };
    }

    EvidenceTensionState {
        current_tension,
        unresolved_pressure,
        last_update_t_us,
        last_pair_cursor,
        history,
        history_head,
        history_len,
    }
}

// ─── v41: framed-byte egress substrate ───────────────────────────────────────

fn write_framed_byte_egress(buf: &mut Vec<u8>, s: &FramedByteEgressState) {
    buf.push(s.phase.as_u8());
    buf.extend_from_slice(&[0u8; 7]);
    buf.extend_from_slice(&s.active_stream_id.to_le_bytes());
    buf.extend_from_slice(&s.current_write_seq.to_le_bytes());
    buf.extend_from_slice(&s.byte_cursor.to_le_bytes());
    buf.extend_from_slice(&(s.n_frames as u64).to_le_bytes());
    buf.extend_from_slice(&(s.n_returns as u64).to_le_bytes());
    buf.extend_from_slice(&s.total_frames.to_le_bytes());
    buf.extend_from_slice(&s.total_returns.to_le_bytes());
    buf.extend_from_slice(&s.last_frame_seq.to_le_bytes());
    buf.extend_from_slice(&s.last_return_window_id.to_le_bytes());
    buf.extend_from_slice(&s.output_pressure.to_le_bytes());
    buf.extend_from_slice(&s.projection_confidence.to_le_bytes());
    buf.extend_from_slice(&s.correction_deviation.to_le_bytes());
    buf.extend_from_slice(&(s.correction_count as u64).to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_framed_byte_egress_snapshot(bytes: &[u8], cur: &mut usize) -> SnapshotFramedByteEgress {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    let phase = bytes[*cur];
    *cur += 8;
    let _ = FramedByteEgressPhase::from_u8(phase);
    SnapshotFramedByteEgress {
        phase,
        active_stream_id: rd_u64(bytes, cur),
        current_write_seq: rd_u64(bytes, cur),
        byte_cursor: rd_u64(bytes, cur),
        n_frames: rd_u64(bytes, cur) as u32,
        n_returns: rd_u64(bytes, cur) as u32,
        total_frames: rd_u64(bytes, cur),
        total_returns: rd_u64(bytes, cur),
        last_frame_seq: rd_u64(bytes, cur),
        last_return_window_id: rd_u64(bytes, cur),
        output_pressure: rd_f64(bytes, cur),
        projection_confidence: rd_f64(bytes, cur),
        correction_deviation: rd_f64(bytes, cur),
        correction_count: rd_u64(bytes, cur) as u32,
        last_update_t_us: rd_u64(bytes, cur),
    }
}

// ─── v46: active reasoning gap observer trace ────────────────────────────────

fn write_active_reasoning_gap(buf: &mut Vec<u8>, s: &ActiveReasoningGapState) {
    buf.extend_from_slice(&s.gap_pressure.to_le_bytes());
    buf.extend_from_slice(&s.drive_score.to_le_bytes());
    buf.extend_from_slice(&s.persistence_score.to_le_bytes());
    buf.extend_from_slice(&s.closure_score.to_le_bytes());
    buf.extend_from_slice(&s.reentry_score.to_le_bytes());
    buf.extend_from_slice(&(s.focus_slot_id as i64).to_le_bytes());
    buf.extend_from_slice(&(s.focus_edge_id as i64).to_le_bytes());
    buf.extend_from_slice(&(s.active_cycles as u64).to_le_bytes());
    buf.extend_from_slice(&s.last_pair_seq.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
    buf.extend_from_slice(&(s.history_len as u64).to_le_bytes());
    buf.extend_from_slice(&(s.history_head as u64).to_le_bytes());
    buf.extend_from_slice(&s.total_samples.to_le_bytes());
    for trace in &s.history {
        buf.extend_from_slice(&trace.t_us.to_le_bytes());
        buf.extend_from_slice(&trace.gap_pressure.to_le_bytes());
        buf.extend_from_slice(&trace.closure_score.to_le_bytes());
        buf.extend_from_slice(&trace.reentry_score.to_le_bytes());
        buf.extend_from_slice(&(trace.active_cycles as u64).to_le_bytes());
    }
}

fn read_active_reasoning_gap_snapshot(bytes: &[u8], cur: &mut usize) -> ActiveReasoningGapState {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_i64(b: &[u8], c: &mut usize) -> i64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        i64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    let gap_pressure = rd_f64(bytes, cur);
    let drive_score = rd_f64(bytes, cur);
    let persistence_score = rd_f64(bytes, cur);
    let closure_score = rd_f64(bytes, cur);
    let reentry_score = rd_f64(bytes, cur);
    let focus_slot_id = rd_i64(bytes, cur) as i8;
    let focus_edge_id = rd_i64(bytes, cur) as i16;
    let active_cycles = rd_u64(bytes, cur) as u32;
    let last_pair_seq = rd_u64(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let history_len = rd_u64(bytes, cur) as u8;
    let history_head = rd_u64(bytes, cur) as usize;
    let total_samples = rd_u64(bytes, cur);
    let mut history = [ActiveReasoningGapTrace::default(); crate::ACTIVE_REASONING_GAP_TRACE_CAP];
    for trace in history.iter_mut() {
        *trace = ActiveReasoningGapTrace {
            t_us: rd_u64(bytes, cur),
            gap_pressure: rd_f64(bytes, cur),
            closure_score: rd_f64(bytes, cur),
            reentry_score: rd_f64(bytes, cur),
            active_cycles: rd_u64(bytes, cur) as u32,
        };
    }

    ActiveReasoningGapState {
        gap_pressure,
        drive_score,
        persistence_score,
        closure_score,
        reentry_score,
        focus_slot_id,
        focus_edge_id,
        active_cycles,
        last_pair_seq,
        last_update_t_us,
        history_len,
        history_head,
        total_samples,
        history,
    }
}

// ─── v37: code-ingress trace substrate ───────────────────────────────────────

fn write_code_ingress(buf: &mut Vec<u8>, s: &CodeIngressState) {
    buf.extend_from_slice(&s.stream_count.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
    buf.extend_from_slice(&(s.recent_frames.len() as u32).to_le_bytes());
    for frame in &s.recent_frames {
        buf.extend_from_slice(&frame.frame_seq.to_le_bytes());
        buf.extend_from_slice(&frame.stream_context_id.to_le_bytes());
        buf.extend_from_slice(&frame.t_start_us.to_le_bytes());
        buf.extend_from_slice(&frame.t_end_us.to_le_bytes());
        buf.extend_from_slice(&frame.byte_len.to_le_bytes());
        buf.extend_from_slice(&frame.byte_hash.to_le_bytes());
        buf.extend_from_slice(&frame.byte_transition_energy.to_le_bytes());
        buf.extend_from_slice(&frame.byte_distribution_skew.to_le_bytes());
        buf.extend_from_slice(&frame.framing_validity.to_le_bytes());
        buf.extend_from_slice(&frame.evidence_ref.to_le_bytes());
        let sample_len = (frame.byte_sample_len as usize).min(CODE_INGRESS_BYTE_SAMPLE_CAP);
        buf.push(sample_len as u8);
        buf.extend_from_slice(&frame.byte_sample[..sample_len]);
    }

    buf.extend_from_slice(&(s.projection_candidates.len() as u32).to_le_bytes());
    for projection in &s.projection_candidates {
        buf.extend_from_slice(&projection.projection_ref.to_le_bytes());
        buf.extend_from_slice(&projection.frame_seq.to_le_bytes());
        buf.push(projection.decoder_kind_code);
        buf.extend_from_slice(&projection.confidence.to_le_bytes());
        buf.extend_from_slice(&projection.validity.to_le_bytes());
        buf.extend_from_slice(&projection.error_magnitude.to_le_bytes());
        buf.extend_from_slice(&projection.code_unit_count.to_le_bytes());
        buf.extend_from_slice(&projection.last_update_t_us.to_le_bytes());
    }

    buf.extend_from_slice(&(s.sequence_windows.len() as u32).to_le_bytes());
    for window in &s.sequence_windows {
        buf.extend_from_slice(&window.window_id.to_le_bytes());
        buf.extend_from_slice(&window.stream_context_id.to_le_bytes());
        buf.extend_from_slice(&window.start_frame_seq.to_le_bytes());
        buf.extend_from_slice(&window.end_frame_seq.to_le_bytes());
        buf.extend_from_slice(&window.t_start_us.to_le_bytes());
        buf.extend_from_slice(&window.t_end_us.to_le_bytes());
        buf.extend_from_slice(&window.recurrence_score.to_le_bytes());
        buf.extend_from_slice(&window.boundary_confidence.to_le_bytes());
        buf.extend_from_slice(&window.ambiguity_level.to_le_bytes());
    }
}

fn read_code_ingress_snapshot(bytes: &[u8], cur: &mut usize) -> CodeIngressState {
    fn rd_u8(b: &[u8], c: &mut usize) -> u8 {
        let v = b[*c];
        *c += 1;
        v
    }
    fn rd_u32(b: &[u8], c: &mut usize) -> u32 {
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        u32::from_le_bytes(a)
    }
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    let stream_count = rd_u32(bytes, cur);
    let last_update_t_us = rd_u64(bytes, cur);
    let n_frames = rd_u32(bytes, cur) as usize;
    let mut recent_frames = Vec::with_capacity(n_frames);
    for _ in 0..n_frames {
        let frame_seq = rd_u64(bytes, cur);
        let stream_context_id = rd_u64(bytes, cur);
        let t_start_us = rd_u64(bytes, cur);
        let t_end_us = rd_u64(bytes, cur);
        let byte_len = rd_u32(bytes, cur);
        let byte_hash = rd_u64(bytes, cur);
        let byte_transition_energy = rd_f64(bytes, cur);
        let byte_distribution_skew = rd_f64(bytes, cur);
        let framing_validity = rd_f64(bytes, cur);
        let evidence_ref = rd_u64(bytes, cur);
        let byte_sample_len = rd_u8(bytes, cur).min(CODE_INGRESS_BYTE_SAMPLE_CAP as u8);
        let mut byte_sample = [0u8; CODE_INGRESS_BYTE_SAMPLE_CAP];
        let sample_len = byte_sample_len as usize;
        byte_sample[..sample_len].copy_from_slice(&bytes[*cur..*cur + sample_len]);
        *cur += sample_len;
        recent_frames.push(CodeIngressFrameTrace {
            frame_seq,
            stream_context_id,
            t_start_us,
            t_end_us,
            byte_len,
            byte_hash,
            byte_transition_energy,
            byte_distribution_skew,
            framing_validity,
            evidence_ref,
            byte_sample,
            byte_sample_len,
        });
    }

    let n_projections = rd_u32(bytes, cur) as usize;
    let mut projection_candidates = Vec::with_capacity(n_projections);
    for _ in 0..n_projections {
        projection_candidates.push(CodeProjectionCandidate {
            projection_ref: rd_u64(bytes, cur),
            frame_seq: rd_u64(bytes, cur),
            decoder_kind_code: rd_u8(bytes, cur),
            confidence: rd_f64(bytes, cur),
            validity: rd_f64(bytes, cur),
            error_magnitude: rd_f64(bytes, cur),
            code_unit_count: rd_u32(bytes, cur),
            last_update_t_us: rd_u64(bytes, cur),
        });
    }

    let n_windows = rd_u32(bytes, cur) as usize;
    let mut sequence_windows = Vec::with_capacity(n_windows);
    for _ in 0..n_windows {
        sequence_windows.push(CodeSequenceWindowTrace {
            window_id: rd_u64(bytes, cur),
            stream_context_id: rd_u64(bytes, cur),
            start_frame_seq: rd_u64(bytes, cur),
            end_frame_seq: rd_u64(bytes, cur),
            t_start_us: rd_u64(bytes, cur),
            t_end_us: rd_u64(bytes, cur),
            recurrence_score: rd_f64(bytes, cur),
            boundary_confidence: rd_f64(bytes, cur),
            ambiguity_level: rd_f64(bytes, cur),
        });
    }

    CodeIngressState::from_snapshot(
        stream_count,
        recent_frames,
        projection_candidates,
        sequence_windows,
        last_update_t_us,
    )
}

fn write_learned_bias_summary(buf: &mut Vec<u8>, s: &LearnedBiasSummary) {
    buf.extend_from_slice(&(s.n_curves as u64).to_le_bytes());
    buf.extend_from_slice(&(s.n_recent_events as u64).to_le_bytes());
    buf.extend_from_slice(&s.total_events.to_le_bytes());
    buf.extend_from_slice(&s.max_abs_delta.to_le_bytes());
    buf.extend_from_slice(&(s.protected_axis_curves as u64).to_le_bytes());
    buf.extend_from_slice(&(s.ordinary_axis_curves as u64).to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}

fn read_learned_bias_summary(bytes: &[u8], cur: &mut usize) -> LearnedBiasSummary {
    fn rd_u64(b: &[u8], c: &mut usize) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    }
    fn rd_f64(b: &[u8], c: &mut usize) -> f64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    }

    LearnedBiasSummary {
        n_curves: rd_u64(bytes, cur) as u32,
        n_recent_events: rd_u64(bytes, cur) as u32,
        total_events: rd_u64(bytes, cur),
        max_abs_delta: rd_f64(bytes, cur),
        protected_axis_curves: rd_u64(bytes, cur) as u32,
        ordinary_axis_curves: rd_u64(bytes, cur) as u32,
        last_update_t_us: rd_u64(bytes, cur),
    }
}
