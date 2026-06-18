pub mod action_consequence;
pub mod net_adapter;
pub mod action_output;
pub mod active_reasoning;
pub mod audio_adapter;
pub mod audio_raw;
pub mod active_surface;
pub mod adapter;
pub mod adaptive_stability;
pub mod substrate;
// Cutover Phase 3: familiar_objects + object_high_layer + SlotCrossLayer
// removed — perturbation_graph (Phase 3) replaces the categorical-object slot
// derivation of attachment/presence + per-slot AR-1 high layer +
// per-slot cross-layer predictor + recursive tier residual tower.
// See perturbation_graph.rs for the new architecture.
pub mod vital_core;
pub mod vital_env;
pub mod self_model;
pub mod dual_drive;
pub mod vocal_drive_field;
pub mod allostatic_stability;
pub mod auditory_visual;
pub mod clock;
pub mod code_ingress;
pub mod code_ingress_adapter;
pub mod capability_curriculum_governor;
pub mod curriculum;
pub mod curriculum_governor;
pub mod curriculum_material;
pub mod evidence_tension;
pub mod evolution_priors;
pub mod five_phase_life;
pub mod framed_byte_carrier;
pub mod framed_byte_egress;
pub mod highlevel;
pub mod language_binding;
pub mod learned_bias;
pub mod life_closure_producers;
pub mod live_byte_carrier;
pub mod live_byte_closure;
pub mod live_byte_loopback;
pub mod live_human_nursery;
pub mod live_io_evidence;
pub mod mark_disturbance;
pub mod post_effect;
pub mod prediction_pressure_field;
pub mod predictor;
pub mod probe_law;
pub mod self_development;
pub mod snapshot;
pub mod source;
pub mod substrate_pressure;
pub mod surface_history;
pub mod surface_projection;
pub mod surface_render;
pub mod synthetic_vib;
pub mod training_curve;
pub mod vib;
pub mod viewport;
pub mod vocal_adapter;
pub mod vocal_imitation;
pub mod vocal_memory;
pub mod vocal_organ;
pub mod vocal_reflex;
pub mod vocal_self_analysis;
pub mod vocal_servo;
pub mod vocal_synthesis;
pub mod vocal_target;
pub mod vocal_target_recall;
pub mod vocal_sequence;
pub mod perturbation_graph;
pub mod vision_tier0;
pub mod phoneme_memory;
pub mod vpc;
pub mod vpc_cross_layer;

// Phase A: virtual neck + efference copy (Phase 2: depth is per-Gaussian — see phot_gaussian)
pub mod efference_copy;
pub mod virtual_neck;

pub const HISTORY_CAP: usize = 1024;

pub const VIB_BANDS: usize = 16;

// --- Unified perception axes (2026-06-01 redesign) ---
// Vision is no longer a per-slot shadow: it becomes shared perception axes that
// flow through the SAME substrate (observed/predicted/err/predictor/prior) as
// hearing, into the SAME abstraction engine and memory. Axes are PHYSICAL
// quantities over a coarse spatial grid — NOT "face/object/use" features.
// Shape/position/identity emerge later from inter-region relations; this layer
// presets no use.
//
// Coarse spatial grid: the frame is split into VIS_REGIONS cells. Each cell
// carries VIS_AXES_PER_REGION physical-quantity axes: brightness, R, G, B,
// saturation, motion, temporal_flux, edge_energy. Region layout is row-major
// (VIS_GRID_ROWS × VIS_GRID_COLS).
pub const VIS_GRID_ROWS: usize = 4;
pub const VIS_GRID_COLS: usize = 4;
pub const VIS_REGIONS: usize = VIS_GRID_ROWS * VIS_GRID_COLS; // 16
pub const VIS_AXES_PER_REGION: usize = 8; // brightness, R, G, B, saturation, motion, temporal_flux, edge_energy
pub const VIS_AXES: usize = VIS_REGIONS * VIS_AXES_PER_REGION; // 128

/// Sub-axis index constants for visual regions (use these instead of bare integers).
pub mod vis_sub {
    pub const BRIGHTNESS:    usize = 0;
    pub const R:             usize = 1;
    pub const G:             usize = 2;
    pub const B:             usize = 3;
    pub const SATURATION:    usize = 4;
    pub const MOTION:        usize = 5;
    pub const TEMPORAL_FLUX: usize = 6;
    pub const EDGE_ENERGY:   usize = 7;
}

