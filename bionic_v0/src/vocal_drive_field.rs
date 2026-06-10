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

/// dual-drive arbitration modulating the external/attachment-driven (region-B)
/// voicing gain. Continuous multiplicative field, NOT a behaviour gate.
/// `net_toward_bond` ∈ [-1,1]: >0 leans toward serving the bond (amplify the
/// cry toward the caregiver), <0 leans toward self-preservation (attenuate).
/// Maps net∈[-1,1] → gain∈[0.5,1.5] linearly; identity (gain=1) at net=0 so an
/// un-arbitrated/default reading leaves region B untouched.
pub fn bond_modulated_region_b(intensity: f64, net_toward_bond: f64) -> f64 {
    // net 0 → 1.0; net +1 → up to ~1.5; net -1 → down to ~0.5. Bounded, smooth.
    let gain = (1.0 + 0.5 * net_toward_bond.clamp(-1.0, 1.0)).clamp(0.0, 2.0);
    (intensity * gain).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In Standby (self_preservation=1.0) the gate must silence the COMBINED
    /// output completely — including any region-B (external) contribution.
    /// This is the metabolic consistency guard: reserve=0 → cannot spend drain.
    #[test]
    fn standby_silences_even_external_drive() {
        // Simulate: region A fully silent (already gated), region B at 0.6.
        let combined_intensity = 0.6; // external target is driving
        let standby_pressure = 1.0;  // Standby pins self_preservation to 1.0
        let result = survival_gated_urge(combined_intensity, standby_pressure);
        assert!(result < 1e-9,
            "Standby must silence combined drive (got {result}); \
             otherwise drain=0.7 prevents recovery");
    }

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

    /// net=0 (default / un-arbitrated reading) must be the identity: the
    /// dual-drive confluence introduces no behaviour change until the two
    /// sources actually lean somewhere.
    #[test]
    fn bond_modulation_is_identity_at_net_zero() {
        for intensity in [0.0, 0.2, 0.5, 1.0] {
            assert!((bond_modulated_region_b(intensity, 0.0) - intensity).abs() < 1e-12);
        }
    }

    /// net>0 (bond pull wins) amplifies the cry toward the caregiver.
    #[test]
    fn bond_modulation_amplifies_when_leaning_toward_bond() {
        let base = 0.4;
        assert!(bond_modulated_region_b(base, 0.5) > base);
        assert!(bond_modulated_region_b(base, 1.0) > bond_modulated_region_b(base, 0.5));
        assert!((bond_modulated_region_b(base, 1.0) - base * 1.5).abs() < 1e-12);
    }

    /// net<0 (self-preservation wins) attenuates the external-driven voicing.
    #[test]
    fn bond_modulation_attenuates_when_leaning_toward_self() {
        let base = 0.4;
        assert!(bond_modulated_region_b(base, -0.5) < base);
        assert!(bond_modulated_region_b(base, -1.0) < bond_modulated_region_b(base, -0.5));
        assert!((bond_modulated_region_b(base, -1.0) - base * 0.5).abs() < 1e-12);
    }

    /// All outputs stay in [0,1] even for out-of-range nets or intensities.
    #[test]
    fn bond_modulation_output_is_bounded() {
        for &intensity in &[0.0, 0.3, 0.9, 1.0, 2.0] {
            for &net in &[-5.0, -1.0, -0.3, 0.0, 0.3, 1.0, 5.0] {
                let out = bond_modulated_region_b(intensity, net);
                assert!((0.0..=1.0).contains(&out), "out={out}");
            }
        }
    }

    #[test]
    fn zero_sources_give_silence() {
        let f = VocalDriveField::combine(&[]);
        assert_eq!(f.intensity, 0.0);
        let f2 = VocalDriveField::combine(&[c(0.0, 0.5)]);
        assert_eq!(f2.intensity, 0.0);
    }
}
