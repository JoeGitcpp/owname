//! Capability curriculum governor — sequential layer unlocking via probe criteria.
//!
//! Five capability layers, each gated by the previous one being "green".
//! Layer N+1 opens only after layer N probe passes. If any layer turns red,
//! all layers above it close immediately.
//!
//! Layers:
//!   ① Encoder  (Gaussian field): splat reconstruction error < copy-baseline, count stable
//!   ② Depth:   near-object displacement > far, depth sort ≥90%, static not dominant
//!   ③ Relation: common-fate cluster score rises, occlusion confidence retained
//!   ④ Attention: protagonist switch follows info gain (non-random)
//!   ⑤ Curiosity: learning-progress axis active, trends toward learnable zone
//!
//! Compliance: no semantic priors, no backprop, no global loss.
//! All probe criteria are numeric thresholds over snapshot state.
//! Probes are configurable; all results are observer-visible.

use crate::snapshot::SnapshotData;

pub const N_CAPABILITY_LAYERS: usize = 5;

/// Layer index constants for clarity in callers.
pub const LAYER_ENCODER: usize = 0;
pub const LAYER_DEPTH: usize = 1;
pub const LAYER_RELATION: usize = 2;
pub const LAYER_ATTENTION: usize = 3;
pub const LAYER_CURIOSITY: usize = 4;

// ---------------------------------------------------------------------------
// Configurable thresholds — one struct per layer
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct EncoderLayerThresholds {
    /// hl_avg_mse must exceed this floor to indicate actual learning data.
    pub min_avg_mse_floor: f64,
    /// hl_pred_mse must be below this fraction of hl_avg_mse (< 1.0 = beats baseline).
    pub pred_to_avg_ratio_max: f64,
    /// Number of snapshots in the slot-count stability window.
    pub stability_window: usize,
    /// Maximum allowed coefficient of variation for active slot count (±20% → 0.20).
    pub slot_count_cv_max: f64,
    /// Fallback: worldline tracking passes if err_norm < span × this multiplier.
    pub worldline_err_span_ratio: f64,
}