// Axis layout in the shared coordinate system:
//   [0 .. VIB_BANDS)                         = hearing (frequency bands)
//   [AXIS_VIS_BASE .. +VIS_AXES)              = vision (region × physical quantity)
//   [AXIS_INTERO_BASE .. +INTERO_AXES)        = interoception (body-internal scalars)
//   [AXIS_CODE_BASE .. +CODE_AXES)            = code/digital objects (STRUCT/ORDER subset)
//
// Interoceptive axes (2026-06-03 redesign — Stage 0):
//   Internal body scalars are mirrored onto the shared substrate so they are
//   predicted, produce err, enter relations, and feed the abstraction tower —
//   EXACTLY as external perception does. This is the substrate-unification fix:
//   internal state stops being isolated boxes and becomes part of the same
//   prediction/error/relation fabric as vision and hearing.
//
//   Each axis name is a numeric engineering locator (like VIS_SUB_BRIGHTNESS),
//   NOT a semantic label. The substrate doesn't "know" reserve is vital or that
//   gap_pressure is cognitive — it just predicts a scalar curve. Feelings
//   (surprise, longing, calm) emerge from prediction error patterns over these
//   axes, not from labels. No absolute positive/negative values are pre-assigned
//   (推论5: 无绝对正负判断, 感受涌现自预测差值).
pub const AXIS_VIS_BASE: usize = VIB_BANDS; // 16
pub const AXIS_INTERO_BASE: usize = VIB_BANDS + VIS_AXES; // 144

// Ten interoceptive axes — the minimal set that covers vital/self/familiar/
// reasoning dynamics. All are scalars in [0,1] already maintained by existing
// box dynamics; Stage 0 only MIRRORS them here (zero behavior change).
pub const INTERO_RESERVE: usize = AXIS_INTERO_BASE + 0; // survival reserve
pub const INTERO_VITAL_ANXIETY: usize = AXIS_INTERO_BASE + 1; // predicted-trajectory dread
pub const INTERO_PAIN: usize = AXIS_INTERO_BASE + 2;       // acute self-damage signal
pub const INTERO_SELF_COHERENCE: usize = AXIS_INTERO_BASE + 3; // self-model stability
pub const INTERO_AGENCY: usize = AXIS_INTERO_BASE + 4;     // sense of causal efficacy
pub const INTERO_ATTACHMENT: usize = AXIS_INTERO_BASE + 5; // strongest emergent bond
pub const INTERO_GAP_PRESSURE: usize = AXIS_INTERO_BASE + 6; // unresolved prediction gap
pub const INTERO_CLOSURE: usize = AXIS_INTERO_BASE + 7;    // gap-closing signal
pub const INTERO_LEARNING_PROGRESS: usize = AXIS_INTERO_BASE + 8; // prediction improving
pub const INTERO_PRESENCE: usize = AXIS_INTERO_BASE + 9;   // familiar-object in-presence
//   Presence axis: high when the most-familiar object is actively seen/heard;
//   low when absent. Persistent high-err on this axis (predicted-present but
//   actually absent) is the substrate form of "longing" — no label needed.

pub const INTERO_AXES: usize = 10;

// Five code/digital-object projection axes — the tier-0 STRUCT/ORDER sub-set as applied to
// digital objects.  Per design doc §4: physical and digital objects are different projections
// of the same coordinate system, not two systems.  Code objects use only the STRUCT and
// STRUCT-ORDER sub-set (no INRT/THRM/ELEC/MAGN/etc.):
//   CODE_POS             → stream-context identity, normalized [0,1]  (pos_* analog)
//   CODE_SPAN            → frame byte length, normalized to CODE_SPAN_MAX_BYTES (span_* analog)
//   CODE_STRUCT_ORDER_POS    → byte distribution skew   (struct_order_pos analog)
//   CODE_STRUCT_ORDER_ORIENT → byte transition energy   (struct_order_orient analog)
//   CODE_STRUCT_ORDER_PERIOD → sequence recurrence score (struct_order_period analog)
pub const AXIS_CODE_BASE: usize = VIB_BANDS + VIS_AXES + INTERO_AXES; // 154
pub const CODE_POS: usize             = AXIS_CODE_BASE + 0;
pub const CODE_SPAN: usize            = AXIS_CODE_BASE + 1;
pub const CODE_STRUCT_ORDER_POS: usize    = AXIS_CODE_BASE + 2;
pub const CODE_STRUCT_ORDER_ORIENT: usize = AXIS_CODE_BASE + 3;
pub const CODE_STRUCT_ORDER_PERIOD: usize = AXIS_CODE_BASE + 4;
pub const CODE_AXES: usize = 5;
/// Normalization cap for CODE_SPAN axis. Byte lengths ≥ this value saturate to 1.0.
pub const CODE_SPAN_MAX_BYTES: f64 = 65536.0;

// --- Phase 1 (keystone ①): Gaussian visual field summary axes ---
// Two scalar summaries of the sparse Gaussian field, mirrored onto the unified
// tier-0 axis stream so they are predicted, produce err, and enter the relation/
// tower fabric — exactly as every other substrate signal does.
//
//   GAUSS_RECON_MSE   → mean-squared reconstruction residual (pre-update, this
//                        tick). Falls as the field learns a stable scene; steady
//                        high value signals that the field is overwhelmed or the
//                        scene is too chaotic to fit.  This is the signal that
//                        ⑤'s encoder-layer probe reads.
//   GAUSS_ACTIVE_FRAC → active_count / GAUSS_CAP, normalized to [0,1].
//                        Tracks sparsity health: near 0 = almost nothing active
//                        (scene absent / all died), near 1 = pool nearly full.
pub const AXIS_GAUSS_BASE: usize = VIB_BANDS + VIS_AXES + INTERO_AXES + CODE_AXES; // 159
pub const GAUSS_RECON_MSE_AXIS: usize  = AXIS_GAUSS_BASE + 0;
pub const GAUSS_ACTIVE_FRAC_AXIS: usize = AXIS_GAUSS_BASE + 1;
pub const GAUSS_SUMMARY_AXES: usize = 2;

