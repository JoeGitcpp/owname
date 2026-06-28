// Numeric bias parameters. All fields are tier-0-indexed or generic
// curve-pressure weights. No pre-decoded world categories live here.

use crate::substrate_pressure::PressureWeights;
use crate::VIB_BANDS;

/// Identifies a generic pressure axis for modification-rate enforcement.
/// These are engineering axes over curve update dynamics, not world types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiasAxis {
    ErrGain,
    RecurrenceGain,
    AmbiguityGain,
    AlignmentGain,
    SupportGain,
    DeviationGain,
    TemporalGapGain,
    CrossChannelGain,
    PressureMultiplier,
}

/// Per-axis cap on the absolute value of any single learned delta.
#[derive(Clone, Debug)]
pub struct ModifyRateCaps {
    pub err_gain: f64,
    pub recurrence_gain: f64,
    pub ambiguity_gain: f64,
    pub alignment_gain: f64,
    pub support_gain: f64,
    pub deviation_gain: f64,
    pub temporal_gap_gain: f64,
    pub cross_channel_gain: f64,
    pub pressure_multiplier: f64,
}

impl Default for ModifyRateCaps {
    fn default() -> Self {
        Self {
            err_gain: 0.05,
            recurrence_gain: 0.05,
            ambiguity_gain: 0.05,
            alignment_gain: 0.05,
            support_gain: 0.05,
            deviation_gain: 0.05,
            temporal_gap_gain: 0.05,
            cross_channel_gain: 0.05,
            pressure_multiplier: 0.001,
        }
    }
}

/// Birth sensitivity curves.
///
/// Band entries index tier-0 VIB coordinates. Pressure weights are generic
/// curve dynamics shared by callers; they must not encode caller domains.
#[derive(Clone, Debug)]
pub struct BiasPriors {
    pub band_sensitivity: [f64; VIB_BANDS],
    pub pressure_weights: PressureWeights,
    pub modify_rate_caps: ModifyRateCaps,
}

impl Default for BiasPriors {
    fn default() -> Self {
        Self {
            band_sensitivity: [0.5; VIB_BANDS],
            pressure_weights: PressureWeights::default(),
            modify_rate_caps: ModifyRateCaps::default(),
        }
    }
}

impl BiasPriors {
    pub fn nursery_newborn() -> Self {
        let mut band_sensitivity = [0.5; VIB_BANDS];
        // Human voice range: boost bands 1-3 (~500 Hz – 3 kHz)
        for band in 1..=3 {
            if band < VIB_BANDS {
                band_sensitivity[band] = 0.85;
            }
        }

        Self {
            band_sensitivity,
            pressure_weights: PressureWeights {
                err: 0.30,
                recurrence: 0.20,
                ambiguity: 0.15,
                alignment: 0.15,
                support: 0.10,
                deviation: 0.15,
                temporal_gap: 0.18,
                cross_channel: 0.12,
                multiplier: 1.3,
            },
            modify_rate_caps: ModifyRateCaps {
                err_gain: 0.10,
                recurrence_gain: 0.08,
                ambiguity_gain: 0.05,
                alignment_gain: 0.05,
                support_gain: 0.05,
                deviation_gain: 0.05,
                temporal_gap_gain: 0.08,
                cross_channel_gain: 0.05,
                pressure_multiplier: 0.001,
            },
        }
    }

    pub fn band_sensitivity_at(&self, band_idx: usize) -> f64 {
        if band_idx < VIB_BANDS {
            self.band_sensitivity[band_idx]
        } else {
            0.5
        }
    }

    pub fn modify_rate_cap_for(&self, axis: BiasAxis) -> f64 {
        match axis {
            BiasAxis::ErrGain => self.modify_rate_caps.err_gain,
            BiasAxis::RecurrenceGain => self.modify_rate_caps.recurrence_gain,
            BiasAxis::AmbiguityGain => self.modify_rate_caps.ambiguity_gain,
            BiasAxis::AlignmentGain => self.modify_rate_caps.alignment_gain,
            BiasAxis::SupportGain => self.modify_rate_caps.support_gain,
            BiasAxis::DeviationGain => self.modify_rate_caps.deviation_gain,
            BiasAxis::TemporalGapGain => self.modify_rate_caps.temporal_gap_gain,
            BiasAxis::CrossChannelGain => self.modify_rate_caps.cross_channel_gain,
            BiasAxis::PressureMultiplier => self.modify_rate_caps.pressure_multiplier,
        }
    }

    pub fn clamp_modification(&self, axis: BiasAxis, requested_delta: f64) -> f64 {
        let cap = self.modify_rate_cap_for(axis);
        let mag = requested_delta.abs().min(cap);
        requested_delta.signum() * mag
    }
}

/// Generic band-error score over the tier-0 VIB coordinate substrate.
pub fn compute_disruption_score(err: &[f64; VIB_BANDS], priors: &BiasPriors) -> f64 {
    err.iter()
        .enumerate()
        .map(|(axis, value)| value.abs() * priors.band_sensitivity_at(axis))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_priors_start_from_uniform_band_curves() {
        let priors = BiasPriors::default();
        assert_eq!(priors.band_sensitivity[1], 0.5);
        assert_eq!(priors.pressure_weights, PressureWeights::default());
    }

    #[test]
    fn generic_modify_caps_keep_multiplier_near_locked() {
        let priors = BiasPriors::default();
        let ordinary = priors.modify_rate_cap_for(BiasAxis::ErrGain);
        let multiplier = priors.modify_rate_cap_for(BiasAxis::PressureMultiplier);

        assert!(ordinary > 0.01);
        assert!(multiplier <= 0.005);
        assert!(multiplier < ordinary);
    }

    #[test]
    fn clamp_modification_respects_generic_cap() {
        let priors = BiasPriors::default();
        let requested = 0.5;
        let cap = priors.modify_rate_cap_for(BiasAxis::PressureMultiplier);
        let applied = priors.clamp_modification(BiasAxis::PressureMultiplier, requested);

        assert!(applied.abs() <= cap);
        assert_eq!(applied.signum(), requested.signum());
    }

    #[test]
    fn band_error_score_tracks_band_sensitivity() {
        let mut priors = BiasPriors::default();
        let err = [0.1, 0.1, 0.8, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let baseline = compute_disruption_score(&err, &priors);
        priors.band_sensitivity[2] = 1.5;
        let amplified = compute_disruption_score(&err, &priors);

        assert!(amplified > baseline);
    }
}