impl Default for EncoderLayerThresholds {
    fn default() -> Self {
        Self {
            min_avg_mse_floor: 1e-6,
            pred_to_avg_ratio_max: 0.95,
            stability_window: 4,
            slot_count_cv_max: 0.20,
            worldline_err_span_ratio: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DepthLayerThresholds {
    /// Minimum active slots with valid spatial worldlines for a depth comparison.
    pub min_slots_with_worldline: usize,
    /// Fraction of near/far slot pairs where the larger-span slot has higher velocity.
    pub depth_sort_accuracy_min: f64,
    /// Velocity (normalized units/s) below which a slot is considered static.
    pub static_velocity_max: f64,
    /// Minimum span difference between two slots to count as a "near vs far" pair.
    pub min_span_diff: f64,
}

impl Default for DepthLayerThresholds {
    fn default() -> Self {
        Self {
            min_slots_with_worldline: 2,
            depth_sort_accuracy_min: 0.90,
            static_velocity_max: 0.05,
            min_span_diff: 0.01,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RelationLayerThresholds {
    /// Minimum co_change_score on any active relation edge (common fate).
    pub min_co_change_score: f64,
    /// Minimum relation_confidence on any active edge.
    pub min_relation_confidence: f64,
    /// Minimum coordinated_event_score from global reasoning.
    pub min_coordinated_event_score: f64,
}

impl Default for RelationLayerThresholds {
    fn default() -> Self {
        Self {
            min_co_change_score: 0.20,
            min_relation_confidence: 0.15,
            min_coordinated_event_score: 0.10,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AttentionLayerThresholds {
    /// A slot must be focused (focused_slot_id >= 0).
    pub require_focused_slot: bool,
    /// Attention entropy must be below this fraction of log2(N_OBJECT_SLOTS).
    pub max_entropy_fraction: f64,
    /// global_surprise_load must exceed this floor (info gain is driving focus).
    pub min_surprise_load: f64,
}

impl Default for AttentionLayerThresholds {
    fn default() -> Self {
        Self {
            require_focused_slot: true,
            max_entropy_fraction: 0.95,
            min_surprise_load: 0.01,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CuriosityLayerThresholds {
    /// Minimum probe law intensity to qualify as active curiosity drive.
    pub min_probe_intensity: f64,
    /// Minimum learning_progress in any slot_xlayers entry.
    pub min_learning_progress: f64,
    /// Minimum explained_fraction in any slot_xlayers entry (in learnable zone).
    pub min_explained_fraction: f64,
}

impl Default for CuriosityLayerThresholds {
    fn default() -> Self {
        Self {
            min_probe_intensity: 1e-9,
            min_learning_progress: 1e-9,
            min_explained_fraction: 0.01,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CapabilityLayerConfig {
    pub encoder: EncoderLayerThresholds,
    pub depth: DepthLayerThresholds,
    pub relation: RelationLayerThresholds,
    pub attention: AttentionLayerThresholds,
    pub curiosity: CuriosityLayerThresholds,
}

impl Default for CapabilityLayerConfig {
    fn default() -> Self {
        Self {
            encoder: EncoderLayerThresholds::default(),
            depth: DepthLayerThresholds::default(),
            relation: RelationLayerThresholds::default(),
            attention: AttentionLayerThresholds::default(),
            curiosity: CuriosityLayerThresholds::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-layer probe result structs — observer-visible numeric evidence
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct EncoderProbeResult {
    pub green: bool,
    pub hl_avg_mse: f64,
    pub hl_pred_mse: f64,
    /// hl_pred_mse / hl_avg_mse (1.0 if hl_avg_mse = 0).
    pub pred_to_avg_ratio: f64,
    pub n_active_slots: u8,
    pub slot_count_stable: bool,
    /// True iff any active slot has a phot band family.
    pub phot_slot_active: bool,
    /// True iff fallback worldline tracking criterion passed.
    pub worldline_tracking_ok: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DepthProbeResult {
    pub green: bool,
    pub n_worldline_slots: u8,
    /// Fraction of near/far pairs where larger-span slot has >= velocity.
    pub depth_sort_accuracy: f64,
    pub near_velocity_mean: f64,
    pub far_velocity_mean: f64,
    /// True if at least one slot velocity <= static_velocity_max.
    pub static_slot_present: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RelationProbeResult {
    pub green: bool,
    pub n_active_edges: u8,
    pub max_co_change_score: f64,
    pub max_relation_confidence: f64,
    pub coordinated_event_score: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AttentionProbeResult {
    pub green: bool,
    /// focused_slot_id from GlobalAttentionState (-1 = none).
    pub focused_slot_id: i8,
    pub attention_entropy: f64,
    pub global_surprise_load: f64,
    pub n_active: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CuriosityProbeResult {
    pub green: bool,
    pub probe_intensity: f64,
    pub max_learning_progress: f64,
    pub max_explained_fraction: f64,
    /// Number of slot_xlayers entries that had any learning progress.
    pub n_active_xlayer_slots: u8,
}

// ---------------------------------------------------------------------------
// Evidence — full observer output for one evaluation tick
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct CapabilityGovernorEvidence {
    pub tick_count: u64,
    /// Whether each layer's probe passed independently.
    pub probe_green: [bool; N_CAPABILITY_LAYERS],
    /// Whether each layer is open (gated: layer N requires all of 0..N green).
    pub layers_open: [bool; N_CAPABILITY_LAYERS],
    /// Highest layer index that is currently open (-1 if none).
    pub highest_open_layer: i8,
    pub encoder: EncoderProbeResult,
    pub depth: DepthProbeResult,
    pub relation: RelationProbeResult,
    pub attention: AttentionProbeResult,
    pub curiosity: CuriosityProbeResult,
}

// ---------------------------------------------------------------------------
// Governor state — maintains rolling window, is updated tick by tick
// ---------------------------------------------------------------------------

pub struct CapabilityGovernorState {
    /// Rolling window of active slot counts (for encoder stability check).
    slot_count_window: Vec<u8>,
    window_cap: usize,
    /// Which layers are currently open (gate output).
    pub layers_open: [bool; N_CAPABILITY_LAYERS],
    /// Latest probe result per layer.
    pub encoder_probe: EncoderProbeResult,
    pub depth_probe: DepthProbeResult,
    pub relation_probe: RelationProbeResult,
    pub attention_probe: AttentionProbeResult,
    pub curiosity_probe: CuriosityProbeResult,
    /// Total snapshots processed.
    pub tick_count: u64,
}

impl CapabilityGovernorState {
    pub fn new(window_cap: usize) -> Self {
        Self {
            slot_count_window: Vec::with_capacity(window_cap.max(1)),
            window_cap: window_cap.max(1),
            layers_open: [false; N_CAPABILITY_LAYERS],
            encoder_probe: EncoderProbeResult::default(),
            depth_probe: DepthProbeResult::default(),
            relation_probe: RelationProbeResult::default(),
            attention_probe: AttentionProbeResult::default(),
            curiosity_probe: CuriosityProbeResult::default(),
            tick_count: 0,
        }
    }

    pub fn is_layer_open(&self, layer: usize) -> bool {
        layer < N_CAPABILITY_LAYERS && self.layers_open[layer]
    }

    /// Ingest one snapshot, re-evaluate all probes, update gate state.
    /// Returns the full evidence record for the observer.
    pub fn update(
        &mut self,
        snap: &SnapshotData,
        config: &CapabilityLayerConfig,
    ) -> CapabilityGovernorEvidence {
        self.tick_count += 1;

        // Update slot-count window (encoder stability).
        let n_active = snap.global_attention.n_active;
        if self.slot_count_window.len() >= self.window_cap {
            self.slot_count_window.remove(0);
        }
        self.slot_count_window.push(n_active);

        // Evaluate probes.
        self.encoder_probe =
            probe_encoder(snap, &config.encoder, &self.slot_count_window);
        self.depth_probe = probe_depth(snap, &config.depth);
        self.relation_probe = probe_relation(snap, &config.relation);
        self.attention_probe = probe_attention(snap, &config.attention);
        self.curiosity_probe = probe_curiosity(snap, &config.curiosity);

        let probe_green = [
            self.encoder_probe.green,
            self.depth_probe.green,
            self.relation_probe.green,
            self.attention_probe.green,
            self.curiosity_probe.green,
        ];

        // Sequential gate: layer N open only if all layers 0..=N are green.
        for i in 0..N_CAPABILITY_LAYERS {
            self.layers_open[i] = if i == 0 {
                probe_green[0]
            } else {
                self.layers_open[i - 1] && probe_green[i]
            };
        }

        let highest_open_layer = self
            .layers_open
            .iter()
            .rposition(|&open| open)
            .map(|i| i as i8)
            .unwrap_or(-1);

        CapabilityGovernorEvidence {
            tick_count: self.tick_count,
            probe_green,
            layers_open: self.layers_open,
            highest_open_layer,
            encoder: self.encoder_probe,
            depth: self.depth_probe,
            relation: self.relation_probe,
            attention: self.attention_probe,
            curiosity: self.curiosity_probe,
        }
    }
}

// ---------------------------------------------------------------------------
// Probe functions — pure, numeric-only evaluation per layer
// ---------------------------------------------------------------------------

fn probe_encoder(
    snap: &SnapshotData,
    t: &EncoderLayerThresholds,
    slot_window: &[u8],
) -> EncoderProbeResult {
    let hl_avg_mse = snap.hl_avg_mse;
    let hl_pred_mse = snap.hl_pred_mse;

    // Primary criterion: hl_pred_mse < hl_avg_mse × ratio_max (prediction beats naive).
    let (ratio, reconstruct_ok) = if hl_avg_mse > t.min_avg_mse_floor {
        let r = hl_pred_mse / hl_avg_mse;
        (r, r < t.pred_to_avg_ratio_max)
    } else {
        (1.0, false)
    };

    // Fallback: any slot whose worldline tracks tighter than span × ratio.
    let worldline_tracking_ok = snap.slots.iter().any(|s| {
        if !s.active {
            return false;
        }
        if let (Some(wl), Some(sp)) = (&s.spatial_worldline, &s.spatial) {
            wl.valid
                && sp.valid
                && sp.span_x > 0.0
                && wl.err_norm < sp.span_x * t.worldline_err_span_ratio
        } else {
            false
        }
    });

    let reconstruct_pass = reconstruct_ok || worldline_tracking_ok;

    // Slot-count stability: CV ≤ slot_count_cv_max over the rolling window.
    let slot_count_stable = if slot_window.len() >= t.stability_window {
        let vals: Vec<f64> = slot_window.iter().map(|&c| c as f64).collect();
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let min_v = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if mean > 0.0 {
            (max_v - min_v) / mean <= t.slot_count_cv_max
        } else {
            // Stably-zero slot count still counts as stable.
            max_v - min_v < 1.0
        }
    } else {
        false
    };

    let phot_slot_active = snap.slots.iter().any(|s| s.active && s.phot.is_some());

    EncoderProbeResult {
        green: reconstruct_pass && slot_count_stable,
        hl_avg_mse,
        hl_pred_mse,
        pred_to_avg_ratio: ratio,
        n_active_slots: snap.global_attention.n_active,
        slot_count_stable,
        phot_slot_active,
        worldline_tracking_ok,
    }
}

/// Compact per-slot data for depth ordering.
struct DepthSlotInfo {
    span: f64,
    velocity: f64,
}

fn probe_depth(snap: &SnapshotData, t: &DepthLayerThresholds) -> DepthProbeResult {
    let slot_infos: Vec<DepthSlotInfo> = snap
        .slots
        .iter()
        .filter(|s| s.active)
        .filter_map(|s| {
            let sp = s.spatial.as_ref()?;
            if !sp.valid {
                return None;
            }
            let wl = s.spatial_worldline.as_ref()?;
            if !wl.valid {
                return None;
            }
            let span = (sp.span_x + sp.span_y) * 0.5;
            let vel = f64::sqrt(
                wl.velocity_x_per_s * wl.velocity_x_per_s
                    + wl.velocity_y_per_s * wl.velocity_y_per_s,
            );
            Some(DepthSlotInfo { span, velocity: vel })
        })
        .collect();

    let n_worldline = slot_infos.len();

    if n_worldline < t.min_slots_with_worldline {
        return DepthProbeResult {
            green: false,
            n_worldline_slots: n_worldline as u8,
            ..DepthProbeResult::default()
        };
    }

    // Depth sort accuracy: for each pair, larger span → higher velocity.
    let mut correct = 0u32;
    let mut total = 0u32;
    for i in 0..slot_infos.len() {
        for j in (i + 1)..slot_infos.len() {
            let a = &slot_infos[i];
            let b = &slot_infos[j];
            let span_diff = (a.span - b.span).abs();
            if span_diff < t.min_span_diff {
                continue;
            }
            total += 1;
            let near_has_higher = if a.span > b.span {
                a.velocity > b.velocity
            } else {
                b.velocity > a.velocity
            };
            if near_has_higher {
                correct += 1;
            }
        }
    }

    let depth_sort_accuracy = if total > 0 {
        correct as f64 / total as f64
    } else {
        // All slots have nearly identical span — can't verify depth sort.
        // Fall through with 0.0; the probe will fail on this criterion.
        0.0
    };

    // Static slot: at least one slot with velocity <= static_velocity_max.
    let static_slot_present = slot_infos.iter().any(|s| s.velocity <= t.static_velocity_max);

    // Near/far mean velocities (sorted by span descending).
    let mut sorted: Vec<_> = slot_infos.iter().collect();
    sorted.sort_by(|a, b| b.span.partial_cmp(&a.span).unwrap_or(std::cmp::Ordering::Equal));
    let half = (sorted.len() / 2).max(1);
    let near_count = sorted.len() - half.min(sorted.len() - 1);
    let near_velocity_mean = sorted[..near_count]
        .iter()
        .map(|s| s.velocity)
        .sum::<f64>()
        / near_count as f64;
    let far_velocity_mean = if half < sorted.len() {
        sorted[near_count..]
            .iter()
            .map(|s| s.velocity)
            .sum::<f64>()
            / (sorted.len() - near_count) as f64
    } else {
        0.0
    };

    let green =
        depth_sort_accuracy >= t.depth_sort_accuracy_min && static_slot_present && total > 0;

    DepthProbeResult {
        green,
        n_worldline_slots: n_worldline as u8,
        depth_sort_accuracy,
        near_velocity_mean,
        far_velocity_mean,
        static_slot_present,
    }
}

fn probe_relation(snap: &SnapshotData, t: &RelationLayerThresholds) -> RelationProbeResult {
    let edges = &snap.global_relations.edges;
    let n_active_edges = snap.global_relations.n_active_edges;

    let mut max_co = 0.0_f64;
    let mut max_conf = 0.0_f64;
    for edge in edges.iter() {
        if !edge.active {
            continue;
        }
        max_co = max_co.max(edge.co_change_score);
        max_conf = max_conf.max(edge.relation_confidence);
    }

    let coordinated_event_score = snap.global_reasoning.coordinated_event_score;

    let green = n_active_edges > 0
        && max_co >= t.min_co_change_score
        && max_conf >= t.min_relation_confidence
        && coordinated_event_score >= t.min_coordinated_event_score;

    RelationProbeResult {
        green,
        n_active_edges,
        max_co_change_score: max_co,
        max_relation_confidence: max_conf,
        coordinated_event_score,
    }
}

fn probe_attention(snap: &SnapshotData, t: &AttentionLayerThresholds) -> AttentionProbeResult {
    let focused_slot_id = snap.global_attention.focused_slot_id;
    let attention_entropy = snap.global_attention.attention_entropy;
    let global_surprise_load = snap.global_reasoning.global_surprise_load;
    let n_active = snap.global_attention.n_active;

    // Compute max possible entropy for the active slot count (log2 of n_active).
    let max_entropy = if n_active > 1 {
        (n_active as f64).log2()
    } else {
        1.0
    };
    let entropy_ok = attention_entropy < t.max_entropy_fraction * max_entropy.max(1.0);
    let focus_ok = !t.require_focused_slot || focused_slot_id >= 0;
    let surprise_ok = global_surprise_load >= t.min_surprise_load;

    AttentionProbeResult {
        green: focus_ok && entropy_ok && surprise_ok,
        focused_slot_id,
        attention_entropy,
        global_surprise_load,
        n_active,
    }
}

fn probe_curiosity(snap: &SnapshotData, t: &CuriosityLayerThresholds) -> CuriosityProbeResult {
    let probe_intensity = snap.probe_law.intensity;

    let mut max_lp = 0.0_f64;
    let mut max_ef = 0.0_f64;
    let mut n_active_xlayer = 0u8;

    for xl_opt in snap.slot_xlayers.iter() {
        if let Some(xl) = xl_opt {
            if xl.learning_progress > t.min_learning_progress {
                n_active_xlayer = n_active_xlayer.saturating_add(1);
            }
            max_lp = max_lp.max(xl.learning_progress);
            max_ef = max_ef.max(xl.explained_fraction);
        }
    }

    // Probe passes if the learning-progress drive is active (probe law) OR
    // cross-layer predictors are making progress in a learnable zone.
    let probe_drive_ok = probe_intensity >= t.min_probe_intensity;
    let xlayer_learning_ok =
        max_lp >= t.min_learning_progress && max_ef >= t.min_explained_fraction;

    CuriosityProbeResult {
        green: probe_drive_ok || xlayer_learning_ok,
        probe_intensity,
        max_learning_progress: max_lp,
        max_explained_fraction: max_ef,
        n_active_xlayer_slots: n_active_xlayer,
    }
}

// ---------------------------------------------------------------------------
// Nursery integration — run the governor over a sequence of snapshots
// ---------------------------------------------------------------------------

/// Run the capability governor over a sequence of snapshots (e.g. from a
/// nursery or training run). Returns the final evidence after all snapshots.
///
/// This mirrors `summarize_live_human_nursery` — both governors can be driven
/// by the same snapshot sequence in parallel.
pub fn summarize_capability_nursery<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
    config: &CapabilityLayerConfig,
) -> CapabilityGovernorEvidence {
    let mut state = CapabilityGovernorState::new(config.encoder.stability_window.max(4));
    let mut evidence = CapabilityGovernorEvidence::default();
    for snap in snapshots {
        evidence = state.update(snap, config);
    }
    evidence
}

// ---------------------------------------------------------------------------
// Sample snapshot helpers — used by tests to produce staged synthetic data
// ---------------------------------------------------------------------------

pub mod test_helpers {
    use crate::active_reasoning::ActiveReasoningGapState;
    use crate::adaptive_stability::AdaptiveStabilityState;
    use crate::code_ingress::CodeIngressState;
    use crate::evidence_tension::EvidenceTensionState;
    use crate::language_binding::LanguageBindingState;
    use crate::learned_bias::LearnedBiasSummary;
    use crate::self_development::SelfDevelopmentState;
    use crate::snapshot::{
        SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub, SnapshotData,
        SnapshotFramedByteEgress, SnapshotLastClaim, SnapshotProbeLaw, SnapshotSpatial,
        SnapshotSlot, SnapshotVocalMemory,
    };
    use crate::viewport::{
        GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState,
        GlobalRelationState, SpatialWorldlineState,
    };
    use crate::vocal_imitation::VocalImitationState;
    use crate::vocal_organ::VocalOrganState;
    use crate::vocal_target::VocalTargetState;
    use crate::vpc::VpcContractState;

    fn base_snap() -> SnapshotData {
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
            hl_modulation: 1.0,
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
        }
    }

    /// Cold snapshot: nothing active, all probes fail.
    pub fn cold_snap() -> SnapshotData {
        base_snap()
    }

    /// Encoder-ready snapshot: hl metrics beat baseline, slot count stable.
    /// Layer 1 probe passes; depth and above do not.
    pub fn encoder_snap() -> SnapshotData {
        let mut s = base_snap();
        s.hl_avg_mse = 0.05;
        s.hl_pred_mse = 0.04; // ratio = 0.80 < 0.95 threshold
        s.global_attention.n_active = 2;
        s
    }

    fn worldline_slot(span: f64, velocity: f64) -> SnapshotSlot {
        SnapshotSlot {
            active: true,
            last_update_t_us: 1_000,
            source_modality: crate::MOD_PHOT,
            nomination_seq: 1,
            vib: None,
            phot: None,
            mech: None,
            spatial: Some(SnapshotSpatial {
                pos_x: 0.5,
                pos_y: 0.5,
                span_x: span,
                span_y: span,
                precision: 0.8,
                last_update_t_us: 1_000,
                valid: true,
            }),
            spatial_worldline: Some(SpatialWorldlineState {
                n_updates: 10,
                prev_t_us: 0,
                last_update_t_us: 1_000,
                prev_pos_x: 0.4,
                prev_pos_y: 0.5,
                predicted_pos_x: 0.5,
                predicted_pos_y: 0.5,
                velocity_x_per_s: velocity,
                velocity_y_per_s: 0.0,
                err_x: 0.005,
                err_y: 0.002,
                err_norm: 0.005,
                last_dt_us: 33_000,
                valid: true,
            }),
            fusion: None,
            high_layer: None,
            attention_share: 0.5,
            precision_bias: 0.0,
        }
    }

    /// Depth-ready snapshot: 2 slots with worldlines where near (large span)
    /// moves faster than far (small span). One static slot.
    /// Layers 1 and 2 probes pass; relation and above do not.
    pub fn depth_snap() -> SnapshotData {
        let mut s = encoder_snap();
        // Near: larger span (0.30), faster (0.40 u/s)
        s.slots.push(worldline_slot(0.30, 0.40));
        // Far: smaller span (0.08), slower (0.10 u/s)
        s.slots.push(worldline_slot(0.08, 0.10));
        // Static: smallest span (0.03), zero velocity — deepest/farthest, consistent
        s.slots.push(worldline_slot(0.03, 0.00));
        s.global_attention.n_active = 3;
        s.global_attention.focused_slot_id = 0;
        s
    }

    /// Relation-ready snapshot: active edges with co-change and confidence above
    /// threshold, plus a coordinated event score.
    /// Layers 1-3 probes pass; attention (layer 4) still fails because
    /// global_surprise_load stays at 0.0 (below the attention floor of 0.01).
    pub fn relation_snap() -> SnapshotData {
        let mut s = depth_snap();
        // Activate an edge with co_change_score and relation_confidence.
        let mut edges = GlobalRelationState::default();
        edges.edges[0].active = true;
        edges.edges[0].src_slot_id = 0;
        edges.edges[0].dst_slot_id = 1;
        edges.edges[0].co_change_score = 0.35;
        edges.edges[0].relation_confidence = 0.25;
        edges.n_active_edges = 1;
        s.global_relations = edges;
        s.global_reasoning.coordinated_event_score = 0.20;
        // global_surprise_load intentionally left 0.0 so attention probe fails.
        s
    }

    /// Attention-ready snapshot: focused slot, non-uniform entropy, surprise load.
    /// Layers 1-4 probes pass.
    pub fn attention_snap() -> SnapshotData {
        let mut s = relation_snap();
        s.global_attention.focused_slot_id = 0;
        s.global_attention.attention_entropy = 0.5; // below max (0.95 * log2(3) ≈ 1.51)
        s.global_attention.n_active = 3;
        s.global_reasoning.global_surprise_load = 0.05;
        s.global_reasoning.routing_focus_slot_ids[0] = 0;
        s
    }

    /// Curiosity-ready snapshot: probe law intensity positive, cross-layer learning active.
    /// All 5 layer probes pass.
    pub fn curiosity_snap() -> SnapshotData {
        use crate::vpc_cross_layer::SlotCrossLayerState;
        let mut s = attention_snap();
        s.probe_law.intensity = 0.15;
        s.slot_xlayers.push(Some(SlotCrossLayerState {
            source_slots: vec![1],
            source_weights: vec![0.5],
            bias: 0.0,
            err_pred_mse: 0.01,
            learning_progress: 1e-5,
            n_updates: 20,
            residual: 0.005,
            explained_fraction: 0.35,
        }));
        s
    }
}
