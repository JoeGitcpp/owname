use crate::VIB_BANDS;

/// Identifies an axis in EvolutionPriors for modification-rate enforcement.
/// Numeric-only (no semantic labels in the AI's body logic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorAxis {
    // Pre-existing axes (allow moderate learning)
    VibBand,
    Surprise,
    VocalConverge,
    SpatialAttention,
    TemporalWindow,
    SlotGrowth,
    // v2.1 Layer 0 axes (near-immutable)
    DisruptionBand,
    DisruptionThreshold,
    DisruptionMultiplier,
    DistressAcoustic,
}

/// Per-axis cap on the absolute value of any single learned delta.
/// Layer 0 axes get caps so low they are effectively immutable.
#[derive(Clone, Debug)]
pub struct ModifyRateCaps {
    pub vib_band: f64,
    pub surprise: f64,
    pub vocal_converge: f64,
    pub spatial_attention: f64,
    pub temporal_window: f64,
    pub slot_growth: f64,
    // Layer 0 — near-immutable
    pub disruption_band: f64,
    pub disruption_threshold: f64,
    pub disruption_multiplier: f64,
    pub distress_acoustic: f64,
}

impl Default for ModifyRateCaps {
    fn default() -> Self {
        Self {
            // Pre-existing axes: moderate learning rate cap
            vib_band: 0.05,
            surprise: 0.05,
            vocal_converge: 0.05,
            spatial_attention: 0.05,
            temporal_window: 0.05,
            slot_growth: 0.05,
            // Layer 0 — orders of magnitude lower (effectively locked)
            disruption_band: 0.001,
            disruption_threshold: 0.001,
            disruption_multiplier: 0.001,
            distress_acoustic: 0.001,
        }
    }
}

/// Evolution priors - numeric bias parameters simulating millions of years of evolution
/// Uses numerical indices, no semantic labels (emotion/feeling/concept_id)
///
/// Index mapping:
/// - vib_band_sensitivity[0..3] corresponds to vib_band_0..3
/// - surprise_sensitivity: lower threshold = more reactive
/// - vocal_converge_rate: higher = faster learning
/// - spatial_attention_bias: center of view preference
#[derive(Clone, Debug)]
pub struct EvolutionPriors {
    /// Voice band sensitivity - human infants most sensitive to 500Hz-2kHz
    /// Index maps to vib_band_0..3
    pub vib_band_sensitivity: [f64; VIB_BANDS],

    /// Surprise sensitivity - innate alertness to sudden changes
    /// Value 0.0-1.0: higher = more reactive (evolutionary: sudden = danger)
    pub surprise_sensitivity: f64,

    /// Vocal learning convergence rate - faster than learning from scratch
    /// Multiplier: 1.5 = 1.5x normal speed
    pub vocal_converge_rate: f64,

    /// Spatial attention bias - center of view preference
    /// Value 0.0-1.0: higher = stronger center preference
    pub spatial_attention_bias: f64,

    /// Temporal window center - social interaction timing preference
    /// In seconds, represents innate sensitivity to 100ms-2s range
    pub temporal_window_center: f64,

    /// Slot growth sensitivity - how quickly to expand slot pool
    /// Threshold 0.0-1.0: lower = more aggressive expansion
    pub slot_growth_sensitivity: f64,

    // --- v2.1 Layer 0 disruption sensitivity (per spec §3.2) ---
    /// Per-band sensitivity boost for VIB disruption signatures.
    /// Index aligns with vib_band_0..3. Bias toward distress acoustic range
    /// (band 2: 2000-5000Hz typically carries cries/screams/破坏noise).
    pub disruption_band_sensitivity: [f64; VIB_BANDS],

    /// Magnitude of |err| (after band sensitivity weighting) above which
    /// the "disruption regime" is considered to have fired this tick.
    pub disruption_err_threshold: f64,

    /// Multiplier applied to surprise when the disruption regime fires.
    /// Must be >1.0 (amplify, never dampen).
    pub disruption_surprise_multiplier: f64,

    /// Extra acoustic sensitivity for distress signals (cries, screams,
    /// sustained high-band energy). Multiplies the band 2 contribution
    /// to disruption score.
    pub distress_acoustic_sensitivity: f64,

