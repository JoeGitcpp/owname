pub mod viewport;
pub mod clock;
pub mod predictor;
pub mod source;
pub mod snapshot;
pub mod language_binding;
pub mod vib;
pub mod highlevel;
pub mod adapter;
pub mod synthetic_vib;

pub const HISTORY_CAP: usize = 1024;

pub const VIB_BANDS: usize = 4;
pub const AXIS_COUNT: usize = VIB_BANDS;
pub const AXIS_VIB: usize = 0;
pub const AXIS_SYNTH: usize = 0; // retained for backward compat

pub const VIB_BAND_CUTOFFS: [f64; 3] = [500.0, 2000.0, 5000.0];

// --- Phase 2.5: object layer contracts ---
pub const N_OBJECT_SLOTS: usize = 4;
pub const SLOT_IDLE_US: u64 = 200_000; // 200 ms

// Modality bitmap codes (numeric only; never strings inside AI body).
pub const MOD_VIB: u8 = 0b0000_0001;
pub const MOD_PHOT: u8 = 0b0000_0010;
pub const MOD_MECH: u8 = 0b0000_0100;

// --- Phase 3C: cross-modal slot reuse window ---
//
// Two adapters may co-inhabit the same ObjectSlot iff their evidence arrives
// within this window. Tighter than SLOT_IDLE_US so same-modality reuse always
// wins over cross-modality when both are in-range.
pub const CROSS_MODAL_WINDOW_US: u64 = 60_000; // 60 ms

/// When both the existing slot and the incoming evidence carry a spatial
/// estimate, their centroids must be no further apart than this (L2 in the
/// normalized [0,1]^2 plane) for cross-modal reuse. Phase 3C uses this as a
/// guardrail, not as a binding proof.
pub const CROSS_MODAL_SPATIAL_DIST: f64 = 0.30;

/// Maximum spatial distance (L2 in normalized [0,1]^2) between an active
/// same-modality slot and incoming evidence for the slot to be eligible for
/// reuse. Slots farther apart than this are treated as distinct objects.
/// Phase 4D: enforces multi-region separation within a single frame.
pub const SAME_MODAL_SPATIAL_DIST: f64 = 0.45;

// --- Phase 3D: evidence scoring / best-match claim constants ---
//
// The viewport — not any adapter — scores every candidate ObjectSlot for an
// incoming EventEvidence using the constants below. All values are numeric
// and deterministic. No semantic labels, no identity tracking. See
// `Viewport::claim_slot_for_event` for the decision rule.

/// Weight on the time-proximity component of the claim score.
pub const CLAIM_W_TIME: f64 = 0.40;
/// Weight on the spatial-proximity component of the claim score.
pub const CLAIM_W_SPATIAL: f64 = 0.25;
/// Weight on the modality-continuation component of the claim score.
pub const CLAIM_W_MODALITY: f64 = 0.20;
/// Weight on the event-energy consistency component of the claim score.
pub const CLAIM_W_ENERGY: f64 = 0.10;
/// Weight on the surprise consistency component of the claim score.
pub const CLAIM_W_SURPRISE: f64 = 0.05;

/// Minimum total score required to reuse an existing slot instead of
/// allocating a new one. Below this, viewport allocates.
pub const CLAIM_REUSE_MIN_SCORE: f64 = 0.50;

/// Modality continuation: same-modality candidate score multiplier.
pub const CLAIM_SAME_MODALITY_BONUS: f64 = 1.0;
/// Modality continuation: cross-modality candidate score multiplier.
pub const CLAIM_CROSS_MODALITY_BASE: f64 = 0.6;

/// Log-scale band (natural log) over which event_energy is considered
/// "consistent" between a slot's ambient energy and the incoming evidence.
pub const CLAIM_LOG_ENERGY_BAND: f64 = 6.0;
/// Linear band over which surprise is considered "consistent".
pub const CLAIM_SURPRISE_BAND: f64 = 1.0;

