use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::RwLock;

use crate::action_consequence::ActionConsequenceState;
use crate::action_output::ActionOutputState;
use crate::language_binding::{BindingEvent, BindingTargetRef, BindingTrace, LanguageBindingState};
use crate::object_high_layer::{ObjectHighLayerPredictor, ObjectHighLayerState};
use crate::phot_predictor::PhotSlotPredictor;
use crate::probe_law::ProbeLawState;
use crate::{
    AXIS_COUNT, ATTN_ERR_REBALANCE_ALPHA, ATTN_PRECISION_BIAS_MAX, BINDING_ATTN_ALPHA,
    BINDING_MAINTENANCE_DECAY_STEP, BINDING_MAINTENANCE_IDLE_US, BINDING_RESONANCE_THRESHOLD,
    CLAIM_CROSS_MODALITY_BASE, CLAIM_KIND_ALLOCATED, CLAIM_KIND_DROPPED, CLAIM_KIND_REUSED,
    CLAIM_LOG_ENERGY_BAND, CLAIM_REUSE_MIN_SCORE, CLAIM_SAME_MODALITY_BONUS, CLAIM_SURPRISE_BAND,
    CLAIM_W_ENERGY, CLAIM_W_MODALITY, CLAIM_W_SPATIAL, CLAIM_W_SURPRISE, CLAIM_W_TIME,
    CROSS_MODAL_SPATIAL_DIST, CROSS_MODAL_WINDOW_US, HEALTH_FAULT, HEALTH_OK, HEALTH_QUIET,
    HEALTH_STALE, HISTORY_CAP, MOD_MECH, MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, N_REL_EDGES,
    N_ROUTING_FOCUS_EDGES, N_ROUTING_FOCUS_SLOTS, PERSISTENCE_MAX_CYCLES, PERSISTENCE_MIN_CYCLES,
    QUIET_ENERGY_THRESHOLD, QUIET_WINDOW_US, REASONING_HISTORY_ALPHA, REASONING_HISTORY_RING_CAP,
    REASONING_INTENSITY_THRESHOLD, REPLAY_ALPHA, REL_ALPHA, REL_PRED_ALPHA, REL_PRED_MSE_ALPHA,
    REL_PRED_PRECISION_MAX, REL_PRED_WARMUP_MIN, SAME_MODAL_SPATIAL_DIST, SLOT_IDLE_US, VIB_BANDS,
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
    pub precision: f64,
    pub last_err: f64,
    pub total_updates: u64,
    pub mean_sq_err: f64,
}