pub const AXIS_COUNT: usize = VIB_BANDS + VIS_AXES + INTERO_AXES + CODE_AXES + GAUSS_SUMMARY_AXES; // 161 = 16+128+10+5+2

pub const AXIS_VIB: usize = 0;
pub const AXIS_SYNTH: usize = 0; // retained for backward compat

/// Per-region visual sub-axis offsets (within a region's VIS_AXES_PER_REGION block).
/// Prefer the `vis_sub` module constants; these flat aliases are retained for
/// backward compatibility with existing call sites.
pub const VIS_SUB_BRIGHTNESS: usize = vis_sub::BRIGHTNESS;
pub const VIS_SUB_R: usize = vis_sub::R;
pub const VIS_SUB_G: usize = vis_sub::G;
pub const VIS_SUB_B: usize = vis_sub::B;
pub const VIS_SUB_MOTION: usize = vis_sub::MOTION;

/// Global axis index for a given visual region's sub-axis.
#[inline]
pub const fn vis_axis(region: usize, sub: usize) -> usize {
    AXIS_VIS_BASE + region * VIS_AXES_PER_REGION + sub
}

pub const VIB_BAND_CUTOFFS: [f64; 15] = [
    100.0, 200.0, 350.0, 500.0, 750.0, 1000.0, 1500.0, 2000.0,
    3000.0, 4000.0, 5000.0, 7000.0, 10000.0, 14000.0, 18000.0,
];

// --- Phase 2.5: object layer contracts ---
pub const N_OBJECT_SLOTS: usize = 8;
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

/// audio_raw ring buffer: 48,000 Hz × 60 seconds
pub const AUDIO_RAW_CAPACITY: usize = 48_000 * 60;

/// Batch write every 10ms: 48,000 × 0.01 = 480 samples
pub const AUDIO_RAW_BATCH_SAMPLES: usize = 480;
pub const AUDIO_RAW_BATCH_INTERVAL_US: u64 = 10_000;

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

// --- Phase 1 (keystone ①): sparse anisotropic Gaussian visual field ---
pub mod phot_gaussian;

// --- Visual impression / temporal-context layer (sister to vocal_sequence) ---
pub mod phot_sequence;

// --- Language binding runtime maintenance ---
pub const BINDING_REFRESH_HZ: u32 = 5;
/// A trace with no updates for this long is considered stale for runtime
/// maintenance and receives one decay step per refresh tick.
pub const BINDING_MAINTENANCE_IDLE_US: u64 = 2_000_000;
/// Per-tick decay amount applied to stale binding strength.
pub const BINDING_MAINTENANCE_DECAY_STEP: f64 = 0.05;

// --- Phase 4B: real camera binding + PHOT staleness ---
/// Provider healthy but no new frame within this window => STALE.
/// At 30 Hz a frame arrives every ~33 ms; 500 ms ≈ 15 missed frames.
pub const PHOT_STALE_WINDOW_US: u64 = 500_000;

/// Engineering approximation of the maximum delay between a write epoch's
/// `start_t_us` and a consequence window's `start_t_us` for the live
/// temporal auto-pairing helper to be eligible to form a link.
///
/// At 30 Hz a PHOT consequence window typically forms within a few hundred
/// milliseconds of a preceding write event; 2 s gives comfortable headroom
/// for slower physical feedback paths.
///
/// This is a substrate-level engineering knob, NOT a fixed theoretical
/// consequence time window.  The actual temporal structure of action-consequence
/// coupling should be formed and revised through exploration history once that
/// substrate is in place.
pub const TEMPORAL_PAIR_MAX_DELAY_US: u64 = 2_000_000; // 2 s

/// Engineering cap on write_epoch_history Vec length.
///
/// Substrate knob — not a theoretical learning window.  The actual number of
/// live epochs needed for nearest-legal-predecessor selection is determined by
/// TEMPORAL_PAIR_MAX_DELAY_US × write-event frequency; 128 gives comfortable
/// headroom at current rates.  Adjust via exploration history as write patterns
/// become known.
pub const WRITE_EPOCH_HISTORY_CAP: usize = 128;

/// Engineering cap on consequence windows Vec length.
///
/// Substrate knob — not a theoretical consequence count.  At 30 Hz PHOT with
/// PHOT_CONSEQUENCE_QUIESCENCE_US ≈ 300 ms, at most a few windows open per
/// second; 128 gives many seconds of headroom.  Adjust via exploration history.
pub const CONSEQUENCE_WINDOW_CAP: usize = 128;

