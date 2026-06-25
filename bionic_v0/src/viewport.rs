use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

use crate::action_consequence::ActionConsequenceState;
use crate::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, ActionOutputState,
    OutputChannelKind,
};
use crate::active_reasoning::ActiveReasoningGapState;
use crate::active_surface::ActiveSurfaceState;
use crate::adaptive_stability::AdaptiveStabilityState;
use crate::audio_raw::AudioRawBuffer;
use crate::auditory_visual::{AuditoryVisualState, ChannelSample, ReturnSample};
use crate::code_ingress::{CodeIngressFrameInput, CodeIngressState};
use crate::evidence_tension::EvidenceTensionState;
use crate::evolution_priors::BiasPriors;
use crate::framed_byte_egress::FramedByteEgressState;
use crate::language_binding::{BindingEvent, BindingTargetRef, BindingTrace, LanguageBindingState};
use crate::learned_bias::{LearnedBiasEventInput, LearnedBiasState};
use crate::efference_copy::EfferenceCopyState;
use crate::mark_disturbance::MarkDisturbanceState;
use crate::phot_predictor::{PhotSlotPredictor, VPC_FAMILY_PHOT};
use crate::prediction_pressure_field::PredictionPressureFieldState;
use crate::probe_law::ProbeLawState;
use crate::self_development::SelfDevelopmentState;
use crate::substrate_pressure::{compute_pressure, PressureSample};
use crate::vocal_imitation::VocalImitationState;
use crate::vocal_memory::VocalMemoryState;
use crate::vocal_organ::{VocalOrganPhase, VocalOrganState};
use crate::vocal_reflex::{VocalReflexDrive, VocalReflexState};
use crate::vocal_target::VocalTargetState;
use crate::vpc::{
    ErrAlignmentKind, HorizonKind, ObservationDomainKind, ObservationDomainRef, PredictionHorizon,
    PredictionPublicationTrace, PredictorLifecycleState, PublicationStatus, VpcContractState,
};
use crate::{
    ATTN_PRECISION_BIAS_MAX, AXIS_COUNT, BINDING_ATTN_ALPHA,
    BINDING_MAINTENANCE_DECAY_STEP, BINDING_MAINTENANCE_IDLE_US, BINDING_RESONANCE_THRESHOLD,
    CLAIM_CROSS_MODALITY_BASE, CLAIM_KIND_ALLOCATED, CLAIM_KIND_DROPPED, CLAIM_KIND_REUSED,
    CLAIM_LOG_ENERGY_BAND, CLAIM_REUSE_MIN_SCORE, CLAIM_SAME_MODALITY_BONUS, CLAIM_SURPRISE_BAND,
    CLAIM_W_ENERGY, CLAIM_W_MODALITY, CLAIM_W_SPATIAL, CLAIM_W_SURPRISE, CLAIM_W_TIME,
    CROSS_MODAL_SPATIAL_DIST, CROSS_MODAL_WINDOW_US, HEALTH_FAULT, HEALTH_OK, HEALTH_QUIET,
    HEALTH_STALE, HISTORY_CAP, MOD_MECH, MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, N_REL_EDGES,
    N_ROUTING_FOCUS_EDGES, N_ROUTING_FOCUS_SLOTS, PERSISTENCE_MAX_CYCLES, PERSISTENCE_MIN_CYCLES,
    PHOT_PRED_WARMUP_MIN, QUIET_ENERGY_THRESHOLD, QUIET_WINDOW_US, REASONING_HISTORY_ALPHA,
    REASONING_HISTORY_RING_CAP, REASONING_INTENSITY_THRESHOLD, REL_ALPHA, REL_PRED_ALPHA,
    REL_PRED_SURPRISE_ALPHA, REL_PRED_SURPRISE_MAX, REL_PRED_WARMUP_MIN, REPLAY_ALPHA,
    SAME_MODAL_SPATIAL_DIST, SLOT_IDLE_US, VIB_BANDS, VOCAL_TARGET_INGRESS_MIN_ENERGY,
    VOCAL_TARGET_WRITE_COUPLED_GUARD_US,
};

/// First-class viewport health state. Stored as a numeric atomic; observers
/// and tests always read this and never recompute thresholds.
///
/// PHOT ingress uses all four variants. VIB uses only Fault/Quiet/Ok.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum HealthState {
    Fault = HEALTH_FAULT, // 0
    Quiet = HEALTH_QUIET, // 1
    Ok = HEALTH_OK,       // 2
    Stale = HEALTH_STALE, // 3 — PHOT only: provider healthy but no frame arriving
}

impl HealthState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            HEALTH_FAULT => HealthState::Fault,
            HEALTH_OK => HealthState::Ok,
            HEALTH_STALE => HealthState::Stale,
            _ => HealthState::Quiet,
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub t_us: u64,
    pub value: f64,
    pub valid: bool,
}

pub struct AxisRing {
    pub buf: [Sample; HISTORY_CAP],
    pub write_idx: usize,
    pub total_written: u64,
}

impl AxisRing {
    pub fn new() -> Self {
        Self {
            buf: [Sample::default(); HISTORY_CAP],
            write_idx: 0,
            total_written: 0,
        }
    }

    pub fn push(&mut self, s: Sample) {
        self.buf[self.write_idx] = s;
        self.write_idx = (self.write_idx + 1) % HISTORY_CAP;
        self.total_written = self.total_written.wrapping_add(1);
    }

    pub fn last(&self, n: usize) -> Vec<Sample> {
        let n = n.min(HISTORY_CAP).min(self.total_written as usize);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (self.write_idx + HISTORY_CAP - 1 - i) % HISTORY_CAP;
            out.push(self.buf[idx]);
        }
        out
    }

    pub fn latest(&self) -> Option<Sample> {
        if self.total_written == 0 {
            None
        } else {
            let idx = (self.write_idx + HISTORY_CAP - 1) % HISTORY_CAP;
            Some(self.buf[idx])
        }
    }

    pub fn snapshot_linear(&self) -> Vec<Sample> {
        let mut out = vec![Sample::default(); HISTORY_CAP];
        for i in 0..HISTORY_CAP {
            let idx = (self.write_idx + i) % HISTORY_CAP;
            out[i] = self.buf[idx];
        }
        out
    }
}

pub struct PredictorState {
    pub a: f64,
    pub b: f64,
    pub surprise: f64,
    pub last_err: f64,
    pub total_updates: u64,
    pub mean_sq_err: f64,
}

impl PredictorState {
    pub fn new() -> Self {
        Self {
            a: 0.0,
            b: 0.0,
            surprise: 0.0,
            last_err: 0.0,
            total_updates: 0,
            mean_sq_err: 0.0,
        }
    }
}

pub struct HighLevelState {
    pub a: f64,
    pub last_avg_mse: f64,
    pub pred_mse: f64,
    pub meta_err: f64,
    pub modulation: f64,
}

impl Default for HighLevelState {
    fn default() -> Self {
        Self {
            a: 0.0,
            last_avg_mse: 0.0,
            pred_mse: 0.0,
            meta_err: 0.0,
            modulation: 1.0,
        }
    }
}

/// AmbientModalityState — per-modality low-precision monitor (Axiom 3,
/// AMBIENT tier). Carries no identity; strictly scalar.
pub struct AmbientModalityState {
    pub event_energy: RwLock<f64>,
    pub surprise: RwLock<f64>,
    pub last_event_t_us: AtomicU64,
    pub candidate_flag: AtomicBool,
}

impl AmbientModalityState {
    pub fn new() -> Self {
        Self {
            event_energy: RwLock::new(0.0),
            surprise: RwLock::new(0.0),
            last_event_t_us: AtomicU64::new(0),
            candidate_flag: AtomicBool::new(false),
        }
    }
}

/// AmbientModalitySnapshot — numeric-only view of one modality's ambient.
#[derive(Clone, Copy, Default)]
pub struct AmbientModalitySnapshot {
    pub event_energy: f64,
    pub surprise: f64,
    pub last_event_t_us: u64,
    pub candidate_flag: bool,
}

/// AmbientChannel — per-modality channels plus a shared `nomination_seq`.
///
/// Each modality writes only its own sub-channel. The nomination counter is
/// global: any rising-edge candidate bumps it once, so slot allocation sees a
/// single monotonic stream of "somewhere, something appeared" signals without
/// ambient ever producing identity.
pub struct AmbientChannel {
    pub nomination_seq: AtomicU64,
    pub vib: AmbientModalityState,
    pub phot: AmbientModalityState,
    pub mech: AmbientModalityState,
}

impl AmbientChannel {
    pub fn new() -> Self {
        Self {
            nomination_seq: AtomicU64::new(0),
            vib: AmbientModalityState::new(),
            phot: AmbientModalityState::new(),
            mech: AmbientModalityState::new(),
        }
    }
}

/// AmbientSnapshot — full read-out across all modality sub-channels.
#[derive(Clone, Copy, Default)]
pub struct AmbientSnapshot {
    pub nomination_seq: u64,
    pub vib: AmbientModalitySnapshot,
    pub phot: AmbientModalitySnapshot,
    pub mech: AmbientModalitySnapshot,
}

/// BandFamilyState — per-modality live-curve fragment.
///
/// Each modality writes its own `n_bands`-wide strip (observed / predicted /
/// err / surprise). Length is not constrained across modalities: VIB's band
/// count and PHOT's band count are independent.
#[derive(Clone, Debug)]
pub struct BandFamilyState {
    pub n_bands: u16,
    pub observed: Vec<f64>,
    pub predicted: Vec<f64>,
    pub err: Vec<f64>,
    pub surprise: Vec<f64>,
}

impl BandFamilyState {
    pub fn zeros(n_bands: u16) -> Self {
        let n = n_bands as usize;
        Self {
            n_bands,
            observed: vec![0.0; n],
            predicted: vec![0.0; n],
            err: vec![0.0; n],
            surprise: vec![0.0; n],
        }
    }
}

/// SpatialState — minimal 2D screen-space carrier for geometry-bearing
/// modalities (PHOT first; MECH later). Coordinates and spans are normalized
/// f64 in `[0.0, 1.0]`; the substrate assigns no coordinate-system name,
/// depth, or eye-specific channel. Interpretation belongs to the writer.
///
/// `precision` is the Axiom-3 LOD parameter for the position estimate itself
/// (how confident the adapter is about `pos_x/pos_y/span_x/span_y`), kept
/// orthogonal to band-family precisions. `valid=false` means "slot has a
/// spatial subtree but no current estimate" — distinct from `Option::None`
/// which means "no adapter has ever claimed a spatial target for this slot".
#[derive(Clone, Copy, Debug, Default)]
pub struct SpatialState {
    pub pos_x: f64,
    pub pos_y: f64,
    pub span_x: f64,
    pub span_y: f64,
    pub precision: f64,
    pub last_update_t_us: u64,
    pub valid: bool,
}

/// Slot-local spatial worldline fragment.
///
/// Numeric-only body substrate: it keeps the spatial estimate in shared time
/// by recording the previous sample, local velocity, prediction, and residual
/// error. This is not a tracker identity, scene parser, or spatial reasoner.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpatialWorldlineState {
    pub n_updates: u32,
    pub prev_t_us: u64,
    pub last_update_t_us: u64,
    pub prev_pos_x: f64,
    pub prev_pos_y: f64,
    pub predicted_pos_x: f64,
    pub predicted_pos_y: f64,
    pub velocity_x_per_s: f64,
    pub velocity_y_per_s: f64,
    pub err_x: f64,
    pub err_y: f64,
    pub err_norm: f64,
    pub last_dt_us: u64,
    pub valid: bool,
}

// ---------------------------------------------------------------------------
// Phase 5A: CrossModalFusionState — numeric-only, same-slot fusion subtree.
//
// Lives inside ObjectSlot. Adapter writes never touch this directly; they
// call Viewport::refresh_slot_fusion() after writing their own modality.
// All fields are numeric. No semantic labels, no tracking IDs.
// ---------------------------------------------------------------------------

/// Per-slot cross-modal fusion state. Tracks temporal alignment and
/// contribution weights for each modality. Reset with the slot.
#[derive(Clone, Copy, Debug, Default)]
pub struct CrossModalFusionState {
    /// Modality bitmap reflecting which modalities have contributed.
    pub modality_mask: u8,
    /// Wall-clock µs of the last fusion refresh.
    pub last_fused_t_us: u64,
    /// Wall-clock µs when VIB last wrote to this slot (0 = never).
    pub last_vib_t_us: u64,
    /// Wall-clock µs when PHOT last wrote to this slot (0 = never).
    pub last_phot_t_us: u64,
    /// [0,1] time-sync score: 1.0 when VIB and PHOT wrote simultaneously.
    pub temporal_alignment_score: f64,
    /// [0,1] overall fused confidence. Capped at 0.5 for single-modality.
    pub cross_modal_confidence: f64,
    /// [0,1] VIB freshness weight; 0 when VIB never wrote or is stale.
    pub vib_weight: f64,
    /// [0,1] PHOT freshness weight; 0 when PHOT never wrote or is stale.
    pub phot_weight: f64,
    /// Weighted mean of per-band surprise values across active modalities [0,1].
    pub fused_surprise: f64,
    /// True iff both VIB and PHOT are present with vib_weight > 0.01 and
    /// phot_weight > 0.01. Single-modality slots are never is_cross_modal.
    pub is_cross_modal: bool,
}

// ---------------------------------------------------------------------------
// Phase 5C: GlobalAttentionState — cross-slot, numeric-only.
//
// Written by Viewport::refresh_global_attention(). Observers render; they
// never recompute scores. `precision_bias` on each slot is a suggestion value
// only — it does NOT feed back into fusion formulas in this phase.
// ---------------------------------------------------------------------------

/// Viewport-level attention allocation state produced by a single pass over
/// all active slots. No semantic labels, no identity tracking.
#[derive(Clone, Copy, Debug)]
pub struct GlobalAttentionState {
    /// Slot index with the highest raw attention score; -1 when no active slot.
    pub focused_slot_id: i8,
    /// Active slot indices sorted by attention score descending. -1 = empty.
    pub top_slot_ids: [i8; N_OBJECT_SLOTS],
    /// Normalised attention shares indexed by slot id. Sums to 1 across active slots.
    pub attention_shares: [f64; N_OBJECT_SLOTS],
    /// Shannon entropy of the attention distribution; 0 = perfectly focused.
    pub attention_entropy: f64,
    /// Raw score of the focused slot (pre-normalisation); 0 when no active slot.
    pub focused_score: f64,
    /// Number of active slots considered in the last refresh.
    pub n_active: u8,
    /// Per-slot bias injected by binding resonance. Added to attention
    /// shares during refresh. [0, ATTN_PRECISION_BIAS_MAX].
    pub slot_biases: [f64; N_OBJECT_SLOTS],
    /// Endogenous activity: replay intensity during QUIET periods.
    /// Rises when reasoning history is replayed; decays otherwise. [0,1].
    pub replay_intensity: f64,
    /// Substrate time of last replay tick. 0 = never replayed.
    pub last_replay_t_us: u64,
}

