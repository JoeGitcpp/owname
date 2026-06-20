//! Unified gaze drive field — the visual sibling of `vocal_drive_field`.
//!
//! The body's lateral gaze (virtual-neck self-motion that makes parallax depth
//! observable) was previously a scripted sine wave. Per the emergence principle
//! (one substrate, many angles — not parallel control modules), gaze is unified
//! into ONE drive field exactly like voicing: emergent CURIOSITY sources (where
//! visual prediction error is high) CONTRIBUTE to the field, a single outlet
//! reads it and commands the neck. "Curiosity / looming / babble" become regions
//! of one landscape, not separate controllers — the same shape as
//! `VocalDriveField`.
//!
//! Compliance: numeric-only; the field stores how-much (intensity) and which-way
//! (direction), no human-meaning labels and no hand-coded scan path. The
//! anti-0×x bootstrap floor (an endogenous exploratory wobble that survives even
//! at zero visual error) lives in the neck motor; this field carries only the
//! emergent, error-derived component plus the survival gate.

use crate::substrate::unit;

/// One contribution to the gaze drive field from a source region.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GazeDriveContribution {
    /// How strong the urge to look is from this source [0,1].
    pub intensity: f64,
    /// The signed lateral direction this source pulls toward, in [-1,1]
    /// (−1 = full left, +1 = full right, 0 = centred).
    pub direction: f64,
}

/// The unified gaze drive field: the integrated "urge to look" plus the lateral
/// direction it pulls toward. Built each tick by combining source contributions,
/// then read by the single neck-command outlet.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GazeDriveField {
    pub intensity: f64,
    pub direction: f64,
}

impl GazeDriveField {
    /// Combine source contributions into one field.
    ///
    /// Intensity is the MAX across sources (no additive blow-up — identical
    /// semantics to `VocalDriveField::combine`). The direction is the
    /// intensity-WEIGHTED blend, so the strongest source steers gaze most while
    /// others still tint it. A source with zero intensity contributes nothing.
    pub fn combine(contribs: &[GazeDriveContribution]) -> Self {
        let mut intensity = 0.0_f64;
        let mut dir = 0.0_f64;
        let mut wsum = 0.0_f64;
        for c in contribs {
            let w = unit(c.intensity);
            if w > intensity {
                intensity = w;
            }
            if w > 0.0 {
                dir += w * c.direction.clamp(-1.0, 1.0);
                wsum += w;
            }
        }
        let direction = if wsum > 0.0 { (dir / wsum).clamp(-1.0, 1.0) } else { 0.0 };
        Self {
            intensity: unit(intensity),
            direction,
        }
    }

    /// The signed emergent gaze offset = direction · intensity, in [-1,1]. This
    /// is the "where to look" the neck adds to its endogenous exploratory wobble.
    pub fn bias(&self) -> f64 {
        (self.direction * self.intensity).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(intensity: f64, direction: f64) -> GazeDriveContribution {
        GazeDriveContribution { intensity, direction }
    }

    #[test]
    fn intensity_is_max_not_sum() {
        let f = GazeDriveField::combine(&[c(0.6, -1.0), c(0.4, 1.0)]);
        assert!((f.intensity - 0.6).abs() < 1e-12, "intensity={}", f.intensity);
    }

    #[test]
    fn direction_is_intensity_weighted_blend() {
        // strong left (0.6 @ -1) should dominate weak right (0.4 @ +1)
        let f = GazeDriveField::combine(&[c(0.6, -1.0), c(0.4, 1.0)]);
        let expected = (0.6 * -1.0 + 0.4 * 1.0) / (0.6 + 0.4);
        assert!((f.direction - expected).abs() < 1e-12, "direction={}", f.direction);
        assert!(f.direction < 0.0, "blend should lean left");
    }

    #[test]
    fn zero_sources_give_no_gaze() {
        assert_eq!(GazeDriveField::combine(&[]).intensity, 0.0);
        assert_eq!(GazeDriveField::combine(&[c(0.0, 1.0)]).intensity, 0.0);
        assert_eq!(GazeDriveField::combine(&[c(0.0, 1.0)]).bias(), 0.0);
    }

    #[test]
    fn bias_is_signed_and_bounded() {
        let f = GazeDriveField::combine(&[c(1.0, 1.0)]);
        assert!((f.bias() - 1.0).abs() < 1e-12);
        let g = GazeDriveField::combine(&[c(0.5, -1.0)]);
        assert!((g.bias() + 0.5).abs() < 1e-12);
        // out-of-range inputs stay bounded
        let h = GazeDriveField::combine(&[c(2.0, 5.0)]);
        assert!((-1.0..=1.0).contains(&h.bias()));
    }
}
