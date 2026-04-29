// Numeric bias parameters. All fields are tier-0-indexed or structural.
// No semantic labels. No pre-decoded world understanding.
// Indices in [] brackets reference vib_band_0..VIB_BANDS.

use crate::VIB_BANDS;

/// Identifies an axis in BiasPriors for modification-rate enforcement.
/// Numeric-only. No semantic labels in the AI's body logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiasAxis {
    // Pre-existing axes (allow moderate learning)
    VibBand,
    Surprise,
    BandLearnRate,
    CenterWeight,
    TemporalWindow,
    SlotGrowth,
    // Layer 0 axes (near-immutable)
    DisruptionBand,
    DisruptionThreshold,
    DisruptionMultiplier,
    Band2Gain,
}

/// Per-axis cap on the absolute value of any single learned delta.
/// Layer 0 axes get caps so low they are effectively immutable.
#[derive(Clone, Debug)]
pub struct ModifyRateCaps {
    pub vib_band: f64,
    pub surprise: f64,
    pub band_learn_rate: f64,
    pub center_weight: f64,
    pub temporal_window: f64,
    pub slot_growth: f64,
    // Layer 0 — near-immutable
    pub disruption_band: f64,
    pub disruption_threshold: f64,
    pub disruption_multiplier: f64,
    pub band2_gain: f64,
}

impl Default for ModifyRateCaps {
    fn default() -> Self {
        Self {
            // Pre-existing axes: moderate learning rate cap
            vib_band: 0.05,
            surprise: 0.05,
            band_learn_rate: 0.05,
            center_weight: 0.05,
            temporal_window: 0.05,
            slot_growth: 0.05,
            // Layer 0 — orders of magnitude lower (effectively locked)
            disruption_band: 0.001,
            disruption_threshold: 0.001,
            disruption_multiplier: 0.001,
            band2_gain: 0.001,
        }
    }
}

/// Bias priors — numeric bias parameters.
/// All indices reference tier-0 band positions [0..VIB_BANDS).
/// Values are engineering starting points, subject to drift through
/// exploration history. Not pre-decoded world understanding.
#[derive(Clone, Debug)]
pub struct BiasPriors {
    /// Per-band sensitivity coefficients [0..VIB_BANDS).
    /// Equal starting point; drifts through err-driven updates.
    pub band_sensitivity: [f64; VIB_BANDS],

    /// Coefficient on |err| spike response. [0, 1].
    pub surprise_coef: f64,

    /// Band-level learning rate multiplier. >= 1.0.
    pub band_learn_rate: f64,

    /// Center-of-frame weighting coefficient. [0, 1].
    pub center_weight: f64,

    /// Temporal window reference point. [s].
    pub temporal_window_ref: f64,

    /// Slot expansion threshold. [0, 1].
    pub slot_growth_threshold: f64,

    // --- Layer 0 disruption sensitivity (per spec §3.2) ---
    /// Per-band sensitivity boost for VIB disruption signatures [0..VIB_BANDS).
    pub disruption_band_coef: [f64; VIB_BANDS],

    /// |err| magnitude above which the disruption regime fires this tick.
    pub disruption_err_threshold: f64,

    /// Multiplier applied to surprise when the disruption regime fires.
    /// Must be > 1.0 (amplify, never dampen).
    pub disruption_surprise_mul: f64,

    /// Extra gain coefficient for band index 2. Numeric only.
    pub band2_gain: f64,

    /// Per-axis modification rate caps. Layer 0 axes are near-immutable.
    pub modify_rate_caps: ModifyRateCaps,
}

impl Default for BiasPriors {
    fn default() -> Self {
        Self {
            // Equal sensitivity across all bands — no pre-decoded preference.
            band_sensitivity: [0.5; VIB_BANDS],

            // Neutral spike response coefficient.
            surprise_coef: 0.5,

            // Neutral band learning rate.
            band_learn_rate: 1.0,

            // Neutral center weighting.
            center_weight: 0.5,

            // 1s reference window — engineering starting point.
            temporal_window_ref: 1.0,

            // Neutral slot expansion threshold.
            slot_growth_threshold: 0.5,

            // Layer 0 disruption defaults — equal across bands.
            disruption_band_coef: [1.0; VIB_BANDS],
            disruption_err_threshold: 0.3,
            disruption_surprise_mul: 2.0,
            band2_gain: 1.0,
            modify_rate_caps: ModifyRateCaps::default(),
        }
    }
}