/// Engineering cap on consequence pairs Vec length.
///
/// Substrate knob — not a theoretical pair count.  Larger than the window cap
/// because multiple epochs may pair with the same window, and history lookback
/// wants more pair records than active windows.
pub const PAIR_HISTORY_CAP: usize = 256;

/// Engineering approximation of the quiescence gap used by the PHOT ingress
/// accumulation wiring to close an Open consequence window when no motion
/// has been observed for this many microseconds.
///
/// At 30 Hz, 300 ms ≈ 9 consecutive no-motion frames.
///
/// This is a substrate-level engineering knob, NOT a fixed theoretical
/// consequence boundary.  The actual quiescence structure should be formed
/// and revised through exploration history once that substrate is in place.
pub const PHOT_CONSEQUENCE_QUIESCENCE_US: u64 = 300_000;

/// Engineering approximation: VIB event energy above this triggers
/// consequence window accumulation. Substrate knob, not theoretical bound.
pub const INGRESS_ACCUMULATION_THRESHOLD: f64 = 0.01;

/// Engineering approximation: VIB ambient energy must exceed this before it can
/// refresh the short-lived vocal target contour. Substrate knob, not a
/// theoretical sound boundary.
pub const VOCAL_TARGET_INGRESS_MIN_ENERGY: f64 = INGRESS_ACCUMULATION_THRESHOLD;

/// Engineering caution window: recent body writes can couple into VIB ingress,
/// so those frames must not overwrite `vocal_target` as if they were external
/// target contours. This is not source identity or ownership.
pub const VOCAL_TARGET_WRITE_COUPLED_GUARD_US: u64 = 300_000;

// --- Phase 4C: slot-local PHOT predictor ---
pub mod phot_predictor;

/// Minimum observations before the PHOT slot predictor emits real predictions.
/// Warm-up calls seed the smoother and output err=0 / surprise=0.
pub const PHOT_PRED_WARMUP_MIN: u32 = 2;

/// AR-1 exponential-smoothing alpha for PHOT band predictions.
/// Higher → tracks faster, noisier; lower → smoother, slower to converge.
pub const PHOT_PRED_ALPHA: f64 = 0.3;

/// EMA alpha for PHOT per-band surprise update (= EMA of squared err).
pub const PHOT_PRED_SURPRISE_ALPHA: f64 = 0.05;

/// Lower bound for PHOT band surprise.
pub const PHOT_PRED_SURPRISE_MIN: f64 = 0.0;

/// Upper bound for PHOT band surprise.
pub const PHOT_PRED_SURPRISE_MAX: f64 = 1.0;

// --- Phase 4D: multi-event PHOT ---

/// Maximum number of low-level PHOT events emitted per camera frame.
pub const PHOT_MAX_EVENTS_PER_FRAME: usize = 4;

/// Minimum per-pixel motion magnitude to include a pixel in region analysis.
pub const PHOT_REGION_THRESHOLD: f64 = 0.05;

/// Minimum number of above-threshold pixels for a region to be emitted.
pub const PHOT_REGION_MIN_PIXELS: usize = 6;

// (Cutover Phase 3) object_high_layer + OBJECT_HL_* constants removed —
// the per-slot AR-1 predictor on top of slot fusion features fed only the
// per-slot SlotCrossLayerPredictor (also removed). perturbation_graph.rs
// is the new substrate for object-level evidence.

// --- Phase 6B: probe-law substrate ---

/// EMA alpha for probe intensity (tracks reasoning novelty).
pub const PROBE_INTENSITY_ALPHA: f64 = 0.20;

/// EMA alpha for probe suppression (slower, acts as adaptive baseline).
pub const PROBE_SUPPRESSION_ALPHA: f64 = 0.05;

/// Short history ring length for probe intensity values.
pub const PROBE_HISTORY_CAP: usize = 8;

// --- v37: computer-native byte disturbance ingress traces ---
pub const CODE_INGRESS_FRAME_HISTORY_CAP: usize = 256;
pub const CODE_INGRESS_PROJECTION_HISTORY_CAP: usize = 128;
pub const CODE_INGRESS_SEQUENCE_WINDOW_CAP: usize = 32;
pub const CODE_INGRESS_BYTE_SAMPLE_CAP: usize = 32;

/// Engineering cap for retained VPC predictor-family traces.
///
/// This is not a theory-level memory boundary and does not decide what the
/// body can learn. It only prevents observer evidence traces from becoming an
/// unbounded snapshot payload during long-running body sessions.
pub const VPC_PREDICTOR_TRACE_CAP: usize = 512;

/// Engineering cap for retained VPC publication traces.
///
/// Publications are numeric/time evidence for observer review. Long-run
/// learning state must live in the substrate, not in an append-only observer
/// ledger.
pub const VPC_PUBLICATION_TRACE_CAP: usize = 512;

/// Engineering cap for retained VPC open-prediction trace handles.
///
/// Recent open predictions are kept for resolution evidence; older unresolved
/// handles may be evicted as stale observer evidence.
pub const VPC_OPEN_PREDICTION_CAP: usize = 256;

/// Short history ring length for evidence-tension pressure traces.
pub const EVIDENCE_TENSION_HISTORY_CAP: usize = 16;