    /// Per-axis modification rate caps. Layer 0 axes are near-immutable.
    pub modify_rate_caps: ModifyRateCaps,
}

impl Default for EvolutionPriors {
    fn default() -> Self {
        Self {
            // Human infant voice band innate preference
            // band 1-2 corresponds to 500Hz-2kHz
            vib_band_sensitivity: [0.4, 1.0, 0.9, 0.3],

            // Innate alertness to sudden changes (evolution: sudden = danger)
            surprise_sensitivity: 0.75,

            // Faster vocal learning (don't need as many samples)
            vocal_converge_rate: 1.5,

            // Center of view innate preference
            spatial_attention_bias: 0.7,

            // Social interaction timing (100ms-2s range)
            temporal_window_center: 1.0,

            // Quick to expand slots when needed
            slot_growth_sensitivity: 0.7,

            // v2.1 Layer 0 disruption defaults
            // Bias toward bands 1-2 (500-5000Hz) where distress signals live
            disruption_band_sensitivity: [0.5, 1.0, 1.4, 0.8],
            disruption_err_threshold: 0.3,
            disruption_surprise_multiplier: 2.0,
            distress_acoustic_sensitivity: 1.5,
            modify_rate_caps: ModifyRateCaps::default(),
        }
    }
}

impl EvolutionPriors {
    /// Apply priors to slot claim weights
    pub fn apply_to_claim_config(&self, time_w: &mut f64, spatial_w: &mut f64, modality_w: &mut f64) {
        // Spatial weight higher (nearby = more important instinctively)
        *spatial_w = 0.35;
        // Modality weight higher (cross-modal binding is innate)
        *modality_w = 0.25;
        // Time weight slightly lower
        *time_w = 0.35;
    }

    /// Vocal convergence threshold - lower = faster to "learn"
    pub fn vocal_converge_threshold(&self) -> f64 {
        0.20 / self.vocal_converge_rate
    }

    /// Surprise trigger threshold - lower = more reactive
    pub fn surprise_trigger_threshold(&self) -> f64 {
        0.50 / self.surprise_sensitivity
    }

    /// Spatial attention bias for given position
    /// Returns bias multiplier based on distance from center
    pub fn spatial_attention(&self, normalized_pos: (f64, f64)) -> f64 {
        let center = 0.5;
        let dist = ((normalized_pos.0 - center).powi(2) + (normalized_pos.1 - center).powi(2)).sqrt();
        // Closer to center = higher attention
        self.spatial_attention_bias * (1.0 - dist)
    }

    /// Get vib band sensitivity for index
    pub fn vib_sensitivity(&self, band_idx: usize) -> f64 {
        if band_idx < VIB_BANDS {
            self.vib_band_sensitivity[band_idx]
        } else {
            0.5 // default
        }
    }

    /// Slot growth threshold
    pub fn slot_growth_threshold(&self) -> f64 {
        1.0 - self.slot_growth_sensitivity
    }

    /// Look up the modify-rate cap for a given axis.
    pub fn modify_rate_cap_for(&self, axis: PriorAxis) -> f64 {
        match axis {
            PriorAxis::VibBand => self.modify_rate_caps.vib_band,
            PriorAxis::Surprise => self.modify_rate_caps.surprise,
            PriorAxis::VocalConverge => self.modify_rate_caps.vocal_converge,
            PriorAxis::SpatialAttention => self.modify_rate_caps.spatial_attention,
            PriorAxis::TemporalWindow => self.modify_rate_caps.temporal_window,
            PriorAxis::SlotGrowth => self.modify_rate_caps.slot_growth,
            PriorAxis::DisruptionBand => self.modify_rate_caps.disruption_band,
            PriorAxis::DisruptionThreshold => self.modify_rate_caps.disruption_threshold,
            PriorAxis::DisruptionMultiplier => self.modify_rate_caps.disruption_multiplier,
            PriorAxis::DistressAcoustic => self.modify_rate_caps.distress_acoustic,
        }
    }