/// Numeric kind codes for LastClaimDebug (observer-facing, numeric-only).
pub const CLAIM_KIND_DROPPED: u8 = 0;
pub const CLAIM_KIND_REUSED: u8 = 1;
pub const CLAIM_KIND_ALLOCATED: u8 = 2;

// --- Phase 3A.1: shared substrate thresholds ---
//
// Both the viewport's first-class health state and any adapter's event
// nomination derive from this common rule family. Adapters must not invent
// independent magic numbers for "this is an event"; they must compare their
// smoothed energy against `EVENT_NOMINATION_THRESHOLD`, which is chosen to be
// the same order of magnitude as `QUIET_ENERGY_THRESHOLD` so that a signal
// quiet enough to keep health in QUIET cannot simultaneously fire nomination.

/// No observations arrived within this many microseconds => QUIET.
pub const QUIET_WINDOW_US: u64 = 200_000; // 200 ms

/// Per-band observed magnitude below this => QUIET.
pub const QUIET_ENERGY_THRESHOLD: f64 = 0.002;

/// Smoothed band-energy (sum of squared magnitudes) above this => event
/// candidate. Derived from `QUIET_ENERGY_THRESHOLD` so a single quiet-scale
/// band cannot by itself cross into event territory.
pub const EVENT_NOMINATION_THRESHOLD: f64 = QUIET_ENERGY_THRESHOLD * QUIET_ENERGY_THRESHOLD;

// Numeric-only health state. Observers render the number; they never
// recompute thresholds.
pub const HEALTH_FAULT: u8 = 0;
pub const HEALTH_QUIET: u8 = 1;
pub const HEALTH_OK: u8 = 2;
/// PHOT-only: provider healthy but no new frame for > PHOT_STALE_WINDOW_US.
pub const HEALTH_STALE: u8 = 3;

// --- Phase 3B: synthetic PHOT module wiring ---
pub mod phot;
pub const PHOT_BANDS: usize = 4;

// --- Phase 4A: real PHOT ingress path ---
pub mod phot_camera;

// --- Phase 4B: real camera binding + PHOT staleness ---
/// Provider healthy but no new frame within this window => STALE.
/// At 30 Hz a frame arrives every ~33 ms; 500 ms ≈ 15 missed frames.
pub const PHOT_STALE_WINDOW_US: u64 = 500_000;

// --- Phase 4C: slot-local PHOT predictor ---
pub mod phot_predictor;

/// Minimum observations before the PHOT slot predictor emits real predictions.
/// Warm-up calls seed the smoother and output err=0 / precision=0.
pub const PHOT_PRED_WARMUP_MIN: u32 = 2;

/// AR-1 exponential-smoothing alpha for PHOT band predictions.
/// Higher → tracks faster, noisier; lower → smoother, slower to converge.
pub const PHOT_PRED_ALPHA: f64 = 0.3;

/// EMA alpha for PHOT per-band precision update (= EMA of squared err).
pub const PHOT_PRED_PRECISION_ALPHA: f64 = 0.1;

/// Lower bound for PHOT band precision.
pub const PHOT_PRED_PRECISION_MIN: f64 = 0.0;

/// Upper bound for PHOT band precision.
pub const PHOT_PRED_PRECISION_MAX: f64 = 100.0;

// --- Phase 4D: multi-event PHOT ---

/// Maximum number of low-level PHOT events emitted per camera frame.
pub const PHOT_MAX_EVENTS_PER_FRAME: usize = 4;

/// Minimum per-pixel motion magnitude to include a pixel in region analysis.
pub const PHOT_REGION_THRESHOLD: f64 = 0.05;

/// Minimum number of above-threshold pixels for a region to be emitted.
pub const PHOT_REGION_MIN_PIXELS: usize = 6;

// --- Phase 5B: Object-level high-layer predictor ---
pub mod object_high_layer;

/// AR-1 EMA alpha for high-layer feature smoothers (confidence, alignment, precision).
pub const OBJECT_HL_ALPHA: f64 = 0.2;

/// EMA alpha for err² used to compute attention_bias. Slower than ALPHA so
/// attention_bias has a memory effect and doesn't bounce on single-tick spikes.
pub const OBJECT_HL_ERR_ALPHA: f64 = 0.1;