/// Engineering scale for mapping bounded tension [0,4] into pressure [0,1].
pub const EVIDENCE_TENSION_PRESSURE_SCALE: f64 = 4.0;

/// Engineering decay rate when new evidence tension is lower than carried load.
pub const EVIDENCE_TENSION_DECAY_ALPHA: f64 = 0.25;

/// Engineering closure rate applied from paired write/return curve support.
pub const EVIDENCE_TENSION_CLOSURE_ALPHA: f64 = 0.5;

/// Fixed observer trace capacity for active-gap numeric curve samples.
pub const ACTIVE_REASONING_GAP_TRACE_CAP: usize = 128;

pub const SNAPSHOT_MAGIC: u64 = 0xB10B10_0000_0001;
// v30: probe_law substrate added (intensity, suppression, phase, output_intensity,
// last_update_t_us, short history). Old snapshots (v29) are rejected.
// v31: curve_similarity added to ActionConsequencePair.
// v32: slot_biases added to GlobalAttentionState.
// v33: VPC contract trace substrate added (publication/open timing provenance).
// v34: VPC predictor-family lifecycle traces added to snapshot payload.
// v35: precision→surprise rename + unified [0,1] scale (PHOT [0,100]→[0,1],
//      relation [0,2]→[0,1], HighLevel [0.2,8]→[0,1]).
// v36: vocal substrate added (vocal_target, vocal_organ, vocal_imitation, vocal_memory).
// v37: code-ingress trace substrate added.
// v38: learned-bias delta summary added; base priors remain separate.
// v39: adaptive stability route-local numeric traces added.
// v40: evidence-tension pressure trace substrate added.
// v41: framed-byte egress substrate added.
// v42: action-consequence pair_seq added as a monotonic engineering cursor.
// v43: slot-local spatial worldline + relation-edge spatial projection fields.
// v44: write epoch start_magnitude added for closed action-consequence evidence.
// v45: cumulative closure counters added for long-run observer traces, split by
//      return modality.
// v46: active reasoning gap observer trace added.
// v47: self-development predictive life-loop observer trace added.
// v48: pre-language solitary growth fields added to self-development trace.
// v49: audio_raw tier-0 waveform buffer added to viewport and snapshot.
// v50: per-slot cross-layer (B1 tier-1) learned relation weights.
// v51: per-slot residual TOWER (recursive PC depth, tiers ≥2).
/// v52: interoceptive substrate unification Stage 0 — AXIS_COUNT 24→34 (10 new
/// interoceptive axes: reserve/anxiety/pain/coherence/agency/attachment/gap/
/// closure/learning_progress/presence). Per-axis snapshot blocks grow accordingly.
/// v53: CODE axis block (5 axes, AXIS_COUNT 34→39) — digital objects enter the
/// unified tier-0 axis stream via push_observation.  code_ingress frames project
/// onto CODE_POS/SPAN/STRUCT_ORDER_POS/ORIENT/PERIOD; no separate representation.
/// v54: prediction_gap_slow added to SelfDevelopmentState — two-timescale noise
/// rejection for INTERO_LEARNING_PROGRESS; probe drive rewired from raw surprise
/// to learning progress.
/// v55: Gaussian visual field (keystone ①) Phase 1 Stage B — GaussianField added
/// to Viewport; two summary axes GAUSS_RECON_MSE / GAUSS_ACTIVE_FRAC (AXIS_COUNT
/// 39→41) mirrored onto the unified tier-0 stream; adapter drives the field from
/// real camera frames; body.rs is already wired through the adapter registry.
/// v56: Maximized CPU spec — VIB_BANDS 4→16, VIS_GRID 2×2→4×4,
/// VIS_AXES_PER_REGION 5→8 (+saturation/temporal_flux/edge_energy); AXIS_COUNT
/// 41→161 (16+128+10+5+2). Old axis layout incompatible; v55 snapshots rejected.
/// v57: B1 robustness — tier-0 thread-fault axis (thread_fault: u64-bool +
/// thread_fault_count: u64) appended at the very end of the snapshot.
/// v58: A1 dual-drive confluence — observer-visible mirror of the per-tick
/// DualDriveReading (5×f64 + sacrificing u8 + 7-byte pad) appended at the very
/// end of the snapshot, after the v57 thread-fault block.
/// v59: A2 axis-level cross-layer bank (VPC mechanism 5 prior write-back leg) —
/// learned per-target source weights / bias / EMAs of the body-resident axis
/// CrossLayerPredictor bank, appended at the very end of the snapshot, after
/// the v58 dual-drive block. Older snapshots restore an empty bank.
/// v60: A3 developmental clock (Viewport::developmental_maturity, one LE f64)
/// appended after the v59 axis cross-layer bank, so the observer can watch the
/// coarse→fine precision annealing curve. Older snapshots read back 0.0.
/// v61: Phase 3 cutover: removed high_layer + slot_xlayers + slot_towers.
/// Older snapshot files at v60 are rejected (the per-slot high_layer 1-byte
/// `present` flag and the v50/v51 cross-layer blocks no longer exist in the
/// format). Independent sidecars (perturbation_graph, vocal_sequence) survive.
/// v62: Phase A1–A9 (groups, first-class relations, kind discovery, FastPath,
/// marks, vocal profile mirror, persistence v2). The snapshot.bin format
/// itself is unchanged — but the perturbation_graph.bin sidecar moved from
/// PGRA v1 to v2 in the same release, and the development-clock implications
/// of the new cognitive substrate (FastPath promotion timing especially) make
/// a fresh lineage maturity reset the cleanest path forward. Existing
/// snapshot.bin v61 files are rejected; perturbation_graph.bin v1 files keep
/// loading under the v2 codepath (groups / relations re-seed empty).
pub const SNAPSHOT_VERSION: u64 = 62;

