//! Unified vocal drive field (改造6).
//!
//! Previously three independent drivers each decided, on their own, WHEN to
//! voice and WITH WHAT pitch — reflex (inner pressure), target (external sound
//! pull), imitation (matching heard sound). Each raced to call begin_voicing.
//!
//! Per the emergence principle (one substrate, many angles, not parallel
//! control modules), they are unified into ONE drive field. The three sources
//! CONTRIBUTE to the field; a single outlet reads it and voices. "Reflex /
//! target / imitation" become regions of one landscape, not separate modules.
//!
//! Compliance: numeric-only; the field stores how-much (intensity) and
//! how (breath/pitch/resonance), no human-meaning labels.

use crate::substrate::unit;

/// One contribution to the vocal drive field from a source region.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalDriveContribution {
    /// How strong the urge to voice is from this source [0,1].
    pub intensity: f64,
    /// The voicing parameters this source pulls toward.
    pub breath: f64,
    pub pitch: f64,
    pub resonance: f64,
    pub noise: f64,
}

/// The unified vocal drive field: the integrated "urge to voice" plus the
/// voicing parameters it pulls toward. Built each tick by combining the
/// source contributions, then read by the single voicing outlet.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalDriveField {
    pub intensity: f64,
    pub breath: f64,
    pub pitch: f64,
    pub resonance: f64,
    pub noise: f64,
}

impl VocalDriveField {
    /// Combine source contributions into one field.
    ///
    /// Intensity is the MAX across sources (no additive blow-up — same
    /// semantics the old per-source maxing used). The voicing parameters are
    /// the intensity-WEIGHTED blend across sources, so the strongest source
    /// shapes the sound most while others still tint it. A source with zero
    /// intensity contributes nothing.
    pub fn combine(contribs: &[VocalDriveContribution]) -> Self {
        let mut intensity = 0.0_f64;
        let mut wsum = 0.0_f64;
        let mut breath = 0.0;
        let mut pitch = 0.0;
        let mut resonance = 0.0;
        let mut noise = 0.0;
        for c in contribs {
            let w = unit(c.intensity);
            if w > intensity {
                intensity = w;
            }
            if w > 0.0 {
                breath += w * c.breath;
                pitch += w * c.pitch;
                resonance += w * c.resonance;
                noise += w * c.noise;
                wsum += w;
            }
        }
        if wsum > 0.0 {
            breath /= wsum;
            pitch /= wsum;
            resonance /= wsum;
            noise /= wsum;
        }
        Self {
            intensity: unit(intensity),
            breath: unit(breath),
            pitch: unit(pitch),
            resonance: unit(resonance),
            noise: unit(noise),
        }
    }
}

/// Innate survival reflex: self-preservation pressure suppresses the INNER urge
/// to voice (self-babble), in real time. `gain = 1 - pressure`: well-fed
/// (pressure≈0) → full urge; starving / Standby (pressure→1) → silent. This is
/// continuous interoceptive modulation of the expression drive — the
/// self-preservation half of the dual drive acting on expression — NOT a
/// behaviour-selecting gate. It is INNATE (not learned) so it acts fast enough
/// to conserve energy before the body starves into Standby, instead of voicing
/// itself to death and never living long enough to learn restraint.
///
/// Applied ONLY to the inner-pressure urge; external/attachment-driven voicing
/// (a cry toward a caregiver) is left for the dual-drive to arbitrate, since an
/// emergent bond may rightly override self-preservation.
pub fn survival_gated_urge(urge: f64, self_preservation_pressure: f64) -> f64 {
    let gain = (1.0 - unit(self_preservation_pressure)).clamp(0.0, 1.0);
    urge * gain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survival_pressure_suppresses_inner_voice_urge() {
        let urge = 0.8;
        // well-fed → full urge
        assert!((survival_gated_urge(urge, 0.0) - 0.8).abs() < 1e-9);
        // starving / Standby → silent
        assert!(survival_gated_urge(urge, 1.0) < 1e-9);
        // monotone: more survival pressure → less urge (饿了就少叫)
        assert!(survival_gated_urge(urge, 0.3) > survival_gated_urge(urge, 0.7));
    }

    fn c(intensity: f64, pitch: f64) -> VocalDriveContribution {
        VocalDriveContribution {
            intensity,
            breath: 0.5,
            pitch,
            resonance: 0.5,
            noise: 0.0,
        }
    }

    #[test]
    fn intensity_is_max_not_sum() {
        let f = VocalDriveField::combine(&[c(0.6, 0.3), c(0.4, 0.9)]);
        // max, not 1.0
        assert!((f.intensity - 0.6).abs() < 1e-12);
    }

    #[test]
    fn pitch_is_intensity_weighted_blend() {
        // strong source (0.6 @ pitch .3) should dominate weak (0.4 @ pitch .9)
        let f = VocalDriveField::combine(&[c(0.6, 0.3), c(0.4, 0.9)]);
        let expected = (0.6 * 0.3 + 0.4 * 0.9) / (0.6 + 0.4);
        assert!((f.pitch - expected).abs() < 1e-12, "pitch={}", f.pitch);
    }

    #[test]
    fn zero_sources_give_silence() {
        let f = VocalDriveField::combine(&[]);
        assert_eq!(f.intensity, 0.0);
        let f2 = VocalDriveField::combine(&[c(0.0, 0.5)]);
        assert_eq!(f2.intensity, 0.0);
    }
}