    /// Clamp a requested delta to the modify-rate cap for the axis.
    /// Returns the clamped value; preserves sign.
    pub fn clamp_modification(&self, axis: PriorAxis, requested_delta: f64) -> f64 {
        let cap = self.modify_rate_cap_for(axis);
        let mag = requested_delta.abs().min(cap);
        requested_delta.signum() * mag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_priors() {
        let priors = EvolutionPriors::default();
        assert_eq!(priors.vib_band_sensitivity[1], 1.0);
        assert!(priors.surprise_sensitivity > 0.5);
        assert!(priors.vocal_converge_rate >= 1.0);
    }

    #[test]
    fn test_vocal_converge_threshold() {
        let priors = EvolutionPriors::default();
        // With rate 1.5, threshold should be 0.20/1.5 = 0.133...
        let threshold = priors.vocal_converge_threshold();
        assert!(threshold < 0.20);
    }

    #[test]
    fn test_spatial_attention_center() {
        let priors = EvolutionPriors::default();
        // Center position should have high attention
        let center_attn = priors.spatial_attention((0.5, 0.5));
        assert!(center_attn > 0.5);

        // Edge position should have lower attention
        let edge_attn = priors.spatial_attention((0.0, 0.0));
        assert!(edge_attn < center_attn);
    }

    #[test]
    fn test_modify_rate_caps_layer_0_locked() {
        let priors = EvolutionPriors::default();

        // Layer 0 axes: extremely low cap (essentially immutable)
        let disruption_cap = priors.modify_rate_cap_for(PriorAxis::DisruptionBand);
        let distress_cap = priors.modify_rate_cap_for(PriorAxis::DistressAcoustic);
        assert!(disruption_cap <= 0.005, "disruption cap should be near-immutable, got {}", disruption_cap);
        assert!(distress_cap <= 0.005, "distress cap should be near-immutable, got {}", distress_cap);

        // Non-Layer-0 axes: moderate cap (learning still possible)
        let vib_band_cap = priors.modify_rate_cap_for(PriorAxis::VibBand);
        let vocal_cap = priors.modify_rate_cap_for(PriorAxis::VocalConverge);
        assert!(vib_band_cap > 0.01, "vib_band cap should allow learning, got {}", vib_band_cap);
        assert!(vocal_cap > 0.01, "vocal_converge cap should allow learning, got {}", vocal_cap);

        // Sanity: Layer 0 caps strictly less than non-Layer-0 caps
        assert!(disruption_cap < vib_band_cap);
        assert!(distress_cap < vocal_cap);
    }

    #[test]
    fn test_clamp_modification_respects_cap() {
        let priors = EvolutionPriors::default();

        // Trying to apply a large delta to a locked axis: clamp to cap
        let requested = 0.5;
        let cap = priors.modify_rate_cap_for(PriorAxis::DisruptionBand);
        let applied = priors.clamp_modification(PriorAxis::DisruptionBand, requested);
        assert!(applied.abs() <= cap, "applied {} must respect cap {}", applied, cap);
        assert_eq!(applied.signum(), requested.signum());

        // Small delta within cap: passes through unchanged
        let small_delta = cap * 0.5;
        let applied_small = priors.clamp_modification(PriorAxis::DisruptionBand, small_delta);
        assert!((applied_small - small_delta).abs() < 1e-12);
    }

    #[test]
    fn test_disruption_sensitivity_defaults() {
        let priors = EvolutionPriors::default();

        // disruption_band_sensitivity should bias toward distress acoustic range
        // (band 2: 2000-5000Hz typically carries cries/screams)
        assert!(
            priors.disruption_band_sensitivity[2] > priors.disruption_band_sensitivity[0],
            "distress band (2) must be more sensitive than band 0"
        );

        // Default disruption multiplier > 1.0 (must amplify, not dampen)
        assert!(
            priors.disruption_surprise_multiplier > 1.0,
            "disruption_surprise_multiplier must amplify (>1.0) by default"
        );

        // Threshold in (0, 1) — sensible range for normalized err
        assert!(priors.disruption_err_threshold > 0.0);
        assert!(priors.disruption_err_threshold < 1.0);

        // Distress acoustic sensitivity > 1.0 (must amplify)
        assert!(priors.distress_acoustic_sensitivity > 1.0);
    }
}