/// Precision-war guard rule 2: hard ceiling on any prior magnitude written
/// into the per-axis prior channel ("prior 精度上限 ≈ 100"). A diverged or
/// degenerate high-layer predictor can at worst push ±this value into the low
/// layer's additive input — never an unbounded finite value.
pub const MAX_PRIOR_MAGNITUDE: f64 = 100.0;

/// Maximum precision_bias suggestion value (Phase 5C). precision_bias is a
/// purely observable suggestion and does NOT feed back into fusion formulas.
pub const ATTN_PRECISION_BIAS_MAX: f64 = 2.0;

/// Minimum binding strength to trigger attention bias injection.
pub const BINDING_RESONANCE_THRESHOLD: f64 = 0.5;

/// Alpha for binding → attention bias injection.
pub const BINDING_ATTN_ALPHA: f64 = 0.05;

/// Engineering approximation over action/consequence curve evidence.
/// High values may support a binding event; this is not a naming gate and does
/// not decide object or relation existence.
pub const BINDING_PAIR_ALIGN_MIN: f64 = 0.70;

/// Engineering approximation over action/consequence curve evidence.
/// Low values may oppose an existing binding trace; this is not a naming gate
/// and does not create an internal concept or object class.
pub const BINDING_PAIR_DEVIATE_MAX: f64 = 0.30;

// --- Developmental precision schedule (2026-06-06) ---
//
// A single developmental clock m∈[0,1] (Viewport::developmental_maturity) makes
// the system's quality thresholds developmental: loose when naive, tight when
// mature. The point is to break the bootstrap deadlock — a newborn's rough,
// low-competence output must yield SOME learning signal (else 0×x=0 forever),
// while a mature body keeps fine discrimination. These are engineering knobs
// over curve dynamics, not naming/concept gates.

/// How much one earned closure advances the developmental clock. Saturating
/// (·(1−m)) so maturation is a slow accrual that can never exceed 1. Small →
/// developmental timescale (hundreds of strong closures to mature).
pub const MATURITY_GAIN: f64 = 0.01;

/// Minimum spacing between maturity-advancing closures. The nursery produces
/// ~6 heard-self evidence closures per second; capping at 1/s stretches the
/// infant period from ~3 minutes to ~25 minutes at nursery density, making the
/// 8× EMA anneal window long enough to matter. Applies at the call-site chokepoints
/// in observe_vocal_reflex_closure and language_binding.
pub const MATURITY_ADVANCE_MIN_INTERVAL_US: u64 = 1_000_000;

/// Minimum spacing between prediction_familiarity accumulation ticks. Mirrors
/// MATURITY_ADVANCE_MIN_INTERVAL_US — prevents the dense pair path from pinning
/// familiarity to 1.0 in seconds, which would silence the endogenous babble
/// baseline before the body has learned anything.
pub const FAMILIARITY_ADVANCE_MIN_INTERVAL_US: u64 = 1_000_000;

/// Minimum spacing between vocal self-play closures, so continuous babble
/// accrues efficacy/maturity at a developmental pace rather than per-tick.
pub const VOCAL_SELF_PLAY_MIN_INTERVAL_US: u64 = 250_000;

/// Phase 1 (2026-06-12): Hz scale for the acoustic servo's two hill-climb knobs,
/// now repurposed from (pitch, resonance-on-the-1D-line) to INDEPENDENT (F1, F2)
/// corrections. The servo offset ∈ [-MAX_OFFSET, MAX_OFFSET] (≈±0.45) is scaled
/// by these to a Hz nudge applied on top of the resonance-derived formant
/// baseline — this is what lets the body leave the 1-D resonance line and reach
/// 2-D vowel targets. ±0.45·600≈±270 Hz on F1, ±0.45·1300≈±585 Hz on F2:
/// enough to cross vowel categories, bounded enough not to destabilise.
/// Atomic file write: write `<path>.tmp` then rename onto `path`. A crash mid-
/// write leaves the stale-but-valid old file, never a truncated one that
/// silently deserializes to empty (audit theme 5: persistence non-atomic). One
/// writer per path. Returns the rename error so callers can stop swallowing it.
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// `atomic_write` that REPORTS a failure to stderr instead of returning it — for
/// the body's checkpoint/shutdown persistence path, which must not fail silently
/// (audit theme 5) yet must keep human-text narration OUT of body.rs itself: the
/// active-surface purity guard forbids `eprintln!`/`println!` on the body runtime
/// path (pure numeric emergence). Centralising the one log line here satisfies
/// both — body.rs calls this, the message lives in the (unguarded) library.
pub fn atomic_write_or_warn(what: &str, path: &std::path::Path, bytes: &[u8]) {
    if let Err(e) = atomic_write(path, bytes) {
        eprintln!("[persist] {what} {} write failed: {e}", path.display());
    }
}