impl Default for GlobalAttentionState {
    fn default() -> Self {
        Self {
            focused_slot_id: -1,
            top_slot_ids: [-1i8; N_OBJECT_SLOTS],
            attention_shares: [0.0f64; N_OBJECT_SLOTS],
            attention_entropy: 0.0,
            focused_score: 0.0,
            n_active: 0,
            slot_biases: [0.0f64; N_OBJECT_SLOTS],
            replay_intensity: 0.0,
            last_replay_t_us: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 5D: GlobalRelationState — cross-object temporal relation substrate.
//
// Written by Viewport::refresh_global_relations(). Observers render; they
// never recompute scores. All fields numeric-only; no semantic labels.
//
// Edge indexing: compact directed pair mapping for src ≠ dst.
//   edge_idx(src, dst) = src * (N_OBJECT_SLOTS-1) + (if dst < src { dst } else { dst-1 })
// Covers all N_OBJECT_SLOTS*(N_OBJECT_SLOTS-1) ordered pairs.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Phase 5E: Relation-level predictor state.
//
// RelationEdgePredState    — ephemeral AR-1 state, stored in Viewport
//                            (relation_pred_bank), never snapshot'd.
// RelationEdgePredictionState — output of the predictor, embedded in
//                            RelationEdgeState, snapshot-visible, observer-
//                            facing. Written by refresh_global_relation_predictor
//                            only; never touched by adapters or observers.
// ---------------------------------------------------------------------------

/// Snapshot-visible prediction output for one directed relation edge.
/// All fields numeric-only. `relation_err` and `relation_surprise` are 0
/// during the warmup window and must not be interpreted as high-confidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationEdgePredictionState {
    /// AR-1 EMA prediction of `co_change_score` for the next cycle.
    pub predicted_co_change: f64,
    /// AR-1 EMA prediction of `lead_lag_score` for the next cycle.
    pub predicted_lead_lag: f64,
    /// AR-1 EMA prediction of `predictive_score` for the next cycle.
    pub predicted_predictive: f64,
    /// AR-1 EMA prediction of `relation_confidence` for the next cycle.
    pub predicted_confidence: f64,
    /// RMS of the 4-dim prediction error at the last predictor tick.
    /// 0 during warmup (`warmup < REL_PRED_WARMUP_MIN`).
    pub relation_err: f64,
    /// EMA of `relation_err²`. Rises with sustained surprise, 0 during warmup.
    pub relation_surprise: f64,
    /// Wall-clock µs of the last predictor update. 0 = never run.
    pub last_predict_t_us: u64,
}

/// Ephemeral AR-1 predictor state for one directed relation edge.
/// Not snapshot'd. Reset to Default when the edge becomes inactive.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationEdgePredState {
    pub ema_co: f64,
    pub ema_lag: f64,
    pub ema_pred: f64,
    pub ema_conf: f64,
    /// EMA of relation_err² — feeds relation_surprise.
    pub ema_mse: f64,
    /// Number of active cycles seen since last reset. Gating warmup output.
    pub warmup: u32,
}

/// Bank of ephemeral relation predictor states, one per directed edge.
/// Parallel to `GlobalRelationState.edges` by index. Never snapshot'd.
pub struct RelationPredBank {
    pub states: [RelationEdgePredState; N_REL_EDGES],
}

impl Default for RelationPredBank {
    fn default() -> Self {
        Self {
            states: [RelationEdgePredState::default(); N_REL_EDGES],
        }
    }
}

/// One directed relation edge (src → dst). Carries EMA-smoothed numeric scores.
/// All score fields are updated only when both slots are active and attended.
/// When inactive they decay toward 0 at rate REL_ALPHA per refresh.
#[derive(Clone, Copy, Debug, Default)]
pub struct RelationEdgeState {
    pub src_slot_id: u8,
    pub dst_slot_id: u8,
    /// True iff both slots were active + attended in the last refresh.
    pub active: bool,
    /// [0,1] EMA of `1 - |bias_src - bias_dst|`: similarity of stability trajectories.
    pub co_change_score: f64,
    /// [-1,1] EMA of `(t_dst − t_src) / CROSS_MODAL_WINDOW_US`.
    /// Positive → src consistently precedes dst; negative → src lags dst.
    pub lead_lag_score: f64,
    /// [0,1] EMA of `clamp01(err_src * err_dst * max(0, lag_obs))`.
    /// Captures: src error-spike before dst error-spike.
    pub predictive_score: f64,
    /// [0,1] EMA of `clamp01(4 * share_src * share_dst)`.
    /// Low when either slot has negligible attention.
    pub relation_confidence: f64,
    /// [0,1] EMA of spatial closeness projected from current valid slot
    /// spatial states. This is a relation view over shared coordinates, not a
    /// stored relation entity.
    pub spatial_distance_score: f64,
    /// [0,1] EMA of normalized 2D extent overlap projected from slot spatial
    /// states. Zero when no valid spatial subtree exists on either side.
    pub spatial_overlap_score: f64,
    pub last_update_t_us: u64,
    /// Phase 5E: predictor output — snapshot-visible, observer-facing.
    pub pred: RelationEdgePredictionState,
}

/// Viewport-level cross-object temporal relation state.
/// Contains all directed edges; inactive entries have active=false and decayed scores.
#[derive(Clone, Copy, Debug)]
pub struct GlobalRelationState {
    pub edges: [RelationEdgeState; N_REL_EDGES],
    /// Number of directed edges where both slots were active + attended.
    pub n_active_edges: u8,
    pub last_update_t_us: u64,
}

impl Default for GlobalRelationState {
    fn default() -> Self {
        Self {
            edges: [RelationEdgeState::default(); N_REL_EDGES],
            n_active_edges: 0,
            last_update_t_us: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 5F: GlobalReasoningState — cross-object routing / reasoning substrate.
//
// Written by Viewport::refresh_global_reasoning(). Observers render; they
// never recompute scores. All fields numeric-only; no semantic labels.
//
// Refresh order: attention → relations → relation predictor → reasoning → snapshot.
// Reads: global_attention.attention_shares, global_relations.edges (scores + pred).
// Writes: global_reasoning only.
// ---------------------------------------------------------------------------

/// Viewport-level cross-object reasoning and routing state.
///
/// All fields numeric-only. Slot/edge references are integer indices; no semantic
/// labels. Fields are bounded: global_surprise_load and coordinated_event_score
/// in [0,1]; network_instability_score in [0,2].
#[derive(Clone, Copy, Debug)]
pub struct GlobalReasoningState {
    /// Weighted-average of relation_err across active edges, weighted by
    /// relation_confidence × attention_product. Rises when the relation network
    /// carries more prediction surprise than expected. Range [0,1].
    pub global_surprise_load: f64,

    /// Weighted-average co_change_score across active edges, weighted by
    /// relation_confidence × attention_product. Near 1 when all attended
    /// objects move together; near 0 when they diverge. Range [0,1].
    pub coordinated_event_score: f64,

    /// Weighted-average of (relation_err + |lead_lag_score|) across active
    /// edges, weighted by attention_product. Rises when the relation network
    /// is structurally shifting. Range [0,2].
    pub network_instability_score: f64,

    /// Top-N_ROUTING_FOCUS_SLOTS slot ids ranked by accumulated routing score
    /// (= sum of routing_weight over all edges touching that slot).
    /// -1 = empty. Descending order.
    pub routing_focus_slot_ids: [i8; N_ROUTING_FOCUS_SLOTS],

    /// Top-N_ROUTING_FOCUS_EDGES directed edge indices ranked by routing weight
    /// (= relation_err × relation_confidence × attention_product). -1 = empty.
    pub routing_focus_edge_ids: [i16; N_ROUTING_FOCUS_EDGES],

    /// Normalised routing weights parallel to routing_focus_edge_ids.
    /// Sum ≤ 1 (may be < 1 if fewer than N_ROUTING_FOCUS_EDGES edges are active).
    pub routing_weights: [f64; N_ROUTING_FOCUS_EDGES],

    pub last_update_t_us: u64,
}

impl Default for GlobalReasoningState {
    fn default() -> Self {
        Self {
            global_surprise_load: 0.0,
            coordinated_event_score: 0.0,
            network_instability_score: 0.0,
            routing_focus_slot_ids: [-1i8; N_ROUTING_FOCUS_SLOTS],
            routing_focus_edge_ids: [-1i16; N_ROUTING_FOCUS_EDGES],
            routing_weights: [0.0f64; N_ROUTING_FOCUS_EDGES],
            last_update_t_us: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 5G: GlobalReasoningHistoryState — temporal memory substrate.
//
// Written by Viewport::refresh_global_reasoning_history(). Observers render.
// Reads: global_reasoning (per cycle signatures).
// Writes: history ring buffer + memory summary.
//
// Provides numeric-only temporal memory: novelty, persistence, stability,
// recurrence. No semantic labels. Bounded [0,1] outputs.
// ---------------------------------------------------------------------------

/// Compact signature of a single reasoning cycle for history storage.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReasoningHistoryEntry {
    /// Core surprise metric from that cycle.
    pub surprise_load: f64,
    /// Coordinated event score from that cycle.
    pub coordinated_event: f64,
    /// Network instability from that cycle.
    pub network_instability: f64,
    /// Top routing focus slot id (-1 if none).
    pub top_slot_id: i8,
    /// Top routing focus edge id (-1 if none).
    pub top_edge_id: i16,
    /// Timestamp of the cycle.
    pub t_us: u64,
}

/// E-1: Detail recovered by re-descending from a recalled gist (公理5).
///
/// Numeric-only. Carries the back-link the gist held (slot/edge/t_us) plus the
/// low-layer detail recovered by following it down. `detail_present` is true
/// when the indexed slot still holds a live low-layer signature (precise, not
/// faded) — false would mean the index outlived its body in the low layer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RecallDetail {
    /// Source slot the gist indexed (>= 0).
    pub slot_id: i8,
    /// Source relation edge the gist indexed (-1 if none).
    pub edge_id: i16,
    /// Timestamp of the recalled gist cycle.
    pub gist_t_us: u64,
    /// Gist surprise signature (the recalled "feeling" of the memory).
    pub surprise_load: f64,
    /// True when the indexed slot still holds live low-layer detail.
    pub detail_present: bool,
    /// Low-layer slot's last update time (how fresh the recovered detail is).
    pub slot_last_update_t_us: u64,
    /// Which modalities wrote the recovered low-layer detail (bitmask).
    pub source_modality: u8,
    /// R1: true when the detail was RECONSTRUCTED from the target's learned
    /// cross-layer relation rule (run forward over relation neighbours), not
    /// fetched raw from the live slot. "用关系把细节推断出来."
    pub reconstructed: bool,
    /// The reconstructed target object_err (0.0 when not reconstructed).
    pub reconstructed_err: f64,
}

/// Ephemeral ring buffer holding recent reasoning cycle signatures.
/// Runtime-only; not directly serialized. Summary is serialized.
pub struct GlobalReasoningHistoryState {
    /// Ring buffer of recent entries.
    pub ring: [ReasoningHistoryEntry; REASONING_HISTORY_RING_CAP],
    /// Current write index in the ring.
    pub write_idx: usize,
    /// Number of valid entries in the ring (0 to REASONING_HISTORY_RING_CAP).
    pub count: usize,
    /// Running EMA of surprise_load for novelty baseline.
    pub surprise_ema: f64,
    /// Previous cycle's top slot id for stability tracking.
    pub last_top_slot: i8,
    /// Previous cycle's top edge id for stability tracking.
    pub last_top_edge: i16,
    /// Consecutive cycles with same top slot.
    pub slot_unchanged_count: u32,
    /// Consecutive cycles with high-intensity reasoning.
    pub high_intensity_count: u32,
}

impl GlobalReasoningHistoryState {
    pub fn new() -> Self {
        Self {
            ring: [ReasoningHistoryEntry::default(); REASONING_HISTORY_RING_CAP],
            write_idx: 0,
            count: 0,
            surprise_ema: 0.0,
            last_top_slot: -1,
            last_top_edge: -1,
            slot_unchanged_count: 0,
            high_intensity_count: 0,
        }
    }

    /// Push a new entry into the ring buffer.
    pub fn push(&mut self, entry: ReasoningHistoryEntry) {
        self.ring[self.write_idx] = entry;
        self.write_idx = (self.write_idx + 1) % REASONING_HISTORY_RING_CAP;
        if self.count < REASONING_HISTORY_RING_CAP {
            self.count += 1;
        }
    }

    /// Get the i-th most recent entry (0 = latest).
    pub fn get_recent(&self, i: usize) -> Option<&ReasoningHistoryEntry> {
        if i >= self.count {
            return None;
        }
        let idx =
            (self.write_idx + REASONING_HISTORY_RING_CAP - 1 - i) % REASONING_HISTORY_RING_CAP;
        Some(&self.ring[idx])
    }
}

/// Snapshot-visible numeric summary of temporal memory patterns.
/// All fields bounded [0,1], numeric-only, no semantic labels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlobalReasoningMemorySummary {
    /// How novel the current reasoning pattern is vs recent history.
    /// High when current surprise deviates from EMA baseline.
    /// Range [0,1].
    pub novelty_score: f64,

    /// How long the current high-intensity pattern has persisted.
    /// Rises when surprise/instability stays above threshold.
    /// Range [0,1].
    pub persistence_score: f64,

    /// Stability of routing focus over recent cycles.
    /// High when top slot/edge remains unchanged.
    /// Range [0,1].
    pub routing_stability_score: f64,

    /// Similarity of current pattern to recent history entries.
    /// High when current signature matches a previous entry.
    /// Range [0,1].
    pub pattern_recurrence_score: f64,

    /// How full the history ring buffer is.
    /// Range [0,1].
    pub history_fill_ratio: f64,

    /// How long the current dominant focus has persisted.
    /// Normalized count of consecutive same-focus cycles.
    /// Range [0,1].
    pub dominant_focus_persistence: f64,

    /// Timestamp of last history update.
    pub last_update_t_us: u64,
}

// Rejected phase-5/6 naming layers were removed by cleanup spec 2026-04-20.
// They made labeling a resource gate over substrate and stamped objects /
// edges with discrete internal ids.

/// Compact directed-pair index: src ≠ dst, both in 0..N_OBJECT_SLOTS.
fn edge_idx(src: usize, dst: usize) -> usize {
    debug_assert!(src != dst);
    debug_assert!(src < N_OBJECT_SLOTS && dst < N_OBJECT_SLOTS);
    if dst < src {
        src * (N_OBJECT_SLOTS - 1) + dst
    } else {
        src * (N_OBJECT_SLOTS - 1) + dst - 1
    }
}

const VPC_FAMILY_RELATION: u8 = 3;

// Helpers used by refresh_slot_fusion. Not methods to avoid borrow conflicts.

fn freshness_weight(last_t_us: u64, now_us: u64) -> f64 {
    if last_t_us == 0 {
        return 0.0;
    }
    let dt = now_us.saturating_sub(last_t_us);
    clamp01(1.0 - dt as f64 / SLOT_IDLE_US as f64)
}

fn mean_band_surprise(fam: Option<&BandFamilyState>) -> f64 {
    match fam {
        None => 0.0,
        Some(f) if f.surprise.is_empty() => 0.0,
        Some(f) => f.surprise.iter().sum::<f64>() / f.surprise.len() as f64,
    }
}

// ---------------------------------------------------------------------------
// Phase 3C: EventEvidence contract + SlotClaim decision.
//
// Adapters submit numeric-only event evidence to the viewport. The viewport
// is the sole legitimate slot-claim authority: adapters never call
// `find_active_slot_for` + `allocate_slot` directly. See
// `Viewport::claim_slot_for_event` for the decision rule.
// ---------------------------------------------------------------------------

fn clamp01(x: f64) -> f64 {
    if !x.is_finite() {
        0.0
    } else if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

// Fold a stream_context_id (arbitrary u64) into [0,1] via splitmix64 bit-mixing
// so that nearby ids spread uniformly across the CODE_POS axis range.
fn code_pos_from_ctx(x: u64) -> f64 {
    let h = x ^ (x >> 30);
    let h = h.wrapping_mul(0xbf58476d1ce4e5b9u64);
    let h = h ^ (h >> 27);
    let h = h.wrapping_mul(0x94d049bb133111ebu64);
    let h = h ^ (h >> 31);
    (h >> 11) as f64 / (1u64 << 53) as f64
}

fn ambient_energy_for(amb: &AmbientSnapshot, modality_bit: u8) -> f64 {
    match modality_bit {
        MOD_VIB => amb.vib.event_energy,
        MOD_PHOT => amb.phot.event_energy,
        MOD_MECH => amb.mech.event_energy,
        _ => 0.0,
    }
}

fn ambient_surprise_for(amb: &AmbientSnapshot, modality_bit: u8) -> f64 {
    match modality_bit {
        MOD_VIB => amb.vib.surprise,
        MOD_PHOT => amb.phot.surprise,
        MOD_MECH => amb.mech.surprise,
        _ => 0.0,
    }
}

/// Log-scale consistency: both values treated as non-negative energies.
fn energy_consistency(ev_energy: f64, slot_energy: f64) -> f64 {
    if slot_energy <= 0.0 {
        return 0.5;
    }
    let eps = 1e-12_f64;
    let a = (ev_energy.max(0.0) + eps).ln();
    let b = (slot_energy.max(0.0) + eps).ln();
    let d = (a - b).abs();
    clamp01(1.0 - d / CLAIM_LOG_ENERGY_BAND.max(1e-9))
}

fn surprise_consistency(ev_surprise: f64, slot_surprise: f64) -> f64 {
    let d = (ev_surprise - slot_surprise).abs();
    clamp01(1.0 - d / CLAIM_SURPRISE_BAND.max(1e-9))
}

/// Optional spatial component of an event. Normalized [0,1] coordinates; the
/// viewport assigns no coordinate-system name.
#[derive(Clone, Copy, Debug)]
pub struct SpatialEvidence {
    pub pos_x: f64,
    pub pos_y: f64,
    pub span_x: f64,
    pub span_y: f64,
    pub precision: f64,
}

/// The minimum adapter -> viewport event message. Carries low-level numbers
/// only. Must not carry semantic labels, slot suggestions, or foreign-
/// modality state.
#[derive(Clone, Copy, Debug)]
pub struct EventEvidence {
    pub modality_bit: u8,
    pub event_t_us: u64,
    pub event_energy: f64,
    pub surprise: f64,
    pub spatial: Option<SpatialEvidence>,
    /// Adapter's last-seen shared nomination_seq (viewport already owns the
    /// counter; this is only used to stamp a newly-allocated slot).
    pub nomination_seq_hint: u64,
}

/// Outcome of `Viewport::claim_slot_for_event`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotClaim {
    /// An active slot was reused. The adapter should now write its modality
    /// payload (band family, spatial) with this id.
    Reused(usize),
    /// A new slot was allocated and seeded. The adapter should now write its
    /// modality payload with this id.
    Allocated(usize),
    /// No reusable slot matched and every slot is full. Adapter must drop
    /// the evidence; viewport state is unchanged.
    Dropped,
}

impl SlotClaim {
    pub fn slot_id(self) -> Option<usize> {
        match self {
            SlotClaim::Reused(i) | SlotClaim::Allocated(i) => Some(i),
            SlotClaim::Dropped => None,
        }
    }
}

/// Per-candidate numeric score produced by `score_slot_for_event`. All fields
/// are in `[0, 1]`; `total` is the weighted sum and is what the best-match
/// decision rule compares across candidates. `None` from the scorer means a
/// hard gate rejected the candidate.
#[derive(Clone, Copy, Debug, Default)]
pub struct CandidateScore {
    pub slot_idx: u16,
    pub total: f64,
    pub time: f64,
    pub spatial: f64,
    pub modality: f64,
    pub energy: f64,
    pub surprise: f64,
}

/// Debug state written by `claim_slot_for_event` on every call. Numeric-only
/// and observer-facing: observers render these fields but never recompute
/// scores. Adapters do not read this field.
#[derive(Clone, Copy, Debug, Default)]
pub struct LastClaimDebug {
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

/// ObjectSlot — container for one or more modality sub-states (Axiom 1).
///
/// Not a semantic object. Not a knowledge-base entry. Slot identity is the
/// array index and `nomination_seq`; modality data is attached via optional
/// sub-trees keyed by modality, never by name.
pub struct ObjectSlot {
    pub active: bool,
    pub last_update_t_us: u64,
    pub source_modality: u8, // bitmap (MOD_VIB | MOD_PHOT | MOD_MECH)
    pub nomination_seq: u64,
    pub vib: Option<BandFamilyState>,
    pub phot: Option<BandFamilyState>,
    pub mech: Option<BandFamilyState>,
    pub spatial: Option<SpatialState>,
    pub spatial_worldline: Option<SpatialWorldlineState>,
    /// Phase 4C: slot-local PHOT predictor. Ephemeral — not snapshot'd.
    /// Reset when the slot is cleared by sweep_slots. None until first PHOT write.
    pub phot_pred: Option<PhotSlotPredictor>,
    /// Phase 5A: cross-modal fusion subtree. Numeric-only. Reset with slot.
    /// Populated by Viewport::refresh_slot_fusion(); adapters never write here.
    pub fusion: Option<CrossModalFusionState>,
    /// Phase 5C: normalised attention share (suggestion only; does NOT modify
    /// fusion). Written by refresh_global_attention; 0.0 by default.
    pub attention_share: f64,
    /// Phase 5C: precision bias suggestion = (2 * attention_share).clamp(0,2).
    /// Purely observable in this phase — does NOT feed back into fusion.
    pub precision_bias: f64,
}

impl ObjectSlot {
    pub fn empty() -> Self {
        Self {
            active: false,
            last_update_t_us: 0,
            source_modality: 0,
            nomination_seq: 0,
            vib: None,
            phot: None,
            mech: None,
            spatial: None,
            spatial_worldline: None,
            phot_pred: None,
            fusion: None,
            attention_share: 0.0,
            precision_bias: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ObjectSlotSnapshot {
    pub active: bool,
    pub last_update_t_us: u64,
    pub source_modality: u8,
    pub nomination_seq: u64,
    pub vib: Option<BandFamilyState>,
    pub phot: Option<BandFamilyState>,
    pub mech: Option<BandFamilyState>,
    pub spatial: Option<SpatialState>,
    pub spatial_worldline: Option<SpatialWorldlineState>,
    /// Phase 5A: cross-modal fusion state snapshot. None until first refresh.
    pub fusion: Option<CrossModalFusionState>,
    /// Phase 5C: normalised attention share suggestion. 0.0 until first refresh.
    pub attention_share: f64,
    /// Phase 5C: precision bias suggestion (2 * share, clamped [0,2]).
    pub precision_bias: f64,
}

fn update_spatial_worldline(
    prev_spatial: Option<SpatialState>,
    prev_worldline: Option<SpatialWorldlineState>,
    t_us: u64,
    pos_x: f64,
    pos_y: f64,
) -> SpatialWorldlineState {
    let Some(prev) = prev_spatial.filter(|sp| sp.valid) else {
        return SpatialWorldlineState {
            n_updates: 1,
            prev_t_us: 0,
            last_update_t_us: t_us,
            prev_pos_x: pos_x,
            prev_pos_y: pos_y,
            predicted_pos_x: pos_x,
            predicted_pos_y: pos_y,
            velocity_x_per_s: 0.0,
            velocity_y_per_s: 0.0,
            err_x: 0.0,
            err_y: 0.0,
            err_norm: 0.0,
            last_dt_us: 0,
            valid: true,
        };
    };

    let dt_us = t_us.saturating_sub(prev.last_update_t_us);
    let dt_s = (dt_us as f64 / 1_000_000.0).max(1e-9);
    let prev_vx = prev_worldline.map(|wl| wl.velocity_x_per_s).unwrap_or(0.0);
    let prev_vy = prev_worldline.map(|wl| wl.velocity_y_per_s).unwrap_or(0.0);
    let predicted_pos_x = (prev.pos_x + prev_vx * dt_s).clamp(0.0, 1.0);
    let predicted_pos_y = (prev.pos_y + prev_vy * dt_s).clamp(0.0, 1.0);
    let err_x = pos_x - predicted_pos_x;
    let err_y = pos_y - predicted_pos_y;

    SpatialWorldlineState {
        n_updates: prev_worldline
            .map(|wl| wl.n_updates.saturating_add(1))
            .unwrap_or(2),
        prev_t_us: prev.last_update_t_us,
        last_update_t_us: t_us,
        prev_pos_x: prev.pos_x,
        prev_pos_y: prev.pos_y,
        predicted_pos_x,
        predicted_pos_y,
        velocity_x_per_s: (pos_x - prev.pos_x) / dt_s,
        velocity_y_per_s: (pos_y - prev.pos_y) / dt_s,
        err_x,
        err_y,
        err_norm: (err_x * err_x + err_y * err_y).sqrt().min(1.0),
        last_dt_us: dt_us,
        valid: true,
    }
}

fn spatial_distance_score(src: SpatialState, dst: SpatialState) -> f64 {
    let dx = src.pos_x - dst.pos_x;
    let dy = src.pos_y - dst.pos_y;
    let d = (dx * dx + dy * dy).sqrt();
    clamp01(1.0 - d / 2.0_f64.sqrt())
}

fn spatial_overlap_score(src: SpatialState, dst: SpatialState) -> f64 {
    let src_left = src.pos_x - src.span_x * 0.5;
    let src_right = src.pos_x + src.span_x * 0.5;
    let src_top = src.pos_y - src.span_y * 0.5;
    let src_bottom = src.pos_y + src.span_y * 0.5;

    let dst_left = dst.pos_x - dst.span_x * 0.5;
    let dst_right = dst.pos_x + dst.span_x * 0.5;
    let dst_top = dst.pos_y - dst.span_y * 0.5;
    let dst_bottom = dst.pos_y + dst.span_y * 0.5;

    let overlap_w = (src_right.min(dst_right) - src_left.max(dst_left)).max(0.0);
    let overlap_h = (src_bottom.min(dst_bottom) - src_top.max(dst_top)).max(0.0);
    let overlap_area = overlap_w * overlap_h;
    let src_area = (src.span_x * src.span_y).max(1e-9);
    let dst_area = (dst.span_x * dst.span_y).max(1e-9);
    clamp01(overlap_area / src_area.min(dst_area))
}

/// Engineering telemetry for the vocal loop (体检). Pure counters/last-values
/// dumped to a numeric trace file by the body's checkpoint ticker; never read
/// back into any law. Answers, with new code actually running: does the body
/// burst at all, does heard-self arrive, do evidence-carrying closures happen,
/// and do efficacy/maturity actually move.
#[derive(Clone, Copy, Debug, Default)]
pub struct VocalLoopDiag {
    /// tick_vocal_drive_field invocations.
    pub outlet_ticks: u64,
    /// Bursts the inner-pressure reflex actually fired.
    pub bursts: u64,
    /// Last raw drive pressure seen at the outlet.
    pub last_drive_pressure: f64,
    /// observe_vocal_reflex_closure invocations.
    pub closures: u64,
    /// ... of which arrived with vocalized=true.
    pub closures_vocalized: u64,
    /// ... of which carried heard-self evidence (acoustic_competence=Some).
    pub closures_with_evidence: u64,
    /// Last heard-self 16-band energy sum at the closure sink.
    pub last_self_bands_sum: f64,
    /// Last acoustic fit (1-shape_dist) on an evidence-carrying closure.
    pub last_fit: f64,

    // ── Phase 0 diagnosis (2026-06-12) ──────────────────────────────────────
    // The servo/learning path is gated on `self_voiced`, NOT on the closure
    // sink — but `last_self_bands_sum`/`last_fit` above only update inside the
    // closure sink, which fires ~0 times (self_match>0.85 needs a ≈22ms window).
    // So the live learning path was INVISIBLE to telemetry. These fields capture
    // it directly each tick, plus the closure-gate components, so a stalled loop
    // reveals exactly which link is starved. Observer-side only.
    /// Region-B (external target / imitation) learning-path tick count.
    pub region_b_ticks: u64,
    /// Did the self-hearing path register voiced output this tick?
    pub last_self_voiced: bool,
    /// Real 64-mel (or 16-band fallback) self↔target shape distance.
    pub last_shape_dist: f64,
    /// The gap actually fed to imitation learning (shape_dist if voiced else parametric).
    pub last_learning_gap: f64,
    /// Live target energy at the outlet (region B only runs if ≥0.01).
    pub last_target_active_energy: f64,
    /// Σ target_mel64 — ZERO means the fine fingerprint path is starved
    /// (parametric feeding) and shape_dist silently falls back to coarse bands.
    pub last_target_mel64_sum: f64,
    /// Σ target_bands (coarse 16-band target contour).
    pub last_target_bands_sum: f64,
    /// Servo committed corrections (steering, not just credit).
    pub last_servo_pitch_off: f64,
    pub last_servo_res_off: f64,
    /// Closure-gate components (from compute_vib_self_match): which one fails?
    pub last_self_match: f64,
    pub last_organ_phase: u8,
    pub last_breath_drive: f64,
    pub last_since_vocal_us: u64,
    /// Phase 2: number of target→correction entries the recall memory has
    /// learned. >0 confirms content-addressed recall is populating live.
    pub recall_entries: u64,
}

pub struct Viewport {
    pub master_tick: AtomicU64,
    pub clock_us: AtomicU64,
    pub start_us: AtomicU64,
    /// Stage 1 (recursive PC depth): counts residual-tower sweeps. Used to give
    /// each ascending tier a SLOWER cadence (tier L ticks once per 2^(L-2)
    /// sweeps). This is a pure rate channel (clock-like), not a semantic gate:
    /// it makes higher abstraction run on a slower timescale (timescale
    /// emergence) and is the groundwork for the Stage-2 R3 invariant (a downward
    /// prior must be a slow contextual bias, never a within-step gradient).
    pub tower_sweep_count: AtomicU64,

    pub source_writes: AtomicU64,
    pub predictor_ticks: AtomicU64,
    pub snapshot_writes: AtomicU64,
    pub snapshot_fail_count: AtomicU64,

    pub hearing_fault: AtomicBool,
    pub hearing_fault_count: AtomicU64,
    /// Tier-0 thread-fault axis: a tick panic absorbed by catch_unwind, or a
    /// rhythm thread observed dead by the main-loop watchdog, increments the
    /// count. Neutral structural fact — no payload, no text, no valence.
    pub thread_fault: AtomicBool,
    pub thread_fault_count: AtomicU64,
    pub last_obs_t_us: AtomicU64,

    /// First-class viewport health. Numeric-only; observers render, never
    /// recompute. Written by `update_health_state`.
    pub health_state: AtomicU8,

    /// Phase 4A: PHOT-ingress health. Independent of the VIB-biased
    /// `health_state` above. Written by the PHOT adapter each tick; observers
    /// render the number and never recompute it.
    pub phot_health: AtomicU8,

    /// Phase 4C.1: wall-clock time (µs) when the PHOT provider was first
    /// confirmed healthy. Set once by the adapter; 0 = not yet started.
    pub phot_started_t_us: AtomicU64,

    /// Phase 4C.1: wall-clock time (µs) of the most recently received camera
    /// frame. Updated on every frame; 0 = no frame ever delivered.
    pub phot_last_frame_t_us: AtomicU64,

    /// Shared birth-sensitivity curves. Numeric physiology only; not a controller
    /// and not a source of world meanings.
    pub birth_sensitivity: Arc<BiasPriors>,

    pub observed: Vec<RwLock<AxisRing>>,
    pub predicted: Vec<RwLock<AxisRing>>,
    pub err_stream: Vec<RwLock<AxisRing>>,
    /// B1 (VPC mechanism 5): per-axis prior channel. A high (cross-layer)
    /// predictor writes a prior here; the low-layer predictor reads it and adds
    /// it to its prediction as an INPUT only. Bit-packed f64. Default 0 = no
    /// prior. The prior must never enter the a/b coefficient update (that would
    /// be backprop).
    pub prior_signal: Vec<AtomicU64>,

    pub predictors: Vec<RwLock<PredictorState>>,
    pub hl: RwLock<HighLevelState>,

    pub ambient: AmbientChannel,
    pub slots: Vec<RwLock<ObjectSlot>>,

    /// Phase 3D: last claim outcome — numeric, observer-facing.
    pub last_claim_debug: RwLock<LastClaimDebug>,
    pub claim_seq: AtomicU64,

    /// Phase 5C: viewport-level attention allocation. Written by
    /// refresh_global_attention; observers render via read_global_attention.
    pub global_attention: RwLock<GlobalAttentionState>,

    /// Phase 5D: cross-object temporal relation substrate. Written by
    /// refresh_global_relations; observers render via read_global_relations.
    pub global_relations: RwLock<GlobalRelationState>,

    /// Phase 5E: ephemeral AR-1 predictor bank for relation edges. Parallel to
    /// global_relations.edges by index. Not snapshot'd; reset per-edge when the
    /// edge goes inactive. Written by refresh_global_relation_predictor.
    pub relation_pred_bank: RwLock<RelationPredBank>,

    /// Phase 5F: cross-object reasoning / routing substrate. Written by
    /// refresh_global_reasoning; observers render via read_global_reasoning.
    pub global_reasoning: RwLock<GlobalReasoningState>,

    /// Phase 5G: temporal memory substrate for global reasoning. Written by
    /// refresh_global_reasoning_history; observers render summary via
    /// read_global_reasoning_memory_summary.
    pub global_reasoning_history: RwLock<GlobalReasoningHistoryState>,

    /// Phase 5G: snapshot-visible summary of temporal memory patterns.
    /// Written by refresh_global_reasoning_history.
    pub global_reasoning_memory_summary: RwLock<GlobalReasoningMemorySummary>,

    /// Runtime active-gap pressure over reasoning history and return pairing.
    pub active_reasoning_gap: RwLock<ActiveReasoningGapState>,

    /// Continuous prediction pressure field. Numeric wave substrate only;
    /// one dominant focus may feed active probe pressure.
    pub prediction_pressure_field: RwLock<PredictionPressureFieldState>,

    /// Body-side self-development substrate. Numeric growth evidence only;
    /// no fixed world content or observer interpretation.
    pub self_development: RwLock<SelfDevelopmentState>,

    /// Parallel focus traces with one active aperture and one bounded echo.
    pub active_surface: RwLock<ActiveSurfaceState>,

    /// Language binding remains an observation-side trace substrate.
    /// It references existing object / relation / time structures but never
    /// decides whether those structures exist.
    pub language_binding: RwLock<LanguageBindingState>,

    /// Code ingress trace substrate (v37). Engineering name only: byte
    /// disturbances enter body time as numeric traces, not text meaning.
    pub code_ingress: RwLock<CodeIngressState>,

    /// VPC contract trace substrate — timing / provenance only.
    /// Tracks publication and open-prediction metadata without altering
    /// observation truth or granting any ontological authority.
    pub vpc_contract: RwLock<VpcContractState>,

    // Rejected naming subtrees were removed by cleanup spec 2026-04-20.
    // Substrate now terminates at global reasoning history / memory summary.
    /// Action-output substrate — body's write-out side (v25).
    /// Numeric-only; no world content; no language gate.
    pub action_output: RwLock<ActionOutputState>,
    /// Monotonic counter incremented on every Begin event. Consumed by
    /// action_output.rs to stamp current_write_seq on each new write trace.
    pub action_output_write_seq: AtomicU64,
    /// Monotonic counter for stable window_id assignment. Parallels action_output_write_seq.
    pub action_consequence_window_seq: AtomicU64,
    /// Monotonic counter for stable pair_seq assignment. Pair sequence is an
    /// engineering locator for downstream cursors, not a consequence meaning.
    pub action_consequence_pair_seq: AtomicU64,
    /// Cursor for life-closure producer consumption of action-consequence pairs.
    /// Engineering locator only; keeps producer ticks idempotent.
    pub life_closure_pair_producer_cursor: AtomicU64,
    /// Cursor for life-closure producer consumption of framed-byte returns.
    /// Engineering locator only; keeps producer ticks idempotent.
    pub life_closure_framed_return_producer_cursor: AtomicU64,
    /// Timestamp cursor (t_us) of the last CODE-axis observation processed by
    /// tick_evidence_tension_from_code_ingress.  Engineering locator only;
    /// keeps producer ticks idempotent.  Not persisted in snapshots.
    pub code_ingress_evidence_cursor: AtomicU64,
    /// Action-consequence loop substrate (v26). Records consequence windows
    /// and explicit pairs. No target inference; no success/failure semantics.
    pub action_consequence: RwLock<ActionConsequenceState>,

    /// Adaptive stability substrate. Route-local numeric/time traces for local
    /// degradation, bypass, slow relearning, and reversible fast paths.
    pub adaptive_stability: RwLock<AdaptiveStabilityState>,

    /// Framed-byte egress substrate (v41). Body-side digital disturbance
    /// write-out; numeric/time/byte-shape only.
    pub framed_byte_egress: RwLock<FramedByteEgressState>,

    /// Probe-law substrate (v30). Intensity + suppression + phase + short history.
    /// Numeric/time-only; no target, result, or semantic fields.
    pub probe_law: RwLock<ProbeLawState>,

    /// Evidence-tension substrate (v40). Numeric pressure trace only.
    pub evidence_tension: RwLock<EvidenceTensionState>,

    /// Vocal substrate (v36). Short-lived external vocal target contour.
    pub vocal_target: RwLock<VocalTargetState>,
    /// Vocal substrate (v36). Primitive vocal organ trajectory state.
    pub vocal_organ: RwLock<VocalOrganState>,
    /// Vocal substrate (v36). Imitation pressure and convergence state.
    pub vocal_imitation: RwLock<VocalImitationState>,
    /// Vocal substrate (v36). Stable vocal prototypes memory.
    pub vocal_memory: RwLock<VocalMemoryState>,
    /// P3 acoustic loop: the body's own ears — PCM→VIB analyzer over a sliding
    /// window with its own EMA state. Lets the body hear its own synthesized voice.
    pub self_ear: RwLock<crate::vocal_self_analysis::SelfEar>,
    /// P3 acoustic loop: latest self-heard 16 VIB bands + timestamp.
    pub self_vocal_bands: RwLock<([f64; VIB_BANDS], u64)>,
    /// Finer acoustic representation: latest self-heard 64-mel fingerprint +
    /// timestamp. Runtime-derived each vocal tick from the body's own synthesized
    /// PCM (the same window the 16-band view comes from); feeds the vocal
    /// self-supervision loop at formant resolution. Not persisted.
    pub self_vocal_mel64: RwLock<([f64; crate::vocal_self_analysis::BANDS], u64)>,
    /// Finer acoustic representation: latest raw 64-mel target fingerprint from
    /// the live audio frame + timestamp, staged by the sensory adapter before the
    /// gated vocal-target write copies it (fresh-only) into VocalTargetState.
    /// Runtime-derived each sensory tick; not persisted.
    pub latest_target_mel64: RwLock<([f64; crate::vocal_self_analysis::BANDS], u64)>,
    /// P3 Stage 3: active acoustic servo — learned pitch/resonance correction that
    /// hill-climbs to minimize the heard shape error (steers, not just credits).
    pub vocal_servo: RwLock<crate::vocal_servo::VocalServoState>,
    /// Phase 2 (2026-06-12): content-addressed motor recall — target 64-mel
    /// fingerprint → the servo (F1,F2) correction that matched it. Lets a
    /// recurring nursery target warm-start the servo instead of cold-climbing.
    pub vocal_target_recall: RwLock<crate::vocal_target_recall::VocalTargetRecall>,
    /// #8 sequence layer: transition edges over recall entries + utterance-DTW fit.
    pub vocal_sequence: RwLock<crate::vocal_sequence::VocalSequence>,
    /// Phase 3 (2026-06-18): tier-0 perturbation graph — replaces the SLOT_COUNT=8
    /// categorical-object architecture with a sparse, capacity-soft node-edge
    /// graph. Affordances emerge from marginal statistics; nodes are continuously
    /// updated while active (live-neuron semantics). See perturbation_graph.rs
    /// for the rollout plan.
    pub perturbation_graph:
        RwLock<crate::perturbation_graph::PerturbationGraph>,
    /// Phase 3 Stage 2: tier-0 visual frontend — edge + motion + CC →
    /// patches → perturbation_graph. Runs in parallel with the legacy
    /// region_perception + slot-claim path, doesn't touch it. Owned here so
    /// the prev-frame state survives across adapter ticks.
    pub vision_tier0:
        RwLock<crate::vision_tier0::VisionTier0>,
    /// Which recall entry the servo is currently tracking (-1 = none/novel).
    /// A change signals a target (re)presentation → warm-start; →none → reset.
    pub last_recall_idx: AtomicI64,
    /// Engineering telemetry for the vocal loop (体检). Written by the live
    /// outlet and the closure sink; read only by the body's checkpoint ticker
    /// to dump a numeric trace file. Never read back into any law — pure
    /// observer-side instrumentation, not body state. Not persisted.
    pub vocal_diag: RwLock<VocalLoopDiag>,
    /// C1: vital core — the self-survival source of the dual-drive soul.
    /// reserve depletes with exertion, restores with continuous input; bottoms
    /// out → readable Standby (待机), revivable. Persisted for 死后保留.
    pub vital_core: RwLock<crate::vital_core::VitalCoreState>,
    /// C1-b (sim): the metabolic environment that drives the vital core's
    /// supply/drain, so reserve/anxiety/pain actually breathe and a real
    /// survival crisis (famine → Standby → revive) can occur instead of the
    /// reserve sitting pinned at 1.0. Deterministic; configurable.
    pub vital_env: crate::vital_env::VitalEnv,
    /// Nursery training mode: generous vital_env + reduced voicing exertion.
    pub nursery_metabolic: bool,
    /// Birth-wall timestamp (µs) of the last vital-core advance. The vital core
    /// integrates at a FIXED step (vital_env::VITAL_STEP_US) independent of the
    /// orchestrator's ~30 Hz rate, so survival dynamics track wall-clock time.
    pub last_vital_advance_us: AtomicU64,
    /// A2: self model — seeded by vital interoception. The self is not born;
    /// it grows around the body's survival-sense as agency accrues. Persisted.
    pub self_model: RwLock<crate::self_model::SelfModelState>,
    /// B1-4: dual-drive arbitration reading (self-preservation vs caregiver
    /// attachment). Where 纠结/痛苦/牺牲 emerge; observer-side, recomputed each tick.
    pub dual_drive: RwLock<crate::dual_drive::DualDriveReading>,
    /// Phase C1: intention substrate — the body's "I am pushing toward
    /// something" target. Owned by tick_intention_arbitrator (Phase C2);
    /// persisted across restarts (commitment_until_us survives reboot).
    pub intention: RwLock<crate::intention::IntentionState>,
    /// A2 (VPC mechanism 5): observer/persistence mirror of the axis-level
    /// cross-layer bank's learned state. The live predictors are owned by the
    /// body main loop; this mirror is refreshed after each bank tick so the
    /// snapshot can persist learned weights across restarts (pure numbers).
    pub axis_xlayer_states: RwLock<Vec<crate::vpc_cross_layer::AxisCrossLayerState>>,
    /// Vocal substrate. Numeric reflex pressure over abrupt prediction load.
    pub vocal_reflex: RwLock<VocalReflexState>,
    /// Substrate-only timestamp (µs) of the most recent vocal reflex burst
    /// that resulted in voicing. Used by tick_vocal_reflex_from_pairs to
    /// determine whether a consequence window originated from vocal output.
    pub last_vocal_output_t_us: AtomicU64,

    /// Hearing/vision coupling substrate. Numeric return and expression trace.
    pub auditory_visual: RwLock<AuditoryVisualState>,

    /// Learned-bias delta substrate (v37). Numeric event/curve sediment only.
    /// Does not mutate base priors and does not accept observer interpretation.
    pub learned_bias: RwLock<LearnedBiasState>,

    /// External mark disturbance substrate. Numeric trace evidence only.
    pub mark_disturbance: RwLock<MarkDisturbanceState>,

    /// Tier-0 raw audio waveform ring buffer (48 kHz, 60 s).
    /// Written by AudioAdapter, read by audio predictors (future).
    pub audio_raw: Arc<AudioRawBuffer>,

    // ── Phase A: virtual neck + efference copy ───────────────────────────────

    /// Most-recently-commanded camera translation (efference copy).
    /// Written by the neck controller each tick; read by
    /// `inject_efference_motion_priors`. Phase 2: per-Gaussian inverse-depth ρ
    /// lives on the Gaussian field (see `gaussian_field`), not a region grid.
    pub efference_copy: RwLock<EfferenceCopyState>,

    // ── Phase 1 (keystone ①): sparse Gaussian visual field ───────────────────

    /// Sparse anisotropic Gaussian visual field (derived layer, like object
    /// slots). Driven each camera frame by `tick_gaussian_field`; summary
    /// scalars mirrored onto GAUSS_RECON_MSE_AXIS / GAUSS_ACTIVE_FRAC_AXIS so
    /// they are predicted, produce err, and enter the relation/tower fabric.
    pub gaussian_field: RwLock<crate::phot_gaussian::GaussianField>,
    /// Visual impression layer (Q2/3 of the 2026-06-18 design pass) — the
    /// symmetric counterpart to `vocal_sequence`. Holds the rolling scene-
    /// fingerprint buffer ("刚刚看到的画面留有印象") and the long-term
    /// transition edges between scene-shape nodes. Refreshed after every
    /// successful `tick_gaussian_field` by computing the active-set
    /// fingerprint and feeding it to `PhotSequence::observe_scene`.
    pub phot_sequence: RwLock<crate::phot_sequence::PhotSequence>,
    /// Born-in looming detector state (2026-06-20): a slow EMA of the global
    /// "visual mass" (mean region brightness + bright-footprint fraction). Each
    /// vision frame, the rectified positive gap between the current mass and this
    /// EMA = the looming/approach signal pushed onto VIS_LOOMING_AXIS and fed to
    /// the innate defensive reflex. `None` until the first frame warms it (so the
    /// cold-start frame doesn't read as a spurious approach). Transient — not
    /// persisted; re-warms in a few frames after a restart.
    pub looming_mass_ema: RwLock<Option<f64>>,

    // ── Developmental precision schedule (2026-06-06) ────────────────────────
    /// Developmental clock m∈[0,1]. 0 = naive newborn (loosest precision),
    /// →1 as competence accrues. Drives the precision/tolerance schedule
    /// (coarse→fine). Runtime-only for now; TODO: persist into learned_bias.bin
    /// so maturity is inheritable via checkpoint (factory_dna).
    pub developmental_maturity: RwLock<f64>,
    /// Rate-limit cursor for maturity advances (see MATURITY_ADVANCE_MIN_INTERVAL_US).
    /// Also shared with the familiarity throttle in observe_vocal_reflex_closure
    /// so both tick at the same developmental pace. The pure-vision closure
    /// (observe_visual_reflex_closure) shares this SAME cursor so vision and the
    /// vocal path draw from one global ~1/s maturity budget — vision becomes a
    /// legitimate maturity source without changing the tuned infant period.
    pub last_maturity_advance_us: AtomicU64,
    /// Slow EMA of the Gaussian-field reconstruction MSE — the baseline the
    /// pure-vision maturity closure credits IMPROVEMENT against (crediting a
    /// DROP, never the absolute value, avoids a cold-start 0×x trap). None until
    /// the first observation. Runtime-only: re-converges after a restart and is
    /// not serialized (like the efference copy).
    pub visual_recon_mse_ema: RwLock<Option<f64>>,
    /// All-time LOWEST value the slow recon-MSE baseline has ever reached — the
    /// visual-competence high-water mark. Vision maturity advances ONLY when the
    /// sustained reconstruction beats this (a genuinely-new level of visual skill),
    /// not on every per-frame error reduction. A live camera scene constantly
    /// changes, so the per-frame "improvement" credit never dried up (the treadmill:
    /// adaptation-to-change mistaken for cumulative skill) and saturated the GLOBAL
    /// developmental clock to 1.0 over a long call — which anneals ALL plasticity
    /// (vocal included). None until the first observation; runtime-only.
    pub visual_recon_mse_best: RwLock<Option<f64>>,
    /// Own 1/s throttle cursor for the vision maturity clock, kept separate from
    /// the vocal and binding cursors so the senses do not steal each other's slot.
    pub last_vision_progress_us: AtomicU64,
    /// vocal_efficacy at the last maturity-throttle tick — the baseline the
    /// developmental clock credits the IMPROVEMENT (rise) of competence against,
    /// never the absolute level. Flat/declining efficacy ⇒ zero improvement ⇒ the
    /// clock stops (kills the hallucinated-maturity ratchet that saturated m to
    /// ~0.9955 while real skill was flat). None until the first throttle tick.
    /// Runtime-only: re-converges after a restart, not serialized (like the EMA).
    pub last_maturity_efficacy: RwLock<Option<f64>>,
    /// Own 1/s throttle cursor for the vocal LEARNING-PROGRESS clock. Kept SEPARATE
    /// from last_maturity_advance_us: the vision path advances that shared cursor
    /// ~50×/s and used to starve the vocal block of its slot, freezing the
    /// last_maturity_efficacy baseline near 0 while efficacy climbed; the first
    /// second vision yielded the slot, the stale gap (≈0.40)·GAIN overshot and
    /// clamped maturity to exactly 1.0 in one tick (the post-deploy t=24s jump).
    pub last_vocal_progress_us: AtomicU64,
    /// Last observed value of the organ's continuous syllable-frame phase. A new
    /// syllable opening is detected as the phase wrapping (current < last) — a
    /// continuous-signal event derived from the aperture oscillator, NOT a discrete
    /// phase-machine check. This is the clock that drives one phoneme-motor sample
    /// per syllable (so the inventory is populated by live voicing, ~3/s, and the
    /// competition/repulsion dynamics actually fire), replacing the old gate that
    /// fired only on a fresh-utterance organ transition the continuous voicing
    /// stream almost never produced.
    pub last_syllable_phase: RwLock<f64>,
    /// Dedupe cursor for the vocal_efficacy EMA step (last now_us it stepped on).
    /// The dense action-consequence pair path calls observe_vocal_reflex_closure K
    /// times per tick with the SAME now_us; an ungated EMA stepped K times while
    /// decay sees elapsed=0 — inflating the effective alpha ~K× and collapsing
    /// VOCAL_EFFICACY_EMA_ALPHA's ~10s window into one tick. Stepping at most once
    /// per DISTINCT tick (now_us > this) drops the same-tick duplicates while
    /// keeping the per-tick cadence the alpha was tuned for (unlike the slow 1/s
    /// maturity clock, efficacy must track at the tick rate). Runtime-only (not
    /// persisted), like the maturity/self-play cursors.
    pub last_efficacy_advance_us: AtomicU64,
    /// Idempotence cursor for live language-binding formation: the handle
    /// (write_seq.rotate_left(32) ^ window_id) of the last action-consequence pair
    /// that grounded a binding, so a single earned pair forms at most one binding
    /// even though the 5 Hz tick re-finds it while the stimulus persists. Inits to
    /// u64::MAX (no pair consumed yet). Runtime-only (not persisted).
    pub last_binding_pair_consumed: AtomicU64,
    /// Companion idempotence cursor for cross-modal AV-action grounding
    /// (tick_form_av_action_binding). Independent from last_binding_pair_consumed
    /// so the VIB side (sound→slot) and the PHOT side (seen-action→slot) can each
    /// ground exactly ONE binding per earned Paired pair without blocking each
    /// other. Same handle encoding. Inits to u64::MAX. Runtime-only.
    pub last_av_action_pair_consumed: AtomicU64,
    /// Q1c observation surface for vocal_sequence quiet-period replay (the
    /// "old impressions actively reproduce in prediction" path): increments
    /// each time `tick_vocal_sequence_quiet_replay` actually computes a chain
    /// (silence gate satisfied, cursor known, edge available). Runtime-only.
    pub last_quiet_replay_count: AtomicU64,
    /// Wall-time of the most recent quiet-replay fire (µs since clock zero),
    /// 0 = never fired this session. Runtime-only.
    pub last_quiet_replay_t_us: AtomicU64,
    /// Length of the most recently computed quiet-replay chain (0 if the
    /// cursor had no outgoing edge). Lets observers verify that the rollout
    /// is producing meaningful chains rather than empty hooks. Runtime-only.
    pub last_quiet_replay_chain_len: AtomicU64,
    /// Phase C2: consecutive arbitrator ticks where the FastPath query on
    /// the current anchor returned NeedsDeliberation. Reset to 0 when a
    /// FastPath fires. When ≥ ACTIVE_REASONING_FAST_PATH_SILENT_TICKS the
    /// arbitrator is allowed to commit a new intention. Runtime-only.
    pub fast_path_silent_ticks: std::sync::atomic::AtomicU32,
    /// Phase D4: last wall-clock µs that tick_spontaneous_vocal_expression
    /// pushed an intention-driven vocal_target. SPARSITY gate uses this to
    /// suppress monotone babble — the design-doc requirement that one
    /// FastPath cannot dominate the vocal channel by firing every tick.
    pub last_spontaneous_expression_t_us: AtomicU64,
    /// Rate-limit cursor for vocal self-play closures (see
    /// VOCAL_SELF_PLAY_MIN_INTERVAL_US).
    pub last_self_play_t_us: AtomicU64,
    /// Phase 1 (2026-06-12): rate-limit cursor for imitation closures routed
    /// from the region-B servo/self_voiced path. The servo path runs every tick
    /// with genuine heard-self evidence + a real shape error, but previously
    /// never reached the closure sink (only the brittle ≈22ms self_match window
    /// did) — so efficacy/maturity stayed frozen. This routes that working path
    /// into the SAME honesty-gated sink, rate-limited like self-play.
    pub last_imitation_closure_t_us: AtomicU64,

    // ── Phoneme prototype memory (4-layer vocal self-learning) ───────────────
    /// L1-4 vocal learning substrate.  Prototypes accumulate from babble
    /// self-play and social responses; exploration noise is drawn here before
    /// each new vocal burst.  Checkpointed to phoneme_memory.bin.
    pub phoneme_memory: RwLock<crate::phoneme_memory::PhonemeMemory>,
}

impl Viewport {
    pub fn new() -> Self {
        let mut observed = Vec::with_capacity(AXIS_COUNT);
        let mut predicted = Vec::with_capacity(AXIS_COUNT);
        let mut err_stream = Vec::with_capacity(AXIS_COUNT);
        let mut predictors = Vec::with_capacity(AXIS_COUNT);
        let mut prior_signal = Vec::with_capacity(AXIS_COUNT);
        for _ in 0..AXIS_COUNT {
            observed.push(RwLock::new(AxisRing::new()));
            predicted.push(RwLock::new(AxisRing::new()));
            err_stream.push(RwLock::new(AxisRing::new()));
            predictors.push(RwLock::new(PredictorState::new()));
            prior_signal.push(AtomicU64::new(0)); // 0.0_f64.to_bits() == 0
        }
        let mut slots = Vec::with_capacity(N_OBJECT_SLOTS);
        for _ in 0..N_OBJECT_SLOTS {
            slots.push(RwLock::new(ObjectSlot::empty()));
        }
        Self {
            master_tick: AtomicU64::new(0),
            tower_sweep_count: AtomicU64::new(0),
            clock_us: AtomicU64::new(0),
            start_us: AtomicU64::new(0),
            source_writes: AtomicU64::new(0),
            predictor_ticks: AtomicU64::new(0),
            snapshot_writes: AtomicU64::new(0),
            snapshot_fail_count: AtomicU64::new(0),
            hearing_fault: AtomicBool::new(false),
            hearing_fault_count: AtomicU64::new(0),
            thread_fault: AtomicBool::new(false),
            thread_fault_count: AtomicU64::new(0),
            last_obs_t_us: AtomicU64::new(0),
            health_state: AtomicU8::new(HEALTH_QUIET),
            phot_health: AtomicU8::new(HEALTH_QUIET),
            phot_started_t_us: AtomicU64::new(0),
            phot_last_frame_t_us: AtomicU64::new(0),
            birth_sensitivity: Arc::new(BiasPriors::default()),
            observed,
            predicted,
            err_stream,
            prior_signal,
            predictors,
            looming_mass_ema: RwLock::new(None),
            hl: RwLock::new(HighLevelState::default()),
            ambient: AmbientChannel::new(),
            slots,
            last_claim_debug: RwLock::new(LastClaimDebug::default()),
            claim_seq: AtomicU64::new(0),
            global_attention: RwLock::new(GlobalAttentionState::default()),
            global_relations: RwLock::new(GlobalRelationState::default()),
            relation_pred_bank: RwLock::new(RelationPredBank::default()),
            global_reasoning: RwLock::new(GlobalReasoningState::default()),
            global_reasoning_history: RwLock::new(GlobalReasoningHistoryState::new()),
            global_reasoning_memory_summary: RwLock::new(GlobalReasoningMemorySummary::default()),
            active_reasoning_gap: RwLock::new(ActiveReasoningGapState::default()),
            prediction_pressure_field: RwLock::new(PredictionPressureFieldState::default()),
            self_development: RwLock::new(SelfDevelopmentState::default()),
            active_surface: RwLock::new(ActiveSurfaceState::default()),
            language_binding: RwLock::new(LanguageBindingState::default()),
            code_ingress: RwLock::new(CodeIngressState::default()),
            vpc_contract: RwLock::new(VpcContractState::default()),
            action_output: RwLock::new(ActionOutputState::default()),
            action_output_write_seq: AtomicU64::new(0),
            action_consequence_window_seq: AtomicU64::new(0),
            action_consequence_pair_seq: AtomicU64::new(0),
            life_closure_pair_producer_cursor: AtomicU64::new(0),
            life_closure_framed_return_producer_cursor: AtomicU64::new(0),
            code_ingress_evidence_cursor: AtomicU64::new(0),
            action_consequence: RwLock::new(ActionConsequenceState::default()),
            adaptive_stability: RwLock::new(AdaptiveStabilityState::default()),
            framed_byte_egress: RwLock::new(FramedByteEgressState::default()),
            probe_law: RwLock::new(ProbeLawState::default()),
            evidence_tension: RwLock::new(EvidenceTensionState::default()),
            vocal_target: RwLock::new(VocalTargetState::default()),
            vocal_organ: RwLock::new(VocalOrganState::default()),
            vocal_imitation: RwLock::new(VocalImitationState::default()),
            vocal_memory: RwLock::new(VocalMemoryState::default()),
            self_ear: RwLock::new(crate::vocal_self_analysis::SelfEar::new(
                crate::vocal_synthesis::VOCAL_SAMPLE_RATE as f64,
            )),
            self_vocal_bands: RwLock::new(([0.0; VIB_BANDS], 0)),
            self_vocal_mel64: RwLock::new(([0.0; crate::vocal_self_analysis::BANDS], 0)),
            latest_target_mel64: RwLock::new(([0.0; crate::vocal_self_analysis::BANDS], 0)),
            vocal_servo: RwLock::new(crate::vocal_servo::VocalServoState::default()),
            vocal_target_recall: RwLock::new(
                crate::vocal_target_recall::VocalTargetRecall::default(),
            ),
            vocal_sequence: RwLock::new(crate::vocal_sequence::VocalSequence::default()),
            perturbation_graph: RwLock::new(
                crate::perturbation_graph::PerturbationGraph::default(),
            ),
            vision_tier0: RwLock::new(crate::vision_tier0::VisionTier0::new()),
            last_recall_idx: AtomicI64::new(-1),
            vocal_diag: RwLock::new(VocalLoopDiag::default()),
            vital_core: RwLock::new(crate::vital_core::VitalCoreState::default()),
            vital_env: crate::vital_env::VitalEnv::default(),
            nursery_metabolic: false,
            last_vital_advance_us: AtomicU64::new(0),
            self_model: RwLock::new(crate::self_model::SelfModelState::default()),
            dual_drive: RwLock::new(crate::dual_drive::DualDriveReading::default()),
            intention: RwLock::new(crate::intention::IntentionState::default()),
            axis_xlayer_states: RwLock::new(Vec::new()),
            vocal_reflex: RwLock::new(VocalReflexState::default()),
            last_vocal_output_t_us: AtomicU64::new(0),
            auditory_visual: RwLock::new(AuditoryVisualState::default()),
            learned_bias: RwLock::new(LearnedBiasState::default()),
            mark_disturbance: RwLock::new(MarkDisturbanceState::default()),
            audio_raw: Arc::new(AudioRawBuffer::new()),
            efference_copy: RwLock::new(EfferenceCopyState::default()),
            gaussian_field: RwLock::new(crate::phot_gaussian::GaussianField::new()),
            phot_sequence: RwLock::new(crate::phot_sequence::PhotSequence::default()),
            developmental_maturity: RwLock::new(0.0),
            last_maturity_advance_us: AtomicU64::new(0),
            visual_recon_mse_ema: RwLock::new(None),
            visual_recon_mse_best: RwLock::new(None),
            last_vision_progress_us: AtomicU64::new(0),
            last_maturity_efficacy: RwLock::new(None),
            last_vocal_progress_us: AtomicU64::new(0),
            last_syllable_phase: RwLock::new(0.0),
            last_efficacy_advance_us: AtomicU64::new(0),
            last_binding_pair_consumed: AtomicU64::new(u64::MAX),
            last_av_action_pair_consumed: AtomicU64::new(u64::MAX),
            last_quiet_replay_count: AtomicU64::new(0),
            last_quiet_replay_t_us: AtomicU64::new(0),
            last_quiet_replay_chain_len: AtomicU64::new(0),
            fast_path_silent_ticks: std::sync::atomic::AtomicU32::new(0),
            last_spontaneous_expression_t_us: AtomicU64::new(0),
            last_self_play_t_us: AtomicU64::new(0),
            last_imitation_closure_t_us: AtomicU64::new(0),
            phoneme_memory: RwLock::new(crate::phoneme_memory::PhonemeMemory::new()),
        }
    }

    pub fn with_birth_sensitivity(birth_sensitivity: Arc<BiasPriors>) -> Self {
        let mut vp = Self::new();
        vp.birth_sensitivity = birth_sensitivity;
        vp.vital_env = crate::vital_env::VitalEnv::nursery_training();
        vp.nursery_metabolic = true;
        vp
    }

    pub fn birth_sensitivity(&self) -> Arc<BiasPriors> {
        self.birth_sensitivity.clone()
    }

    pub fn read_evidence_tension(&self) -> EvidenceTensionState {
        self.evidence_tension.read().unwrap().clone()
    }

    pub fn tick_evidence_tension(&self, sample: PressureSample, t_us: u64) {
        self.evidence_tension.write().unwrap().absorb_features(
            sample,
            &self.birth_sensitivity,
            t_us,
        );
    }

    pub fn tick_evidence_tension_from_pairs(&self, t_us: u64) {
        let last_pair_cursor = self.evidence_tension.read().unwrap().last_pair_cursor;
        let (pair_cursor, support) = {
            let ac = self.action_consequence.read().unwrap();
            let cursor = ac
                .pairs
                .iter()
                .map(|p| p.pair_seq)
                .max()
                .unwrap_or(last_pair_cursor);
            let support = ac
                .pairs
                .iter()
                .filter(|p| p.pair_seq > last_pair_cursor)
                .map(|p| p.curve_similarity)
                .fold(0.0_f64, f64::max);
            (cursor, support)
        };
        self.evidence_tension
            .write()
            .unwrap()
            .absorb_return_support(support, pair_cursor, t_us);
    }

    pub fn tick_evidence_tension_from_code_ingress(&self, t_us: u64) {
        // Cursor stores the last processed observation timestamp (t_us), not a
        // frame_seq.  We read from the tier-0 CODE axes — the single
        // authoritative representation of digital objects — rather than
        // duplicating the same values out of the extraction buffer.
        let last_obs_t_us = self.code_ingress_evidence_cursor.load(Ordering::Relaxed);

        let orient_samples = self
            .read_latest_observations(crate::CODE_STRUCT_ORDER_ORIENT, crate::CODE_INGRESS_FRAME_HISTORY_CAP)
            .into_iter()
            .filter(|s| s.t_us > last_obs_t_us)
            .collect::<Vec<_>>();
        let pos_samples = self
            .read_latest_observations(crate::CODE_STRUCT_ORDER_POS, crate::CODE_INGRESS_FRAME_HISTORY_CAP)
            .into_iter()
            .filter(|s| s.t_us > last_obs_t_us)
            .collect::<Vec<_>>();

        if orient_samples.is_empty() && pos_samples.is_empty() {
            return;
        }

        let max_obs_t_us = orient_samples
            .iter()
            .chain(pos_samples.iter())
            .map(|s| s.t_us)
            .max()
            .unwrap_or(last_obs_t_us);

        let avg_transition = if orient_samples.is_empty() {
            0.0
        } else {
            orient_samples.iter().map(|s| s.value).sum::<f64>() / orient_samples.len() as f64
        };
        let avg_skew = if pos_samples.is_empty() {
            0.0
        } else {
            pos_samples.iter().map(|s| s.value).sum::<f64>() / pos_samples.len() as f64
        };

        let recurrence = self
            .latest_observation(crate::CODE_STRUCT_ORDER_PERIOD)
            .map(|s| s.value)
            .unwrap_or(0.0);
        // ambiguity is the complement of recurrence confidence; no separate axis.
        let ambiguity = 1.0 - recurrence;

        // stream_count has no CODE-axis equivalent (it is a buffer-level
        // multiplicity statistic, not a content property).  Read from the
        // extraction buffer only for this cross-channel scalar.
        let cross_channel = {
            let code = self.code_ingress.read().unwrap();
            code.stream_count.saturating_sub(1) as f64
        };

        let sample = PressureSample {
            err: avg_transition,
            recurrence,
            ambiguity,
            deviation: avg_skew,
            cross_channel,
            ..Default::default()
        };

        self.tick_evidence_tension(sample, t_us);
        self.code_ingress_evidence_cursor
            .store(max_obs_t_us, Ordering::Relaxed);
    }

    // --- AmbientChannel API ---

    fn ambient_sub(&self, modality_bit: u8) -> Option<&AmbientModalityState> {
        match modality_bit {
            crate::MOD_VIB => Some(&self.ambient.vib),
            MOD_PHOT => Some(&self.ambient.phot),
            MOD_MECH => Some(&self.ambient.mech),
            _ => None,
        }
    }

    /// Write ambient state for one modality. Other modalities' channels are
    /// untouched. A rising-edge `candidate` bumps the shared nomination_seq.
    pub fn write_ambient_for(
        &self,
        modality_bit: u8,
        energy: f64,
        surprise: f64,
        event_t_us: Option<u64>,
        candidate: bool,
    ) {
        let Some(sub) = self.ambient_sub(modality_bit) else {
            return;
        };
        {
            let mut e = sub.event_energy.write().unwrap();
            *e = energy;
        }
        {
            let mut s = sub.surprise.write().unwrap();
            *s = surprise;
        }
        if let Some(t) = event_t_us {
            sub.last_event_t_us.store(t, Ordering::Relaxed);
        }
        let prev = sub.candidate_flag.swap(candidate, Ordering::AcqRel);
        if candidate && !prev {
            self.ambient.nomination_seq.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn read_ambient_sub(sub: &AmbientModalityState) -> AmbientModalitySnapshot {
        AmbientModalitySnapshot {
            event_energy: *sub.event_energy.read().unwrap(),
            surprise: *sub.surprise.read().unwrap(),
            last_event_t_us: sub.last_event_t_us.load(Ordering::Relaxed),
            candidate_flag: sub.candidate_flag.load(Ordering::Relaxed),
        }
    }

    pub fn read_ambient(&self) -> AmbientSnapshot {
        AmbientSnapshot {
            nomination_seq: self.ambient.nomination_seq.load(Ordering::Relaxed),
            vib: Self::read_ambient_sub(&self.ambient.vib),
            phot: Self::read_ambient_sub(&self.ambient.phot),
            mech: Self::read_ambient_sub(&self.ambient.mech),
        }
    }

    // --- ObjectSlot API ---

    /// Decay expired slots. Callers invoke periodically (e.g., per adapter tick).
    pub fn sweep_slots(&self, now_us: u64) {
        for slot in self.slots.iter() {
            let mut s = slot.write().unwrap();
            if s.active && now_us.saturating_sub(s.last_update_t_us) > SLOT_IDLE_US {
                *s = ObjectSlot::empty();
            }
        }
    }

    /// Find an active slot already touched by `modality_bit` whose last update
    /// is recent. Returns the slot_id if one exists.
    pub fn find_active_slot_for(&self, modality_bit: u8, now_us: u64) -> Option<usize> {
        for (i, slot) in self.slots.iter().enumerate() {
            let s = slot.read().unwrap();
            if s.active
                && (s.source_modality & modality_bit) != 0
                && now_us.saturating_sub(s.last_update_t_us) <= SLOT_IDLE_US
            {
                return Some(i);
            }
        }
        None
    }

    /// Phase 3D: evidence-scoring / best-match slot claim.
    ///
    /// The viewport is the *sole* legitimate decision point. Adapters submit
    /// `EventEvidence`; the viewport:
    ///   1. Scores every active slot via `score_slot_for_event` (numeric-only,
    ///      no semantic labels). Hard gates (inactivity, idle/window timeout,
    ///      far-apart spatial) eliminate candidates outright.
    ///   2. Picks the highest-scoring survivor.
    ///   3. If `best.total >= CLAIM_REUSE_MIN_SCORE`, reuses it; otherwise
    ///      allocates a fresh slot.
    ///   4. If the slot pool is full, drops.
    ///
    /// All Phase 3C guardrails (stale / cross-modal window / spatial
    /// proximity) remain enforced as hard gates inside the scorer.
    pub fn claim_slot_for_event(&self, ev: &EventEvidence) -> SlotClaim {
        let ambient = self.read_ambient();

        let mut best: Option<CandidateScore> = None;
        let mut n_considered: u8 = 0;

        for (i, slot) in self.slots.iter().enumerate() {
            let s = slot.read().unwrap();
            if let Some(score) = self.score_slot_for_event(i, &s, ev, &ambient) {
                n_considered = n_considered.saturating_add(1);
                let keep = match best {
                    None => true,
                    Some(b) => score.total > b.total,
                };
                if keep {
                    best = Some(score);
                }
            }
        }

        let mut claim = SlotClaim::Dropped;

        if let Some(b) = best {
            if b.total >= CLAIM_REUSE_MIN_SCORE {
                claim = SlotClaim::Reused(b.slot_idx as usize);
            }
        }

        if matches!(claim, SlotClaim::Dropped) {
            for (i, slot) in self.slots.iter().enumerate() {
                let mut s = slot.write().unwrap();
                if !s.active {
                    *s = ObjectSlot::empty();
                    s.active = true;
                    s.last_update_t_us = ev.event_t_us;
                    s.source_modality = ev.modality_bit;
                    s.nomination_seq = ev.nomination_seq_hint;
                    claim = SlotClaim::Allocated(i);
                    break;
                }
            }
        }

        self.record_claim_debug(ev, claim, best, n_considered);
        claim
    }

    /// Produce a numeric `CandidateScore` for one candidate slot against an
    /// incoming `EventEvidence`, or `None` if a hard gate eliminates the
    /// candidate outright. Hard gates:
    ///   * inactive slot
    ///   * slot last update older than `SLOT_IDLE_US`
    ///   * cross-modal candidate older than `CROSS_MODAL_WINDOW_US`
    ///   * cross-modal candidate whose spatial centroid is farther than
    ///     `CROSS_MODAL_SPATIAL_DIST` when both sides carry spatial
    fn score_slot_for_event(
        &self,
        slot_idx: usize,
        slot: &ObjectSlot,
        ev: &EventEvidence,
        ambient: &AmbientSnapshot,
    ) -> Option<CandidateScore> {
        if !slot.active {
            return None;
        }
        let dt = ev.event_t_us.saturating_sub(slot.last_update_t_us);
        if dt > SLOT_IDLE_US {
            return None;
        }

        let same_modality = (slot.source_modality & ev.modality_bit) != 0;

        // Cross-modal hard gates.
        if !same_modality {
            if dt > CROSS_MODAL_WINDOW_US {
                return None;
            }
            if let (Some(sp_slot), Some(sp_ev)) = (slot.spatial, ev.spatial) {
                if sp_slot.valid {
                    let dx = sp_slot.pos_x - sp_ev.pos_x;
                    let dy = sp_slot.pos_y - sp_ev.pos_y;
                    let d2 = dx * dx + dy * dy;
                    if d2 > CROSS_MODAL_SPATIAL_DIST * CROSS_MODAL_SPATIAL_DIST {
                        return None;
                    }
                }
            }
        }

        // Same-modal spatial hard gate: if the slot already has a valid
        // spatial position and the incoming evidence is too far away, the slot
        // represents a different object. This prevents multi-region frames from
        // collapsing all regions into a single slot. Phase 4D.
        if same_modality {
            if let (Some(sp_slot), Some(sp_ev)) = (slot.spatial, ev.spatial) {
                if sp_slot.valid {
                    let dx = sp_slot.pos_x - sp_ev.pos_x;
                    let dy = sp_slot.pos_y - sp_ev.pos_y;
                    let d2 = dx * dx + dy * dy;
                    if d2 > SAME_MODAL_SPATIAL_DIST * SAME_MODAL_SPATIAL_DIST {
                        return None;
                    }
                }
            }
        }

        // ---- soft components ----
        let time_denom = if same_modality {
            SLOT_IDLE_US as f64
        } else {
            CROSS_MODAL_WINDOW_US as f64
        };
        let time = clamp01(1.0 - (dt as f64) / time_denom.max(1.0));

        let spatial = match (slot.spatial, ev.spatial) {
            (Some(sp_slot), Some(sp_ev)) if sp_slot.valid => {
                let dx = sp_slot.pos_x - sp_ev.pos_x;
                let dy = sp_slot.pos_y - sp_ev.pos_y;
                let d = (dx * dx + dy * dy).sqrt();
                let norm = if same_modality {
                    SAME_MODAL_SPATIAL_DIST
                } else {
                    CROSS_MODAL_SPATIAL_DIST
                };
                clamp01(1.0 - d / norm.max(1e-9))
            }
            _ => 0.5, // neutral when spatial info is absent on either side
        };

        let modality = if same_modality {
            CLAIM_SAME_MODALITY_BONUS
        } else {
            CLAIM_CROSS_MODALITY_BASE
        };

        let ambient_energy = ambient_energy_for(ambient, ev.modality_bit);
        let energy = energy_consistency(ev.event_energy, ambient_energy);

        let ambient_surprise = ambient_surprise_for(ambient, ev.modality_bit);
        let surprise = surprise_consistency(ev.surprise, ambient_surprise);

        let total = CLAIM_W_TIME * time
            + CLAIM_W_SPATIAL * spatial
            + CLAIM_W_MODALITY * modality
            + CLAIM_W_ENERGY * energy
            + CLAIM_W_SURPRISE * surprise;

        Some(CandidateScore {
            slot_idx: slot_idx as u16,
            total: clamp01(total),
            time,
            spatial,
            modality,
            energy,
            surprise,
        })
    }

    fn record_claim_debug(
        &self,
        ev: &EventEvidence,
        claim: SlotClaim,
        best: Option<CandidateScore>,
        n_considered: u8,
    ) {
        let seq = self.claim_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let (kind, chosen) = match claim {
            SlotClaim::Dropped => (CLAIM_KIND_DROPPED, -1i16),
            SlotClaim::Reused(i) => (CLAIM_KIND_REUSED, i as i16),
            SlotClaim::Allocated(i) => (CLAIM_KIND_ALLOCATED, i as i16),
        };
        let b = best.unwrap_or_default();
        let dbg = LastClaimDebug {
            seq,
            event_t_us: ev.event_t_us,
            modality_bit: ev.modality_bit,
            kind,
            chosen_slot: chosen,
            best_score: b.total,
            best_time_score: b.time,
            best_spatial_score: b.spatial,
            best_modality_score: b.modality,
            best_energy_score: b.energy,
            best_surprise_score: b.surprise,
            n_candidates_considered: n_considered,
            min_reuse_score: CLAIM_REUSE_MIN_SCORE,
        };
        let mut w = self.last_claim_debug.write().unwrap();
        *w = dbg;
    }

    /// Numeric-only read of the last claim debug record. Observers render;
    /// they do not recompute scores.
    pub fn read_last_claim_debug(&self) -> LastClaimDebug {
        *self.last_claim_debug.read().unwrap()
    }

    /// Allocate the first inactive slot. Returns None if all slots are full.
    pub fn allocate_slot(
        &self,
        modality_bit: u8,
        now_us: u64,
        nomination_seq: u64,
    ) -> Option<usize> {
        for (i, slot) in self.slots.iter().enumerate() {
            let mut s = slot.write().unwrap();
            if !s.active {
                *s = ObjectSlot::empty();
                s.active = true;
                s.last_update_t_us = now_us;
                s.source_modality = modality_bit;
                s.nomination_seq = nomination_seq;
                return Some(i);
            }
        }
        None
    }

    /// Write VIB band data into slot's `vib` sub-state. Fixed-length input is
    /// part of the adapter contract; internal storage is a length-carrying
    /// `BandFamilyState`, so PHOT/MECH are not constrained to `VIB_BANDS`.
    pub fn write_slot_vib(
        &self,
        slot_id: usize,
        t_us: u64,
        observed: &[f64; VIB_BANDS],
        predicted: &[f64; VIB_BANDS],
        err: &[f64; VIB_BANDS],
        surprise: &[f64; VIB_BANDS],
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        s.last_update_t_us = t_us;
        s.source_modality |= crate::MOD_VIB;
        s.vib = Some(BandFamilyState {
            n_bands: VIB_BANDS as u16,
            observed: observed.to_vec(),
            predicted: predicted.to_vec(),
            err: err.to_vec(),
            surprise: surprise.to_vec(),
        });
    }

    /// Write PHOT band data into slot's `phot` sub-state. Substrate-level API;
    /// no PHOT adapter is wired in Phase 3A, but the path exists and the
    /// storage shape is independent of `VIB_BANDS`.
    pub fn write_slot_phot(
        &self,
        slot_id: usize,
        t_us: u64,
        observed: &[f64],
        predicted: &[f64],
        err: &[f64],
        surprise: &[f64],
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if predicted.len() != n || err.len() != n || surprise.len() != n {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        s.last_update_t_us = t_us;
        s.source_modality |= MOD_PHOT;
        s.phot = Some(BandFamilyState {
            n_bands: n as u16,
            observed: observed.to_vec(),
            predicted: predicted.to_vec(),
            err: err.to_vec(),
            surprise: surprise.to_vec(),
        });
    }

    /// Phase 4C: write PHOT band data into a slot, running the slot-local AR-1
    /// predictor to produce real `predicted`, `err`, and `surprise` values.
    ///
    /// The predictor state (`phot_pred`) lives inside the slot and resets
    /// automatically when the slot is cleared by `sweep_slots`. Adapters call
    /// this instead of `write_slot_phot` so the slot's PHOT family always
    /// carries genuine prediction output rather than placeholder zeros.
    ///
    /// Rules:
    ///   * Slot must be active; no-op otherwise.
    ///   * Predictor warms up for `PHOT_PRED_WARMUP_MIN` calls; during warmup
    ///     `err=0` and `surprise=0` to prevent fake-confidence reads.
    ///   * After warmup: `predicted` = previous AR-1 state, `err` = residual,
    ///     `surprise` = EMA of squared err (rises with prediction error).
    ///   * Does NOT touch claim/scoring or any other modality's fields.
    pub fn update_slot_phot_with_predictor(&self, slot_id: usize, t_us: u64, observed: &[f64]) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if n == 0 {
            return;
        }
        let lifecycle = {
            let mut s = self.slots[slot_id].write().unwrap();
            if !s.active {
                return;
            }
            if s.phot_pred.is_none() {
                s.phot_pred = Some(PhotSlotPredictor::new(n));
            }
            let out = s.phot_pred.as_mut().unwrap().update(observed);
            let lifecycle = if s.phot_pred.as_ref().unwrap().n_updates < PHOT_PRED_WARMUP_MIN {
                PredictorLifecycleState::Warmup
            } else {
                PredictorLifecycleState::ResolvedPublication
            };
            s.last_update_t_us = t_us;
            s.source_modality |= MOD_PHOT;
            s.phot = Some(BandFamilyState {
                n_bands: n as u16,
                observed: observed.to_vec(),
                predicted: out.predicted,
                err: out.err,
                surprise: out.surprise,
            });
            lifecycle
        };
        self.record_vpc_slot_family_predictor_trace(
            VPC_FAMILY_PHOT,
            slot_id,
            MOD_PHOT as u16,
            t_us,
            t_us,
            lifecycle,
        );
    }

    /// Write MECH band data into slot's `mech` sub-state. Placeholder path.
    pub fn write_slot_mech(
        &self,
        slot_id: usize,
        t_us: u64,
        observed: &[f64],
        predicted: &[f64],
        err: &[f64],
        surprise: &[f64],
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if predicted.len() != n || err.len() != n || surprise.len() != n {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        s.last_update_t_us = t_us;
        s.source_modality |= MOD_MECH;
        s.mech = Some(BandFamilyState {
            n_bands: n as u16,
            observed: observed.to_vec(),
            predicted: predicted.to_vec(),
            err: err.to_vec(),
            surprise: surprise.to_vec(),
        });
    }

    /// Attach/update the 2D spatial subtree of an active slot. No coordinate-
    /// system name is stored; `pos_*` / `span_*` are numbers in `[0.0, 1.0]`
    /// by convention, enforced only by a soft clamp at write time.
    ///
    /// `modality_bit` records which adapter produced the estimate (for the
    /// slot's source_modality bitmap). Writing does not itself activate a
    /// slot — the slot must already be active.
    pub fn write_slot_spatial(
        &self,
        slot_id: usize,
        modality_bit: u8,
        t_us: u64,
        pos_x: f64,
        pos_y: f64,
        span_x: f64,
        span_y: f64,
        precision: f64,
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let clamp01 = |v: f64| v.clamp(0.0, 1.0);
        let clamp_span = |v: f64| v.clamp(0.0, 1.0);
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        let pos_x = clamp01(pos_x);
        let pos_y = clamp01(pos_y);
        let next_worldline =
            update_spatial_worldline(s.spatial, s.spatial_worldline, t_us, pos_x, pos_y);
        s.last_update_t_us = t_us;
        s.source_modality |= modality_bit;
        s.spatial = Some(SpatialState {
            pos_x,
            pos_y,
            span_x: clamp_span(span_x),
            span_y: clamp_span(span_y),
            precision: precision.max(0.0),
            last_update_t_us: t_us,
            valid: true,
        });
        s.spatial_worldline = Some(next_worldline);
    }

    /// Mark the spatial subtree as present but currently invalid (the adapter
    /// claimed a spatial target earlier but has no fresh estimate this tick).
    pub fn invalidate_slot_spatial(&self, slot_id: usize, t_us: u64) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        let mut sp = s.spatial.unwrap_or(SpatialState::default());
        sp.valid = false;
        sp.last_update_t_us = t_us;
        s.spatial = Some(sp);
        if let Some(mut wl) = s.spatial_worldline {
            wl.valid = false;
            wl.last_update_t_us = t_us;
            s.spatial_worldline = Some(wl);
        }
    }

    pub fn read_slot(&self, slot_id: usize) -> Option<ObjectSlotSnapshot> {
        if slot_id >= N_OBJECT_SLOTS {
            return None;
        }
        let s = self.slots[slot_id].read().unwrap();
        Some(ObjectSlotSnapshot {
            active: s.active,
            last_update_t_us: s.last_update_t_us,
            source_modality: s.source_modality,
            nomination_seq: s.nomination_seq,
            vib: s.vib.clone(),
            phot: s.phot.clone(),
            mech: s.mech.clone(),
            spatial: s.spatial,
            spatial_worldline: s.spatial_worldline,
            fusion: s.fusion,
            attention_share: s.attention_share,
            precision_bias: s.precision_bias,
        })
    }

    pub fn read_language_binding(&self) -> LanguageBindingState {
        self.language_binding.read().unwrap().clone()
    }

    pub fn read_code_ingress(&self) -> CodeIngressState {
        self.code_ingress.read().unwrap().clone()
    }

    pub fn observe_code_ingress_frame(&self, input: CodeIngressFrameInput<'_>) {
        let t_us = input.t_end_us;
        let stream_ctx = input.stream_context_id;
        let byte_len = input.bytes.len();
        self.code_ingress.write().unwrap().observe_frame(input);
        // Project byte-object measurements onto tier-0 CODE axes.
        // Per design doc §4: digital objects are the same coordinate system projected
        // onto the STRUCT/ORDER sub-set; not a separate representation.
        let (order_pos, order_orient, order_period) = {
            let ci = self.code_ingress.read().unwrap();
            let op = ci.recent_frames.last().map(|t| t.byte_distribution_skew).unwrap_or(0.0);
            let oo = ci.recent_frames.last().map(|t| t.byte_transition_energy).unwrap_or(0.0);
            let per = ci
                .sequence_windows
                .last()
                .filter(|w| w.stream_context_id == stream_ctx)
                .map(|w| w.recurrence_score)
                .unwrap_or(0.0);
            (op, oo, per)
        };
        let code_pos = code_pos_from_ctx(stream_ctx);
        let code_span = (byte_len as f64 / crate::CODE_SPAN_MAX_BYTES).min(1.0);
        self.push_observation(crate::CODE_POS, t_us, code_pos);
        self.push_observation(crate::CODE_SPAN, t_us, code_span);
        self.push_observation(crate::CODE_STRUCT_ORDER_POS, t_us, order_pos);
        self.push_observation(crate::CODE_STRUCT_ORDER_ORIENT, t_us, order_orient);
        self.push_observation(crate::CODE_STRUCT_ORDER_PERIOD, t_us, order_period);
    }

    pub fn read_learned_bias(&self) -> LearnedBiasState {
        let mut state = self.learned_bias.read().unwrap().clone();
        // A3: the developmental clock's source of truth is the Viewport field;
        // copy it into the checkpoint-bound state so every existing save path
        // (periodic + exit) persists it without extra wiring.
        state.developmental_maturity = self.developmental_maturity();
        state
    }

    pub fn apply_learned_bias_event(&self, event: LearnedBiasEventInput) {
        self.learned_bias
            .write()
            .unwrap()
            .apply_event(&self.birth_sensitivity, event);
    }

    pub fn push_language_binding_trace(&self, trace: BindingTrace) {
        self.language_binding.write().unwrap().push_trace(trace);
    }

    /// Developmental clock: 0 = naive newborn, →1 as competence accrues.
    /// Drives the precision/tolerance schedule (coarse→fine).
    pub fn developmental_maturity(&self) -> f64 {
        *self.developmental_maturity.read().unwrap()
    }

    /// Advance the developmental clock by one earned closure. Saturating:
    /// m += support·MATURITY_GAIN·(1−m), so maturation is a slow accrual and
    /// can never exceed 1. No-op for non-positive support. This is the ONE
    /// chokepoint that matures the body; every other gate reads the clock.
    pub fn advance_maturity(&self, closure_support: f64) {
        let c = closure_support.clamp(0.0, 1.0);
        if c <= 0.0 {
            return;
        }
        let mut m = self.developmental_maturity.write().unwrap();
        *m = (*m + c * crate::MATURITY_GAIN * (1.0 - *m)).clamp(0.0, 1.0);
    }

    /// Learning-progress developmental clock: advance maturity on the IMPROVEMENT
    /// (rise) of competence, never the absolute level. `improvement` is the
    /// positive per-throttle-tick rise of vocal_efficacy; a flat or declining
    /// competence yields ≤0 and the clock holds — so maturity can no longer
    /// ratchet to ~1.0 while real skill is flat/declining (the verified
    /// hallucinated-maturity bug). Monotone-saturating (·(1−m)): maturity is a
    /// competence high-water-mark — once mastered, consolidated; never advanced by
    /// noise. Route-A: the signal is a prediction-error-derived competence rise
    /// (Oudeyer IAC learning progress), no label, no reward. Sibling of the
    /// vision path's improvement-credit (observe_visual_reflex_closure).
    pub fn advance_maturity_by_progress(&self, improvement: f64) {
        // Cap the per-tick improvement BEFORE applying gain: if improvement·GAIN ≥ 1
        // the saturating (1−m) term cannot tame the step and m clamps straight to
        // 1.0 in a single tick (the verified t=24s overshoot). MATURITY_PROGRESS_
        // STEP_MAX keeps any one second's step far below saturation.
        let imp = improvement.clamp(0.0, crate::MATURITY_PROGRESS_STEP_MAX);
        if imp <= 0.0 {
            return;
        }
        let mut m = self.developmental_maturity.write().unwrap();
        *m = (*m + imp * crate::MATURITY_PROGRESS_GAIN * (1.0 - *m)).clamp(0.0, 1.0);
    }

    /// Restore the developmental clock from a checkpoint. Boot-only; clamps to
    /// [0,1]. Keeps newborn default 0.0 when no checkpoint exists
    /// (advance_maturity remains the only forward chokepoint during life).
    pub fn restore_maturity(&self, m: f64) {
        *self.developmental_maturity.write().unwrap() = m.clamp(0.0, 1.0);
    }

    pub fn apply_language_binding_event(&self, event: BindingEvent) {
        let update_pressure = self.binding_update_pressure(&event);
        self.language_binding
            .write()
            .unwrap()
            .apply_binding_event_with_pressure(event, update_pressure);
    }

    pub fn apply_binding_event_from_adapter(&self, event: BindingEvent) {
        self.apply_language_binding_event(event);
    }

    fn binding_update_pressure(&self, event: &BindingEvent) -> f64 {
        let (repeat, correction_density, support_history, recent_alignment) = {
            let bindings = self.language_binding.read().unwrap();
            if let Some(trace) = bindings.traces.iter().find(|trace| {
                trace.pattern_kind == event.pattern_kind
                    && trace.pattern_hash == event.pattern_hash
                    && trace.target == event.target
            }) {
                let dt = event.event_t_us.saturating_sub(trace.last_update_t_us);
                (
                    1.0,
                    ((trace.reinforce_count + trace.weaken_count) as f64 / 4.0).clamp(0.0, 1.0),
                    (trace.evidence_refs.len() as f64 / 4.0).clamp(0.0, 1.0),
                    (1.0 - dt as f64 / BINDING_MAINTENANCE_IDLE_US as f64).clamp(0.0, 1.0),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        };

        let communication_pressure = compute_pressure(
            PressureSample {
                recurrence: repeat,
                alignment: recent_alignment,
                support: support_history,
                deviation: correction_density,
                ..Default::default()
            },
            &self.birth_sensitivity.pressure_weights,
        );

        let cross_modal_slot_pressure = match event.target {
            BindingTargetRef::ObjectSlot { slot_id } => self
                .read_slot(slot_id as usize)
                .and_then(|s| s.fusion)
                .map(|f| f.cross_modal_confidence)
                .unwrap_or(0.0),
            _ => 0.0,
        };
        let cross_modal_pressure = compute_pressure(
            PressureSample {
                recurrence: repeat,
                alignment: cross_modal_slot_pressure,
                cross_channel: cross_modal_slot_pressure,
                ..Default::default()
            },
            &self.birth_sensitivity.pressure_weights,
        );

        (communication_pressure + cross_modal_pressure).clamp(0.0, 2.0)
    }

    pub fn tick_binding_maintenance(&self, now_us: u64) {
        self.language_binding.write().unwrap().tick_maintenance(
            now_us,
            BINDING_MAINTENANCE_IDLE_US,
            BINDING_MAINTENANCE_DECAY_STEP,
        );
    }

    pub fn tick_binding_resonance(&self, now_us: u64) {
        let _ = now_us;
        let bindings = self.language_binding.read().unwrap();
        for trace in bindings.traces.iter() {
            if trace.strength.value < BINDING_RESONANCE_THRESHOLD {
                continue;
            }
            if let BindingTargetRef::ObjectSlot { slot_id } = trace.target {
                let slot_id = slot_id as usize;
                let mut attn = self.global_attention.write().unwrap();
                if slot_id < attn.slot_biases.len() {
                    let bias = trace.strength.value * BINDING_ATTN_ALPHA;
                    attn.slot_biases[slot_id] =
                        (attn.slot_biases[slot_id] + bias).min(ATTN_PRECISION_BIAS_MAX);
                }
            }
        }
    }

    /// Phase 5A: recompute the fusion subtree for an active slot.
    ///
    /// Called by each modality adapter after writing its own data. The
    /// `modality_bit` tells the fusion engine which modality just wrote so it
    /// can stamp `last_vib_t_us` or `last_phot_t_us` with `now_us`.
    ///
    /// Rules:
    ///   * Only reads `slot.{vib,phot,source_modality}` and writes `slot.fusion`.
    ///   * Does NOT touch claim, scoring, spatial, or any other subtree.
    ///   * No-op for inactive slots.
    pub fn refresh_slot_fusion(&self, slot_id: usize, modality_bit: u8, now_us: u64) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }

        // Carry forward per-modality timestamps from existing fusion state.
        let (mut last_vib_t_us, mut last_phot_t_us) = match s.fusion {
            Some(ref f) => (f.last_vib_t_us, f.last_phot_t_us),
            None => (0u64, 0u64),
        };
        if (modality_bit & MOD_VIB) != 0 {
            last_vib_t_us = now_us;
        }
        if (modality_bit & MOD_PHOT) != 0 {
            last_phot_t_us = now_us;
        }

        let source_modality = s.source_modality;
        let vib_surprise = mean_band_surprise(s.vib.as_ref());
        let phot_surprise = mean_band_surprise(s.phot.as_ref());

        // Freshness weights: decay linearly to 0 over SLOT_IDLE_US.
        let vib_weight = freshness_weight(last_vib_t_us, now_us);
        let phot_weight = freshness_weight(last_phot_t_us, now_us);

        // Temporal alignment: how close in wall-clock time the two modalities are.
        let temporal_alignment_score = if last_vib_t_us > 0 && last_phot_t_us > 0 {
            let dt = last_vib_t_us.abs_diff(last_phot_t_us);
            clamp01(1.0 - dt as f64 / CROSS_MODAL_WINDOW_US as f64)
        } else {
            0.0
        };

        // Hard gate: both modalities must be fresh for cross-modal status.
        let is_cross_modal = (source_modality & MOD_VIB) != 0
            && (source_modality & MOD_PHOT) != 0
            && vib_weight > 0.01
            && phot_weight > 0.01;

        // Confidence: elevated only when cross-modal and well-aligned.
        // Single-modality is hard-capped at 0.5.
        let cross_modal_confidence = if is_cross_modal {
            clamp01(temporal_alignment_score * (vib_weight + phot_weight) / 2.0)
        } else {
            0.5 * vib_weight.max(phot_weight)
        };

        let total_w = vib_weight + phot_weight;
        let fused_surprise = if total_w > 1e-12 {
            (vib_weight * vib_surprise + phot_weight * phot_surprise) / total_w
        } else {
            0.0
        };

        s.fusion = Some(CrossModalFusionState {
            modality_mask: source_modality,
            last_fused_t_us: now_us,
            last_vib_t_us,
            last_phot_t_us,
            temporal_alignment_score,
            cross_modal_confidence,
            vib_weight,
            phot_weight,
            fused_surprise,
            is_cross_modal,
        });
    }

    /// Phase 5C: recompute global attention allocation across all active slots.
    ///
    /// Reads each slot's `fusion.cross_modal_confidence` and a recency factor
    /// from `fusion.last_fused_t_us`. Produces normalised attention shares and
    /// writes them back as `attention_share` and `precision_bias` (suggestion
    /// values only — these do NOT feed back into the fusion formula).
    ///
    /// Phase 3 cutover: the prior `high_layer.attention_bias` weighting was
    /// dropped along with the slot high-layer subtree. The fusion-only score
    /// is the same shape (confidence × recency), with `attention_bias = 1`.
    ///
    /// Rules:
    ///   * Two-pass: read pass collects data (all read locks released before
    ///     write pass). Prevents lock-ordering deadlocks.
    ///   * Degenerate case (all raw scores < 1e-15): equal distribution across
    ///     active slots.
    ///   * Does NOT touch claim/scoring, band families, or fusion formulas.
    pub fn refresh_global_attention(&self, now_us: u64) {
        // First pass: collect (confidence, last_fused_t_us).
        let mut info = [None::<(f64, u64)>; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            let s = self.slots[i].read().unwrap();
            if s.active {
                if let Some(fusion) = s.fusion {
                    info[i] = Some((
                        fusion.cross_modal_confidence,
                        fusion.last_fused_t_us,
                    ));
                }
            }
        }

        // Compute raw scores: confidence * recency (attention_bias = 1).
        let mut raw = [0.0f64; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            if let Some((conf, last_t)) = info[i] {
                let dt = now_us.saturating_sub(last_t);
                let recency = (1.0 - dt as f64 / SLOT_IDLE_US as f64).clamp(0.0, 1.0);
                raw[i] = conf * recency;
            }
        }

        // Normalise to get attention shares.
        let total: f64 = raw.iter().sum();
        let mut shares = [0.0f64; N_OBJECT_SLOTS];
        if total < 1e-15 {
            let n_active = info.iter().filter(|x| x.is_some()).count();
            if n_active > 0 {
                let eq = 1.0 / n_active as f64;
                for i in 0..N_OBJECT_SLOTS {
                    if info[i].is_some() {
                        shares[i] = eq;
                    }
                }
            }
        } else {
            for i in 0..N_OBJECT_SLOTS {
                shares[i] = raw[i] / total;
            }
        }

        // Sort active slot indices by raw score descending.
        let mut active: Vec<usize> = (0..N_OBJECT_SLOTS).filter(|&i| info[i].is_some()).collect();
        active.sort_unstable_by(|&a, &b| {
            raw[b]
                .partial_cmp(&raw[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut top_slot_ids = [-1i8; N_OBJECT_SLOTS];
        for (pos, &id) in active.iter().enumerate() {
            top_slot_ids[pos] = id as i8;
        }
        let focused_slot_id = top_slot_ids[0];
        let focused_score = if focused_slot_id >= 0 {
            raw[focused_slot_id as usize]
        } else {
            0.0
        };
        let attention_entropy: f64 = shares
            .iter()
            .filter(|&&p| p > 1e-15)
            .map(|&p| -p * p.ln())
            .sum();
        let n_active = active.len() as u8;

        // Second pass: write per-slot attention_share and precision_bias.
        for i in 0..N_OBJECT_SLOTS {
            let mut s = self.slots[i].write().unwrap();
            if s.active {
                s.attention_share = shares[i];
                s.precision_bias = (2.0 * shares[i]).clamp(0.0, 2.0);
            } else {
                s.attention_share = 0.0;
                s.precision_bias = 1.0;
            }
        }

        let mut attn = self.global_attention.write().unwrap();
        let prev_replay = attn.replay_intensity;
        let prev_replay_t = attn.last_replay_t_us;
        *attn = GlobalAttentionState {
            focused_slot_id,
            top_slot_ids,
            attention_shares: shares,
            attention_entropy,
            focused_score,
            n_active,
            slot_biases: [0.0f64; N_OBJECT_SLOTS],
            replay_intensity: prev_replay,
            last_replay_t_us: prev_replay_t,
        };
    }

    pub fn read_global_attention(&self) -> GlobalAttentionState {
        *self.global_attention.read().unwrap()
    }

    /// Phase 5D: recompute cross-object temporal relation scores.
    ///
    /// Must be called AFTER `refresh_global_attention` so `attention_share`
    /// fields are current. Only considers slots with `attention_share > 1e-6`
    /// that have `fusion` populated.
    ///
    /// Per directed edge (src→dst), three EMA scores are updated:
    ///   - `co_change_score`   — `1 - |bias_src - bias_dst|`
    ///   - `lead_lag_score`    — `(t_dst - t_src) / CROSS_MODAL_WINDOW_US`
    ///   - `predictive_score`  — `clamp01(err_src * err_dst * max(0, lag_obs))`
    /// All decay toward 0 when either slot is inactive or unattended.
    ///
    /// Phase 3 cutover: the prior `high_layer.attention_bias / object_err`
    /// inputs were dropped along with the slot high-layer subtree. `bias` is
    /// fixed at 1 and `err` at 0, so the co/predictive channels collapse to
    /// constant signals while the lead/lag/spatial channels remain meaningful.
    ///
    /// Rules:
    ///   * Two-pass: read pass first (all locks released), then write pass.
    ///   * Does NOT touch claim/scoring, band families, fusion, or adapters.
    ///   * `precision_bias` / `attention_share` are never written here.
    pub fn refresh_global_relations(&self, now_us: u64) {
        // First pass: collect per-slot data (bias, err, last_fused_t_us, share, spatial).
        // bias/err are now constants — see method-level note on the Phase 3 cutover.
        let mut slot_data = [None::<(f64, f64, u64, f64, Option<SpatialState>)>; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            let s = self.slots[i].read().unwrap();
            if s.active {
                if let Some(fusion) = s.fusion {
                    slot_data[i] = Some((
                        1.0,
                        0.0,
                        fusion.last_fused_t_us,
                        s.attention_share,
                        s.spatial.filter(|sp| sp.valid),
                    ));
                }
            }
        }

        // Read previous edge state for EMA continuity.
        let prev = *self.global_relations.read().unwrap();
        let mut new_edges = prev.edges;
        let mut n_active = 0u8;

        for src in 0..N_OBJECT_SLOTS {
            for dst in 0..N_OBJECT_SLOTS {
                if src == dst {
                    continue;
                }
                let idx = edge_idx(src, dst);
                let edge = &mut new_edges[idx];
                edge.src_slot_id = src as u8;
                edge.dst_slot_id = dst as u8;

                let decay = |v: f64| v * (1.0 - REL_ALPHA);
                let Some((s_bias, s_err, s_t, s_share, s_spatial)) = slot_data[src] else {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
                    edge.spatial_distance_score = decay(edge.spatial_distance_score);
                    edge.spatial_overlap_score = decay(edge.spatial_overlap_score);
                    continue;
                };
                let Some((d_bias, d_err, d_t, d_share, d_spatial)) = slot_data[dst] else {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
                    edge.spatial_distance_score = decay(edge.spatial_distance_score);
                    edge.spatial_overlap_score = decay(edge.spatial_overlap_score);
                    continue;
                };

                if s_share <= 1e-6 || d_share <= 1e-6 {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
                    edge.spatial_distance_score = decay(edge.spatial_distance_score);
                    edge.spatial_overlap_score = decay(edge.spatial_overlap_score);
                    continue;
                }

                edge.active = true;
                n_active += 1;

                // co_change: similarity of attention_bias trajectories.
                let obs_co = (1.0 - (s_bias - d_bias).abs()).clamp(0.0, 1.0);
                edge.co_change_score =
                    REL_ALPHA * obs_co + (1.0 - REL_ALPHA) * edge.co_change_score;

                // lead_lag: (t_dst - t_src) / window; positive = src precedes dst.
                let t_diff_i = d_t as i64 - s_t as i64;
                let lag_obs = (t_diff_i as f64 / CROSS_MODAL_WINDOW_US as f64).clamp(-1.0, 1.0);
                edge.lead_lag_score = REL_ALPHA * lag_obs + (1.0 - REL_ALPHA) * edge.lead_lag_score;

                // predictive: src err spike before dst err spike.
                let obs_pred = (s_err * d_err * 0.0_f64.max(lag_obs)).clamp(0.0, 1.0);
                edge.predictive_score =
                    REL_ALPHA * obs_pred + (1.0 - REL_ALPHA) * edge.predictive_score;

                // relation_confidence: scales with both slots' attention shares.
                let obs_conf = (4.0 * s_share * d_share).clamp(0.0, 1.0);
                edge.relation_confidence =
                    REL_ALPHA * obs_conf + (1.0 - REL_ALPHA) * edge.relation_confidence;

                if let (Some(src_sp), Some(dst_sp)) = (s_spatial, d_spatial) {
                    let obs_distance = spatial_distance_score(src_sp, dst_sp);
                    let obs_overlap = spatial_overlap_score(src_sp, dst_sp);
                    edge.spatial_distance_score =
                        REL_ALPHA * obs_distance + (1.0 - REL_ALPHA) * edge.spatial_distance_score;
                    edge.spatial_overlap_score =
                        REL_ALPHA * obs_overlap + (1.0 - REL_ALPHA) * edge.spatial_overlap_score;
                } else {
                    edge.spatial_distance_score = decay(edge.spatial_distance_score);
                    edge.spatial_overlap_score = decay(edge.spatial_overlap_score);
                }

                edge.last_update_t_us = now_us;
            }
        }

        *self.global_relations.write().unwrap() = GlobalRelationState {
            edges: new_edges,
            n_active_edges: n_active,
            last_update_t_us: now_us,
        };
    }

    pub fn read_global_relations(&self) -> GlobalRelationState {
        *self.global_relations.read().unwrap()
    }

    /// Phase 5E: run the AR-1 relation-level predictor for every directed edge.
    ///
    /// Must be called AFTER `refresh_global_relations` so that edge scores are
    /// current. For each active edge, the predictor:
    ///   1. Computes per-dimension errors against the previous EMA prediction.
    ///   2. Updates the EMA for each of the 4 scores.
    ///   3. Writes prediction output (predicted_*, relation_err, relation_surprise)
    ///      back into `GlobalRelationState.edges[i].pred`.
    ///
    /// Inactive edges reset the predictor bank entry to Default so the next
    /// time the edge activates it starts a fresh warmup window.
    ///
    /// Rules:
    ///   * Takes write locks on both `global_relations` and `relation_pred_bank`.
    ///     These are different Viewport fields — no deadlock risk.
    ///   * During warmup (`warmup < REL_PRED_WARMUP_MIN`): `relation_err = 0`,
    ///     `relation_surprise = 0` — no fake high confidence.
    ///   * Does NOT touch claim/scoring, band families, fusion, adapters, or
    ///     the attention allocator.
    pub fn refresh_global_relation_predictor(&self, now_us: u64) {
        let mut gr = self.global_relations.write().unwrap();
        let mut bank = self.relation_pred_bank.write().unwrap();
        let mut lifecycle_traces = Vec::new();
        let mut resolved_publications = Vec::new();

        for i in 0..N_REL_EDGES {
            let edge = &mut gr.edges[i];
            let ps = &mut bank.states[i];

            if !edge.active {
                *ps = RelationEdgePredState::default();
                edge.pred = RelationEdgePredictionState {
                    last_predict_t_us: now_us,
                    ..Default::default()
                };
                continue;
            }

            // Prediction errors: observed minus previous EMA (= previous prediction).
            let err_co = edge.co_change_score - ps.ema_co;
            let err_lag = edge.lead_lag_score - ps.ema_lag;
            let err_pred_s = edge.predictive_score - ps.ema_pred;
            let err_conf = edge.relation_confidence - ps.ema_conf;

            // Update AR-1 EMA for each dimension.
            ps.ema_co = REL_PRED_ALPHA * edge.co_change_score + (1.0 - REL_PRED_ALPHA) * ps.ema_co;
            ps.ema_lag = REL_PRED_ALPHA * edge.lead_lag_score + (1.0 - REL_PRED_ALPHA) * ps.ema_lag;
            ps.ema_pred =
                REL_PRED_ALPHA * edge.predictive_score + (1.0 - REL_PRED_ALPHA) * ps.ema_pred;
            ps.ema_conf =
                REL_PRED_ALPHA * edge.relation_confidence + (1.0 - REL_PRED_ALPHA) * ps.ema_conf;

            // RMS error across all 4 predicted dimensions.
            let rms_err = ((err_co * err_co
                + err_lag * err_lag
                + err_pred_s * err_pred_s
                + err_conf * err_conf)
                / 4.0)
                .sqrt();

            // Update EMA of squared error (feeds relation_surprise).
            ps.ema_mse = (1.0 - REL_PRED_SURPRISE_ALPHA) * ps.ema_mse
                + REL_PRED_SURPRISE_ALPHA * rms_err * rms_err;

            ps.warmup = ps.warmup.saturating_add(1);

            // Write prediction output.
            edge.pred.predicted_co_change = ps.ema_co;
            edge.pred.predicted_lead_lag = ps.ema_lag;
            edge.pred.predicted_predictive = ps.ema_pred;
            edge.pred.predicted_confidence = ps.ema_conf;
            edge.pred.last_predict_t_us = now_us;

            if ps.warmup < REL_PRED_WARMUP_MIN {
                edge.pred.relation_err = 0.0;
                edge.pred.relation_surprise = 0.0;
                lifecycle_traces.push((
                    i,
                    edge.src_slot_id,
                    edge.dst_slot_id,
                    PredictorLifecycleState::Warmup,
                ));
            } else {
                edge.pred.relation_err = rms_err;
                edge.pred.relation_surprise = ps.ema_mse.min(REL_PRED_SURPRISE_MAX);
                resolved_publications.push((i, edge.src_slot_id, edge.dst_slot_id));
            }
        }

        drop(bank);
        drop(gr);

        for (edge_index, src_slot_id, dst_slot_id, lifecycle) in lifecycle_traces {
            self.record_vpc_relation_edge_predictor_trace(
                VPC_FAMILY_RELATION,
                edge_index,
                src_slot_id,
                dst_slot_id,
                now_us,
                now_us,
                lifecycle,
            );
        }
        for (edge_index, src_slot_id, dst_slot_id) in resolved_publications {
            self.record_vpc_relation_edge_resolved_publication(
                VPC_FAMILY_RELATION,
                edge_index,
                src_slot_id,
                dst_slot_id,
                now_us,
                now_us,
            );
        }
    }

    /// Phase 5F: compute cross-object reasoning / routing state.
    ///
    /// Reads `global_attention` and `global_relations` (post Phase-5E predictor).
    /// Writes `global_reasoning`. Does NOT touch claim/scoring, adapters, fusion,
    /// or any relation/predictor logic.
    ///
    /// Ordered call: must run after `refresh_global_relation_predictor`.
    pub fn refresh_global_reasoning(&self, now_us: u64) {
        let ga = *self.global_attention.read().unwrap();
        let gr = *self.global_relations.read().unwrap();

        // Per-edge routing weight = relation_err × relation_confidence × attn_product.
        // attn_product = 4 × share_src × share_dst  (maps AM-GM product to [0,1]).
        let mut edge_raw_weights = [0.0f64; N_REL_EDGES];
        let mut slot_scores = [0.0f64; N_OBJECT_SLOTS];
        let mut surprise_sum = 0.0f64;
        let mut instability_sum = 0.0f64;
        let mut surprise_weight_sum = 0.0f64;
        let mut instability_weight_sum = 0.0f64;
        let mut coord_num = 0.0f64;
        let mut coord_denom = 0.0f64;

        for (i, edge) in gr.edges.iter().enumerate() {
            if !edge.active {
                continue;
            }

            let share_src = ga.attention_shares[edge.src_slot_id as usize];
            let share_dst = ga.attention_shares[edge.dst_slot_id as usize];
            let attn_product = (4.0 * share_src * share_dst).clamp(0.0, 1.0);

            // Routing weight: surprise on this edge, scaled by attention and confidence.
            let rw = edge.pred.relation_err * edge.relation_confidence * attn_product;
            edge_raw_weights[i] = rw;
            slot_scores[edge.src_slot_id as usize] += rw;
            slot_scores[edge.dst_slot_id as usize] += rw;

            // Surprise load: weighted relation_err, normalized by attention weight sum.
            surprise_sum += edge.pred.relation_err * attn_product;
            surprise_weight_sum += attn_product;

            // Network instability: surprise + temporal structure change, normalized by attention weight sum.
            instability_sum += (edge.pred.relation_err + edge.lead_lag_score.abs()) * attn_product;
            instability_weight_sum += attn_product;

            // Coordinated event: weighted co_change_score.
            let w = edge.relation_confidence * attn_product;
            coord_num += edge.co_change_score * w;
            coord_denom += w;
        }

        // Phase 5F.1: Normalize by actual evidence weight sum, not edge count.
        // Low-attention edges cannot dilute scores by merely being counted.
        let global_surprise_load = if surprise_weight_sum > 1e-12 {
            (surprise_sum / surprise_weight_sum).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let network_instability_score = if instability_weight_sum > 1e-12 {
            (instability_sum / instability_weight_sum).clamp(0.0, 2.0)
        } else {
            0.0
        };
        let coordinated_event_score = if coord_denom > 1e-12 {
            (coord_num / coord_denom).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Phase B2.0: the perturbation_graph's own surprise / coordination /
        // instability. The slot-edge fabric above is the legacy routing surface;
        // the graph IS the substrate the design doc wants reasoning to live on.
        // - surprise = "uncommittedness" (1 - strength*expressivity): a
        //   many-weak-relations graph is uncommitted (high surprise); a
        //   few-strong-relations graph has habits covering most perturbations
        //   (low surprise). (True per-relation predictive error is the dead C6
        //   post_hoc_check's job; until then, uncommittedness is the proxy.)
        // - coordination = graph_density (denser graph → more seen together).
        // - instability = spread of relation maturities (structurally shifting).
        let (graph_surprise, graph_coord, graph_instability, n_relations) = {
            let g = self.perturbation_graph.read().unwrap();
            let n = g.relations.len();
            if n == 0 {
                (0.0, 0.0, 0.0, 0usize)
            } else {
                let mut s = 0.0;
                for r in &g.relations {
                    s += 1.0 - r.strength * r.expressivity;
                }
                (
                    (s / n as f64).clamp(0.0, 1.0),
                    g.graph_density(),
                    g.graph_instability(),
                    n,
                )
            }
        };
        // B2.0 maturity-ramped blend (replaces the fixed PG_BLEND=0.30): a cold
        // graph leans on the slot fabric (no regression / no bootstrap deadlock),
        // a mature graph (>= PG_REASONING_FULL_RELATIONS relations) fully owns the
        // three reasoning scalars — at which point slot attention_shares stop
        // feeding them. routing_focus_* below is still slot-sourced (that is B2.1).
        let pg_blend = crate::pg_reasoning_blend(n_relations);
        let global_surprise_load =
            ((1.0 - pg_blend) * global_surprise_load + pg_blend * graph_surprise).clamp(0.0, 1.0);
        let coordinated_event_score =
            ((1.0 - pg_blend) * coordinated_event_score + pg_blend * graph_coord).clamp(0.0, 1.0);
        let network_instability_score = ((1.0 - pg_blend) * network_instability_score
            + pg_blend * graph_instability)
            .clamp(0.0, 2.0);

        // Top-N_ROUTING_FOCUS_EDGES edges by routing weight.
        let mut edge_order: [(f64, usize); N_REL_EDGES] =
            core::array::from_fn(|i| (edge_raw_weights[i], i));
        edge_order
            .sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let total_rw: f64 = edge_order.iter().map(|(w, _)| w).sum();

        let mut routing_focus_edge_ids = [-1i16; N_ROUTING_FOCUS_EDGES];
        let mut routing_weights = [0.0f64; N_ROUTING_FOCUS_EDGES];
        for k in 0..N_ROUTING_FOCUS_EDGES.min(N_REL_EDGES) {
            if edge_order[k].0 <= 0.0 {
                break;
            }
            routing_focus_edge_ids[k] = edge_order[k].1 as i16;
            routing_weights[k] = if total_rw > 1e-12 {
                edge_order[k].0 / total_rw
            } else {
                0.0
            };
        }

        // Top-N_ROUTING_FOCUS_SLOTS slots by accumulated routing score.
        let mut slot_order: [(f64, usize); N_OBJECT_SLOTS] =
            core::array::from_fn(|i| (slot_scores[i], i));
        slot_order
            .sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut routing_focus_slot_ids = [-1i8; N_ROUTING_FOCUS_SLOTS];
        for k in 0..N_ROUTING_FOCUS_SLOTS {
            if slot_order[k].0 > 0.0 {
                routing_focus_slot_ids[k] = slot_order[k].1 as i8;
            }
        }

        *self.global_reasoning.write().unwrap() = GlobalReasoningState {
            global_surprise_load,
            coordinated_event_score,
            network_instability_score,
            routing_focus_slot_ids,
            routing_focus_edge_ids,
            routing_weights,
            last_update_t_us: now_us,
        };
    }

    pub fn read_global_reasoning(&self) -> GlobalReasoningState {
        *self.global_reasoning.read().unwrap()
    }

    /// Phase 5G: Refresh temporal memory substrate.
    ///
    /// Ordered call: must run after `refresh_global_reasoning`.
    /// Reads: current `global_reasoning` state.
    /// Writes: `global_reasoning_history` (ring buffer) and
    ///         `global_reasoning_memory_summary` (snapshot-visible summary).
    ///
    /// Computes numeric-only temporal metrics:
    /// - novelty_score: deviation from recent EMA baseline
    /// - persistence_score: duration of high-intensity pattern
    /// - routing_stability_score: consistency of routing focus
    /// - pattern_recurrence_score: similarity to recent history
    pub fn refresh_global_reasoning_history(&self, now_us: u64) {
        let rs = *self.global_reasoning.read().unwrap();
        let mut hist = self.global_reasoning_history.write().unwrap();
        let mut summary = self.global_reasoning_memory_summary.write().unwrap();

        // Extract top focus elements
        let top_slot = rs.routing_focus_slot_ids[0];
        let top_edge = rs.routing_focus_edge_ids[0];

        // Update EMA baseline for novelty calculation
        // Alpha is small so baseline tracks medium-term average
        // Initialize EMA with first non-zero value to avoid cold-start issues
        if hist.surprise_ema < 1e-12 && rs.global_surprise_load > 1e-12 {
            hist.surprise_ema = rs.global_surprise_load;
        } else {
            hist.surprise_ema = (1.0 - REASONING_HISTORY_ALPHA) * hist.surprise_ema
                + REASONING_HISTORY_ALPHA * rs.global_surprise_load;
        }

        // Calculate novelty: how much current surprise deviates from baseline
        // Use absolute deviation normalized by (baseline + current) to handle edge cases
        let baseline = hist.surprise_ema.max(1e-6);
        let deviation = (rs.global_surprise_load - hist.surprise_ema).abs();
        // Normalize: if baseline is small but deviation is large, still show novelty
        let novelty_score = if rs.global_surprise_load > 0.01 {
            // Non-trivial surprise: calculate relative deviation
            let relative_dev = deviation / (baseline + rs.global_surprise_load);
            relative_dev.clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Update persistence tracking
        // High intensity = significant surprise OR significant instability
        let is_high_intensity = rs.global_surprise_load > REASONING_INTENSITY_THRESHOLD
            || rs.network_instability_score > REASONING_INTENSITY_THRESHOLD;
        if is_high_intensity {
            hist.high_intensity_count = hist.high_intensity_count.saturating_add(1);
        } else {
            hist.high_intensity_count = 0;
        }
        let persistence_score = if hist.high_intensity_count >= PERSISTENCE_MIN_CYCLES {
            let normalized = (hist.high_intensity_count - PERSISTENCE_MIN_CYCLES + 1) as f64
                / (PERSISTENCE_MAX_CYCLES - PERSISTENCE_MIN_CYCLES + 1) as f64;
            normalized.min(1.0)
        } else {
            0.0
        };

        // Update routing stability tracking
        // Only count stability if there's a valid focus slot
        let slot_changed = top_slot != hist.last_top_slot || top_slot < 0;
        if slot_changed {
            hist.slot_unchanged_count = 0;
        } else if top_slot >= 0 {
            // Only increment if we have a valid focus
            hist.slot_unchanged_count = hist.slot_unchanged_count.saturating_add(1);
        }
        hist.last_top_slot = top_slot;
        hist.last_top_edge = top_edge;

        // Routing stability: how long has focus remained stable
        // Normalize by assuming 10 cycles = fully stable
        let routing_stability_score = if top_slot >= 0 {
            (hist.slot_unchanged_count as f64 / 10.0).min(1.0)
        } else {
            0.0
        };

        // Calculate pattern recurrence: similarity to recent history
        let current_signature = ReasoningHistoryEntry {
            surprise_load: rs.global_surprise_load,
            coordinated_event: rs.coordinated_event_score,
            network_instability: rs.network_instability_score,
            top_slot_id: top_slot,
            top_edge_id: top_edge,
            t_us: now_us,
        };

        let mut pattern_recurrence_score = 0.0f64;
        if hist.count > 0 {
            // Compare to recent entries (last 8 cycles)
            let compare_window = hist.count.min(8);
            let mut max_similarity = 0.0f64;
            for i in 0..compare_window {
                if let Some(prev) = hist.get_recent(i) {
                    // Simple signature similarity metric
                    let slot_match = if top_slot >= 0 && prev.top_slot_id == top_slot {
                        1.0
                    } else {
                        0.0
                    };
                    let surprise_diff = (rs.global_surprise_load - prev.surprise_load).abs();
                    let surprise_sim = 1.0 - surprise_diff.min(1.0);
                    let coord_diff = (rs.coordinated_event_score - prev.coordinated_event).abs();
                    let coord_sim = 1.0 - coord_diff.min(1.0);

                    // Weighted combination: slot match is strong signal
                    let similarity = 0.4 * slot_match + 0.35 * surprise_sim + 0.25 * coord_sim;
                    max_similarity = max_similarity.max(similarity);
                }
            }
            pattern_recurrence_score = max_similarity;
        }

        // Dominant focus persistence: how long same focus maintained
        let dominant_focus_persistence = if top_slot >= 0 {
            (hist.slot_unchanged_count as f64 / PERSISTENCE_MAX_CYCLES as f64).min(1.0)
        } else {
            0.0
        };

        // History fill ratio
        let history_fill_ratio = hist.count as f64 / REASONING_HISTORY_RING_CAP as f64;

        // Push current entry to ring buffer
        hist.push(current_signature);

        // Update summary
        *summary = GlobalReasoningMemorySummary {
            novelty_score,
            persistence_score,
            routing_stability_score,
            pattern_recurrence_score,
            history_fill_ratio,
            dominant_focus_persistence,
            last_update_t_us: now_us,
        };
    }

    pub fn read_global_reasoning_memory_summary(&self) -> GlobalReasoningMemorySummary {
        *self.global_reasoning_memory_summary.read().unwrap()
    }

    /// E-1: Recall re-descend (公理5 认知索引而非存储).
    ///
    /// A reasoning-history entry is only an INDEX: a gist (surprise / event /
    /// instability signature) plus a back-link to the source slot and the
    /// cycle timestamp. The body of the memory lives in the low layer.
    ///
    /// "Recalling a gist and thinking harder to recover detail" = following the
    /// `top_slot_id` back-link down to the low-layer slot that is still alive in
    /// the viewport, recovering its precise current signature. Read-only.
    ///
    /// `i` is the history index (0 = most recent gist). Returns `None` when the
    /// index is out of range or the gist carried no slot back-link.
    pub fn recall_redescend(&self, i: usize) -> Option<RecallDetail> {
        // 1. Recall the gist (the index entry) — read-only on the history ring.
        let gist = {
            let hist = self.global_reasoning_history.read().unwrap();
            *hist.get_recent(i)?
        };

        // 2. Follow the back-link down to the low layer.
        //    No back-link (-1) → the gist indexed no slot → cannot re-descend.
        if gist.top_slot_id < 0 {
            return None;
        }
        let slot_id = gist.top_slot_id as usize;

        // 3. Re-descend. Historically the target slot's learned cross-layer
        //    relation rule reconstructed the detail by running forward over
        //    the relation-neighbours' current `object_err`. Phase 3 cutover
        //    removed BOTH the slot high-layer subtree (which carried per-slot
        //    `object_err`) and the per-slot cross-layer predictor, so the
        //    reconstruction path is gone — we now always fall back to the raw
        //    live detail carried by the slot itself.
        let slot = self.read_slot(slot_id)?;
        let (reconstructed, reconstructed_err) = (false, 0.0);

        Some(RecallDetail {
            slot_id: gist.top_slot_id,
            edge_id: gist.top_edge_id,
            gist_t_us: gist.t_us,
            surprise_load: gist.surprise_load,
            // The slot still holding a live low-layer signature = detail present.
            detail_present: slot.active,
            slot_last_update_t_us: slot.last_update_t_us,
            source_modality: slot.source_modality,
            reconstructed,
            reconstructed_err,
        })
    }

    pub fn read_vpc_contract(&self) -> VpcContractState {
        self.vpc_contract.read().unwrap().clone()
    }

    pub fn record_vpc_predictor_trace(
        &self,
        predictor_family: u8,
        domain: ObservationDomainRef,
        horizon: PredictionHorizon,
        lifecycle: PredictorLifecycleState,
    ) {
        self.vpc_contract.write().unwrap().record_predictor_trace(
            predictor_family,
            domain,
            horizon,
            lifecycle,
        );
    }

    pub fn record_vpc_axis_predictor_trace(
        &self,
        predictor_family: u8,
        axis: usize,
        publish_t_us: u64,
        target_t_us: u64,
        lifecycle: PredictorLifecycleState,
    ) {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::Axis,
            primary: axis as u16,
            secondary: 0,
            tertiary: 0,
        };
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: target_t_us,
            target_end_t_us: target_t_us,
        };
        self.record_vpc_predictor_trace(predictor_family, domain, horizon, lifecycle);
    }

    pub fn record_vpc_slot_family_predictor_trace(
        &self,
        predictor_family: u8,
        slot_id: usize,
        family_locator: u16,
        publish_t_us: u64,
        target_t_us: u64,
        lifecycle: PredictorLifecycleState,
    ) {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::SlotFamily,
            primary: slot_id as u16,
            secondary: family_locator,
            tertiary: 0,
        };
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: target_t_us,
            target_end_t_us: target_t_us,
        };
        self.record_vpc_predictor_trace(predictor_family, domain, horizon, lifecycle);
    }

    pub fn record_vpc_relation_edge_predictor_trace(
        &self,
        predictor_family: u8,
        edge_index: usize,
        src_slot_id: u8,
        dst_slot_id: u8,
        publish_t_us: u64,
        target_t_us: u64,
        lifecycle: PredictorLifecycleState,
    ) {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::RelationEdge,
            primary: edge_index as u16,
            secondary: src_slot_id as u16,
            tertiary: dst_slot_id as u16,
        };
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: target_t_us,
            target_end_t_us: target_t_us,
        };
        self.record_vpc_predictor_trace(predictor_family, domain, horizon, lifecycle);
    }

    pub fn record_vpc_relation_edge_resolved_publication(
        &self,
        predictor_family: u8,
        edge_index: usize,
        src_slot_id: u8,
        dst_slot_id: u8,
        publish_t_us: u64,
        target_t_us: u64,
    ) -> u32 {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::RelationEdge,
            primary: edge_index as u16,
            secondary: src_slot_id as u16,
            tertiary: dst_slot_id as u16,
        };
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: target_t_us,
            target_end_t_us: target_t_us,
        };
        self.vpc_contract
            .write()
            .unwrap()
            .record_resolved_publication(PredictionPublicationTrace {
                predictor_family,
                domain,
                horizon,
                alignment: ErrAlignmentKind::PointExact,
                status: PublicationStatus::Resolved,
            })
    }

    pub fn record_vpc_axis_publication(
        &self,
        predictor_family: u8,
        axis: usize,
        publish_t_us: u64,
        target_t_us: u64,
    ) -> u32 {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::Axis,
            primary: axis as u16,
            secondary: 0,
            tertiary: 0,
        };
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: target_t_us,
            target_end_t_us: target_t_us,
        };
        self.vpc_contract.write().unwrap().record_open_publication(
            PredictionPublicationTrace {
                predictor_family,
                domain,
                horizon,
                alignment: ErrAlignmentKind::PointExact,
                status: PublicationStatus::Open,
            },
            predictor_family,
            domain,
            horizon,
            ErrAlignmentKind::PointExact,
        )
    }

    pub fn resolve_vpc_axis_publication(
        &self,
        predictor_family: u8,
        axis: usize,
        target_t_us: u64,
    ) -> bool {
        let domain = ObservationDomainRef {
            kind: ObservationDomainKind::Axis,
            primary: axis as u16,
            secondary: 0,
            tertiary: 0,
        };
        self.vpc_contract.write().unwrap().resolve_open_publication(
            predictor_family,
            domain,
            target_t_us,
            target_t_us,
        )
    }

    // Rejected naming-layer API calls were removed by cleanup spec
    // 2026-04-20. They are not part of the substrate API.

    pub fn push_observation(&self, axis: usize, t_us: u64, value: f64) {
        let mut w = self.observed[axis].write().unwrap();
        w.push(Sample {
            t_us,
            value,
            valid: true,
        });
        self.source_writes.fetch_add(1, Ordering::Relaxed);
        // Interoceptive axes are internal body signals, not external sensory input.
        // Updating last_obs_t_us from them would reset the continuity clock and
        // prevent prolonged silence from depleting the reserve.
        let intero_range =
            crate::AXIS_INTERO_BASE..crate::AXIS_INTERO_BASE + crate::INTERO_AXES;
        if !intero_range.contains(&axis) {
            self.last_obs_t_us.store(t_us, Ordering::Relaxed);
        }
    }

    pub fn read_last_obs_t_us(&self) -> u64 {
        self.last_obs_t_us.load(Ordering::Relaxed)
    }

    pub fn push_prediction(&self, axis: usize, t_us: u64, value: f64) {
        let mut w = self.predicted[axis].write().unwrap();
        w.push(Sample {
            t_us,
            value,
            valid: true,
        });
    }

    pub fn push_err(&self, axis: usize, t_us: u64, err: f64) {
        let mut w = self.err_stream[axis].write().unwrap();
        w.push(Sample {
            t_us,
            value: err,
            valid: true,
        });
    }

    pub fn read_latest_observations(&self, axis: usize, n: usize) -> Vec<Sample> {
        let r = self.observed[axis].read().unwrap();
        r.last(n)
    }

    pub fn latest_observation(&self, axis: usize) -> Option<Sample> {
        let r = self.observed[axis].read().unwrap();
        r.latest()
    }

    pub fn latest_err(&self, axis: usize) -> Option<f64> {
        let r = self.err_stream[axis].read().unwrap();
        r.latest().map(|s| s.value)
    }

    pub fn latest_prediction(&self, axis: usize) -> Option<f64> {
        let r = self.predicted[axis].read().unwrap();
        r.latest().map(|s| s.value)
    }

    /// B1 (VPC mechanism 5): write a prior for `axis`. Called by a cross-layer
    /// predictor that has learned to predict this axis's error. Lock-free.
    /// A2 precision-war guard rule 2: magnitude hard-clamped to
    /// ±MAX_PRIOR_MAGNITUDE (non-finite collapses to 0), so a diverged high
    /// layer can never push an unbounded value into the low layer's additive
    /// input.
    pub fn push_prior(&self, axis: usize, prior: f64) {
        let v = if prior.is_finite() {
            prior.clamp(-crate::MAX_PRIOR_MAGNITUDE, crate::MAX_PRIOR_MAGNITUDE)
        } else {
            0.0
        };
        self.prior_signal[axis].store(v.to_bits(), Ordering::Relaxed);
    }

    /// B1: read the current prior for `axis` (0.0 if none). Read by the
    /// low-layer predictor; added to its prediction as an input only.
    pub fn read_prior(&self, axis: usize) -> f64 {
        f64::from_bits(self.prior_signal[axis].load(Ordering::Relaxed))
    }

    /// A2: replace the observer-facing mirror of the axis-level cross-layer
    /// bank's learned state. Written by the body's global refresh cycle right
    /// after the bank ticks; read by the snapshot writer (pure numbers).
    pub fn store_axis_xlayer_states(
        &self,
        states: Vec<crate::vpc_cross_layer::AxisCrossLayerState>,
    ) {
        let mut w = self
            .axis_xlayer_states
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *w = states;
    }

    /// A2: read the current mirror of the axis-level cross-layer bank state.
    pub fn read_axis_xlayer_states(&self) -> Vec<crate::vpc_cross_layer::AxisCrossLayerState> {
        self.axis_xlayer_states
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn update_predictor(&self, axis: usize, a: f64, b: f64, last_err: f64) {
        let mut ps = self.predictors[axis].write().unwrap();
        ps.a = a;
        ps.b = b;
        ps.last_err = last_err;
        ps.total_updates = ps.total_updates.wrapping_add(1);
        let alpha = 0.02_f64;
        ps.mean_sq_err = (1.0 - alpha) * ps.mean_sq_err + alpha * last_err * last_err;
    }

    pub fn set_surprise(&self, axis: usize, v: f64) {
        let mut ps = self.predictors[axis].write().unwrap();
        ps.surprise = v;
    }

    pub fn read_predictor(&self, axis: usize) -> (f64, f64, f64, f64, f64, u64) {
        let ps = self.predictors[axis].read().unwrap();
        (
            ps.a,
            ps.b,
            ps.surprise,
            ps.last_err,
            ps.mean_sq_err,
            ps.total_updates,
        )
    }

    pub fn mark_snapshot_fail(&self) {
        self.snapshot_fail_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snap_fail_count(&self) -> u64 {
        self.snapshot_fail_count.load(Ordering::Relaxed)
    }

    pub fn set_hearing_fault(&self, fault: bool) {
        self.hearing_fault.store(fault, Ordering::Relaxed);
    }

    pub fn report_audio_error(&self) {
        self.hearing_fault.store(true, Ordering::Relaxed);
        self.hearing_fault_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hearing_status(&self) -> (bool, u64) {
        (
            self.hearing_fault.load(Ordering::Relaxed),
            self.hearing_fault_count.load(Ordering::Relaxed),
        )
    }

    pub fn mark_thread_fault(&self) {
        self.thread_fault.store(true, Ordering::Relaxed);
        self.thread_fault_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn thread_fault_status(&self) -> (bool, u64) {
        (
            self.thread_fault.load(Ordering::Relaxed),
            self.thread_fault_count.load(Ordering::Relaxed),
        )
    }

    pub fn update_hl_state(&self, state: HighLevelState) {
        let mut hl = self.hl.write().unwrap();
        *hl = state;
    }

    pub fn read_hl_state(&self) -> (f64, f64, f64, f64, f64) {
        let hl = self.hl.read().unwrap();
        (
            hl.a,
            hl.last_avg_mse,
            hl.pred_mse,
            hl.meta_err,
            hl.modulation,
        )
    }

    // --- Phase 3A.1: first-class health state ---

    pub fn read_health_state(&self) -> HealthState {
        HealthState::from_u8(self.health_state.load(Ordering::Relaxed))
    }

    /// Phase 4A: PHOT ingress health read. The PHOT adapter is the sole
    /// writer; observers render.
    pub fn read_phot_health(&self) -> HealthState {
        HealthState::from_u8(self.phot_health.load(Ordering::Relaxed))
    }

    /// Phase 4A: PHOT ingress health write. Called by the PHOT adapter only.
    pub fn set_phot_health(&self, h: HealthState) {
        self.phot_health.store(h.as_u8(), Ordering::Relaxed);
    }

    /// Phase 4C.1: Record the time the PHOT provider first became healthy.
    /// Called once by the adapter on its first non-fault tick. Idempotent.
    pub fn set_phot_started_t_us(&self, t_us: u64) {
        self.phot_started_t_us.store(t_us, Ordering::Relaxed);
    }

    pub fn read_phot_started_t_us(&self) -> u64 {
        self.phot_started_t_us.load(Ordering::Relaxed)
    }

    /// Phase 4C.1: Record the time of the most recently received camera frame.
    /// Called by the adapter whenever a frame arrives.
    pub fn set_phot_last_frame_t_us(&self, t_us: u64) {
        self.phot_last_frame_t_us.store(t_us, Ordering::Relaxed);
    }

    pub fn read_phot_last_frame_t_us(&self) -> u64 {
        self.phot_last_frame_t_us.load(Ordering::Relaxed)
    }

    /// Recompute health from substrate fields against the shared thresholds.
    ///
    /// Precedence:
    ///   FAULT wins if hearing fault is set.
    ///   QUIET if no observation has ever arrived, or the last one is stale,
    ///         or the peak observed band is below the quiet energy threshold.
    ///   OK    only if fresh observations exist and at least one band is
    ///         above the quiet energy threshold.
    pub fn update_health_state(&self, now_us: u64) {
        let next = self.compute_health_state(now_us);
        self.health_state.store(next.as_u8(), Ordering::Relaxed);
    }

    fn compute_health_state(&self, now_us: u64) -> HealthState {
        if self.hearing_fault.load(Ordering::Relaxed) {
            return HealthState::Fault;
        }
        let last = self.last_obs_t_us.load(Ordering::Relaxed);
        if last == 0 {
            return HealthState::Quiet;
        }
        if now_us.saturating_sub(last) > QUIET_WINDOW_US {
            return HealthState::Quiet;
        }
        let mut peak = 0.0_f64;
        for axis in 0..AXIS_COUNT {
            if let Some(s) = self.latest_observation(axis) {
                let v = s.value.abs();
                if v > peak {
                    peak = v;
                }
            }
        }
        if peak < QUIET_ENERGY_THRESHOLD {
            HealthState::Quiet
        } else {
            HealthState::Ok
        }
    }

    // --- Endogenous Activity Laws (spec 2026-04-20) ---

    /// Err-driven attention rebalancing (self-regulation law §2.2).
    ///
    /// Historically read per-slot `high_layer.object_err` to bias attention
    /// toward high-err slots. Phase 3 cutover removed the slot high-layer
    /// subtree, so the per-slot residual signal is no longer available here.
    /// With every slot's `err` pinned to 0 the total-err guard fires
    /// immediately and the function becomes a no-op until a replacement
    /// residual source (e.g. fusion.fused_surprise) is wired in.
    ///
    /// `ATTN_ERR_REBALANCE_ALPHA` remains an engineering approximation.
    pub fn tick_err_driven_attention(&self, _now_us: u64) {
        // No per-slot err source after the Phase 3 cutover. Leave attention
        // shares untouched.
    }

    /// Quiet-period history replay (endogenous activity law §2.1).
    ///
    /// When the system is in HEALTH_QUIET (no external stimulus), this
    /// replays entries from the reasoning history ring as微弱 attention
    /// perturbations, maintaining endogenous activity without external input.
    ///
    /// `REPLAY_ALPHA` is an engineering approximation — not a fixed replay law.
    pub fn tick_quiet_history_replay(&self, now_us: u64) {
        let health = self.health_state.load(Ordering::Relaxed);
        if health != HEALTH_QUIET {
            return;
        }

        // Phase C5 / B2.2b: spreading graph replay + consolidation. When the
        // perturbation_graph has a meaningful node population, replay FROM IT:
        // consolidate_quiet_neighborhood picks a salient internal cue, spreads
        // along LEARNED relations, and (a) returns the neighbourhood salience to
        // drive replay_intensity while (b) giving the rehearsed nodes bounded
        // eviction protection — so rest rehearsal resists forgetting without
        // masquerading as live perception. Reasoning-history is the fallback
        // below the 8-node floor.
        let graph_replay_novelty: Option<f64> = self
            .perturbation_graph
            .write()
            .unwrap()
            .consolidate_quiet_neighborhood(now_us, crate::QUIET_REPLAY_SPREAD_DEPTH);

        let replay_novelty = if let Some(n) = graph_replay_novelty {
            n
        } else {
            let history = self.global_reasoning_history.read().unwrap();
            if history.count == 0 {
                return;
            }
            // Pick a recent entry by time-based rotation (deterministic, no RNG).
            let idx = ((now_us / 100_000) as usize) % history.count;
            let entry = match history.get_recent(idx) {
                Some(e) => *e,
                None => return,
            };
            drop(history);
            entry.surprise_load.clamp(0.0, 1.0)
        };

        // Inject replay signal as a small attention perturbation.
        let mut attn = self.global_attention.write().unwrap();
        attn.replay_intensity =
            (1.0 - REPLAY_ALPHA) * attn.replay_intensity + REPLAY_ALPHA * replay_novelty;

        // Perturb the top slot's share by a tiny amount proportional to
        // replay intensity. This keeps the attention distribution flowing
        // even without external input.
        if attn.n_active > 0 && attn.top_slot_ids[0] >= 0 {
            let top = attn.top_slot_ids[0] as usize;
            let perturbation = attn.replay_intensity * REPLAY_ALPHA * 0.1;
            attn.attention_shares[top] = (attn.attention_shares[top] + perturbation).min(1.0);
        }

        attn.last_replay_t_us = now_us;
    }

    // --- Vocal Substrate API (Phase 1, spec 2026-04-23) ---
    // Numeric-only; no text, no TTS, no token ontology.

    pub fn tick_vocal_target_from_vib(&self, now_us: u64) {
        let ambient = self.read_ambient();
        let vib = ambient.vib;
        if !vib.candidate_flag || vib.event_energy < VOCAL_TARGET_INGRESS_MIN_ENERGY {
            return;
        }
        if now_us.saturating_sub(vib.last_event_t_us) > QUIET_WINDOW_US {
            return;
        }
        // Echo guard + vocal self-play. A self-generated VIB return must NOT be
        // chased as an external target (anti self-excitation) — so it is still
        // dropped from the vocal_target path below. BUT the reliable motor→sound
        // contingency it carries IS vocal self-play: the body acts and hears a
        // predictable consequence with NO caregiver. Route a slow, rate-limited
        // self-play closure so babble alone can bootstrap vocal_efficacy +
        // familiarity + the developmental clock — breaking the cold-start
        // deadlock (a newborn's echo-cancelled babble previously produced no
        // learnable consequence at all).
        let self_match = self.compute_vib_self_match(now_us);
        // Phase 1 (2026-06-12): widened from >0.85. compute_vib_self_match is a
        // pure time-since-voicing heuristic over a 150ms half-window, so 0.85
        // meant "VIB within ≈22ms of voicing" — at 50Hz essentially only the
        // single tick right after a burst, which almost never coincides with an
        // echo return. 0.5 = "within ≈75ms" (≈3-4 ticks), a realistic babble→
        // hear-self latency, so solitary babble can actually bootstrap. Honesty
        // is unchanged: the closure sink still re-derives acoustic fit and gates
        // on self_bands>1e-6; a weak/absent self-echo still credits ~nothing.
        if self_match > crate::VOCAL_SELF_PLAY_MATCH_MIN {
            let last_sp = self.last_self_play_t_us.load(Ordering::Relaxed);
            if now_us.saturating_sub(last_sp) >= crate::VOCAL_SELF_PLAY_MIN_INTERVAL_US {
                // closure_support is the motor→sound contingency strength (how much
                // sound the act produced), NOT the echo timing. self_match (the
                // ~150ms voiced-recently freshness signal) is the GATE above; it
                // must NOT scale the credit magnitude. Live telemetry showed it did:
                // corr(efficacy, self_match)=+0.69 vs corr(efficacy, shape_dist)
                // (real target match)≈−0.38 — so efficacy tracked "am I voicing in a
                // tight rhythm and hearing my echo," not "do I sound like the
                // target," and craved/crashed when late-call voicing got sparse and
                // self_match collapsed even though shape_dist stayed fine. Dropping
                // self_match from the magnitude lets the acoustic-fit honesty inside
                // observe_vocal_reflex_closure carry the credit (target match), so
                // efficacy reflects competence, not echo timing.
                let support = vib.event_energy.clamp(0.0, 1.0);
                self.observe_vocal_reflex_closure(now_us, support, true);
                self.last_self_play_t_us.store(now_us, Ordering::Relaxed);
            }
            // L1/L2 (PhonemeMemory): self-echo confirmed → refine acoustic
            // fingerprint of the most recently voiced proto.  We only have
            // envelope here (spectral data computed below); spectral fields
            // will be further refined on subsequent self-play iterations.
            {
                use crate::phoneme_memory::AcousticState;
                // P3: real self-spectral fingerprint from our own heard output
                // (replaces the former 0.5/0.5 placeholders).
                let (sb, _) = self.read_self_vocal_bands();
                let acoustic_obs = AcousticState {
                    spectral_center: crate::vocal_self_analysis::vib_spectral_center(&sb),
                    resonance: crate::vocal_self_analysis::vib_resonance(&sb),
                    envelope: vib.event_energy.clamp(0.0, 1.0),
                };
                self.phoneme_memory
                    .write()
                    .unwrap()
                    .update_acoustic_latest(acoustic_obs, now_us);
            }
            let reduced_energy = vib.event_energy * (1.0 - self_match);
            if reduced_energy < crate::VOCAL_TARGET_INGRESS_MIN_ENERGY {
                return;
            }
        }

        // L3/L4 (PhonemeMemory): non-self-echo VIB → social response candidate.
        // Any external sound arriving within SOCIAL_RESPONSE_WINDOW_US of the
        // last vocalization is attributed as a response; credit_response
        // strengthens the social_response_rate of the proto that spoke.
        if self_match < 0.7 {
            let mut pm = self.phoneme_memory.write().unwrap();
            if let Some((proto_idx, confidence)) =
                pm.observe_vib(Default::default(), self_match, now_us)
            {
                pm.credit_response(proto_idx, confidence);
            }
        }

        let mut bands = [0.0_f64; VIB_BANDS];
        for (axis, band) in bands.iter_mut().enumerate() {
            let Some(sample) = self.latest_observation(axis) else {
                return;
            };
            if !sample.valid || now_us.saturating_sub(sample.t_us) > QUIET_WINDOW_US {
                return;
            }
            *band = sample.value.abs();
        }

        let band_sum = bands.iter().sum::<f64>();
        if band_sum <= f64::EPSILON {
            return;
        }

        let denom = (VIB_BANDS.saturating_sub(1).max(1)) as f64;
        let spectral_center = bands
            .iter()
            .enumerate()
            .map(|(axis, value)| *value * (axis as f64 / denom))
            .sum::<f64>()
            / band_sum;
        let high_band_sum = bands
            .iter()
            .enumerate()
            .filter(|(axis, _)| *axis >= VIB_BANDS / 2)
            .map(|(_, value)| *value)
            .sum::<f64>();
        let resonance = high_band_sum / band_sum;
        let envelope = (vib.surprise.abs()
            / (vib.event_energy.abs() + vib.surprise.abs() + f64::EPSILON))
            .clamp(0.0, 1.0);

        // Finer acoustic representation: carry the raw 64-mel target fingerprint
        // alongside the 16-band contour, but only if the staged frame is fresh
        // (same QUIET_WINDOW_US freshness the band reconstruction above requires).
        // A stale/absent mel frame stays zeros → the self-supervision loop falls
        // back to the 16-band path (no spurious "match" from an empty fingerprint).
        let target_mel64 = {
            let (mel, t) = *self.latest_target_mel64.read().unwrap();
            if now_us.saturating_sub(t) <= QUIET_WINDOW_US {
                mel
            } else {
                [0.0; crate::vocal_self_analysis::BANDS]
            }
        };
        self.observe_vocal_target_full(
            now_us,
            vib.event_energy.clamp(0.0, 1.0),
            spectral_center.clamp(0.0, 1.0),
            resonance.clamp(0.0, 1.0),
            envelope,
            bands,
            target_mel64,
        );
    }

    /// Compute match between current VIB signal and vocal_organ output prediction.
    /// Returns [0.0, 1.0]:
    ///   1.0 = VIB signal is entirely self-generated (echo)
    ///   0.0 = VIB signal is entirely external
    /// Pure efference-copy freshness/precision weight in [0,1]: 1 just after the
    /// body voiced, decaying linearly to 0 at the 150ms DIVA self-feedback guard,
    /// 0 when the organ is idle. "How fresh/trustworthy is this self-hearing as
    /// evidence of the body's own just-produced sound." No side effects — safe to
    /// call on the dense closure hot path (the diag-writing wrapper is separate).
    fn vib_self_match_weight(&self, now_us: u64) -> f64 {
        let organ = self.read_vocal_organ();
        // Use the tick's now_us so the elapsed time since voicing is correct.
        let since_last_vocal = now_us.saturating_sub(organ.last_update_t_us);
        // Guard half-window: 150ms. Past this, treat VIB as external.
        let guard_us = crate::VOCAL_TARGET_WRITE_COUPLED_GUARD_US / 2;
        let active = organ.phase == crate::vocal_organ::VocalOrganPhase::Active
            && organ.breath_drive >= 0.01;
        // Simple time-based decay: shorter since vocal = more likely self.
        if !active || since_last_vocal > guard_us {
            0.0
        } else {
            (1.0 - (since_last_vocal as f64 / guard_us as f64)).clamp(0.0, 1.0)
        }
    }

    fn compute_vib_self_match(&self, now_us: u64) -> f64 {
        let organ = self.read_vocal_organ();
        let since_last_vocal = now_us.saturating_sub(organ.last_update_t_us);
        let m = self.vib_self_match_weight(now_us);
        // Phase 0 diagnosis: record the gate components so a stalled loop reveals
        // WHICH condition fails (phase idle? breath decayed? window missed?).
        {
            let mut diag = self.vocal_diag.write().unwrap();
            diag.last_self_match = m;
            diag.last_organ_phase = organ.phase.as_u8();
            diag.last_breath_drive = organ.breath_drive;
            diag.last_since_vocal_us = since_last_vocal;
        }
        m
    }

    fn has_recent_write_coupled_vib_caution(&self, now_us: u64) -> bool {
        let out = self.action_output.read().unwrap();
        if out.channels.iter().any(|ch| {
            ch.phase != ActionOutputPhase::Idle
                && now_us.saturating_sub(ch.last_update_t_us) <= VOCAL_TARGET_WRITE_COUPLED_GUARD_US
        }) {
            return true;
        }
        out.write_epoch_history
            .iter()
            .rev()
            .any(|ep| now_us.saturating_sub(ep.start_t_us) <= VOCAL_TARGET_WRITE_COUPLED_GUARD_US)
    }

    pub fn observe_vocal_target(
        &self,
        now_us: u64,
        energy: f64,
        pitch: f64,
        resonance: f64,
        envelope: f64,
    ) {
        self.observe_vocal_target_bands(now_us, energy, pitch, resonance, envelope, [0.0; VIB_BANDS]);
    }

    /// P3: like observe_vocal_target but retains the full 16-band target contour
    /// for band-wise self↔target comparison (the acoustic loop). The fine 64-mel
    /// target fingerprint defaults to zeros on this path (callers without a raw
    /// mel frame, e.g. tests/parametric ingress).
    pub fn observe_vocal_target_bands(
        &self,
        now_us: u64,
        energy: f64,
        pitch: f64,
        resonance: f64,
        envelope: f64,
        bands: [f64; VIB_BANDS],
    ) {
        self.observe_vocal_target_full(
            now_us, energy, pitch, resonance, envelope, bands,
            [0.0; crate::vocal_self_analysis::BANDS],
        );
    }

    /// P3 (finer acoustic representation): retain BOTH the 16-band VIB contour
    /// and the full 64-mel target fingerprint. The 64-mel path feeds the vocal
    /// self-supervision loop (imitation / efficacy / developmental clock), which
    /// needs formant-level resolution to tell whether the body's own voice
    /// actually matches the target — the 16-band view collapses distinct vowels.
    pub fn observe_vocal_target_full(
        &self,
        now_us: u64,
        energy: f64,
        pitch: f64,
        resonance: f64,
        envelope: f64,
        bands: [f64; VIB_BANDS],
        mel64: [f64; crate::vocal_self_analysis::BANDS],
    ) {
        let mut vt = self.vocal_target.write().unwrap();
        vt.observe_external_contour(now_us, energy, pitch, resonance, envelope, bands, mel64);
    }

    pub fn read_vocal_target(&self) -> VocalTargetState {
        *self.vocal_target.read().unwrap()
    }

    /// P3: hear our own synthesized PCM through the same analyzer the target
    /// uses, storing the resulting 16 VIB bands. Called after synthesis each tick.
    pub fn observe_self_vocal_pcm(&self, now_us: u64, samples: &[f32]) {
        let (bands, mel64) = {
            let mut ear = self.self_ear.write().unwrap();
            let bands = ear.observe(samples);
            (bands, ear.read_self_mel64())
        };
        *self.self_vocal_bands.write().unwrap() = (bands, now_us);
        *self.self_vocal_mel64.write().unwrap() = (mel64, now_us);
    }

    pub fn read_self_vocal_bands(&self) -> ([f64; VIB_BANDS], u64) {
        *self.self_vocal_bands.read().unwrap()
    }

    /// Latest self-heard 64-mel fingerprint + timestamp.
    pub fn read_self_vocal_mel64(&self) -> ([f64; crate::vocal_self_analysis::BANDS], u64) {
        *self.self_vocal_mel64.read().unwrap()
    }

    /// Stage the latest raw 64-mel target fingerprint from the sensory frame.
    /// The gated vocal-target write (tick_vocal_target_from_vib) later copies it,
    /// fresh-only, into VocalTargetState so the fine target tracks the same gating
    /// as the 16-band contour.
    pub fn stage_target_mel64(&self, now_us: u64, mel64: [f64; crate::vocal_self_analysis::BANDS]) {
        *self.latest_target_mel64.write().unwrap() = (mel64, now_us);
    }

    /// P3 (observable): band-wise perceptual distance between what we just heard
    /// ourselves produce and the current target contour. [0,1].
    pub fn vocal_acoustic_distance(&self) -> f64 {
        let (self_bands, _) = self.read_self_vocal_bands();
        let target = self.read_vocal_target();
        crate::vocal_self_analysis::spectral_distance(&self_bands, &target.target_bands)
    }

    /// P3 (observable): loudness-invariant SHAPE distance between self and target
    /// band distributions — the formant-pattern error the acoustic servo targets.
    pub fn vocal_acoustic_shape_distance(&self) -> f64 {
        let (self_bands, _) = self.read_self_vocal_bands();
        let target = self.read_vocal_target();
        crate::vocal_self_analysis::spectral_shape_distance(&self_bands, &target.target_bands)
    }

    pub fn apply_vocal_write_event(&self, event: ActionOutputEvent, pitch: f64, resonance: f64) {
        let magnitude = event.magnitude.clamp(0.0, 1.0);
        let mut action_magnitude = magnitude;
        let event = ActionOutputEvent { magnitude, ..event };
        match event.feedback {
            ActionOutputFeedback::Begin => {
                let mut organ = self.vocal_organ.write().unwrap();
                organ.begin_voicing(event.event_t_us, magnitude, pitch, resonance);
            }
            ActionOutputFeedback::Adjust => {
                let mut organ = self.vocal_organ.write().unwrap();
                if organ.phase == VocalOrganPhase::Active {
                    organ.adjust_toward(event.event_t_us, magnitude, pitch, resonance);
                    action_magnitude = organ.breath_drive;
                }
            }
            ActionOutputFeedback::Decay => {
                let mut organ = self.vocal_organ.write().unwrap();
                if organ.phase == VocalOrganPhase::Active {
                    organ.breath_drive = (organ.breath_drive - magnitude).max(0.0);
                    organ.last_update_t_us = event.event_t_us;
                    if organ.breath_drive < 0.01 {
                        organ.breath_drive = 0.0;
                        organ.phase = VocalOrganPhase::Idle;
                    }
                }
            }
            ActionOutputFeedback::Stop => {
                let mut organ = self.vocal_organ.write().unwrap();
                organ.breath_drive = 0.0;
                organ.phase = VocalOrganPhase::Idle;
                organ.last_update_t_us = event.event_t_us;
            }
        }
        self.apply_action_output_event(ActionOutputEvent {
            magnitude: action_magnitude,
            ..event
        });
    }

    pub fn apply_probe_vocal_write_event(&self, event: ActionOutputEvent) {
        let event = ActionOutputEvent {
            magnitude: event.magnitude.clamp(0.0, 1.0),
            ..event
        };
        if event.feedback == ActionOutputFeedback::Adjust {
            {
                let mut organ = self.vocal_organ.write().unwrap();
                if organ.phase == VocalOrganPhase::Active {
                    organ.breath_drive = event.magnitude;
                    organ.last_update_t_us = event.event_t_us;
                }
            }
            self.apply_action_output_event(event);
            return;
        }

        let (pitch, resonance) = {
            let organ = self.vocal_organ.read().unwrap();
            if organ.phase == VocalOrganPhase::Active {
                (organ.pitch_tension, organ.resonance_position)
            } else {
                let target = self.vocal_target.read().unwrap();
                if target.active_energy >= 0.01
                    && event.event_t_us.saturating_sub(target.last_update_t_us) <= QUIET_WINDOW_US
                {
                    (target.pitch_hint, target.resonance_hint)
                } else {
                    (0.5, 0.5)
                }
            }
        };
        self.apply_vocal_write_event(event, pitch, resonance);
    }

    pub fn begin_voicing(&self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.apply_vocal_write_event(
            ActionOutputEvent {
                channel_kind: OutputChannelKind::GenericWrite,
                feedback: ActionOutputFeedback::Begin,
                magnitude: breath,
                event_t_us: now_us,
            },
            pitch,
            resonance,
        );
    }

    pub fn adjust_voicing(&self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.apply_vocal_write_event(
            ActionOutputEvent {
                channel_kind: OutputChannelKind::GenericWrite,
                feedback: ActionOutputFeedback::Adjust,
                magnitude: breath,
                event_t_us: now_us,
            },
            pitch,
            resonance,
        );
    }

    pub fn read_vocal_organ(&self) -> VocalOrganState {
        *self.vocal_organ.read().unwrap()
    }

    pub fn read_vocal_imitation(&self) -> VocalImitationState {
        *self.vocal_imitation.read().unwrap()
    }

    pub fn read_vocal_memory(&self) -> VocalMemoryState {
        self.vocal_memory.read().unwrap().clone()
    }

    pub fn read_vocal_reflex(&self) -> VocalReflexState {
        *self.vocal_reflex.read().unwrap()
    }

    pub fn read_auditory_visual(&self) -> AuditoryVisualState {
        *self.auditory_visual.read().unwrap()
    }

    pub fn tick_auditory_visual_from_pairs(&self, now_us: u64) {
        let ac = self.read_action_consequence();
        let out = self.read_action_output();
        let cursor = self.auditory_visual.read().unwrap().pair_cursor;
        let mut samples = Vec::new();

        for pair in ac.pairs.iter().filter(|pair| pair.pair_seq > cursor) {
            let Some(window) = ac
                .windows
                .iter()
                .find(|window| window.window_id == pair.window_id)
            else {
                continue;
            };
            let Some(epoch) = out
                .write_epoch_history
                .iter()
                .find(|epoch| epoch.write_seq == pair.write_seq)
            else {
                continue;
            };
            samples.push((
                pair.pair_seq,
                ReturnSample {
                    delay_us: pair.delay_us,
                    curve_similarity: pair.curve_similarity,
                    return_magnitude: window.magnitude,
                    write_magnitude: epoch.start_magnitude,
                },
            ));
        }

        if samples.is_empty() {
            return;
        }

        let mut av = self.auditory_visual.write().unwrap();
        for (pair_seq, sample) in samples {
            av.observe_self_return(now_us, sample);
            av.pair_cursor = av.pair_cursor.max(pair_seq);
        }
    }

    /// Route action-consequence pair closure to the vocal reflex substrate.
    ///
    /// Reads new pairs formed since the last call, determines whether each
    /// pair corresponds to a vocal output (by comparing write epoch timing
    /// against `last_vocal_output_t_us`), and calls
    /// `observe_vocal_reflex_closure` to update `prediction_familiarity`
    /// and `action_suppression` in `learned_bias`.
    ///
    /// This is the runtime closure path that was previously missing: the
    /// vocal reflex could produce output but never learned from the
    /// consequences of that output.
    pub fn tick_vocal_reflex_from_pairs(&self, now_us: u64) {
        let ac = self.read_action_consequence();
        let out = self.read_action_output();
        let cursor = self.vocal_reflex.read().unwrap().last_pair_cursor;

        for pair in ac.pairs.iter().filter(|pair| pair.pair_seq > cursor) {
            // Determine whether this pair originated from vocal output.
            // Compare the vocal organ's last update time against the write
            // epoch start time. If the vocal organ was active within a
            // window around the epoch, treat it as vocal.
            let vocal_output_t = self.last_vocal_output_t_us.load(Ordering::Relaxed);
            let was_vocal = out
                .write_epoch_history
                .iter()
                .any(|ep| {
                    ep.write_seq == pair.write_seq
                        && vocal_output_t > 0
                        && ep.start_t_us.abs_diff(vocal_output_t) < 500_000 // 500 ms
                });

            let closure_support = pair.curve_similarity;
            self.observe_vocal_reflex_closure(now_us, closure_support, was_vocal);
        }

        // Advance cursor past the highest pair_seq processed.
        let max_pair_seq = ac
            .pairs
            .iter()
            .map(|p| p.pair_seq)
            .max()
            .unwrap_or(cursor);
        if max_pair_seq > cursor {
            let mut reflex = self.vocal_reflex.write().unwrap();
            reflex.last_pair_cursor = max_pair_seq;
        }
    }

    pub fn tick_auditory_visual_from_fusion(&self, now_us: u64) {
        let cursor = self.auditory_visual.read().unwrap().fusion_cursor_t_us;
        let mut samples = Vec::new();
        let mut next_cursor = cursor;

        for slot_id in 0..N_OBJECT_SLOTS {
            let Some(slot) = self.read_slot(slot_id) else {
                continue;
            };
            let Some(fusion) = slot.fusion else {
                continue;
            };
            if fusion.last_fused_t_us <= cursor {
                continue;
            }
            if fusion.last_vib_t_us == 0 || fusion.last_phot_t_us == 0 {
                next_cursor = next_cursor.max(fusion.last_fused_t_us);
                continue;
            }
            let pressure = fusion
                .cross_modal_confidence
                .max(fusion.fused_surprise)
                .max(fusion.vib_weight.min(fusion.phot_weight));
            samples.push(ChannelSample {
                alignment: fusion.temporal_alignment_score,
                confidence: fusion.cross_modal_confidence,
                pressure,
            });
            next_cursor = next_cursor.max(fusion.last_fused_t_us);
        }

        if samples.is_empty() {
            let mut av = self.auditory_visual.write().unwrap();
            av.fusion_cursor_t_us = next_cursor;
            return;
        }

        let mut av = self.auditory_visual.write().unwrap();
        for sample in samples {
            av.observe_channel_alignment(now_us, sample);
        }
        av.fusion_cursor_t_us = next_cursor;
    }

    pub fn tick_auditory_visual_expression(&self, now_us: u64) {
        let drive = self
            .auditory_visual
            .write()
            .unwrap()
            .drive_expression(now_us);
        let Some(drive) = drive else {
            return;
        };

        let is_active = self.read_vocal_organ().phase == VocalOrganPhase::Active;
        if is_active {
            self.adjust_voicing(now_us, drive.breath, drive.pitch, drive.resonance);
        } else {
            self.begin_voicing(now_us, drive.breath, drive.pitch, drive.resonance);
        }
    }

    pub fn observe_auditory_visual_expression_closure(&self, now_us: u64, support: f64) {
        self.auditory_visual
            .write()
            .unwrap()
            .observe_expression_closure(now_us, support);
    }

    pub fn observe_vocal_reflex_closure(&self, now_us: u64, closure_support: f64, vocalized: bool) {
        let (delta_fam, delta_sup) = {
            let mut reflex = self.vocal_reflex.write().unwrap();
            reflex.observe_closure(now_us, closure_support, vocalized)
        };
        // Vocal-acoustic competence. A closure carries competence evidence ONLY
        // when the body actually heard itself voice this round AND can compare
        // that to the target contour. A closure with no heard-self — a silent
        // moment, or no self-hearing data yet — carries NO acoustic evidence and
        // must credit neither efficacy nor the developmental clock. (The earlier
        // "no evidence → fit 1.0, no penalty" fallback was the leak: the
        // action-consequence pair path fires at the orchestrator rate, mostly
        // during silent moments, so those evidence-less closures sailed through
        // ungated and saturated efficacy/maturity to 1.0 — the illusory
        // convergence.) shape_dist is a prediction-error-derived neutral
        // quantity used as a continuous weight, NOT a reward and NOT a threshold
        // gate. None = no evidence, skip; Some(c) = acoustic-fit-weighted level.
        // D-fix: evidence requires FRESH efference-copy self-hearing (the body
        // actually voiced within the 150ms DIVA self-feedback window), not merely
        // a non-silent latched buffer. The old presence gate (self_bands.sum()>1e-6,
        // a threshold ~6-7 orders of magnitude below the operating regime) passed
        // ~100% of closures — including the dense action-consequence pair path
        // firing during SILENT moments — so it rubber-stamped credit and flooded
        // the developmental clock (closures == closures_with_evidence on every row
        // of the 13-min call). vib_self_match_weight>0 ⟺ organ Active && voiced
        // within 150ms. (Brown/Friston active inference: self-generated feedback is
        // precision-weighted, never counted at full strength merely because present.)
        let self_fresh = self.vib_self_match_weight(now_us);
        let acoustic_competence: Option<f64> = if vocalized && self_fresh > 0.0 {
            let (self_bands, _) = self.read_self_vocal_bands();
            if self_bands.iter().sum::<f64>() > 1e-6 {
                let target = self.read_vocal_target();
                // Finer acoustic representation: when BOTH self and target carry a
                // 64-mel fingerprint, compute the shape fit at formant resolution
                // (resolves vowels/syllables the 16-band view collapses → does not
                // hand out credit for a coarse "looks-close" match). Otherwise fall
                // back to the 16-band shape distance (still valid). The 64-mel-empty
                // case here is only a representation fallback, NOT the evidence gate:
                // the self_bands>1e-6 check above is what gates "no evidence → None".
                let (self_mel64, _) = self.read_self_vocal_mel64();
                let self_mel_has = self_mel64.iter().sum::<f64>() > 1e-6;
                let target_mel_has = target.target_mel64.iter().sum::<f64>() > 1e-6;
                let shape_dist = if self_mel_has && target_mel_has {
                    // Efficacy CREDIT keeps the full-resolution mel64 distance
                    // (not the liftered formant metric): the cepstral envelope is
                    // deliberately smooth/source-invariant — great as the servo's
                    // control signal, but it would hand out credit for a coarse
                    // envelope match. The fine TV distance withholds credit the
                    // 16-band path wrongly grants (vocal_mel64_discrimination), an
                    // anti-cheating guard the developmental clock must keep.
                    crate::vocal_self_analysis::spectral_shape_distance_mel64(
                        &self_mel64,
                        &target.target_mel64,
                    )
                } else {
                    crate::vocal_self_analysis::spectral_shape_distance(
                        &self_bands,
                        &target.target_bands,
                    )
                };
                let per_frame_fit = (1.0 - shape_dist).clamp(0.0, 1.0);
                // #8 Stage 2: a sequence/utterance is more than one frame. The DTW
                // alignment fit (SAME strict full-mel64 cell cost — no liftered
                // shortcut) credits getting the temporal ORDER right, which can
                // exceed the single-frame timbre floor (~0.48) that caps
                // per_frame_fit at ~0.52 and pins efficacy ~0.23. max() can only
                // LIFT efficacy, never lower it; None (buffer too short) → 0 → no
                // change. The self_bands>1e-6 evidence gate above is untouched, so
                // there is no evidence-less inflation (the old maturity=1.0 leak).
                let seq_fit = self.vocal_sequence.read().unwrap().seq_fit().unwrap_or(0.0);
                // F-fix lives in vocal_sequence::seq_fit (symmetric2 DTW so the
                // (n+m) divisor is EXACT → seq_fit is a true average-per-step
                // distance, no longer upward-biased). max() is then a fair "best of
                // single-frame timbre vs temporal-order" view — it preserves the #8
                // sequence-layer breakthrough (which broke the 0.23 timbre floor)
                // without the old normalization inflating it.
                let fit = per_frame_fit.max(seq_fit);
                // self_fresh is the evidence GATE only (checked above), deliberately
                // NOT folded into the magnitude: the self-play path's closure_support
                // already carries self_match (= the same vib_self_match_weight), so
                // multiplying here would SQUARE it and amplify the echo-domination F
                // diagnosed. Credit = contingency × match-quality.
                Some((closure_support.clamp(0.0, 1.0) * fit).clamp(0.0, 1.0))
            } else {
                None
            }
        } else {
            None
        };
        {
            // 体检 telemetry: count closures / vocalized / evidence at the sink.
            let mut diag = self.vocal_diag.write().unwrap();
            diag.closures += 1;
            if vocalized {
                diag.closures_vocalized += 1;
                let (sb, _) = self.read_self_vocal_bands();
                diag.last_self_bands_sum = sb.iter().sum::<f64>();
            }
            if let Some(c) = acoustic_competence {
                diag.closures_with_evidence += 1;
                diag.last_fit = c;
            }
        }
        {
            let mut bias = self.learned_bias.write().unwrap();
            // B2 机制A: suppression 的时间遗忘与其累积共用同一把写锁、同一节律。
            bias.decay_vocal_suppression(now_us);
            // efficacy 的纯时间松弛保留为重启松弛兜底（不持久化时戳）。
            bias.decay_vocal_efficacy(now_us);
            // familiarity 的纯时间松弛（τ=2 min）——让遗忘律与限速积累共同
            // 把平衡点压到 ~0.5–0.7，而非钉死 1.0 压制内生 babble 基线。
            bias.decay_prediction_familiarity(now_us);
            // familiarity 限速积累：至多 1 tick/FAMILIARITY_ADVANCE_MIN_INTERVAL_US，
            // 防止密集 nursery 对（~6/s）在几十秒内把 familiarity 打到 1.0。
            if now_us.saturating_sub(bias.last_familiarity_advance_us)
                >= crate::FAMILIARITY_ADVANCE_MIN_INTERVAL_US
            {
                bias.prediction_familiarity = (bias.prediction_familiarity + delta_fam).min(1.0);
                // 可疑-2: predictedness-driven suppression (habituation). A
                // well-predicted, evidence-bearing vocalization (high acoustic
                // competence c) deposits suppression; a novel/poorly-predicted one
                // deposits ~0 → passes through. This replaces the categorical
                // vocalized?-label gate that left action_suppression pinned at 0 in
                // an all-vocalized session. Shares the 1/s familiarity throttle so
                // the dense pair path cannot ratchet the body mute (runaway self-
                // silencing); decay_vocal_suppression (300s) gives recovery.
                if let Some(c) = acoustic_competence {
                    bias.action_suppression =
                        (bias.action_suppression + c * crate::VOCAL_SUPPRESSION_GAIN).min(1.0);
                }
                bias.last_familiarity_advance_us = now_us;
            }
            // Quiet-closure suppression floor (vocalized==false → silent self-
            // regulation success), unchanged — a legitimate predictedness≈1 case.
            bias.action_suppression = (bias.action_suppression + delta_sup).min(1.0);
            // efficacy is an EMA of the acoustic-fit-weighted competence, updated
            // ONLY on closures that carry heard-self evidence. It converges toward
            // recent genuine competence and falls when the heard-self stops
            // matching — reporting present ability, not "ever voiced". A ratchet
            // (even with slow decay) saturates under the high-rate pair path; an
            // evidence-gated EMA does not.
            // Defect #3: step the efficacy EMA at most ONCE per distinct tick
            // (now_us), not once per closure. The dense action-consequence pair path
            // fires K closures within a SINGLE tick, all sharing the same now_us;
            // stepping each compounds alpha*(c-efficacy) K times in zero elapsed time,
            // inflating the effective alpha ~K× and collapsing VOCAL_EFFICACY_EMA_ALPHA's
            // ~10s smoothing window into one tick. One step PER TICK is exactly what the
            // alpha was tuned for (≈10s at the tick rate), so we dedupe by now_us rather
            // than throttle to a wall-clock interval (efficacy must still track per-tick,
            // unlike the slow 1/s maturity clock). Honesty gate UNTOUCHED: only
            // evidence-carrying closures (Some(c), self_bands>1e-6 + fit-weighting) reach
            // here; we drop same-tick duplicates, never the evidence test.
            if let Some(c) = acoustic_competence {
                let last_eff = self.last_efficacy_advance_us.load(Ordering::Relaxed);
                // now_us advances each tick; same-tick (== last) duplicates are dropped.
                // last==0 (never stepped) ⇒ now_us>0 fires the first step immediately.
                if now_us > last_eff {
                    self.last_efficacy_advance_us.store(now_us, Ordering::Relaxed);
                    // A3-family anneal: newborns take coarse, fast EMA steps so the
                    // first genuine fits actually lower the reflex gate (bootstrap
                    // escape); the multiplier anneals to exactly 1.0 at m=1.
                    let alpha = crate::VOCAL_EFFICACY_EMA_ALPHA
                        * crate::vocal_efficacy_alpha_scale(self.developmental_maturity());
                    bias.vocal_efficacy += alpha * (c - bias.vocal_efficacy);
                    bias.vocal_efficacy = bias.vocal_efficacy.clamp(0.0, 1.0);
                }
            }
        }
        // Developmental clock: advances on the IMPROVEMENT of earned competence,
        // not its absolute level, and at most once per MATURITY_ADVANCE_MIN_INTERVAL_US.
        // The old law fed the absolute per-closure competence `c` into a monotone
        // ratchet, so with the nursery's ~6 evidence-closures/s it saturated m to
        // ~0.9955 in one 13-min call while real skill (shape_dist) was FLAT and
        // efficacy was DECLINING — the verified hallucinated-maturity bug. Now the
        // clock credits the per-tick RISE of vocal_efficacy (the evidence-gated
        // competence EMA): a flat/declining competence earns ~0 → the clock holds.
        // Maturity becomes a competence high-water-mark (Knudsen: close on achieved
        // competence, not age; Oudeyer IAC: learning progress, not level). The 1/s
        // throttle is preserved; the baseline (last_maturity_efficacy) tracks each
        // throttle tick so the rise is measured per-second, not per-closure.
        if acoustic_competence.is_some() {
            // Own throttle cursor (NOT the shared last_maturity_advance_us): the
            // vision path advances that cursor ~50×/s and would otherwise starve
            // this block of its 1/s slot, freezing the baseline (the t=24s jump).
            let last = self.last_vocal_progress_us.load(Ordering::Relaxed);
            if now_us.saturating_sub(last) >= crate::MATURITY_ADVANCE_MIN_INTERVAL_US {
                self.last_vocal_progress_us.store(now_us, Ordering::Relaxed);
                let eff_now = self.learned_bias.read().unwrap().vocal_efficacy;
                // Credit only a NEW competence HIGH-WATER-MARK, measured against the
                // running max (not the previous tick). Three properties fall out:
                //   * jitter BELOW the peak earns 0 — no rectified-noise inflation
                //     (a per-tick delta would sum every up-wiggle of the noisy EMA);
                //   * a sustained plateau earns 0 — the clock holds (Knudsen: close
                //     on achieved competence; Oudeyer IAC: progress, not level);
                //   * a stale/late baseline can no longer manufacture a large step,
                //     because the max only ever RISES (eff_now − peak ≤ tick delta).
                // advance_maturity_by_progress additionally caps the step.
                let improvement = {
                    let mut peak = self.last_maturity_efficacy.write().unwrap();
                    let imp = match *peak {
                        Some(p) => (eff_now - p).max(0.0),
                        None => 0.0, // first tick only seeds the baseline
                    };
                    *peak = Some(peak.map_or(eff_now, |p| p.max(eff_now)));
                    imp
                };
                self.advance_maturity_by_progress(improvement);
            }
        }
    }

    /// Pure-vision developmental closure — the visual sibling of
    /// `observe_vocal_reflex_closure`. It lets the body earn maturity from
    /// SEEING alone (prediction-error reduction on its Gaussian reconstruction),
    /// not only from audio-visual co-occurrence. Called every live visual frame
    /// from `tick_gaussian_field` / `tick_gaussian_field_from_net` with that
    /// tick's reconstruction `mse` and field `active_frac`.
    ///
    /// Design (route-A, anti-deadlock):
    ///   * Evidence gate: `active_frac > VIS_CLOSURE_ACTIVE_FRAC_MIN` and a
    ///     finite mse — i.e. real structure is present. An empty/near-empty field
    ///     carries no evidence and credits nothing (no fabricated maturity).
    ///   * c = recon-MSE *reduction* (`fit`) relative to a slow EMA, NOT the
    ///     absolute MSE. Crediting the drop ("the body got better at
    ///     reconstructing what it sees") is the visual analog of (1 − shape_dist)
    ///     and sidesteps the cold-start 0×x trap (an empty field reports a large,
    ///     uninformative MSE; a trivially-blank scene a tiny one — neither is
    ///     competence). `active_frac` is the evidence GATE (structure present),
    ///     not the magnitude — keeping vision at parity with the vocal path.
    ///   * Shares the global 1/s maturity throttle with the vocal path, so the
    ///     developmental pace / infant period is unchanged.
    /// coarse→fine is handled by `advance_maturity`'s built-in (1−m) term — no
    /// extra anneal, exactly like the vocal path.
    pub fn observe_visual_reflex_closure(&self, now_us: u64, mse: f64, active_frac: f64) {
        if !mse.is_finite() || active_frac <= crate::VIS_CLOSURE_ACTIVE_FRAC_MIN {
            return;
        }
        // Update the slow recon-MSE baseline (it tracks the CURRENT scene).
        let ema_now = {
            let mut ema = self.visual_recon_mse_ema.write().unwrap();
            match *ema {
                None => {
                    *ema = Some(mse);
                    // Seed the high-water mark alongside the baseline (separate lock,
                    // no nesting) so the very next observation can already earn
                    // credit; this first one credits nothing.
                    *self.visual_recon_mse_best.write().unwrap() = Some(mse);
                    return;
                }
                Some(prev) => {
                    let v = (1.0 - crate::VIS_RECON_MSE_EMA_BETA) * prev
                        + crate::VIS_RECON_MSE_EMA_BETA * mse;
                    *ema = Some(v);
                    v
                }
            }
        };
        // HIGH-WATER-MARK: credit maturity ONLY when the sustained reconstruction
        // reaches a new all-time best (lowest baseline ever). The old law credited
        // every frame whose error beat the recent baseline — which a live, ever-
        // changing scene supplies endlessly (the treadmill), saturating the global
        // clock. Crediting only a genuinely-new-best stops that: once the body has
        // achieved its best sustained prediction, scene-churn no longer matures it.
        // Mirror of the vocal competence high-water mark.
        let improvement = {
            let mut best = self.visual_recon_mse_best.write().unwrap();
            match *best {
                None => {
                    *best = Some(ema_now); // seed the high-water mark, credit nothing
                    0.0
                }
                Some(b) => {
                    if ema_now < b && b > 1e-9 {
                        let imp = ((b - ema_now) / b).clamp(0.0, 1.0); // relative new-best gain
                        *best = Some(ema_now);
                        imp
                    } else {
                        0.0
                    }
                }
            }
        };
        if improvement <= 0.0 {
            return;
        }
        // Own throttle cursor + the same capped progress law as the vocal clock, so
        // vision earns honest, bounded maturity that cannot saturate the global
        // clock from scene-churn.
        let last = self.last_vision_progress_us.load(Ordering::Relaxed);
        if now_us.saturating_sub(last) >= crate::MATURITY_ADVANCE_MIN_INTERVAL_US {
            self.last_vision_progress_us.store(now_us, Ordering::Relaxed);
            self.advance_maturity_by_progress(improvement);
        }
    }

    pub fn tick_vocal_reflex_from_pressure(&self, now_us: u64) {
        let field = self.read_prediction_pressure_field();
        let field_pressure = field
            .waves
            .iter()
            .map(|wave| wave.amplitude)
            .fold(field.dominant_pressure, f64::max);
        let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
        let gap_pressure = self.read_active_reasoning_gap().gap_pressure;

        // Endogenous baseline: slow oscillation that drives exploratory
        // vocalization even without external stimuli. High for newborns,
        // dampened as prediction_familiarity and action_suppression grow.
        let bias = self.learned_bias.read().unwrap().summary();
        // B2 enhancement: suppression damping floor annealed by the
        // developmental clock — continuous numeric modulation (wide at birth
        // for bootstrap escape, narrowing to the no-op 0.45 endpoint at m=1).
        let damp_floor = crate::suppression_damp_floor(self.developmental_maturity());
        let babble_floor = crate::vocal_babble_floor(self.developmental_maturity());
        let baseline = VocalReflexState::endogenous_baseline(
            now_us,
            bias.prediction_familiarity,
            bias.action_suppression,
            damp_floor,
            babble_floor,
        );

        // Quiet-history replay intensity: when the system has prior experience,
        // past reasoning entries are replayed as attention perturbations. Feed
        // this replay signal into the vocal drive so that past stimulus patterns
        // create periodic internal "waves" that may trigger vocalization.
        let replay = self.global_attention.read().unwrap().replay_intensity;

        let drive_pressure = field_pressure
            .max(evidence_pressure)
            .max(gap_pressure)
            .max(baseline)
            .max(replay * 0.25);
        let drive = VocalReflexDrive {
            drive_pressure,
            energy: drive_pressure,
        };
        let burst = {
            let mut reflex = self.vocal_reflex.write().unwrap();
            reflex.tick_drive(
                now_us,
                drive,
                bias.prediction_familiarity,
                bias.action_suppression,
                bias.vocal_efficacy,
            )
        };
        let Some(burst) = burst else {
            return;
        };

        // Record that a vocal output was triggered. Downstream pair
        // processing uses this to route closure feedback.
        self.last_vocal_output_t_us
            .store(now_us, Ordering::Relaxed);

        let is_active = self.read_vocal_organ().phase == VocalOrganPhase::Active;
        if is_active {
            self.adjust_voicing(now_us, burst.breath, burst.pitch, burst.resonance);
        } else {
            self.begin_voicing(now_us, burst.breath, burst.pitch, burst.resonance);
        }
        let mut organ = self.vocal_organ.write().unwrap();
        organ.noise_mix = organ.noise_mix.max(burst.noise);
    }

    /// 改造6: unified vocal drive field — ONE outlet.
    ///
    /// The three former drivers (inner-pressure reflex, external-sound target,
    /// imitation) now CONTRIBUTE to one field; this single outlet reads the
    /// combined field and voices once. No source races begin_voicing on its own.
    pub fn tick_vocal_drive_field(&self, now_us: u64) {
        use crate::vocal_drive_field::{VocalDriveContribution, VocalDriveField};

        let mut contribs: Vec<VocalDriveContribution> = Vec::with_capacity(2);

        // --- Source region A: inner-pressure reflex ---
        // Reuse the validated reflex gating (threshold / bypass / learned-bias
        // regulation) to decide whether inner pressure wants to voice, and with
        // what burst. A burst becomes a contribution; no burst = no contribution.
        let field = self.read_prediction_pressure_field();
        let field_pressure = field
            .waves
            .iter()
            .map(|wave| wave.amplitude)
            .fold(field.dominant_pressure, f64::max);
        let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
        let gap_pressure = self.read_active_reasoning_gap().gap_pressure;
        let bias = self.learned_bias.read().unwrap().summary();
        // B2 enhancement: same developmental anneal of the damping floor as in
        // tick_vocal_reflex_from_pressure (single clock, single schedule).
        let damp_floor = crate::suppression_damp_floor(self.developmental_maturity());
        let babble_floor = crate::vocal_babble_floor(self.developmental_maturity());
        let baseline = crate::vocal_reflex::VocalReflexState::endogenous_baseline(
            now_us,
            bias.prediction_familiarity,
            bias.action_suppression,
            damp_floor,
            babble_floor,
        );
        let replay = self.global_attention.read().unwrap().replay_intensity;
        // P2: active surface aperture → vocal raw_urge.
        // The active surface integrates attention focus across multiple wave sources
        // and retains pressure between orchestrator ticks via chain memory/reentry.
        // Including it here lets sustained perceptual focus — even after raw
        // prediction waves have decayed — continue to drive exploratory vocalization.
        // Coefficient 0.60 keeps the contribution subordinate to strong direct
        // evidence while allowing surface_probe ≥ 0.60 to independently cross
        // the 0.36 burst threshold.
        let surface_probe = self.active_surface.read().unwrap().effective_probe_pressure();
        let raw_urge = field_pressure
            .max(evidence_pressure)
            .max(gap_pressure)
            .max(surface_probe * 0.60)
            .max(baseline)
            .max(replay * 0.25);
        // Innate survival reflex (治本 of the silent-run starvation): the inner
        // urge to voice yields in REAL TIME to self-preservation pressure —
        // 饿了就少叫,吃饱了才咿呀. The pressure scales the urge BELOW the reflex's
        // own threshold when survival is at stake, so no burst fires → no voicing
        // exertion → reserve recovers. Continuous interoceptive modulation of the
        // one drive field (NOT a state-named behaviour gate); fast enough to
        // survive before the body could ever learn restraint. Only the
        // inner-pressure self-babble is gated here; external/attachment-driven
        // voicing (region B below) is left for the dual-drive to arbitrate.
        let self_preservation = self.read_vital_core().self_preservation_pressure();
        let drive_pressure =
            crate::vocal_drive_field::survival_gated_urge(raw_urge, self_preservation);
        let drive = crate::vocal_reflex::VocalReflexDrive {
            drive_pressure,
            energy: drive_pressure,
        };
        let burst = {
            let mut reflex = self.vocal_reflex.write().unwrap();
            reflex.tick_drive(
                now_us,
                drive,
                bias.prediction_familiarity,
                bias.action_suppression,
                bias.vocal_efficacy,
            )
        };
        {
            // 体检 telemetry: outlet cadence, raw drive, burst count.
            // Phase 0: also snapshot the live target state here (always runs,
            // unlike region B which is gated on energy≥0.01) so the CSV shows
            // whether the caregiver target — and its fine mel64 fingerprint —
            // is reaching the loop at all. Read target before taking the diag
            // lock (read_vocal_target returns a copy; no nested lock).
            let t = self.read_vocal_target();
            let mut diag = self.vocal_diag.write().unwrap();
            diag.outlet_ticks += 1;
            diag.last_drive_pressure = drive_pressure;
            if burst.is_some() {
                diag.bursts += 1;
            }
            diag.last_target_active_energy = t.active_energy;
            diag.last_target_mel64_sum = t.target_mel64.iter().sum();
            diag.last_target_bands_sum = t.target_bands.iter().sum();
            diag.recall_entries = self.vocal_target_recall.read().unwrap().entries.len() as u64;
        }
        if let Some(b) = burst {
            contribs.push(VocalDriveContribution {
                intensity: b.breath,
                breath: b.breath,
                pitch: b.pitch,
                resonance: b.resonance,
                noise: b.noise,
            });
        }

        // --- Source region B: external-sound target / imitation pull ---
        // Phase 1: the acoustic servo's correction, captured here and applied to
        // the organ's explicit F1/F2 AFTER the outlet voices (voicing overwrites
        // formants from resonance, so the servo nudge must come last).
        let mut region_b_formant_offset: Option<(f64, f64)> = None;
        let target = self.read_vocal_target();
        if target.active_energy >= 0.01 {
            let organ = self.read_vocal_organ();
            let pitch_gap = (target.pitch_hint - organ.pitch_tension).abs();
            let resonance_gap = (target.resonance_hint - organ.resonance_position).abs();
            let total_gap = (pitch_gap + resonance_gap) / 2.0;
            // P3 Stage 2: drive imitation LEARNING by the real acoustic shape error
            // (self heard vs target through the same de-saturated ears), replacing
            // the near-constant 2-scalar parametric proxy that produced illusory
            // convergence. Until the body has actually voiced (self silent → no
            // shape), fall back to the parametric gap so pre-voicing still feels
            // pressure to vocalize. Threshold scale matches observe_gap (measured:
            // good match ≈0.12 < 0.2 converge; far ≈0.6 > 0.5 fail).
            let (self_bands_now, _) = *self.self_vocal_bands.read().unwrap();
            let acoustic_dist =
                crate::vocal_self_analysis::spectral_distance(&self_bands_now, &target.target_bands);
            // Finer acoustic representation for the imitation/servo error: prefer
            // the 64-mel shape distance when both fingerprints are present, so the
            // learning gap and the servo's hill-climb track real formant error
            // rather than a coarse 16-band proxy. Fall back to 16-band otherwise.
            let (self_mel64_now, _) = *self.self_vocal_mel64.read().unwrap();
            let shape_dist = if self_mel64_now.iter().sum::<f64>() > 1e-6
                && target.target_mel64.iter().sum::<f64>() > 1e-6
            {
                // Source/timbre-normalized formant-pattern error: the part of the
                // mismatch the body can actually control (F1/F2/F3), with the
                // glottal/recording-timbre floor liftered out. This is the signal
                // the servo + learning should chase.
                crate::vocal_self_analysis::formant_shape_distance_mel64(
                    &self_mel64_now,
                    &target.target_mel64,
                )
            } else {
                crate::vocal_self_analysis::spectral_shape_distance(&self_bands_now, &target.target_bands)
            };
            let self_voiced = self_bands_now.iter().sum::<f64>() > 1e-6;
            let learning_gap = if self_voiced { shape_dist } else { total_gap };
            {
                let mut imit = self.vocal_imitation.write().unwrap();
                imit.observe_gap(now_us, learning_gap, organ.breath_drive);
                imit.last_acoustic_distance = acoustic_dist;
            }
            // P3 Stage 3 / Phase 1: active acoustic servo. The feedforward hint
            // sets the vowel BASELINE (resonance→F1/F2 on the 1-D line); the servo
            // hill-climbs a bounded INDEPENDENT (F1, F2) correction to reduce the
            // shape error the body actually HEARS — this is what reaches 2-D vowel
            // targets the single resonance scalar cannot. It steps on its own
            // ~90 ms cadence (organ smoothing must settle between probes); the aim
            // is read every tick so the held probe is applied continuously.
            // `recall_snapshot` is Some((committed_pitch, committed_res, shape_ema))
            // on every voiced tick — the COMMITTED base (not the live probe-perturbed
            // `aim()`) paired with the SMOOTHED objective (not a raw single-tick error).
            // It is what gets memorised for content-addressed recall. Snapshotting
            // every voiced tick (rather than only on a servo-commit) is deliberate:
            // a converged servo by definition stops producing commits, so a
            // commit-only snapshot memorises a mid-descent base and never the final
            // deep one. `store` keeps best-ever (lowest-ema) fit, so the converged
            // state wins precisely because it is sampled here every tick.
            let (servo_p, servo_r, recall_snapshot) = {
                let mut servo = self.vocal_servo.write().unwrap();
                // Phase 2: content-addressed warm-start. On a target
                // (re)presentation that matches a remembered correction, jump the
                // servo straight to it (skip the cold climb); on a novel target
                // after a known one, reset so we don't chase a stale offset.
                let matched = self
                    .vocal_target_recall
                    .read()
                    .unwrap()
                    .recall(&target.target_mel64);
                let last = self.last_recall_idx.load(Ordering::Relaxed);
                match matched {
                    Some((idx, _)) if idx as i64 != last => {
                        let e = self.vocal_target_recall.read().unwrap().entries[idx].clone();
                        servo.warm_start(e.f1_off, e.f2_off);
                        self.last_recall_idx.store(idx as i64, Ordering::Relaxed);
                    }
                    Some((idx, _)) => {
                        // Same target still held: re-anchor if a high-confidence
                        // correction exists and the noisy hill-climb has drifted far
                        // from it (see RECALL_REANCHOR_*). Recall warm-started only on
                        // target CHANGE, so a servo that random-walked off a learned
                        // correction was never pulled back — defeating recall's
                        // purpose. Bounded re-anchoring keeps it near its own best
                        // result while still allowing local probing inside DRIFT.
                        let e = self.vocal_target_recall.read().unwrap().entries[idx].clone();
                        if e.fit >= crate::vocal_target_recall::RECALL_REANCHOR_MIN_FIT
                            && (servo.pitch_offset - e.f1_off).abs()
                                + (servo.resonance_offset - e.f2_off).abs()
                                > crate::vocal_target_recall::RECALL_REANCHOR_DRIFT
                        {
                            servo.warm_start(e.f1_off, e.f2_off);
                        }
                    }
                    None if last != -1 => {
                        // #8 Stage 3: the target just moved off a known sound and the
                        // new one isn't recognised yet. If the body is developmentally
                        // mature AND has learned what usually FOLLOWS `last`, ANTICIPATE
                        // it — warm-start the servo toward the predicted successor's
                        // correction instead of a cold reset — so a familiar SEQUENCE is
                        // produced fluently (a faster/closer match raises self_match →
                        // closure_support, the factor that caps efficacy at its plateau).
                        // Maturity-gated so it anneals in and never smothers a young
                        // body's babble. last_recall_idx stays -1 (still unrecognised) so
                        // Stage 1 logs no spurious edge; a wrong guess just hill-climbs
                        // from there — no worse than a cold reset.
                        let predicted = if self.developmental_maturity()
                            > crate::vocal_sequence::SEQ_ANTICIPATE_MATURITY_FLOOR
                        {
                            self.vocal_sequence.read().unwrap().predict_next(last)
                        } else {
                            None
                        };
                        let mut warmed = false;
                        if let Some((to_idx, _)) = predicted {
                            let recall = self.vocal_target_recall.read().unwrap();
                            if let Some(e) = recall.entries.get(to_idx as usize) {
                                let (f1, f2) = (e.f1_off, e.f2_off);
                                drop(recall);
                                servo.warm_start(f1, f2);
                                warmed = true;
                            }
                        }
                        if !warmed {
                            servo.reset();
                        }
                        self.last_recall_idx.store(-1, Ordering::Relaxed);
                    }
                    _ => {}
                }
                if self_voiced {
                    // Denoise the formant objective every tick; the servo's
                    // baseline/probe comparison (~90 ms apart) then descends a
                    // stable surface instead of chasing per-tick jitter.
                    servo.observe_shape(shape_dist);
                }
                if self_voiced
                    && now_us.saturating_sub(servo.last_update_t_us)
                        >= crate::vocal_servo::SERVO_INTERVAL_US
                {
                    let smoothed = servo.shape_ema;
                    servo.step(now_us, smoothed);
                }
                // Snapshot the COMMITTED base + smoothed objective for memorisation
                // (defect #4): the committed offsets — never the probe-perturbed
                // aim() — and shape_ema — never a raw single-tick error — so a
                // lucky-noise sample can neither be the stored offset nor clear the
                // fit gate. Captured every voiced tick (once warm) so the converged
                // optimum is sampled, then best-ever-fit selection in `store` keeps
                // it. The warm-up gate (is_recall_warm) refuses to memorise during
                // the ema init/settling transient, where a lucky low bounce at a
                // still-suboptimal offset would otherwise be locked in.
                let snap = if self_voiced && servo.is_recall_warm() {
                    Some((servo.pitch_offset, servo.resonance_offset, servo.shape_ema))
                } else {
                    None
                };
                let (ap, ar) = servo.aim();
                (ap, ar, snap)
            };
            region_b_formant_offset = Some((servo_p, servo_r));
            // Phase 2: memorise a correction that actually worked, keyed by the
            // target fingerprint, so the next time this sound recurs the servo is
            // warm-started above instead of re-solving from zero. Keyed to the
            // COMMITTED offset + SMOOTHED fit (defect #4): the old code stored the
            // probe-perturbed `aim()` with a raw per-tick `shape_dist`, so a lucky
            // single tick could be memorised as "the correction that worked" and
            // stick (store keeps best-ever fit). Honesty gate preserved: fit is
            // still real self-heard shape error (shape_ema), only denoised.
            if let Some((commit_p, commit_r, ema)) = recall_snapshot {
                let fit = (1.0_f64 - ema).clamp(0.0, 1.0);
                if fit >= crate::vocal_target_recall::RECALL_STORE_MIN_FIT {
                    self.vocal_target_recall.write().unwrap().store(
                        now_us,
                        &target.target_mel64,
                        commit_p,
                        commit_r,
                        fit,
                    );
                    // Phase A6 mirror: register a VocalProfile on a
                    // perturbation_graph node that carries the same mel64
                    // identity. If the graph already has a vocal node within
                    // VOCAL_MEL_MATCH_DIST of this target, refresh its
                    // profile; otherwise spawn a brand-new node (24-D sig
                    // derived from the first 24 mel bands, normalised) and
                    // register its profile. Additive — vocal_target_recall
                    // continues to drive the formant servo; the mirror gives
                    // downstream consumers a unified PerturbationGraph view.
                    {
                        let mut graph = self.perturbation_graph.write().unwrap();
                        let node_id = match graph
                            .nearest_vocal_node(&target.target_mel64)
                        {
                            Some((nid, _)) => nid,
                            None => {
                                // Build a 24-D sig from the first 24 mel bands
                                // (normalised by their sum) so the new vocal
                                // node sits in the same tier-0 signature space
                                // as visual nodes, even if the projection is
                                // coarse. Visual nodes that happen to align
                                // remain distinguishable via the VocalProfile
                                // sidecar; non-vocal nodes have no profile.
                                let mut sig =
                                    [0.0_f64; crate::perturbation_graph::PG_NODE_SIG_DIM];
                                let s: f64 = target.target_mel64[..crate::perturbation_graph::PG_NODE_SIG_DIM]
                                    .iter()
                                    .map(|v| v.abs())
                                    .sum();
                                if s > 1e-9 {
                                    for i in 0..crate::perturbation_graph::PG_NODE_SIG_DIM {
                                        sig[i] = target.target_mel64[i].abs() / s;
                                    }
                                }
                                graph.add_node(now_us, sig)
                            }
                        };
                        graph.register_vocal_profile(
                            node_id,
                            target.target_mel64,
                            commit_p,
                            commit_r,
                            fit,
                            now_us,
                        );
                    }
                }
            }
            // #8 Stage 1: learn the ORDER of vocal targets — an edge forms between
            // RECOGNISED recall entries when the committed target changes — and
            // buffer the self/target utterance for the Stage 2 DTW alignment fit.
            // Read the mel64 (drop its read lock) THEN take the seq write: no
            // nested locks; vocal-thread-only writes, so no new cross-thread order.
            if self_voiced {
                let cur_recall_idx = self.last_recall_idx.load(Ordering::Relaxed);
                let (self_mel_now, _) = *self.self_vocal_mel64.read().unwrap();
                // Phase A8: capture prev recall idx BEFORE observe_transition
                // updates it. If we have a valid (prev → cur) transition that
                // is genuinely a change between two recognised entries, mirror
                // it into perturbation_graph as a Sequence relation between
                // the corresponding vocal NodeIds (resolved via nearest_vocal_node
                // on each entry's mel64). Additive — vocal_sequence still
                // drives the DTW alignment path.
                let prev_recall_idx = {
                    let seq_ro = self.vocal_sequence.read().unwrap();
                    seq_ro.last_voiced_idx
                };
                let mut seq = self.vocal_sequence.write().unwrap();
                seq.observe_transition(now_us, cur_recall_idx);
                if self_mel_now.iter().sum::<f64>() > 1e-6 {
                    seq.push_self(&self_mel_now);
                }
                if target.target_mel64.iter().sum::<f64>() > 1e-6 {
                    seq.push_target(&target.target_mel64);
                }
                drop(seq);
                if prev_recall_idx >= 0
                    && cur_recall_idx >= 0
                    && prev_recall_idx as i64 != cur_recall_idx
                {
                    let recall = self.vocal_target_recall.read().unwrap();
                    let prev_mel = recall
                        .entries
                        .get(prev_recall_idx as usize)
                        .map(|e| e.target_mel64);
                    let cur_mel = recall
                        .entries
                        .get(cur_recall_idx as usize)
                        .map(|e| e.target_mel64);
                    drop(recall);
                    if let (Some(prev_mel), Some(cur_mel)) = (prev_mel, cur_mel) {
                        let mut graph = self.perturbation_graph.write().unwrap();
                        let prev_node = graph.nearest_vocal_node(&prev_mel).map(|(id, _)| id);
                        let cur_node = graph.nearest_vocal_node(&cur_mel).map(|(id, _)| id);
                        if let (Some(p), Some(c)) = (prev_node, cur_node) {
                            graph.observe_relation(
                                now_us,
                                crate::perturbation_graph::Endpoint::Node(p),
                                crate::perturbation_graph::Endpoint::Node(c),
                                crate::perturbation_graph::EdgeKind::Sequence,
                            );
                        }
                    }
                }
            }
            // Phase 0: capture the live region-B learning path (invisible before
            // — it runs off self_voiced, not the closure sink). Reveals whether
            // the body hears itself, the real shape error, and the servo's steer.
            {
                let mut diag = self.vocal_diag.write().unwrap();
                diag.region_b_ticks += 1;
                diag.last_self_voiced = self_voiced;
                diag.last_shape_dist = shape_dist;
                diag.last_learning_gap = learning_gap;
                diag.last_servo_pitch_off = servo_p;
                diag.last_servo_res_off = servo_r;
            }
            // Phase 1 (2026-06-12): route the WORKING servo/self_voiced path into
            // the evidence-gated closure sink. The body genuinely heard its own
            // voice this round (self_voiced) and has a real self↔target shape
            // error — credit a rate-limited closure so vocal_efficacy AND the
            // developmental clock actually advance, instead of staying frozen
            // behind the ≈22ms self_match window that almost never opens. The sink
            // re-derives acoustic fit (self vs target) under the SAME honesty gate
            // (self_bands>1e-6) and weights by closure_support; support reflects
            // active voicing strength. Not a reward — a prediction-error-weighted
            // competence level on a path that was already running.
            if self_voiced {
                let last_ic = self.last_imitation_closure_t_us.load(Ordering::Relaxed);
                if now_us.saturating_sub(last_ic) >= crate::VOCAL_SELF_PLAY_MIN_INTERVAL_US {
                    let support = organ.breath_drive.clamp(0.0, 1.0);
                    self.observe_vocal_reflex_closure(now_us, support, true);
                    self.last_imitation_closure_t_us
                        .store(now_us, Ordering::Relaxed);
                }
            }
            // A1: dual-drive confluence flows back here — and ONLY here. The
            // arbitrated reading (already ticked each orchestrator tick) leans
            // the external/attachment-driven voicing continuously: net>0 (bond
            // pull wins) amplifies the cry toward the caregiver, net<0
            // (self-preservation wins) attenuates it. Region A keeps its own
            // survival gate untouched (no double-gating / feedback loop), and
            // the combined survival gate below still applies. Continuous
            // multiplicative field, NOT a behaviour gate; identity at net=0.
            let dd = self.read_dual_drive();
            let bond_drive = crate::vocal_drive_field::bond_modulated_region_b(
                target.active_energy * 0.5,
                dd.net_toward_bond,
            );
            // Feedforward only here: pitch from the target's pitch hint, vowel
            // baseline from its resonance hint. The servo's correction is NOT
            // folded into resonance any more (that kept it trapped on the 1-D
            // line) — it is applied as an independent F1/F2 nudge after voicing.
            contribs.push(VocalDriveContribution {
                intensity: bond_drive,
                breath: bond_drive,
                pitch: target.pitch_hint.clamp(0.0, 1.0),
                resonance: target.resonance_hint.clamp(0.0, 1.0),
                noise: 0.0,
            });
        }

        // --- Combine into one field, then ONE outlet voices ---
        let combined = VocalDriveField::combine(&contribs);
        // Apply survival gate to the FULL combined drive (region A was already
        // gated; this catches region B in Standby). In Standby (reserve=0,
        // self_preservation=1.0) → combined.intensity→0 → decay path → drain
        // returns to base metabolism 0.2 → reserve can actually recover. Without
        // this, region B (ambient-sound imitation) keeps firing at 0.7 drain and
        // makes recovery physically impossible. This is not suppressing expression;
        // it is metabolic self-consistency: you cannot spend energy you don't have.
        let combined = VocalDriveField {
            intensity: crate::vocal_drive_field::survival_gated_urge(
                combined.intensity,
                self_preservation,
            ),
            ..combined
        };
        if combined.intensity <= 0.0 {
            // No drive this tick: let the organ decay (and emit stop/decay
            // output events), the way the old imitation loop did when idle.
            let (was_active, is_idle, decay_magnitude) = {
                let mut organ = self.vocal_organ.write().unwrap();
                let was_active = organ.phase == VocalOrganPhase::Active;
                let prev_breath = organ.breath_drive;
                organ.tick_decay(now_us);
                (
                    was_active,
                    organ.phase == VocalOrganPhase::Idle,
                    (prev_breath - organ.breath_drive).max(0.0),
                )
            };
            if was_active {
                let feedback = if is_idle {
                    ActionOutputFeedback::Stop
                } else {
                    ActionOutputFeedback::Decay
                };
                self.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback,
                    magnitude: decay_magnitude,
                    event_t_us: now_us,
                });
            }
            return;
        }

        self.last_vocal_output_t_us.store(now_us, Ordering::Relaxed);
        let is_active = self.read_vocal_organ().phase == VocalOrganPhase::Active;

        // ── L1/L2 (PhonemeMemory) ─────────────────────────────────────────────
        // A new SYLLABLE opening clocks one phoneme-motor sample. The opening is
        // detected as the continuous aperture-oscillator phase wrapping past its
        // peak — a signal-derived event, NOT a discrete phase-machine check. Each
        // opening draws an exploration motor (babble-or-practice) and records it
        // (which fires the inter-prototype competition + repulsion that crystallise
        // a discrete inventory), then sets this syllable's articulation; between
        // openings the live drive carries it. This replaces the prior gate that
        // fired only on a fresh-utterance organ transition — which the continuous
        // voicing stream produces almost never, so the inventory was never written
        // from live voicing (observed: every proto frozen at its seed value).
        let syllable_onset = {
            let cur = self.read_vocal_organ().syllable_phase;
            let mut last = self.last_syllable_phase.write().unwrap();
            let onset = cur + 1e-9 < *last; // phase wrapped 2π→0 → a new opening
            *last = cur;
            onset
        };
        let (final_pitch, final_resonance, final_noise) = if syllable_onset || !is_active {
            use crate::phoneme_memory::{AcousticState, MotorState};
            let base = MotorState {
                pitch_tension: combined.pitch,
                resonance_pos: combined.resonance,
                noise_mix: combined.noise.clamp(0.0, 1.0),
            };
            let mut pm = self.phoneme_memory.write().unwrap();
            let explore = pm.exploration_motor(base);
            // Acoustic estimate derived from motor params.  Self-play loop will
            // refine spectral_center/resonance via update_acoustic_latest once
            // the mic confirms the echo.
            let acoustic_est = AcousticState {
                spectral_center: explore.pitch_tension,
                resonance: explore.resonance_pos,
                envelope: combined.breath.clamp(0.0, 1.0),
            };
            pm.record_voiced(explore, acoustic_est, now_us);
            (explore.pitch_tension, explore.resonance_pos, explore.noise_mix)
        } else {
            (combined.pitch, combined.resonance, combined.noise)
        };

        if is_active {
            self.adjust_voicing(now_us, combined.breath, final_pitch, final_resonance);
        } else {
            self.begin_voicing(now_us, combined.breath, final_pitch, final_resonance);
        }
        if final_noise > 0.0 {
            let mut organ = self.vocal_organ.write().unwrap();
            organ.noise_mix = organ.noise_mix.max(final_noise);
        }
        // Phase 1: apply the acoustic servo's INDEPENDENT F1/F2 correction on top
        // of the resonance-derived baseline the voicing above just set. The servo
        // hill-climbs these (in normalised units) to minimise the heard shape
        // error, so this is the step that actually lets the body move OFF the 1-D
        // resonance line into 2-D vowel space. Bounded + clamped → no instability.
        if let Some((off_f1, off_f2)) = region_b_formant_offset {
            let mut organ = self.vocal_organ.write().unwrap();
            organ.formant_f1 =
                (organ.formant_f1 + off_f1 * crate::F1_SERVO_HZ).clamp(150.0, 1200.0);
            organ.formant_f2 =
                (organ.formant_f2 + off_f2 * crate::F2_SERVO_HZ).clamp(400.0, 3500.0);
        }
    }

    pub fn tick_vocal_target_decay(&self, now_us: u64) {
        let mut vt = self.vocal_target.write().unwrap();
        vt.tick_decay(now_us);
    }

    /// Phase 3 cutover replacement for the deleted `tick_familiar_objects`:
    /// pushes the perturbation_graph's attachment + presence readings onto the
    /// shared INTERO axes so dual_drive and the predictor fabric keep getting
    /// fresh inputs.
    ///
    /// ATTACHMENT = strongest emergent bond (highest `bind_strength` node, i.e.
    /// the 责任人 analog).
    /// PRESENCE   = whether that most-bound node is currently active (perceived
    /// this frame or very recently) AND its bind_strength is above a small
    /// floor — a high→low PRESENCE transition next tick is the "longing"
    /// substrate that dual_drive feeds on.
    ///
    /// Always pushes (even when the graph is empty / readings are 0) so the
    /// axes never carry a stale value across a cold start.
    pub fn tick_perturbation_intero(&self, now_us: u64) {
        // Phase B3: also surface the group-level attachment, graph density,
        // and FastPath hit rate. The legacy ATTACHMENT axis still reflects
        // the single most-bound node so downstream consumers don't shift
        // distribution under them; new axes carry the richer signals.
        let (
            attachment,
            present,
            group_bind_top,
            density,
            fp_hit_rate,
            post_hoc_disagree,
            fp_probe_hit,
        ) = {
            let graph = self.perturbation_graph.read().unwrap();
            let (attachment, present) = match graph.most_bound() {
                Some(most) => {
                    let attachment = most.bind_strength.clamp(0.0, 1.0);
                    let present =
                        graph.is_active(most.id, now_us) && most.bind_strength > 0.3;
                    (attachment, present)
                }
                None => (0.0, false),
            };
            let group_bind_top = graph
                .most_bound_group()
                .map(|g| g.bind_strength.clamp(0.0, 1.0))
                .unwrap_or(0.0);
            let density = graph.graph_density();
            let fp_hit_rate = graph.fast_path_hit_rate();
            let post_hoc_disagree = graph.post_hoc_disagree_ema.clamp(0.0, 1.0);
            // Probe a FastPath query from the most-bound node — keeps the
            // hit-rate EMA alive even before Phase C wires real callers in.
            let probe_hit = match graph.most_bound() {
                Some(most) => matches!(
                    graph.default_response(crate::perturbation_graph::Endpoint::Node(most.id)),
                    crate::perturbation_graph::ResponseOutcome::FastPath { .. }
                ),
                None => false,
            };
            (
                attachment,
                present,
                group_bind_top,
                density,
                fp_hit_rate,
                post_hoc_disagree,
                probe_hit,
            )
        };
        // Update the hit-rate EMA from the probe before publishing.
        {
            let mut graph = self.perturbation_graph.write().unwrap();
            graph.note_fast_path_query(fp_probe_hit);
        }
        let fp_hit_rate_post = self
            .perturbation_graph
            .read()
            .unwrap()
            .fast_path_hit_rate();
        self.push_observation(crate::INTERO_ATTACHMENT, now_us, attachment);
        self.push_observation(
            crate::INTERO_PRESENCE,
            now_us,
            if present { 1.0 } else { 0.0 },
        );
        self.push_observation(crate::INTERO_GROUP_BIND_TOP, now_us, group_bind_top);
        self.push_observation(crate::INTERO_GRAPH_DENSITY, now_us, density);
        self.push_observation(
            crate::INTERO_FAST_PATH_HIT_RATE,
            now_us,
            fp_hit_rate_post.max(fp_hit_rate),
        );
        self.push_observation(
            crate::INTERO_POST_HOC_DISAGREE_RATE,
            now_us,
            post_hoc_disagree,
        );
        // INTERO_INTENTION_PRESSURE is published by tick_intention_arbitrator
        // (Phase C). Until that lands, the axis stays at 0 — which is the
        // honest "no active deliberation" state.
        self.push_observation(crate::INTERO_INTENTION_PRESSURE, now_us, 0.0);
    }

    /// #8 Stage 1: per-vocal-tick maintenance of the sequence layer — decay the
    /// eligibility traces and close an utterance after a silence gap (so its
    /// buffered self/target frames get a DTW seq_fit stamp before the next one
    /// starts). Owned by the vocal ticker only (Stage 2 efficacy lift is in
    /// observe_vocal_reflex_closure; Stage 3 anticipation is in tick_vocal_drive_field).
    pub fn tick_vocal_sequence_boundary(&self, now_us: u64) {
        let mut seq = self.vocal_sequence.write().unwrap();
        seq.decay_eligibility(now_us);
        let last_voice = self.last_vocal_output_t_us.load(Ordering::Relaxed);
        if last_voice != 0
            && now_us.saturating_sub(last_voice) > crate::vocal_sequence::SEQ_UTTERANCE_GAP_US
        {
            seq.end_utterance();
        }
    }

    /// Q1c quiet-period replay: during a long enough vocal silence the body
    /// rehearses the most-likely next utterance from its persistent edge table
    /// — the operational form of "old impressions actively reproduce in
    /// prediction". Pure read on `vocal_sequence`: it computes a multi-step
    /// rollout from `last_voiced_idx` but does NOT mutate the edge table (no
    /// false refresh of `last_t_us`, so the age-protection clock from Q1b
    /// stays honest). The chain is published via three atomic observation
    /// counters so downstream consumers (servo planning, attention, observer
    /// dashboards) can subscribe; the actual chain-to-action wiring is left
    /// as an explicit next integration step. Gated on
    /// `SEQ_QUIET_REPLAY_GAP_US` of silence so it never interrupts active
    /// babble, and on a known `last_voiced_idx` so a cold body doesn't
    /// rehearse from nothing.
    pub fn tick_vocal_sequence_quiet_replay(&self, now_us: u64) -> bool {
        let last_voice = self.last_vocal_output_t_us.load(Ordering::Relaxed);
        if last_voice == 0
            || now_us.saturating_sub(last_voice)
                < crate::vocal_sequence::SEQ_QUIET_REPLAY_GAP_US
        {
            return false;
        }
        let seq = self.vocal_sequence.read().unwrap();
        if seq.last_voiced_idx < 0 {
            return false;
        }
        let chain = seq.quiet_replay_chain(crate::vocal_sequence::SEQ_QUIET_REPLAY_STEPS);
        drop(seq);
        if chain.is_empty() {
            return false;
        }
        self.last_quiet_replay_count
            .fetch_add(1, Ordering::Relaxed);
        self.last_quiet_replay_t_us.store(now_us, Ordering::Relaxed);
        self.last_quiet_replay_chain_len
            .store(chain.len() as u64, Ordering::Relaxed);
        true
    }

    /// C1: drive the vital core. The reserve is shaped by TWO things:
    ///  - the SIMULATED metabolic world (`vital_env`): a circadian supply that
    ///    makes reserve breathe, plus periodic famines that push it toward the
    ///    floor (engaging anxiety, Standby/revive, and self-preservation). This
    ///    is what fixes the "reserve pinned at 1.0 forever" problem.
    ///  - the body's REAL sensory continuity: a stalled input stream means the
    ///    engine cannot keep predicting, so it starves regardless of the world.
    ///
    /// The two are combined by `min`: you are nourished only when the world is
    /// supplying AND your senses are live. Drain = the world's metabolism plus
    /// recent vocal/byte output exertion (the spike-above-baseline pain path).
    ///
    /// The vital core integrates at a FIXED step (vital_env::VITAL_STEP_US),
    /// decoupled from the orchestrator's ~30 Hz call rate, so survival dynamics
    /// depend on wall-clock time rather than how fast the loop spins.
    pub fn tick_vital_core(&self, now_us: u64) {
        // Fixed-rate gate: advance at most once per VITAL_STEP_US.
        let last_adv = self.last_vital_advance_us.load(Ordering::Relaxed);
        if last_adv != 0 && now_us.saturating_sub(last_adv) < crate::vital_env::VITAL_STEP_US {
            return;
        }
        self.last_vital_advance_us.store(now_us, Ordering::Relaxed);

        // Birth-relative clock for the day/famine schedule (each session starts
        // a fresh day 0 = plenty; the persisted reserve still carries over).
        let start = self.start_us.load(Ordering::Relaxed);
        let env_now = now_us.saturating_sub(start);

        // Real sensory continuity → [0,1]. Recent observation = full.
        let last_obs = self.last_obs_t_us.load(Ordering::Relaxed);
        let age_us = now_us.saturating_sub(last_obs);
        let continuity = if last_obs == 0 {
            1.0 // no input yet at birth: let the world decide (no false starve)
        } else if age_us <= 1_000_000 {
            1.0
        } else if age_us >= 10_000_000 {
            0.0
        } else {
            1.0 - (age_us - 1_000_000) as f64 / 9_000_000.0
        };

        // World resource gated by the senses: fed only if both are present.
        let supply = continuity.min(self.vital_env.supply(env_now));

        // Drain = the world's resting metabolism + recent output exertion.
        let recently_voiced = {
            let t = self.last_vocal_output_t_us.load(Ordering::Relaxed);
            t != 0 && now_us.saturating_sub(t) < 1_000_000
        };
        let voicing_cost = if self.nursery_metabolic { 0.20 } else { 0.50 };
        let exertion = if recently_voiced { voicing_cost } else { 0.0 };
        let drain = (self.vital_env.drain(env_now) + exertion).clamp(0.0, 1.0);

        self.vital_core.write().unwrap().tick(now_us, supply, drain);
        // Stage 0 (interoceptive substrate): mirror vital scalars onto shared axes
        // so they are predicted, produce err, and enter the relation/tower fabric.
        // The box retains full autonomy over its dynamics; this is a READ + mirror.
        // 推论5: no positive/negative labels — these are substrate quantities only.
        let vc = self.vital_core.read().unwrap();
        self.push_observation(crate::INTERO_RESERVE, now_us, vc.reserve);
        self.push_observation(crate::INTERO_VITAL_ANXIETY, now_us, vc.vital_anxiety);
        self.push_observation(crate::INTERO_PAIN, now_us, vc.pain);
    }

    pub fn read_vital_core(&self) -> crate::vital_core::VitalCoreState {
        *self.vital_core.read().unwrap()
    }

    /// A2: drive the self model. Interoception (the body-survival sense) is the
    /// seed — reserve blended with calm (1 - anxiety). Agency comes from the
    /// self-caused-event ratio. The self grows around this; it is not born.
    pub fn tick_self_model(&self, now_us: u64) {
        let vital = self.read_vital_core();
        let interoception =
            (vital.reserve * 0.6 + (1.0 - vital.vital_anxiety) * 0.4).clamp(0.0, 1.0);
        let agency = self.self_development.read().unwrap().self_caused_event_ratio;
        self.self_model
            .write()
            .unwrap()
            .tick(now_us, interoception, agency);
        // Stage 0: mirror self-model scalars onto shared axes.
        let sm = self.self_model.read().unwrap();
        self.push_observation(crate::INTERO_SELF_COHERENCE, now_us, sm.self_coherence);
        self.push_observation(crate::INTERO_AGENCY, now_us, sm.agency);
    }

    pub fn read_self_model(&self) -> crate::self_model::SelfModelState {
        *self.self_model.read().unwrap()
    }

    /// B1-4: arbitrate the two drive sources into a reading.
    /// self-preservation from C1; attachment = the responsible person's
    /// long-earned familiarity from C5; principle_floor from 大脑警察 = the
    /// vital core's C6 acute pain (the body's own "this is hurting me now"
    /// transient). Continuous [0,1] mapping, no threshold gate: pain 0 keeps
    /// the floor branch inert (bit-identical to the unsourced default), and
    /// pain is already a fast-decaying EMA inside vital_core, so no extra
    /// smoothing is layered here.
    pub fn tick_dual_drive(&self, now_us: u64) {
        let vital = self.read_vital_core();
        let self_preservation = vital.self_preservation_pressure();
        // Phase B3: attachment is the max of (most_bound_node, most_bound_group)
        // — once groups form, the group-level reading subsumes the single-node
        // one because a stable group implies a coherent ensemble the body is
        // attached to (caregiver = several patches consistently co-active).
        // Additionally: if a FastPath has fired within the recent window, the
        // habit channel adds a small attachment bonus (capped at 0.10) — the
        // body's "I just used this familiar pathway, so it is here" signal.
        let (attachment, fp_recent_bonus) = {
            let graph = self.perturbation_graph.read().unwrap();
            let node_bind = graph
                .most_bound()
                .map(|n| n.bind_strength)
                .unwrap_or(0.0);
            let group_bind = graph
                .most_bound_group()
                .map(|g| g.bind_strength)
                .unwrap_or(0.0);
            // Treat any FastPath fired in the last 2 seconds as "habit alive".
            const FP_RECENT_WINDOW_US: u64 = 2_000_000;
            let any_recent = graph.fast_paths.by_endpoints.values().any(|fp| {
                fp.last_fired_t_us != 0
                    && now_us.saturating_sub(fp.last_fired_t_us) <= FP_RECENT_WINDOW_US
            });
            let bonus = if any_recent { 0.10 } else { 0.0 };
            (node_bind.max(group_bind), bonus)
        };
        let attachment = (attachment + fp_recent_bonus).clamp(0.0, 1.0);
        let principle_floor = vital.pain.clamp(0.0, 1.0);
        let reading =
            crate::dual_drive::arbitrate(self_preservation, attachment, principle_floor);
        *self.dual_drive.write().unwrap() = reading;
    }

    pub fn read_dual_drive(&self) -> crate::dual_drive::DualDriveReading {
        *self.dual_drive.read().unwrap()
    }

    /// Phase C2: intention arbitrator. The body's slow-deliberation slot —
    /// runs at the orchestrator cadence (~30 Hz).
    ///
    /// Triggers active deliberation when the FastPath is silent for several
    /// consecutive ticks (the design-doc "FastPath silent → wake the slow
    /// path"). Picks the next intention target from the strongest available
    /// relation out of the body's current "where it is" anchor (most_bound
    /// node, falling back to most_bound_group). Respects the commitment
    /// window so a fresh stimulus can't yank the target away in 100 ms.
    ///
    /// Side effects:
    /// - Stamps a MarkChannel::Intention pointing at the target on the graph
    /// - Updates `viewport.intention` (pressure / satisfaction tick)
    /// - Publishes INTERO_INTENTION_PRESSURE on the shared axis
    /// - Increments the FastPath-silent counter when no FastPath fires
    pub fn tick_intention_arbitrator(&self, now_us: u64) {
        use crate::perturbation_graph::{Endpoint, MarkChannel, MarkRef, ResponseOutcome};

        // Read-only first pass: discover the anchor and whether FastPath
        // would fire on it. Held under a read lock to keep the critical
        // section short.
        let (anchor_ep, anchor_sig, fp_fires, candidate, candidate_score) = {
            let graph = self.perturbation_graph.read().unwrap();
            // Anchor preference: most-bound node first (sharpest target),
            // falling back to most-bound group (the design-doc "ensemble"
            // when no single patch dominates).
            let (anchor_ep, anchor_sig): (Option<Endpoint>, Option<[f64; crate::perturbation_graph::PG_NODE_SIG_DIM]>) =
                match graph.most_bound() {
                    Some(n) => (Some(Endpoint::Node(n.id)), Some(n.sig)),
                    None => match graph.most_bound_group() {
                        Some(g) => (Some(Endpoint::Group(g.id)), Some(g.centroid_sig)),
                        None => (None, None),
                    },
                };
            // FastPath query on the anchor — the muscle-memory check that
            // gates whether slow reasoning needs to wake.
            let fp_fires = if let Some(ep) = anchor_ep {
                matches!(
                    graph.default_response(ep),
                    ResponseOutcome::FastPath { .. }
                )
            } else {
                false
            };
            // Find the strongest out-relation from the anchor as the slow
            // path's candidate. Score is the design-doc strength × expressivity.
            let (candidate, candidate_score) = if let Some(ep) = anchor_ep {
                if let Some(r) = graph.strongest_out_relation(ep) {
                    let score = r.strength * r.expressivity;
                    (Some((r.to, r.id)), score)
                } else {
                    (None, 0.0)
                }
            } else {
                (None, 0.0)
            };
            (anchor_ep, anchor_sig, fp_fires, candidate, candidate_score)
        };

        // Note the FastPath outcome (B3 EMA telemetry — also keeps the rate
        // honest even outside the intero probe path).
        {
            let mut g = self.perturbation_graph.write().unwrap();
            g.note_fast_path_query(fp_fires);
        }

        // Bump the FastPath-silent counter — when it crosses
        // ACTIVE_REASONING_FAST_PATH_SILENT_TICKS we are allowed to take
        // the slow path. We store this in a Viewport-owned atomic.
        let silent = if fp_fires {
            self.fast_path_silent_ticks.store(0, Ordering::Relaxed);
            0
        } else {
            self.fast_path_silent_ticks.fetch_add(1, Ordering::Relaxed) + 1
        };

        // Decide whether to actively commit a new intention this tick.
        // Three gates: silent run long enough, candidate exists, candidate
        // score above the minimum bar.
        const ACTIVE_REASONING_FAST_PATH_SILENT_TICKS: u32 = 3;
        const ACTIVE_REASONING_MIN_SCORE: f64 = 0.05;
        let mut intention = self.intention.write().unwrap();
        if silent >= ACTIVE_REASONING_FAST_PATH_SILENT_TICKS {
            // 1) Perception-derived candidate: the live anchor's strongest relation.
            if let Some((target_ep, rel_id)) = candidate {
                if candidate_score >= ACTIVE_REASONING_MIN_SCORE {
                    intention.try_commit(target_ep, Some(rel_id), candidate_score, now_us);
                }
            }
            // 2) B2.3 held-scene candidate — INPUT-DECOUPLED. When a fresh scene
            //    is held, its lead competes as an intention candidate scored by
            //    the neighbourhood salience. With no live perception candidate it
            //    fills the slot (the body forms an intention from what it holds
            //    in mind while idle); against a live candidate it must beat
            //    try_commit's dominance window, so perception still wins unless
            //    the scene is markedly stronger. It can only ever offer a node
            //    the graph itself evoked from learned relations — never a
            //    fabricated target.
            if let Some((lead, scene_score)) =
                self.perturbation_graph.read().unwrap().held_scene_lead(now_us)
            {
                if scene_score >= ACTIVE_REASONING_MIN_SCORE {
                    intention.try_commit(Endpoint::Node(lead), None, scene_score, now_us);
                }
            }
        }

        // Update satisfaction: how close is the current anchor's sig to
        // the target's sig?  Read the target sig via Endpoint dereference.
        if let (Some(target), Some(anchor)) = (intention.current_target, anchor_sig) {
            let g = self.perturbation_graph.read().unwrap();
            let target_sig: Option<[f64; crate::perturbation_graph::PG_NODE_SIG_DIM]> = match target {
                Endpoint::Node(id) => g.node(id).map(|n| n.sig),
                Endpoint::Group(gid) => g.group(gid).map(|gg| gg.centroid_sig),
                Endpoint::Relation(_) => None,
            };
            if let Some(t) = target_sig {
                // Bounded sim: 1.0 - distance / sqrt(N). Distance saturates
                // around sqrt(24) ≈ 4.9 for orthogonal sigs.
                let dist = crate::perturbation_graph::sig_distance(&anchor, &t);
                let sim = (1.0 - dist / (24.0_f64).sqrt()).clamp(0.0, 1.0);
                intention.update_satisfaction(sim);
            }
        }
        intention.tick(now_us);

        // Stamp the intention mark on the graph so downstream consumers
        // (attention bias, vocal drive field region C) can see what the
        // body is currently pushing toward.
        if let Some(target) = intention.current_target {
            let mark_ref = match target {
                Endpoint::Node(id) => MarkRef::Node(id),
                Endpoint::Group(gid) => MarkRef::Group(gid),
                Endpoint::Relation(rid) => MarkRef::Relation(rid),
            };
            let mark_strength = intention.intention_pressure.max(0.1);
            let mut g = self.perturbation_graph.write().unwrap();
            g.stamp_mark(MarkChannel::Intention, mark_ref, mark_strength, now_us);
        }

        // Publish INTERO_INTENTION_PRESSURE for the prediction fabric.
        let pressure = intention.intention_pressure;
        drop(intention);
        self.push_observation(crate::INTERO_INTENTION_PRESSURE, now_us, pressure);

        // Phase C: graph maintenance — decay marks + group members so the
        // arbitrator's stamps don't pile up forever. Cheap and idempotent.
        {
            let mut g = self.perturbation_graph.write().unwrap();
            g.decay_marks(now_us);
            g.decay_group_members(now_us);
            g.tick_fast_paths(now_us);
        }

        // Anchor-side audit hook is best-effort: if a FastPath fired AND
        // the post-fire observed state differs from its `to` endpoint, log
        // a disagreement. Cheap heuristic — true Phase C6 post-hoc check.
        if let (Some(anchor), Some(anchor_ep_v)) = (anchor_sig, anchor_ep) {
            let g_read = self.perturbation_graph.read().unwrap();
            if let ResponseOutcome::FastPath { to, .. } = g_read.default_response(anchor_ep_v) {
                let predicted_sig: Option<[f64; crate::perturbation_graph::PG_NODE_SIG_DIM]> = match to {
                    Endpoint::Node(id) => g_read.node(id).map(|n| n.sig),
                    Endpoint::Group(gid) => g_read.group(gid).map(|gg| gg.centroid_sig),
                    Endpoint::Relation(_) => None,
                };
                drop(g_read);
                if let Some(p) = predicted_sig {
                    let d = crate::perturbation_graph::sig_distance(&anchor, &p);
                    // > 0.4 in sig space → "wrong neighborhood" by tier-0.
                    let disagreed = d > 0.4;
                    let mut g = self.perturbation_graph.write().unwrap();
                    g.note_post_hoc_outcome(disagreed);
                }
            }
        }
    }

    pub fn read_intention(&self) -> crate::intention::IntentionState {
        *self.intention.read().unwrap()
    }

    /// Phase D1: spontaneous vocal expression region C.
    ///
    /// When the IntentionState commits to a node that has a VocalProfile,
    /// AND the recent sparsity gate clears (haven't fired this profile
    /// lately — design-doc D4 sparsity guard against monotone babble),
    /// push the profile's mel64 + f1/f2 hint onto vocal_target so the
    /// servo + vocal organ start aiming for it. This is the body
    /// "deciding to say something" rather than echoing what it just heard.
    ///
    /// Also handles the chained case (D2): if the intention sits on a
    /// vocal node with a Sequence relation out, the next-step vocal
    /// profile is queued so the body produces a small "phrase" instead
    /// of a single recurring sound.
    pub fn tick_spontaneous_vocal_expression(&self, now_us: u64) {
        use crate::perturbation_graph::{Endpoint, RELATION_TRAVERSAL_MAX_DEPTH};

        let intention = *self.intention.read().unwrap();
        let Some(target_ep) = intention.current_target else { return; };
        // Only express when there is meaningful pressure to do so — keeps
        // the channel quiet when the body is just observing.
        if intention.intention_pressure < 0.40 {
            return;
        }
        // D4 sparsity gate: don't re-express the same target within the
        // SPARSITY window; the gate enforces babble diversity rather than
        // monotone repetition of one habit.
        const SPONTANEOUS_SPARSITY_US: u64 = 3_000_000;
        let last_t = self
            .last_spontaneous_expression_t_us
            .load(Ordering::Relaxed);
        if last_t != 0 && now_us.saturating_sub(last_t) < SPONTANEOUS_SPARSITY_US {
            return;
        }

        // Resolve the target's vocal profile. If the target is a relation
        // endpoint, follow at most one step toward its `to` looking for a
        // vocal-bearing node.
        let chosen_profile: Option<(crate::perturbation_graph::VocalProfile, u32)> = {
            let g = self.perturbation_graph.read().unwrap();
            match target_ep {
                Endpoint::Node(id) => g.vocal_profile(id).cloned().map(|p| (p, id)),
                Endpoint::Group(gid) => {
                    // Prefer a vocal-profile member of the group, weighted by
                    // member.weight (the design-doc soft-membership rule).
                    if let Some(grp) = g.group(gid) {
                        let mut best: Option<(f64, crate::perturbation_graph::VocalProfile, u32)> = None;
                        for m in &grp.members {
                            if let Some(p) = g.vocal_profile(m.node_id) {
                                let s = m.weight * p.fit;
                                if best.as_ref().map(|(bs, _, _)| s > *bs).unwrap_or(true) {
                                    best = Some((s, p.clone(), m.node_id));
                                }
                            }
                        }
                        best.map(|(_, p, id)| (p, id))
                    } else {
                        None
                    }
                }
                Endpoint::Relation(rid) => {
                    // Walk along the relation toward its `to` looking for a
                    // vocal-bearing endpoint. Cycle-safe via the bounded depth.
                    let mut visited = std::collections::HashSet::new();
                    visited.insert(Endpoint::Relation(rid));
                    let mut current = g.relation(rid).map(|r| r.to);
                    let mut depth = 0usize;
                    let mut found: Option<(crate::perturbation_graph::VocalProfile, u32)> = None;
                    // B2.1: each hop is temperature-sampled rather than hard
                    // argmax, so a mature graph can wander to a non-strongest
                    // edge (input-decoupled association). Temperature ramps with
                    // graph maturity: a cold graph stays argmax (zero regression).
                    // The LCG (same constants as phoneme_memory::LcgRng) only
                    // picks which vocal node we express and never feeds
                    // persistence, so seed-determinism from now_us is enough.
                    let temp = crate::PG_WALK_MAX_TEMP
                        * crate::pg_reasoning_blend(g.relations.len());
                    let mut rng = now_us ^ 0x9e37_79b9_7f4a_7c15;
                    while let Some(ep) = current {
                        if depth >= RELATION_TRAVERSAL_MAX_DEPTH {
                            break;
                        }
                        if !visited.insert(ep) {
                            break;
                        }
                        if let Endpoint::Node(id) = ep {
                            if let Some(p) = g.vocal_profile(id) {
                                found = Some((p.clone(), id));
                                break;
                            }
                        }
                        rng = rng
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        let uu = ((rng >> 11) as f64) / ((1u64 << 52) as f64);
                        current = g.sample_out_relation(ep, temp, uu).map(|r| r.to);
                        depth += 1;
                    }
                    found
                }
            }
        };

        let Some((profile, _node_id)) = chosen_profile else { return; };

        // Synthesize a vocal_target from the profile. We don't have a
        // separate target_pitch in the profile — derive a neutral mid
        // pitch and use f1/f2 offsets as resonance hints.
        let pitch_hint = 0.5; // mid baseline; the servo will refine
        let resonance_hint = (profile.f1_off + profile.f2_off) / 2.0 + 0.5;
        let resonance_hint = resonance_hint.clamp(0.0, 1.0);
        // Bands proxy: zero for the legacy 16-band shape (the mel64 is the
        // true target signal).
        let bands_proxy = [0.0f64; crate::VIB_BANDS];
        let env = intention.intention_pressure.clamp(0.05, 0.95);

        self.observe_vocal_target_full(
            now_us,
            intention.intention_pressure,
            pitch_hint,
            resonance_hint,
            env,
            bands_proxy,
            profile.mel64,
        );
        self.last_spontaneous_expression_t_us
            .store(now_us, Ordering::Relaxed);
    }

    pub fn tick_vocal_imitation_loop(&self, now_us: u64) {
        let target = self.read_vocal_target();

        if target.active_energy < 0.01 {
            let (was_active, is_idle, decay_magnitude) = {
                let mut organ = self.vocal_organ.write().unwrap();
                let was_active = organ.phase == crate::vocal_organ::VocalOrganPhase::Active;
                let prev_breath = organ.breath_drive;
                organ.tick_decay(now_us);
                (
                    was_active,
                    organ.phase == crate::vocal_organ::VocalOrganPhase::Idle,
                    (prev_breath - organ.breath_drive).max(0.0),
                )
            };
            if was_active {
                let feedback = if is_idle {
                    ActionOutputFeedback::Stop
                } else {
                    ActionOutputFeedback::Decay
                };
                self.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback,
                    magnitude: decay_magnitude,
                    event_t_us: now_us,
                });
            }
            return;
        }

        let organ = self.read_vocal_organ();

        let pitch_gap = (target.pitch_hint - organ.pitch_tension).abs();
        let resonance_gap = (target.resonance_hint - organ.resonance_position).abs();
        let total_gap = (pitch_gap + resonance_gap) / 2.0;

        {
            let mut imit = self.vocal_imitation.write().unwrap();
            imit.observe_gap(now_us, total_gap, organ.breath_drive);
        }

        if organ.phase == crate::vocal_organ::VocalOrganPhase::Idle {
            self.begin_voicing(
                now_us,
                target.active_energy * 0.5,
                target.pitch_hint,
                target.resonance_hint,
            );
        } else {
            self.adjust_voicing(
                now_us,
                target.active_energy * 0.5,
                target.pitch_hint,
                target.resonance_hint,
            );
        }

        let imit = self.read_vocal_imitation();
        // P3: graded credit — last_gap is now the real acoustic shape distance;
        // record acoustically-ballpark matches (the servo refines quality over time)
        // rather than demanding a near-perfect score the synthesis may never reach.
        if imit.last_gap < 0.40 && imit.converge_count >= 3 {
            let organ = self.read_vocal_organ();
            {
                let mut mem = self.vocal_memory.write().unwrap();
                mem.record_success(
                    now_us,
                    organ.breath_drive,
                    organ.pitch_tension,
                    organ.resonance_position,
                );
            }
        }

        {
            let mut vt = self.vocal_target.write().unwrap();
            vt.tick_decay(now_us);
        }
    }

    pub fn tick_vocal_memory(&self, now_us: u64) {
        // Record a prototype when the imitation loop has converged on a target
        // AND the organ actually phonated (real breath, not a survival-gated
        // zero). This is the LEARNING side of the vocal loop.
        //
        // 整合修复: this used to live in tick_vocal_imitation_loop, which the
        // 改造6 unified-drive-field refactor orphaned — body.rs drives voicing
        // via tick_vocal_drive_field but never called the imitation loop, so
        // converged sounds were never stored (n_prototypes stayed 0 forever
        // despite converge_count climbing into the hundreds). Moved here, the
        // recording runs every tick right after tick_vocal_drive_field updates
        // the imitation gap.
        let imit = self.read_vocal_imitation();
        // P3: graded credit — last_gap is now the real acoustic shape distance;
        // record acoustically-ballpark matches (the servo refines quality over time)
        // rather than demanding a near-perfect score the synthesis may never reach.
        if imit.last_gap < 0.40 && imit.converge_count >= 3 {
            let organ = self.read_vocal_organ();
            if organ.phase == crate::vocal_organ::VocalOrganPhase::Active
                && organ.breath_drive > 0.01
            {
                let mut mem = self.vocal_memory.write().unwrap();
                mem.record_success(
                    now_us,
                    organ.breath_drive,
                    organ.pitch_tension,
                    organ.resonance_position,
                );
            }
        }

        let mut mem = self.vocal_memory.write().unwrap();
        mem.tick_decay(now_us);
    }

    // ── Phase 1 (keystone ①): Gaussian visual field ──────────────────────────

    /// Drive one predictive-coding tick of the Gaussian field from a camera
    /// frame, then mirror the two summary scalars onto the unified tier-0 axis
    /// stream.  Called by RealPhotAdapter::tick() with every received frame.
    ///
    /// DoD (b): GAUSS_RECON_MSE_AXIS and GAUSS_ACTIVE_FRAC_AXIS are pushed via
    /// push_observation so they enter the same observed/predicted/err/prior fabric
    /// as every other tier-0 axis.
    pub fn tick_gaussian_field(
        &self,
        frame: &crate::phot_camera::CameraFrame,
        now_us: u64,
    ) {
        let tt = self.read_efference_copy().translation_term();
        let mse = {
            let mut field = self.gaussian_field.write().unwrap();
            let mse = field.tick_frame(frame);
            // Phase 2: per-Gaussian depth-from-parallax. Uses the body's own
            // commanded self-motion (efference tt). The real-camera path has no
            // efference → tt = None → depth marked unobservable, ρ untouched.
            field.update_depth(tt);
            mse
        };
        let (active_frac, scene_fp) = {
            let field = self.gaussian_field.read().unwrap();
            (
                field.active_count() as f64 / crate::phot_gaussian::GAUSS_CAP as f64,
                crate::phot_sequence::scene_fingerprint(&field),
            )
        };
        // Visual impression layer (Q2/3): refresh the scene-fingerprint buffer
        // and let the persistent edge table learn any transition that just
        // happened. Sentinel-safe (a fingerprint of 0 = empty scene, ignored
        // by observe_scene), so a dark frame doesn't poison the impression.
        self.phot_sequence
            .write()
            .unwrap()
            .observe_scene(now_us, scene_fp);
        self.push_observation(crate::GAUSS_RECON_MSE_AXIS,   now_us, mse.clamp(0.0, 1.0));
        self.push_observation(crate::GAUSS_ACTIVE_FRAC_AXIS, now_us, active_frac.clamp(0.0, 1.0));
        // Pure-vision developmental closure: earn maturity from prediction-error
        // reduction on what the body sees (evidence-gated on real structure).
        self.observe_visual_reflex_closure(now_us, mse, active_frac);
        // Born-in looming/approach detector + innate defensive reflex (runs on the
        // region brightness axes region_perception just pushed for this frame).
        self.tick_looming(now_us);
    }

    /// Read a snapshot clone of the Gaussian field state (for observers / tests).
    pub fn read_gaussian_field(&self) -> crate::phot_gaussian::GaussianField {
        self.gaussian_field.read().unwrap().clone()
    }

    /// Per-vocal-tick-style maintenance for the visual impression layer:
    /// decay the per-edge eligibility trace by wall time, mirroring
    /// `tick_vocal_sequence_boundary` on the acoustic side. Cheap; safe to
    /// call from the same 30 Hz visual ticker as `tick_gaussian_field`.
    pub fn tick_phot_sequence_boundary(&self, now_us: u64) {
        self.phot_sequence.write().unwrap().decay_eligibility(now_us);
    }

    /// Feed a browser-side 32×24 luma grid (headless / NetSensesAdapter) into
    /// the Gaussian visual field.  Upscales to GAUSS_REC_W×GAUSS_REC_H with
    /// Upsample 160×120 RGB planar grid → Gaussian field target (320×240) and tick.
    pub fn tick_gaussian_field_from_net(&self, grid: &[f32], now_us: u64) {
        use crate::phot_gaussian::{GAUSS_PIX, GAUSS_REC_W, GAUSS_REC_H, GAUSS_CAP};
        const SRC_W: usize = 160;
        const SRC_H: usize = 120;
        const SRC_PIX: usize = SRC_W * SRC_H;

        // Upsample 160×120 → 320×240 (2× nearest-neighbor) and create 3-channel target
        let mut target = vec![0.0f64; 3 * GAUSS_PIX];
        for ty in 0..GAUSS_REC_H {
            let sy = ty / 2;  // source y: exact 2× mapping
            for tx in 0..GAUSS_REC_W {
                let sx = tx / 2;
                let src_i = sy * SRC_W + sx;
                let dst_i = ty * GAUSS_REC_W + tx;
                if src_i < SRC_PIX {
                    target[dst_i]               = grid.get(src_i).copied().unwrap_or(0.0) as f64;  // R
                    target[GAUSS_PIX + dst_i]   = grid.get(SRC_PIX + src_i).copied().unwrap_or(0.0) as f64;  // G
                    target[2*GAUSS_PIX + dst_i] = grid.get(2*SRC_PIX + src_i).copied().unwrap_or(0.0) as f64; // B
                }
            }
        }
        let tt = self.read_efference_copy().translation_term();
        let mse = {
            let mut field = self.gaussian_field.write().unwrap();
            let mse = field.tick_grid(&target);
            field.update_depth(tt);
            mse
        };
        let (active_frac, scene_fp) = {
            let field = self.gaussian_field.read().unwrap();
            (
                field.active_count() as f64 / GAUSS_CAP as f64,
                crate::phot_sequence::scene_fingerprint(&field),
            )
        };
        // Visual impression layer (Q2/3) — same hook as the real-camera path.
        self.phot_sequence
            .write()
            .unwrap()
            .observe_scene(now_us, scene_fp);
        self.push_observation(crate::GAUSS_RECON_MSE_AXIS,   now_us, mse.clamp(0.0, 1.0));
        self.push_observation(crate::GAUSS_ACTIVE_FRAC_AXIS, now_us, active_frac.clamp(0.0, 1.0));
        // Pure-vision developmental closure (same hook as the real-camera path):
        // earn maturity from prediction-error reduction on what the body sees.
        self.observe_visual_reflex_closure(now_us, mse, active_frac);
        // Born-in looming/approach detector + innate defensive reflex (production
        // headless path; net_adapter pushed the region vis axes before this call).
        self.tick_looming(now_us);
    }

    /// Born-in LOOMING / APPROACH detector + innate defensive reflex.
    ///
    /// Computes a coarse global "visual mass" from the per-region BRIGHTNESS axes
    /// (mean brightness + bright-footprint fraction — the same proxy validated by
    /// `bin/sense_probe`'s looming test), tracks its slow EMA, and emits the
    /// rectified positive GROWTH as VIS_LOOMING_AXIS: high when something is
    /// approaching / expanding, ~0 for a stable or receding scene.
    ///
    /// This is a born-in FEATURE detector (sensitive to approach, NOT to identity
    /// — no face/predator template; route-A compliant). It then fires the innate
    /// UNCONDITIONED defensive reflex (`apply_perceptual_threat` → raises
    /// vital_anxiety), giving perception its first path into the negative-valence
    /// value system. The matching LEARNED ("prepared") leg is the VPC predictor
    /// that learns VIS_LOOMING_AXIS predicts INTERO_VITAL_ANXIETY.
    ///
    /// Called from both vision ingresses (`tick_gaussian_field` real camera +
    /// `tick_gaussian_field_from_net` headless) after the region axes are pushed.
    pub fn tick_looming(&self, now_us: u64) -> f64 {
        // Growth gain: per-frame visual-mass deltas are small, so amplify the
        // mass-vs-EMA gap into a usable [0,1] approach signal.
        const LOOMING_GROWTH_GAIN: f64 = 6.0;
        // EMA follow rate for the mass baseline. Slow enough that a sustained
        // approach keeps reading positive; fast enough that a settled scene
        // relaxes the signal back to ~0 within a second or so.
        const LOOMING_EMA_ALPHA: f64 = 0.10;
        // Per-region brightness above this counts toward the bright footprint
        // (mirrors sense_probe::bright_footprint's 0.10 threshold).
        const BRIGHT_THRESH: f64 = 0.10;

        // Global visual mass = mean region brightness blended with the fraction of
        // regions that are "lit". A blob that approaches grows both.
        let mut sum = 0.0;
        let mut lit = 0.0;
        for r in 0..crate::VIS_REGIONS {
            let b = self
                .latest_observation(crate::vis_axis(r, crate::vis_sub::BRIGHTNESS))
                .map(|o| o.value)
                .unwrap_or(0.0);
            sum += b;
            if b > BRIGHT_THRESH {
                lit += 1.0;
            }
        }
        let regions = crate::VIS_REGIONS as f64;
        let mass = sum / regions; // [0,1]
        let footprint = lit / regions; // [0,1]
        let combined = 0.5 * mass + 0.5 * footprint;

        // Rectified growth vs the slow baseline = the looming signal.
        let looming = {
            let mut ema = self.looming_mass_ema.write().unwrap();
            match *ema {
                None => {
                    // First frame: seed the baseline, emit no approach.
                    *ema = Some(combined);
                    0.0
                }
                Some(prev) => {
                    let l = ((combined - prev) * LOOMING_GROWTH_GAIN).clamp(0.0, 1.0);
                    *ema = Some(prev + LOOMING_EMA_ALPHA * (combined - prev));
                    l
                }
            }
        };

        // Mirror onto the shared substrate (predicted / err / VPC source / relations).
        self.push_observation(crate::VIS_LOOMING_AXIS, now_us, looming);

        // Innate defensive reflex: a looming stimulus raises dread directly.
        if looming > 0.0 {
            self.vital_core.write().unwrap().apply_perceptual_threat(looming);
        }
        looming
    }

    /// Compute this tick's emergent gaze command — the visual sibling of the
    /// vocal drive field. Returns `(bias, floor_frac)`:
    ///   * `bias` ∈ [-1,1]: the EMERGENT "where to look" — the err-weighted
    ///     lateral centroid of per-region visual PREDICTION ERROR (curiosity:
    ///     look where the body fails to predict), arbitrated with the looming
    ///     source through `GazeDriveField::combine`, then survival-gated. No
    ///     labels, no scripted scan path.
    ///   * `floor_frac` ∈ [0,1]: the endogenous wobble amplitude — the anti-0×x
    ///     bootstrap floor, maturity-annealed coarse→fine. NOT survival-gated:
    ///     the always-on micro-wobble must survive even a metabolic crisis so the
    ///     no-motion→no-depth→no-error→no-drive loop can never form.
    /// The neck adds them: cam_x = bias·NECK_BIAS_MAX + floor·NECK_AMP_MAX·sin(…).
    pub fn compute_gaze_drive(&self, _now_us: u64) -> (f64, f64) {
        use crate::{vis_axis, vis_sub, VIS_GRID_COLS, VIS_REGIONS};
        // Per-region curiosity = how much the body fails to predict that region's
        // visual signal (AR-predictor err summed over the structural sub-axes) —
        // genuine prediction pressure, not raw motion.
        const CURIO_SUBS: [usize; 3] =
            [vis_sub::BRIGHTNESS, vis_sub::MOTION, vis_sub::EDGE_ENERGY];
        let half = (VIS_GRID_COLS as f64 - 1.0) / 2.0; // column-centre index
        let mut total = 0.0;
        let mut signed = 0.0;
        for r in 0..VIS_REGIONS {
            let mut curio = 0.0;
            for &sub in &CURIO_SUBS {
                curio += self.latest_err(vis_axis(r, sub)).unwrap_or(0.0).abs();
            }
            let gc = (r % VIS_GRID_COLS) as f64;
            let col_sign = if half > 0.0 { (gc - half) / half } else { 0.0 }; // [-1,1]
            total += curio;
            signed += curio * col_sign;
        }
        let direction = if total > 1e-9 { (signed / total).clamp(-1.0, 1.0) } else { 0.0 };
        // Normalize total err into an urge in [0,1] (a very surprising scene caps).
        let curio_intensity = (total / VIS_REGIONS as f64).clamp(0.0, 1.0);
        // Looming (approach) is a second source that raises the urge to look
        // toward the same curious centroid.
        let looming = self
            .latest_observation(crate::VIS_LOOMING_AXIS)
            .map(|o| o.value)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let field = crate::gaze_drive_field::GazeDriveField::combine(&[
            crate::gaze_drive_field::GazeDriveContribution {
                intensity: curio_intensity,
                direction,
            },
            crate::gaze_drive_field::GazeDriveContribution {
                intensity: looming,
                direction,
            },
        ]);
        // Survival gate (dual-drive self-preservation): a starving body stops the
        // energy-spending DIRECTED gaze — same metabolic guard voicing uses. The
        // floor below is deliberately left ungated (see doc) to preserve the
        // anti-deadlock guarantee even mid-crisis.
        let sp = self.read_vital_core().self_preservation_pressure();
        let gain = (1.0 - sp.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let bias = (field.bias() * gain).clamp(-1.0, 1.0);
        let floor = crate::gaze_babble_floor(self.developmental_maturity()).clamp(0.0, 1.0);
        (bias, floor)
    }

    /// Phase 3 Stage 3: read the body's current cross-modal + vital state so
    /// vision_tier0 can accumulate those correlations onto refreshed nodes.
    ///
    /// cross_modal[8]: 8 bins of the body's own VOICE — pairs of self_vocal_bands
    /// averaged down from 16 to 8. A node that is reliably active while the
    /// body voices spectrum X accumulates that signature in its cross_modal
    /// stats. The "can sound" / "speaks-like-this" affordance emerges as the
    /// MARGINAL STATISTIC of which spectral bins co-fire with this node, not
    /// as a label.
    ///
    /// vital_coupling[6]: the body's current metabolic + protective scalars
    /// (reserve, vital_anxiety, pain, drain_baseline, last_supply, last_drain).
    /// A node that is reliably active when the body feels safe/well accumulates
    /// positive vital_coupling; a node active during pain/anxiety accumulates
    /// the opposite. The "friendly / dangerous" affordance emerges from the
    /// signed pattern of these stats, never as a binary flag.
    fn read_tier0_couplings(
        &self,
    ) -> (
        [f64; crate::perturbation_graph::PG_CROSS_MODAL_DIM],
        [f64; crate::perturbation_graph::PG_VITAL_DIM],
    ) {
        let mut cm = [0.0f64; crate::perturbation_graph::PG_CROSS_MODAL_DIM];
        let (bands, _) = self.read_self_vocal_bands();
        // 16 → 8 averaging pairs (VIB_BANDS / PG_CROSS_MODAL_DIM = 2).
        let stride = crate::VIB_BANDS / crate::perturbation_graph::PG_CROSS_MODAL_DIM;
        for i in 0..crate::perturbation_graph::PG_CROSS_MODAL_DIM {
            let mut acc = 0.0;
            for j in 0..stride {
                acc += bands[i * stride + j].abs();
            }
            cm[i] = (acc / stride as f64).clamp(0.0, 1.0);
        }
        let vital = self.read_vital_core();
        let vc = [
            vital.reserve.clamp(0.0, 1.0),
            vital.vital_anxiety.clamp(0.0, 1.0),
            vital.pain.clamp(0.0, 1.0),
            vital.drain_baseline.clamp(0.0, 1.0),
            vital.last_supply.clamp(0.0, 1.0),
            vital.last_drain.clamp(0.0, 1.0),
        ];
        (cm, vc)
    }

    /// Phase 3 Stage 2/3: drive the tier-0 visual frontend from one CameraFrame.
    /// Used by the hardware path (RealPhotAdapter) and the virtual-neck
    /// curriculum path. Cross-modal (8 vocal bins) + vital_coupling (6 vital
    /// scalars) are read here and forwarded so refreshed nodes accumulate
    /// affordance statistics. Novel nodes start with zero couplings — the
    /// stats only accumulate on RE-RECOGNISED nodes (Stage 1 contract).
    pub fn tick_vision_tier0_from_frame(
        &self,
        frame: &crate::phot_camera::CameraFrame,
        now_us: u64,
    ) {
        let (cm, vc) = self.read_tier0_couplings();
        let mut vt = self.vision_tier0.write().unwrap();
        let mut graph = self.perturbation_graph.write().unwrap();
        vt.process_frame_into_graph(frame, now_us, &mut graph, Some(&cm), Some(&vc));
    }

    /// Phase 3 Stage 2/3: drive the tier-0 visual frontend from a headless
    /// 160×120 RGB planar grid (the gateway's SensesFrame.grid). Same Stage 3
    /// coupling reads as the from_frame path.
    pub fn tick_vision_tier0_from_net(&self, grid: &[f32], now_us: u64) {
        use crate::phot_camera::{CameraFrame, CAM_H, CAM_PIX, CAM_W};
        const SRC_W: usize = 160;
        const SRC_H: usize = 120;
        const SRC_PIX: usize = SRC_W * SRC_H;
        if grid.len() < 3 * SRC_PIX {
            return; // malformed frame — silently skip rather than panic
        }
        let mut frame = CameraFrame {
            t_us: now_us,
            luma: vec![0.0; CAM_PIX],
            r: vec![0.0; CAM_PIX],
            g: vec![0.0; CAM_PIX],
            b: vec![0.0; CAM_PIX],
        };
        // 2× nearest-neighbour upsample matching tick_gaussian_field_from_net.
        for ty in 0..CAM_H {
            let sy = (ty / 2).min(SRC_H - 1);
            for tx in 0..CAM_W {
                let sx = (tx / 2).min(SRC_W - 1);
                let src_i = sy * SRC_W + sx;
                let dst_i = ty * CAM_W + tx;
                let r = grid[src_i] as f64;
                let g = grid[SRC_PIX + src_i] as f64;
                let b = grid[2 * SRC_PIX + src_i] as f64;
                frame.r[dst_i] = r;
                frame.g[dst_i] = g;
                frame.b[dst_i] = b;
                frame.luma[dst_i] = (r + g + b) / 3.0;
            }
        }
        let (cm, vc) = self.read_tier0_couplings();
        let mut vt = self.vision_tier0.write().unwrap();
        let mut graph = self.perturbation_graph.write().unwrap();
        vt.process_frame_into_graph(&frame, now_us, &mut graph, Some(&cm), Some(&vc));
    }
}
