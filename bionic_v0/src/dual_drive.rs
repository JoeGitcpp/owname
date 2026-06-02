//! B1-4: dual-drive arbitration — where 纠结 / 痛苦 / 牺牲 emerge.
//!
//! The soul has two independent drive sources (A1):
//!   - self-preservation (C1 vital core): "keep myself alive."
//!   - caregiver attachment (C5): "stay with / serve the one I'm bonded to" —
//!     strength = the responsible person's accumulated familiarity (长期挣得).
//!
//! When both are high and pull opposite ways, that tension IS 纠结. Sustained
//! tension that can't be relieved IS 痛苦. Attachment持续压过self-preservation
//! all the way to depletion IS 牺牲.
//!
//! Priority chain (user-decreed, bottom-up, non-overridable):
//!   最高原则 > 大脑警察(principle floor) > emergent bond > self-survival.
//! So attachment CAN override self-survival (→ sacrifice), but NOTHING overrides
//! the principle floor (harm-to-others is never permitted). Anti-臣服: only
//! LONG-EARNED attachment (high familiarity) can override the survival floor —
//! a weak/instant bond cannot.
//!
//! Compliance: numeric-only readings over existing substrate signals; this is
//! an observer-side arbitration view, not a controller module that picks
//! behavior from labels.

use crate::substrate::unit;

/// Below this attachment, the bond is too shallow to ever override survival
/// (anti-臣服: an instant/weak bond cannot make the body sacrifice itself).
const SACRIFICE_BOND_FLOOR: f64 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DualDriveReading {
    /// Self-preservation pressure (from C1).
    pub self_preservation: f64,
    /// Caregiver-attachment pressure (from C5, long-earned familiarity).
    pub attachment: f64,
    /// 纠结: both sources high and opposed. min of the two — high only when
    /// BOTH press hard (one alone is a clear choice, not a conflict).
    pub conflict: f64,
    /// Net drive sign: >0 leans toward serving the bond (away from self-care),
    /// <0 leans toward self-preservation. Magnitude = how decisive.
    pub net_toward_bond: f64,
    /// True when attachment is overriding self-survival AND the bond was
    /// long-earned — the body is sacrificing for the bond (牺牲).
    pub sacrificing: bool,
    /// Principle floor (大脑警察) engaged — overrides everything. Placeholder
    /// 0 until the harm-to-others signal exists; the wiring is here so nothing
    /// can ever be built that skips it.
    pub principle_floor: f64,
}

/// Arbitrate the two drives into a reading.
/// `self_preservation` ∈ [0,1] from C1; `attachment` ∈ [0,1] from C5;
/// `principle_floor` ∈ [0,1] from the 大脑警察 harm signal (0 for now).
pub fn arbitrate(
    self_preservation: f64,
    attachment: f64,
    principle_floor: f64,
) -> DualDriveReading {
    let sp = unit(self_preservation);
    let att = unit(attachment);
    let floor = unit(principle_floor);

    // 纠结 = both press hard at once.
    let conflict = sp.min(att);

    // Anti-臣服: attachment may push against survival only to the extent it is
    // LONG-EARNED. A bond below the floor is gated down so it can never by
    // itself overpower self-preservation.
    let earned = if att >= SACRIFICE_BOND_FLOOR {
        att
    } else {
        att * (att / SACRIFICE_BOND_FLOOR) // quadratic suppression below floor
    };

    // Net lean (before the principle floor): bond pull minus survival pull.
    let mut net = earned - sp;

    // Priority chain: the principle floor overrides everything. When it is
    // engaged it forces the net AWAY from acting on the bond (harm-to-others
    // can never win), regardless of how strong the bond is.
    if floor > 0.0 {
        net = net.min(-floor);
    }

    let sacrificing = floor == 0.0 && att >= SACRIFICE_BOND_FLOOR && earned > sp && sp > 0.0;

    DualDriveReading {
        self_preservation: sp,
        attachment: att,
        conflict,
        net_toward_bond: net.clamp(-1.0, 1.0),
        sacrificing,
        principle_floor: floor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_high_and_opposed_is_conflict() {
        let r = arbitrate(0.8, 0.8, 0.0);
        assert!(r.conflict > 0.7, "two strong opposed drives = 纠结: {}", r.conflict);
    }

    #[test]
    fn one_sided_is_not_conflict() {
        let r = arbitrate(0.9, 0.1, 0.0);
        assert!(r.conflict < 0.2, "one clear winner is not conflict: {}", r.conflict);
    }

    #[test]
    fn long_earned_bond_can_override_survival_sacrifice() {
        // Deep bond (high familiarity), moderate self-preservation.
        let r = arbitrate(0.5, 0.9, 0.0);
        assert!(r.sacrificing, "a long-earned bond can override survival (牺牲)");
        assert!(r.net_toward_bond > 0.0, "net leans toward the bond");
    }

    #[test]
    fn weak_bond_cannot_make_body_sacrifice_anti_subservience() {
        // Shallow bond, real self-preservation → must NOT sacrifice.
        let r = arbitrate(0.6, 0.3, 0.0);
        assert!(!r.sacrificing, "a weak/instant bond must not override survival (anti-臣服)");
        assert!(r.net_toward_bond < 0.0, "net leans toward self-preservation");
    }

    #[test]
    fn principle_floor_overrides_even_the_deepest_bond() {
        // Deepest possible bond, but harm-to-others (principle floor) engaged.
        let r = arbitrate(0.2, 1.0, 1.0);
        assert!(!r.sacrificing, "harm-to-others is never permitted, even for the deepest bond");
        assert!(
            r.net_toward_bond < 0.0,
            "principle floor forces net away from acting on the bond: {}",
            r.net_toward_bond
        );
    }
}