impl BiasPriors {
    /// Apply priors to slot claim weights.
    pub fn apply_to_claim_config(
        &self,
        time_w: &mut f64,
        spatial_w: &mut f64,
        modality_w: &mut f64,
    ) {
        *time_w = 0.35;
        *spatial_w = 0.35;
        *modality_w = 0.25;
    }

    /// Band learning convergence threshold — lower = faster to converge.
    pub fn band_converge_threshold(&self) -> f64 {
        0.20 / self.band_learn_rate
    }

    /// Surprise trigger threshold — lower = more reactive.
    pub fn surprise_trigger_threshold(&self) -> f64 {
        0.50 / self.surprise_coef
    }

    /// Center weighting for a given normalized position.
    /// Returns a bias multiplier based on distance from (0.5, 0.5).
    pub fn center_bias(&self, normalized_pos: (f64, f64)) -> f64 {
        let center = 0.5;
        let dist =
            ((normalized_pos.0 - center).powi(2) + (normalized_pos.1 - center).powi(2)).sqrt();
        self.center_weight * (1.0 - dist)
    }

    /// Look up band sensitivity for index.
    pub fn band_sensitivity_at(&self, band_idx: usize) -> f64 {
        if band_idx < VIB_BANDS {
            self.band_sensitivity[band_idx]
        } else {
            0.5
        }
    }

    /// Slot growth threshold.
    pub fn slot_growth_threshold(&self) -> f64 {
        1.0 - self.slot_growth_threshold
    }

    /// Look up the modify-rate cap for a given axis.
    pub fn modify_rate_cap_for(&self, axis: BiasAxis) -> f64 {
        match axis {
            BiasAxis::VibBand => self.modify_rate_caps.vib_band,
            BiasAxis::Surprise => self.modify_rate_caps.surprise,
            BiasAxis::BandLearnRate => self.modify_rate_caps.band_learn_rate,
            BiasAxis::CenterWeight => self.modify_rate_caps.center_weight,
            BiasAxis::TemporalWindow => self.modify_rate_caps.temporal_window,
            BiasAxis::SlotGrowth => self.modify_rate_caps.slot_growth,
            BiasAxis::DisruptionBand => self.modify_rate_caps.disruption_band,
            BiasAxis::DisruptionThreshold => self.modify_rate_caps.disruption_threshold,
            BiasAxis::DisruptionMultiplier => self.modify_rate_caps.disruption_multiplier,
            BiasAxis::Band2Gain => self.modify_rate_caps.band2_gain,
        }
    }

    /// Clamp a requested delta to the modify-rate cap for the axis.
    /// Returns the clamped value; preserves sign.
    pub fn clamp_modification(&self, axis: BiasAxis, requested_delta: f64) -> f64 {
        let cap = self.modify_rate_cap_for(axis);
        let mag = requested_delta.abs().min(cap);
        requested_delta.signum() * mag
    }
}