impl PredictorState {
    pub fn new() -> Self {
        Self {
            a: 0.0,
            b: 0.0,
            precision: 1.0,
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
/// err / precision). Length is not constrained across modalities: VIB's band
/// count and PHOT's band count are independent.
#[derive(Clone, Debug)]
pub struct BandFamilyState {
    pub n_bands: u16,
    pub observed: Vec<f64>,
    pub predicted: Vec<f64>,
    pub err: Vec<f64>,
    pub precision: Vec<f64>,
}

impl BandFamilyState {
    pub fn zeros(n_bands: u16) -> Self {
        let n = n_bands as usize;
        Self {
            n_bands,
            observed: vec![0.0; n],
            predicted: vec![0.0; n],
            err: vec![0.0; n],
            precision: vec![0.0; n],
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
    /// Weighted mean of per-band precision values across active modalities.
    pub fused_precision: f64,
    /// Weighted mean of per-band |err| values across active modalities.
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
/// All fields numeric-only. `relation_err` and `relation_precision` are 0
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
    pub relation_precision: f64,
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
    /// EMA of relation_err² — feeds relation_precision.
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
#[derive(Clone, Copy, Debug, Default)]
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

// SemanticReadinessState (Phase 5H) and SymbolizerState (Phase 6A) were
// removed by cleanup spec 2026-04-20. They made naming / labeling a
// resource gate over substrate and stamped objects / edges with discrete
// concept ids — both forbidden by language-system axioms.

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

// Helpers used by refresh_slot_fusion. Not methods to avoid borrow conflicts.

fn freshness_weight(last_t_us: u64, now_us: u64) -> f64 {
    if last_t_us == 0 {
        return 0.0;
    }
    let dt = now_us.saturating_sub(last_t_us);
    clamp01(1.0 - dt as f64 / SLOT_IDLE_US as f64)
}

fn mean_band_precision(fam: Option<&BandFamilyState>) -> f64 {
    match fam {
        None => 0.0,
        Some(f) if f.precision.is_empty() => 0.0,
        Some(f) => f.precision.iter().sum::<f64>() / f.precision.len() as f64,
    }
}

fn mean_abs_band_err(fam: Option<&BandFamilyState>) -> f64 {
    match fam {
        None => 0.0,
        Some(f) if f.err.is_empty() => 0.0,
        Some(f) => f.err.iter().map(|e| e.abs()).sum::<f64>() / f.err.len() as f64,
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
    /// Phase 4C: slot-local PHOT predictor. Ephemeral — not snapshot'd.
    /// Reset when the slot is cleared by sweep_slots. None until first PHOT write.
    pub phot_pred: Option<PhotSlotPredictor>,
    /// Phase 5A: cross-modal fusion subtree. Numeric-only. Reset with slot.
    /// Populated by Viewport::refresh_slot_fusion(); adapters never write here.
    pub fusion: Option<CrossModalFusionState>,
    /// Phase 5B: high-layer output subtree. Snapshot'd. Reset with slot.
    /// Populated by Viewport::refresh_slot_high_layer() after fusion refresh.
    pub high_layer: Option<ObjectHighLayerState>,
    /// Phase 5B: high-layer predictor state. Ephemeral — not snapshot'd.
    /// Reset with slot lifecycle. None until first high-layer refresh.
    pub hl_pred: Option<ObjectHighLayerPredictor>,
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
            phot_pred: None,
            fusion: None,
            high_layer: None,
            hl_pred: None,
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
    /// Phase 5A: cross-modal fusion state snapshot. None until first refresh.
    pub fusion: Option<CrossModalFusionState>,
    /// Phase 5B: high-layer output state snapshot. None until first hl refresh.
    pub high_layer: Option<ObjectHighLayerState>,
    /// Phase 5C: normalised attention share suggestion. 0.0 until first refresh.
    pub attention_share: f64,
    /// Phase 5C: precision bias suggestion (2 * share, clamped [0,2]).
    pub precision_bias: f64,
}

pub struct Viewport {
    pub master_tick: AtomicU64,
    pub clock_us: AtomicU64,
    pub start_us: AtomicU64,

    pub source_writes: AtomicU64,
    pub predictor_ticks: AtomicU64,
    pub snapshot_writes: AtomicU64,
    pub snapshot_fail_count: AtomicU64,

    pub hearing_fault: AtomicBool,
    pub hearing_fault_count: AtomicU64,
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

    pub observed: Vec<RwLock<AxisRing>>,
    pub predicted: Vec<RwLock<AxisRing>>,
    pub err_stream: Vec<RwLock<AxisRing>>,

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

    /// Language binding remains an observation-side trace substrate.
    /// It references existing object / relation / time structures but never
    /// decides whether those structures exist.
    pub language_binding: RwLock<LanguageBindingState>,

    // semantic_readiness and symbolizer subtrees were removed by cleanup
    // spec 2026-04-20. Substrate now terminates at global_reasoning_history
    // / global_reasoning_memory_summary.
    /// Action-output substrate — body's write-out side (v25).
    /// Numeric-only; no world content; no language gate.
    pub action_output: RwLock<ActionOutputState>,
    /// Monotonic counter incremented on every Begin event. Consumed by
    /// action_output.rs to stamp current_write_seq on each new write trace.
    pub action_output_write_seq: AtomicU64,
    /// Monotonic counter for stable window_id assignment. Parallels action_output_write_seq.
    pub action_consequence_window_seq: AtomicU64,
    /// Action-consequence loop substrate (v26). Records consequence windows
    /// and explicit pairs. No target inference; no success/failure semantics.
    pub action_consequence: RwLock<ActionConsequenceState>,

    /// Probe-law substrate (v30). Intensity + suppression + phase + short history.
    /// Numeric/time-only; no target, result, or semantic fields.
    pub probe_law: RwLock<ProbeLawState>,
}

impl Viewport {
    pub fn new() -> Self {
        let mut observed = Vec::with_capacity(AXIS_COUNT);
        let mut predicted = Vec::with_capacity(AXIS_COUNT);
        let mut err_stream = Vec::with_capacity(AXIS_COUNT);
        let mut predictors = Vec::with_capacity(AXIS_COUNT);
        for _ in 0..AXIS_COUNT {
            observed.push(RwLock::new(AxisRing::new()));
            predicted.push(RwLock::new(AxisRing::new()));
            err_stream.push(RwLock::new(AxisRing::new()));
            predictors.push(RwLock::new(PredictorState::new()));
        }
        let mut slots = Vec::with_capacity(N_OBJECT_SLOTS);
        for _ in 0..N_OBJECT_SLOTS {
            slots.push(RwLock::new(ObjectSlot::empty()));
        }
        Self {
            master_tick: AtomicU64::new(0),
            clock_us: AtomicU64::new(0),
            start_us: AtomicU64::new(0),
            source_writes: AtomicU64::new(0),
            predictor_ticks: AtomicU64::new(0),
            snapshot_writes: AtomicU64::new(0),
            snapshot_fail_count: AtomicU64::new(0),
            hearing_fault: AtomicBool::new(false),
            hearing_fault_count: AtomicU64::new(0),
            last_obs_t_us: AtomicU64::new(0),
            health_state: AtomicU8::new(HEALTH_QUIET),
            phot_health: AtomicU8::new(HEALTH_QUIET),
            phot_started_t_us: AtomicU64::new(0),
            phot_last_frame_t_us: AtomicU64::new(0),
            observed,
            predicted,
            err_stream,
            predictors,
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
            language_binding: RwLock::new(LanguageBindingState::default()),
            action_output: RwLock::new(ActionOutputState::default()),
            action_output_write_seq: AtomicU64::new(0),
            action_consequence_window_seq: AtomicU64::new(0),
            action_consequence: RwLock::new(ActionConsequenceState::default()),
            probe_law: RwLock::new(ProbeLawState::default()),
        }
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
        precision: &[f64; VIB_BANDS],
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
            precision: precision.to_vec(),
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
        precision: &[f64],
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if predicted.len() != n || err.len() != n || precision.len() != n {
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
            precision: precision.to_vec(),
        });
    }

    /// Phase 4C: write PHOT band data into a slot, running the slot-local AR-1
    /// predictor to produce real `predicted`, `err`, and `precision` values.
    ///
    /// The predictor state (`phot_pred`) lives inside the slot and resets
    /// automatically when the slot is cleared by `sweep_slots`. Adapters call
    /// this instead of `write_slot_phot` so the slot's PHOT family always
    /// carries genuine prediction output rather than placeholder zeros.
    ///
    /// Rules:
    ///   * Slot must be active; no-op otherwise.
    ///   * Predictor warms up for `PHOT_PRED_WARMUP_MIN` calls; during warmup
    ///     `err=0` and `precision=0` to prevent fake-confidence reads.
    ///   * After warmup: `predicted` = previous AR-1 state, `err` = residual,
    ///     `precision` = EMA of squared err (rises with surprise).
    ///   * Does NOT touch claim/scoring or any other modality's fields.
    pub fn update_slot_phot_with_predictor(&self, slot_id: usize, t_us: u64, observed: &[f64]) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if n == 0 {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        if s.phot_pred.is_none() {
            s.phot_pred = Some(PhotSlotPredictor::new(n));
        }
        let out = s.phot_pred.as_mut().unwrap().update(observed);
        s.last_update_t_us = t_us;
        s.source_modality |= MOD_PHOT;
        s.phot = Some(BandFamilyState {
            n_bands: n as u16,
            observed: observed.to_vec(),
            predicted: out.predicted,
            err: out.err,
            precision: out.precision,
        });
    }

    /// Write MECH band data into slot's `mech` sub-state. Placeholder path.
    pub fn write_slot_mech(
        &self,
        slot_id: usize,
        t_us: u64,
        observed: &[f64],
        predicted: &[f64],
        err: &[f64],
        precision: &[f64],
    ) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let n = observed.len();
        if predicted.len() != n || err.len() != n || precision.len() != n {
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
            precision: precision.to_vec(),
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
        s.last_update_t_us = t_us;
        s.source_modality |= modality_bit;
        s.spatial = Some(SpatialState {
            pos_x: clamp01(pos_x),
            pos_y: clamp01(pos_y),
            span_x: clamp_span(span_x),
            span_y: clamp_span(span_y),
            precision: precision.max(0.0),
            last_update_t_us: t_us,
            valid: true,
        });
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
            fusion: s.fusion,
            high_layer: s.high_layer,
            attention_share: s.attention_share,
            precision_bias: s.precision_bias,
        })
    }

    pub fn read_language_binding(&self) -> LanguageBindingState {
        self.language_binding.read().unwrap().clone()
    }

    pub fn push_language_binding_trace(&self, trace: BindingTrace) {
        self.language_binding.write().unwrap().push_trace(trace);
    }

    pub fn apply_language_binding_event(&self, event: BindingEvent) {
        self.language_binding
            .write()
            .unwrap()
            .apply_binding_event(event);
    }

    pub fn apply_binding_event_from_adapter(&self, event: BindingEvent) {
        self.apply_language_binding_event(event);
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
                    attn.slot_biases[slot_id] = (attn.slot_biases[slot_id] + bias)
                        .min(ATTN_PRECISION_BIAS_MAX);
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
        let vib_prec = mean_band_precision(s.vib.as_ref());
        let phot_prec = mean_band_precision(s.phot.as_ref());
        let vib_surprise_val = mean_abs_band_err(s.vib.as_ref());
        let phot_surprise_val = mean_abs_band_err(s.phot.as_ref());

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
        let fused_precision = if total_w > 1e-12 {
            (vib_weight * vib_prec + phot_weight * phot_prec) / total_w
        } else {
            0.0
        };
        let fused_surprise = if total_w > 1e-12 {
            (vib_weight * vib_surprise_val + phot_weight * phot_surprise_val) / total_w
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
            fused_precision,
            fused_surprise,
            is_cross_modal,
        });
    }

    /// Phase 5B: recompute the high-layer subtree for an active slot.
    ///
    /// Must be called AFTER `refresh_slot_fusion` so the fusion subtree is
    /// current. The method:
    ///   1. Reads `slot.fusion` — no-op if fusion is None (e.g. slot not yet
    ///      fused) or slot is inactive.
    ///   2. Initialises `slot.hl_pred` on first call.
    ///   3. Feeds the three fusion scalars (confidence, alignment, precision)
    ///      into the AR-1 predictor.
    ///   4. Writes the predictor output to `slot.high_layer`.
    ///
    /// Rules:
    ///   * Does NOT touch claim, scoring, spatial, band families, or adapters.
    ///   * Predictor state (`hl_pred`) is ephemeral — reset when slot clears.
    ///   * `high_layer` is observer-facing and snapshot'd.
    pub fn refresh_slot_high_layer(&self, slot_id: usize, now_us: u64) {
        if slot_id >= N_OBJECT_SLOTS {
            return;
        }
        let mut s = self.slots[slot_id].write().unwrap();
        if !s.active {
            return;
        }
        let Some(fusion) = s.fusion else {
            return;
        };
        if s.hl_pred.is_none() {
            s.hl_pred = Some(ObjectHighLayerPredictor::new());
        }
        let out = s.hl_pred.as_mut().unwrap().update(
            fusion.cross_modal_confidence,
            fusion.temporal_alignment_score,
            fusion.fused_precision,
            now_us,
        );
        s.high_layer = Some(out);
    }

    /// Phase 5C: recompute global attention allocation across all active slots.
    ///
    /// Reads each slot's `high_layer.attention_bias` and
    /// `fusion.cross_modal_confidence`, plus a recency factor from
    /// `fusion.last_fused_t_us`. Produces normalised attention shares and
    /// writes them back as `attention_share` and `precision_bias` (suggestion
    /// values only — these do NOT feed back into the fusion formula).
    ///
    /// Rules:
    ///   * Two-pass: read pass collects data (all read locks released before
    ///     write pass). Prevents lock-ordering deadlocks.
    ///   * Degenerate case (all raw scores < 1e-15): equal distribution across
    ///     active slots.
    ///   * Does NOT touch claim/scoring, band families, or fusion formulas.
    pub fn refresh_global_attention(&self, now_us: u64) {
        // First pass: collect (attention_bias, confidence, last_fused_t_us).
        let mut info = [None::<(f64, f64, u64)>; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            let s = self.slots[i].read().unwrap();
            if s.active {
                if let (Some(hl), Some(fusion)) = (s.high_layer, s.fusion) {
                    info[i] = Some((
                        hl.attention_bias,
                        fusion.cross_modal_confidence,
                        fusion.last_fused_t_us,
                    ));
                }
            }
        }

        // Compute raw scores: attention_bias * confidence * recency.
        let mut raw = [0.0f64; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            if let Some((bias, conf, last_t)) = info[i] {
                let dt = now_us.saturating_sub(last_t);
                let recency = (1.0 - dt as f64 / SLOT_IDLE_US as f64).clamp(0.0, 1.0);
                raw[i] = bias * conf * recency;
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
    /// that also have both `high_layer` and `fusion` populated.
    ///
    /// Per directed edge (src→dst), three EMA scores are updated:
    ///   - `co_change_score`   — `1 - |bias_src - bias_dst|`
    ///   - `lead_lag_score`    — `(t_dst - t_src) / CROSS_MODAL_WINDOW_US`
    ///   - `predictive_score`  — `clamp01(err_src * err_dst * max(0, lag_obs))`
    /// All decay toward 0 when either slot is inactive or unattended.
    ///
    /// Rules:
    ///   * Two-pass: read pass first (all locks released), then write pass.
    ///   * Does NOT touch claim/scoring, band families, fusion, or adapters.
    ///   * `precision_bias` / `attention_share` are never written here.
    pub fn refresh_global_relations(&self, now_us: u64) {
        // First pass: collect per-slot data (bias, err, last_fused_t_us, share).
        let mut slot_data = [None::<(f64, f64, u64, f64)>; N_OBJECT_SLOTS];
        for i in 0..N_OBJECT_SLOTS {
            let s = self.slots[i].read().unwrap();
            if s.active {
                if let (Some(hl), Some(fusion)) = (s.high_layer, s.fusion) {
                    slot_data[i] = Some((
                        hl.attention_bias,
                        hl.object_err.min(1.0),
                        fusion.last_fused_t_us,
                        s.attention_share,
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
                let Some((s_bias, s_err, s_t, s_share)) = slot_data[src] else {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
                    continue;
                };
                let Some((d_bias, d_err, d_t, d_share)) = slot_data[dst] else {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
                    continue;
                };

                if s_share <= 1e-6 || d_share <= 1e-6 {
                    edge.active = false;
                    edge.co_change_score = decay(edge.co_change_score);
                    edge.lead_lag_score = decay(edge.lead_lag_score);
                    edge.predictive_score = decay(edge.predictive_score);
                    edge.relation_confidence = decay(edge.relation_confidence);
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
    ///   3. Writes prediction output (predicted_*, relation_err, relation_precision)
    ///      back into `GlobalRelationState.edges[i].pred`.
    ///
    /// Inactive edges reset the predictor bank entry to Default so the next
    /// time the edge activates it starts a fresh warmup window.
    ///
    /// Rules:
    ///   * Takes write locks on both `global_relations` and `relation_pred_bank`.
    ///     These are different Viewport fields — no deadlock risk.
    ///   * During warmup (`warmup < REL_PRED_WARMUP_MIN`): `relation_err = 0`,
    ///     `relation_precision = 0` — no fake high confidence.
    ///   * Does NOT touch claim/scoring, band families, fusion, adapters, or
    ///     the attention allocator.
    pub fn refresh_global_relation_predictor(&self, now_us: u64) {
        let mut gr = self.global_relations.write().unwrap();
        let mut bank = self.relation_pred_bank.write().unwrap();

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

            // Update EMA of squared error (feeds relation_precision).
            ps.ema_mse =
                (1.0 - REL_PRED_MSE_ALPHA) * ps.ema_mse + REL_PRED_MSE_ALPHA * rms_err * rms_err;

            ps.warmup = ps.warmup.saturating_add(1);

            // Write prediction output.
            edge.pred.predicted_co_change = ps.ema_co;
            edge.pred.predicted_lead_lag = ps.ema_lag;
            edge.pred.predicted_predictive = ps.ema_pred;
            edge.pred.predicted_confidence = ps.ema_conf;
            edge.pred.last_predict_t_us = now_us;

            if ps.warmup < REL_PRED_WARMUP_MIN {
                edge.pred.relation_err = 0.0;
                edge.pred.relation_precision = 0.0;
            } else {
                edge.pred.relation_err = rms_err;
                edge.pred.relation_precision = ps.ema_mse.min(REL_PRED_PRECISION_MAX);
            }
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

    // refresh_semantic_readiness / read_semantic_readiness /
    // refresh_symbolizer / read_symbolizer were removed by cleanup spec
    // 2026-04-20. They are not part of the substrate API.

    pub fn push_observation(&self, axis: usize, t_us: u64, value: f64) {
        let mut w = self.observed[axis].write().unwrap();
        w.push(Sample {
            t_us,
            value,
            valid: true,
        });
        self.source_writes.fetch_add(1, Ordering::Relaxed);
        self.last_obs_t_us.store(t_us, Ordering::Relaxed);
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

    pub fn update_predictor(&self, axis: usize, a: f64, b: f64, last_err: f64) {
        let mut ps = self.predictors[axis].write().unwrap();
        ps.a = a;
        ps.b = b;
        ps.last_err = last_err;
        ps.total_updates = ps.total_updates.wrapping_add(1);
        let alpha = 0.02_f64;
        ps.mean_sq_err = (1.0 - alpha) * ps.mean_sq_err + alpha * last_err * last_err;
    }

    pub fn set_precision(&self, axis: usize, p: f64) {
        let mut ps = self.predictors[axis].write().unwrap();
        ps.precision = p;
    }

    pub fn read_predictor(&self, axis: usize) -> (f64, f64, f64, f64, f64, u64) {
        let ps = self.predictors[axis].read().unwrap();
        (
            ps.a,
            ps.b,
            ps.precision,
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
    /// Reads per-slot high_layer.err, computes err-weighted target shares,
    /// and EMA-shifts attention shares toward those targets. High-err slots
    /// gain attention; low-err slots lose attention.
    ///
    /// `ATTN_ERR_REBALANCE_ALPHA` is an engineering approximation — not a
    /// theoretical resource law. Should be revised as exploration history
    /// accumulates.
    pub fn tick_err_driven_attention(&self, _now_us: u64) {
        // Collect per-slot err from active slots.
        let mut errs: Vec<(usize, f64)> = Vec::new();
        for (i, slot_lock) in self.slots.iter().enumerate() {
            let slot = slot_lock.read().unwrap();
            if slot.active {
                let err = slot
                    .high_layer
                    .as_ref()
                    .map(|hl| hl.object_err)
                    .unwrap_or(0.0);
                errs.push((i, err));
            }
        }

        if errs.is_empty() {
            return;
        }

        let total_err: f64 = errs.iter().map(|(_, e)| e).sum();
        if total_err < 1e-9 {
            return;
        }

        let mut attn = self.global_attention.write().unwrap();
        for (slot_id, err) in errs.iter() {
            let target_share = err / total_err;
            attn.attention_shares[*slot_id] =
                (1.0 - ATTN_ERR_REBALANCE_ALPHA) * attn.attention_shares[*slot_id]
                    + ATTN_ERR_REBALANCE_ALPHA * target_share;
        }
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

        // Inject replay signal as a small attention perturbation.
        let replay_novelty = entry.surprise_load.clamp(0.0, 1.0);
        let mut attn = self.global_attention.write().unwrap();
        attn.replay_intensity = (1.0 - REPLAY_ALPHA) * attn.replay_intensity
            + REPLAY_ALPHA * replay_novelty;

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
}