pub const F1_SERVO_HZ: f64 = 600.0;
pub const F2_SERVO_HZ: f64 = 1300.0;

/// Phase 1 (2026-06-12): minimum `compute_vib_self_match` to treat a VIB return
/// as self-echo and route a self-play closure. self_match is a time-since-
/// voicing heuristic over a 150ms half-window, so this threshold maps directly
/// to a latency window: 0.5 ⇒ "VIB within ≈75ms of voicing". The former 0.85
/// (≈22ms, ~1 tick at 50Hz) almost never coincided with an echo return, so
/// solitary babble produced ~0 self-play closures and could not bootstrap.
pub const VOCAL_SELF_PLAY_MATCH_MIN: f64 = 0.5;

/// EMA smoothing for vocal_efficacy. Closures arrive at the orchestrator rate
/// (tens of Hz via the action-consequence pair path), so vocal_efficacy is an
/// exponential moving average of the acoustic-fit-weighted closure level rather
/// than a ratchet — it converges toward recent competence (and falls when the
/// heard-self stops matching the target) instead of saturating to 1.0 from mere
/// high-rate voicing. Small alpha → a smoothing window of order ~10s at 30 Hz.
pub const VOCAL_EFFICACY_EMA_ALPHA: f64 = 0.003;

/// A3-family: naive multiplier for the vocal_efficacy EMA step. At m=0 each
/// evidence-carrying closure moves efficacy this many times faster — the
/// newborn escape from the vocal bootstrap deadlock (efficacy 0 → reflex gate
/// at its 0.40 ceiling → few bursts → few evidence closures → at base alpha
/// 0.003 efficacy needs hundreds of closures to move → gate never drops:
/// 0×x=0). Coarse, fast, tolerant early learning; anneals linearly to exactly
/// 1.0 at m=1 so the mature body keeps the hand-tuned base alpha and its
/// ~10s smoothing window untouched. Evidence gating is NOT relaxed: only
/// closures that carry real heard-self evidence step the EMA at all.
pub const VOCAL_EFFICACY_ALPHA_SCALE_NAIVE: f64 = 8.0;

/// Maturity schedule for the vocal_efficacy EMA step:
/// VOCAL_EFFICACY_ALPHA_SCALE_NAIVE at m=0, exactly 1.0 at m=1. Pure function
/// over the developmental clock; a continuous local scalar, never a gate.
pub fn vocal_efficacy_alpha_scale(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    VOCAL_EFFICACY_ALPHA_SCALE_NAIVE + (1.0 - VOCAL_EFFICACY_ALPHA_SCALE_NAIVE) * m
}

/// Newborn-neutral midpoint for pair-evidence classification. At maturity 0 the
/// align/deviate thresholds collapse to this midpoint (no dead zone), so a rough
/// match still yields a weak signal; they widen to the strict ALIGN_MIN /
/// DEVIATE_MAX as the clock matures (coarse→fine precision).
pub const BINDING_PAIR_NEUTRAL: f64 = 0.50;

/// Maturity-scaled lower bound for an Align classification: NEUTRAL at m=0,
/// strict BINDING_PAIR_ALIGN_MIN at m=1.
pub fn binding_align_min(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    BINDING_PAIR_NEUTRAL + (BINDING_PAIR_ALIGN_MIN - BINDING_PAIR_NEUTRAL) * m
}

/// Maturity-scaled upper bound for a Deviate classification: NEUTRAL at m=0,
/// strict BINDING_PAIR_DEVIATE_MAX at m=1.
pub fn binding_deviate_max(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    BINDING_PAIR_NEUTRAL - (BINDING_PAIR_NEUTRAL - BINDING_PAIR_DEVIATE_MAX) * m
}

/// A3: naive multiplier for the AR-2 axis predictor's effective learning rate.
/// At m=0 the per-tick Widrow-Hoff step is this many times larger — coarse,
/// fast, tolerant learning so a near-zero newborn coefficient still moves
/// (0×x=0 bootstrap escape). Anneals linearly to exactly 1.0 at m=1 so the
/// mature body keeps the existing hand-tuned base_lr untouched.
pub const AR2_LR_SCALE_NAIVE: f64 = 3.0;

/// Maturity schedule for the AR-2 effective learning rate: AR2_LR_SCALE_NAIVE
/// at m=0, exactly 1.0 at m=1 (mature behaviour identical to the pre-schedule
/// hard-coded value). Pure function over the developmental clock; a continuous
/// local scalar, never a capability gate.
pub fn ar2_lr_scale(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    AR2_LR_SCALE_NAIVE + (1.0 - AR2_LR_SCALE_NAIVE) * m
}