/// Compute a numeric disruption score from per-band prediction error.
///
/// Combines:
/// - per-band sensitivity weighting (priors.disruption_band_coef)
/// - band-2 extra gain (priors.band2_gain)
/// - sum of weighted |err| across bands
///
/// Higher score = more broadband high-energy pattern in the input.
/// Score is unbounded above (caller compares against thresholds).
pub fn compute_disruption_score(err: &[f64; VIB_BANDS], priors: &BiasPriors) -> f64 {
    let mut score = 0.0;
    for axis in 0..VIB_BANDS {
        let weighted = err[axis].abs() * priors.disruption_band_coef[axis];
        let with_band2_boost = if axis == 2 {
            weighted * priors.band2_gain
        } else {
            weighted
        };
        score += with_band2_boost;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_priors() {
        let priors = BiasPriors::default();
        assert_eq!(priors.band_sensitivity[1], 0.5);
        assert!(priors.surprise_coef > 0.0);
        assert!(priors.band_learn_rate >= 1.0);
    }

    #[test]
    fn test_band_converge_threshold() {
        let priors = BiasPriors::default();
        // With band_learn_rate = 1.0, threshold = 0.20/1.0 = 0.20
        let threshold = priors.band_converge_threshold();
        assert!((threshold - 0.20).abs() < 0.01);
    }

    #[test]
    fn test_center_bias_center() {
        let priors = BiasPriors::default();
        let center_bias = priors.center_bias((0.5, 0.5));
        assert!(center_bias > 0.0);

        let edge_bias = priors.center_bias((0.0, 0.0));
        assert!(edge_bias < center_bias);
    }

    #[test]
    fn test_modify_rate_caps_layer_0_locked() {
        let priors = BiasPriors::default();

        let disruption_cap = priors.modify_rate_cap_for(BiasAxis::DisruptionBand);
        let band2_cap = priors.modify_rate_cap_for(BiasAxis::Band2Gain);
        assert!(
            disruption_cap <= 0.005,
            "disruption cap should be near-immutable, got {}",
            disruption_cap
        );
        assert!(
            band2_cap <= 0.005,
            "band2 cap should be near-immutable, got {}",
            band2_cap
        );

        let vib_band_cap = priors.modify_rate_cap_for(BiasAxis::VibBand);
        let band_learn_cap = priors.modify_rate_cap_for(BiasAxis::BandLearnRate);
        assert!(
            vib_band_cap > 0.01,
            "vib_band cap should allow learning, got {}",
            vib_band_cap
        );
        assert!(
            band_learn_cap > 0.01,
            "band_learn cap should allow learning, got {}",
            band_learn_cap
        );

        assert!(disruption_cap < vib_band_cap);
        assert!(band2_cap < band_learn_cap);
    }

    #[test]
    fn test_clamp_modification_respects_cap() {
        let priors = BiasPriors::default();

        let requested = 0.5;
        let cap = priors.modify_rate_cap_for(BiasAxis::DisruptionBand);
        let applied = priors.clamp_modification(BiasAxis::DisruptionBand, requested);
        assert!(
            applied.abs() <= cap,
            "applied {} must respect cap {}",
            applied,
            cap
        );
        assert_eq!(applied.signum(), requested.signum());

        let small_delta = cap * 0.5;
        let applied_small = priors.clamp_modification(BiasAxis::DisruptionBand, small_delta);
        assert!((applied_small - small_delta).abs() < 1e-12);
    }

    #[test]
    fn test_disruption_defaults() {
        let priors = BiasPriors::default();

        // Equal across bands by default.
        assert_eq!(priors.disruption_band_coef[0], 1.0);
        assert_eq!(priors.disruption_band_coef[2], 1.0);

        // Default disruption multiplier > 1.0 (must amplify, not dampen).
        assert!(
            priors.disruption_surprise_mul > 1.0,
            "disruption_surprise_mul must amplify (>1.0) by default"
        );

        // Threshold in (0, 1) — sensible range for normalized err.
        assert!(priors.disruption_err_threshold > 0.0);
        assert!(priors.disruption_err_threshold < 1.0);

        // Band-2 gain = 1.0 by default (neutral, no special boost).
        assert_eq!(priors.band2_gain, 1.0);
    }

    #[test]
    fn test_disruption_score_quiet_input_low() {
        let priors = BiasPriors::default();
        let err = [0.05, 0.05, 0.05, 0.05];
        let score = compute_disruption_score(&err, &priors);
        assert!(
            score < 0.5,
            "quiet err should yield low score, got {}",
            score
        );
    }

    #[test]
    fn test_disruption_score_band2_dominant() {
        let priors = BiasPriors::default();
        let err = [0.05, 0.1, 0.8, 0.1];
        let score = compute_disruption_score(&err, &priors);
        assert!(
            score > 1.0,
            "band-2 dominant err should produce high score, got {}",
            score
        );
    }

    #[test]
    fn test_disruption_score_broadband_burst_high() {
        let priors = BiasPriors::default();
        let err = [0.6, 0.7, 0.8, 0.5];
        let score = compute_disruption_score(&err, &priors);
        assert!(
            score > 1.5,
            "broadband burst should produce very high score, got {}",
            score
        );
    }

    #[test]
    fn test_disruption_score_monotonic_in_err() {
        let priors = BiasPriors::default();
        let small = [0.1, 0.1, 0.1, 0.1];
        let medium = [0.3, 0.3, 0.3, 0.3];
        let large = [0.6, 0.6, 0.6, 0.6];
        let s_small = compute_disruption_score(&small, &priors);
        let s_medium = compute_disruption_score(&medium, &priors);
        let s_large = compute_disruption_score(&large, &priors);
        assert!(
            s_small < s_medium && s_medium < s_large,
            "disruption score must be monotonic in err magnitude: {} {} {}",
            s_small,
            s_medium,
            s_large
        );
    }

    #[test]
    fn test_disruption_score_band2_gain_amplifies_band_2() {
        let mut priors = BiasPriors::default();
        let err = [0.05, 0.05, 0.5, 0.05];

        let baseline = compute_disruption_score(&err, &priors);
        priors.band2_gain = 3.0;
        let amplified = compute_disruption_score(&err, &priors);

        assert!(
            amplified > baseline,
            "band2_gain bump must amplify band-2-dominant score: baseline {} vs amplified {}",
            baseline,
            amplified
        );
    }
}