/// Warm-up ticks before the high-layer predictor emits real predictions.
/// During warm-up, object_err = 0 and attention_bias = 0.5 (neutral).
pub const OBJECT_HL_WARMUP_MIN: u32 = 2;

pub const SNAPSHOT_MAGIC: u64 = 0xB10B10_0000_0001;
// v24: add language-binding trace snapshot state without reviving naming
// gates or concept allocators.
pub const SNAPSHOT_VERSION: u64 = 24;

/// Maximum precision_bias suggestion value (Phase 5C). precision_bias is a
/// purely observable suggestion and does NOT feed back into fusion formulas.
pub const ATTN_PRECISION_BIAS_MAX: f64 = 2.0;

// --- Phase 5D: Cross-object temporal relation substrate ---

/// Number of directed relation edges in GlobalRelationState.
/// Covers every ordered (src, dst) pair among N_OBJECT_SLOTS, src ≠ dst.
pub const N_REL_EDGES: usize = N_OBJECT_SLOTS * (N_OBJECT_SLOTS - 1);

/// EMA alpha for all relation edge scores. Slower than OBJECT_HL_ALPHA so
/// relation scores accumulate evidence over multiple seconds.
pub const REL_ALPHA: f64 = 0.15;

// --- Phase 5E: Relation-level predictor ---

/// AR-1 EMA alpha for the relation edge predictor. Faster than REL_ALPHA so
/// the predictor tracks score changes more responsively than the scores themselves.
pub const REL_PRED_ALPHA: f64 = 0.3;

/// EMA alpha for relation_precision (= EMA of relation_err²). Slower than
/// REL_PRED_ALPHA so precision accumulates evidence over multiple cycles.
pub const REL_PRED_MSE_ALPHA: f64 = 0.1;

/// Minimum predictor cycles before a relation edge emits non-zero relation_err
/// and relation_precision. Prevents new edges from faking high confidence.
pub const REL_PRED_WARMUP_MIN: u32 = 2;

/// Upper bound for relation_precision (= EMA of relation_err²). The inputs
/// are in [-1,1] so the max RMS² ≈ 1.75; 2.0 gives headroom.
pub const REL_PRED_PRECISION_MAX: f64 = 2.0;

// --- Phase 5F: Cross-object reasoning / routing substrate ---

/// Number of top-priority edge slots in GlobalReasoningState.routing_focus_edge_ids.
pub const N_ROUTING_FOCUS_EDGES: usize = 8;

/// Number of top-priority slot slots in GlobalReasoningState.routing_focus_slot_ids.
pub const N_ROUTING_FOCUS_SLOTS: usize = 4;

// --- Phase 5G: Global reasoning temporal memory substrate ---

/// Ring buffer capacity for reasoning history entries.
/// Stores recent N cycles of compact reasoning signatures.
pub const REASONING_HISTORY_RING_CAP: usize = 32;

/// EMA alpha for surprise_load baseline used in novelty calculation.
/// Slower than reasoning refresh so baseline tracks medium-term average.
pub const REASONING_HISTORY_ALPHA: f64 = 0.05;

/// Minimum cycles of high-intensity reasoning to count as "persistent pattern".
pub const PERSISTENCE_MIN_CYCLES: u32 = 3;

/// Maximum cycles considered for persistence scoring (caps at 1.0).
pub const PERSISTENCE_MAX_CYCLES: u32 = 20;

/// Threshold for "high-intensity" reasoning (surprise_load or instability above this).
pub const REASONING_INTENSITY_THRESHOLD: f64 = 0.1;

// Phase 5H Semantic Readiness Contract and Phase 6A Symbolizer / Concept-ID
// Layer were removed by cleanup spec 2026-04-20. Reason: they made naming /
// labeling a prerequisite resource gate over substrate, and stamped objects
// and edges with discrete concept ids — both forbidden by language-system
// axioms (axioms 1, 3, 5, 8). The substrate they consumed (fusion,
// high_layer, attention, relations, reasoning, memory) remains intact.