/// A3: naive multiplier for the cross-layer (mechanism 5) prior write-back
/// gain. At m=0 the high layer's coarse prior reaches the low layer this many
/// times stronger — an early, tolerant prior helps the low layer move off its
/// 0×x=0 dead point. Anneals linearly to exactly 1.0 at m=1 so the mature
/// noise-rejection gate (prior_gain·explained) is untouched. The downstream
/// push_prior ±MAX_PRIOR_MAGNITUDE clamp still bounds the worst case.
pub const VPC_PRIOR_GAIN_SCALE_NAIVE: f64 = 2.0;

/// Maturity schedule for the cross-layer prior write-back gain:
/// VPC_PRIOR_GAIN_SCALE_NAIVE at m=0, exactly 1.0 at m=1. Pure function over
/// the developmental clock; scales one local scalar of the writing predictor
/// only — never crosses layers, never gates a capability.
pub fn vpc_prior_gain_scale(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    VPC_PRIOR_GAIN_SCALE_NAIVE + (1.0 - VPC_PRIOR_GAIN_SCALE_NAIVE) * m
}

/// B2 enhancement: mature endpoint for the endogenous-baseline suppression
/// damping floor. 0.45 = 1 − 0.55·1.0, the natural lower bound of the
/// multiplicative damping term itself — at m=1 the floor can never bind for
/// any suppression in [0,1], so the mature body runs the ORIGINAL un-floored
/// damping math bit-for-bit. Not a vocal target, not a capability gate.
pub const SUPPRESSION_DAMP_FLOOR_MATURE: f64 = 0.45;

/// Maturity schedule for the endogenous-baseline suppression damping floor:
/// SUPPRESSION_DAMP_FLOOR (0.96, the B2 newborn value that takes over exactly
/// at the historical deadlock onset S*≈0.0717) at m=0, annealing linearly to
/// SUPPRESSION_DAMP_FLOOR_MATURE (0.45) at m=1 where the floor mathematically
/// vanishes (1 − 0.55·s ≥ 0.45 for all s∈[0,1]). Wide when naive — the
/// multiplicative 0×x=0 degenerate fixed point cannot form during bootstrap —
/// then narrowing continuously so the mature body returns to the original
/// damping law. Pure function over the developmental clock; a continuous
/// local scalar, never a capability/eligibility gate, never a reward.
pub fn suppression_damp_floor(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    // Two-coefficient lerp so both endpoints are exact in f64 (the a+(b−a)·m
    // form lands at 0.44999999999999996 at m=1).
    (1.0 - m) * crate::vocal_reflex::SUPPRESSION_DAMP_FLOOR + m * SUPPRESSION_DAMP_FLOOR_MATURE
}

// --- Endogenous Activity Laws (spec 2026-04-20) ---

/// Engineering approximation: EMA alpha for err-driven attention rebalancing.
/// High predictor-err slots gain attention share; low-err slots lose share.
/// This is a substrate knob, not a theoretical resource law.
pub const ATTN_ERR_REBALANCE_ALPHA: f64 = 0.10;

/// Engineering approximation: EMA alpha for quiet-period history replay
/// intensity. During HEALTH_QUIET, reasoning history entries are replayed
/// as微弱 attention perturbations to maintain endogenous activity.
/// This is a substrate knob, not a fixed replay law.
pub const REPLAY_ALPHA: f64 = 0.05;

// --- Phase 5D: Cross-object temporal relation substrate ---

/// Number of directed relation edges in GlobalRelationState.
/// Covers every ordered (src, dst) pair among N_OBJECT_SLOTS, src ≠ dst.
pub const N_REL_EDGES: usize = N_OBJECT_SLOTS * (N_OBJECT_SLOTS - 1);

/// EMA alpha for all relation edge scores. Slow enough that scores accumulate
/// evidence over multiple seconds rather than bouncing tick to tick.
pub const REL_ALPHA: f64 = 0.15;

// --- Phase 5E: Relation-level predictor ---

/// AR-1 EMA alpha for the relation edge predictor. Faster than REL_ALPHA so
/// the predictor tracks score changes more responsively than the scores themselves.
pub const REL_PRED_ALPHA: f64 = 0.3;

/// EMA alpha for relation_surprise (= EMA of relation_err²). Slower than
/// REL_PRED_ALPHA so surprise accumulates evidence over multiple cycles.
pub const REL_PRED_SURPRISE_ALPHA: f64 = 0.1;

/// Minimum predictor cycles before a relation edge emits non-zero relation_err
/// and relation_surprise. Prevents new edges from faking high confidence.
pub const REL_PRED_WARMUP_MIN: u32 = 2;

/// Upper bound for relation_surprise (= EMA of relation_err²). The inputs
/// are in [-1,1] so the max RMS² ≈ 1.75; 1.0 gives a tighter cap.
pub const REL_PRED_SURPRISE_MAX: f64 = 1.0;

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
