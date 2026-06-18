//! Phase C1: IntentionState — the body's "I am pushing toward something"
//! signal.
//!
//! Design-doc role: when FastPath (muscle memory) goes silent on the perceived
//! state for several consecutive ticks, the body needs an EXPLICIT target to
//! deliberate toward. The intention substrate provides that target: a single
//! endpoint of the perturbation_graph the body is currently committed to
//! reaching/maintaining/recognizing, plus a relation (the suspected pathway
//! that should get the body there) and a commitment window so a fresh
//! stimulus can't yank the intention away mid-stride.
//!
//! Constraints from the design doc:
//! - Minimum commitment of INTENTION_COMMITMENT_MIN_US (default 2 s) before
//!   a new target can preempt.
//! - Preempting target must score at least INTENTION_PREEMPT_DOMINANCE × the
//!   current target's score (default 1.5).
//! - intention_pressure rises while the relation toward target stays
//!   unsatisfied (low strength × expressivity match against current state),
//!   and decays when satisfied — published on INTERO_INTENTION_PRESSURE so
//!   the prediction fabric sees "the body is trying".
//! - When satisfied, the intention either yields (target drops to None) or
//!   transitions to the next link in the relation chain (left to the
//!   arbitrator — this module just owns the data).
//!
//! Persisted alongside the snapshot. A reboot resumes the same intention
//! unless its commitment window has expired (in which case it clears).

use crate::perturbation_graph::{Endpoint, RelId};

pub const INTENTION_COMMITMENT_MIN_US: u64 = 2_000_000;
pub const INTENTION_PREEMPT_DOMINANCE: f64 = 1.5;
/// Decay constant for `intention_pressure` once the intention is satisfied
/// or absent. Half-life ≈ 1.5 s under a 30 Hz orchestrator tick.
pub const INTENTION_PRESSURE_DECAY: f64 = 0.015;
/// Growth per tick when intention is unsatisfied. Caps at 1.0.
pub const INTENTION_PRESSURE_GROWTH: f64 = 0.030;

/// The body's commitment to a target endpoint. `current_target` is the
/// endpoint the body is reaching/maintaining/recognizing; `target_rel` is
/// the suspected pathway the slow walk picked toward it (may be None when
/// the intention arose from raw stimulus salience rather than a known
/// relation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntentionState {
    pub current_target: Option<Endpoint>,
    pub target_rel: Option<RelId>,
    /// In [0,1] — rises while target unsatisfied, decays once satisfied.
    pub intention_pressure: f64,
    /// In [0,1] — match between current state and the target's signature.
    /// 0 = no overlap; 1 = body is already where it wants to be.
    pub satisfaction: f64,
    /// Wall-clock µs of last update. Used for persistence + commitment.
    pub last_t_us: u64,
    /// Wall-clock µs at which this target was committed. New targets cannot
    /// preempt until INTENTION_COMMITMENT_MIN_US has elapsed.
    pub commitment_until_us: u64,
    /// Origin score that won this target the slot. Stored so a preempting
    /// candidate can be compared against the COMMITTED dominance threshold
    /// even after pressure has drifted.
    pub origin_score: f64,
}

impl Default for IntentionState {
    fn default() -> Self {
        Self {
            current_target: None,
            target_rel: None,
            intention_pressure: 0.0,
            satisfaction: 0.0,
            last_t_us: 0,
            commitment_until_us: 0,
            origin_score: 0.0,
        }
    }
}

impl IntentionState {
    /// Tick the dynamics: rise pressure if unsatisfied, decay if satisfied
    /// or absent. Called once per orchestrator tick AFTER the arbitrator
    /// (which may have picked a new target this tick).
    pub fn tick(&mut self, now_us: u64) {
        if self.current_target.is_none() {
            self.intention_pressure =
                (self.intention_pressure - INTENTION_PRESSURE_DECAY).max(0.0);
        } else if self.satisfaction >= 0.8 {
            self.intention_pressure =
                (self.intention_pressure - INTENTION_PRESSURE_DECAY).max(0.0);
        } else {
            self.intention_pressure =
                (self.intention_pressure + INTENTION_PRESSURE_GROWTH).min(1.0);
        }
        self.last_t_us = now_us;
    }

    /// Returns true if the commitment window has expired and the slot is
    /// open for re-evaluation (no current target OR commitment elapsed).
    pub fn commitment_open(&self, now_us: u64) -> bool {
        self.current_target.is_none() || now_us >= self.commitment_until_us
    }

    /// Try to commit a fresh candidate. Returns true if accepted.
    /// Rules: if no current target → accept. If commitment open and
    /// `score >= origin_score * INTENTION_PREEMPT_DOMINANCE` → accept.
    /// Otherwise reject (current target keeps the slot).
    pub fn try_commit(
        &mut self,
        candidate_target: Endpoint,
        candidate_rel: Option<RelId>,
        candidate_score: f64,
        now_us: u64,
    ) -> bool {
        if self.current_target.is_none() {
            self.current_target = Some(candidate_target);
            self.target_rel = candidate_rel;
            self.origin_score = candidate_score;
            self.commitment_until_us = now_us.saturating_add(INTENTION_COMMITMENT_MIN_US);
            self.satisfaction = 0.0;
            return true;
        }
        let open = now_us >= self.commitment_until_us;
        let dominant = candidate_score >= self.origin_score * INTENTION_PREEMPT_DOMINANCE;
        if open && dominant {
            self.current_target = Some(candidate_target);
            self.target_rel = candidate_rel;
            self.origin_score = candidate_score;
            self.commitment_until_us = now_us.saturating_add(INTENTION_COMMITMENT_MIN_US);
            self.satisfaction = 0.0;
            true
        } else {
            false
        }
    }

    /// Update satisfaction based on how well the body's current state
    /// (best-bound endpoint sig) matches the target sig. Caller supplies
    /// the similarity score in [0,1].
    pub fn update_satisfaction(&mut self, similarity: f64) {
        self.satisfaction = similarity.clamp(0.0, 1.0);
    }

    /// Clear the intention — used when satisfaction reaches a stable plateau
    /// and the arbitrator decides to release the target for the next round.
    pub fn release(&mut self) {
        self.current_target = None;
        self.target_rel = None;
        self.origin_score = 0.0;
        self.satisfaction = 0.0;
        self.commitment_until_us = 0;
    }

    // ── Persistence ────────────────────────────────────────────────────────
    //
    // The snapshot block is a fixed 56-byte stride so the reader can advance
    // even if Endpoint / RelId widths change later. Layout:
    //   target_kind: u8       (0=None, 1=Node, 2=Group, 3=Relation)
    //   target_id  : u32      (zero when target_kind=0)
    //   rel_present: u8       (0 / 1)
    //   rel_id     : u32      (zero when rel_present=0)
    //   pressure   : f64
    //   satisfaction: f64
    //   last_t_us  : u64
    //   commitment_until_us: u64
    //   origin_score: f64
    //   pad        : 2 bytes  (align)
    pub const SNAPSHOT_BYTES: usize = 56;

    pub fn serialize(&self) -> [u8; Self::SNAPSHOT_BYTES] {
        let mut out = [0u8; Self::SNAPSHOT_BYTES];
        let (kind, id) = match self.current_target {
            None => (0u8, 0u32),
            Some(Endpoint::Node(n)) => (1, n),
            Some(Endpoint::Group(g)) => (2, g),
            Some(Endpoint::Relation(r)) => (3, r),
        };
        out[0] = kind;
        out[1..5].copy_from_slice(&id.to_le_bytes());
        out[5] = if self.target_rel.is_some() { 1 } else { 0 };
        out[6..10].copy_from_slice(&self.target_rel.unwrap_or(0).to_le_bytes());
        out[10..18].copy_from_slice(&self.intention_pressure.to_le_bytes());
        out[18..26].copy_from_slice(&self.satisfaction.to_le_bytes());
        out[26..34].copy_from_slice(&self.last_t_us.to_le_bytes());
        out[34..42].copy_from_slice(&self.commitment_until_us.to_le_bytes());
        out[42..50].copy_from_slice(&self.origin_score.to_le_bytes());
        // 50..56 stays zero (padding for future expansion).
        out
    }

    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SNAPSHOT_BYTES {
            return None;
        }
        let kind = bytes[0];
        let id = u32::from_le_bytes(bytes[1..5].try_into().ok()?);
        let current_target = match kind {
            0 => None,
            1 => Some(Endpoint::Node(id)),
            2 => Some(Endpoint::Group(id)),
            3 => Some(Endpoint::Relation(id)),
            _ => return None,
        };
        let rel_present = bytes[5] != 0;
        let rid = u32::from_le_bytes(bytes[6..10].try_into().ok()?);
        let target_rel = if rel_present { Some(rid) } else { None };
        let intention_pressure = f64::from_le_bytes(bytes[10..18].try_into().ok()?);
        let satisfaction = f64::from_le_bytes(bytes[18..26].try_into().ok()?);
        let last_t_us = u64::from_le_bytes(bytes[26..34].try_into().ok()?);
        let commitment_until_us = u64::from_le_bytes(bytes[34..42].try_into().ok()?);
        let origin_score = f64::from_le_bytes(bytes[42..50].try_into().ok()?);
        Some(Self {
            current_target,
            target_rel,
            intention_pressure,
            satisfaction,
            last_t_us,
            commitment_until_us,
            origin_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perturbation_graph::Endpoint;

    #[test]
    fn default_is_empty() {
        let s = IntentionState::default();
        assert!(s.current_target.is_none());
        assert_eq!(s.intention_pressure, 0.0);
        assert_eq!(s.origin_score, 0.0);
    }

    #[test]
    fn try_commit_into_empty_slot_accepts() {
        let mut s = IntentionState::default();
        let ok = s.try_commit(Endpoint::Node(7), None, 0.5, 1_000_000);
        assert!(ok);
        assert_eq!(s.current_target, Some(Endpoint::Node(7)));
        assert_eq!(s.commitment_until_us, 1_000_000 + INTENTION_COMMITMENT_MIN_US);
    }

    #[test]
    fn preempt_rejected_within_commitment_window() {
        let mut s = IntentionState::default();
        s.try_commit(Endpoint::Node(1), None, 0.5, 1_000_000);
        // Same time, much stronger candidate — commitment window still holds.
        let stole = s.try_commit(Endpoint::Node(2), None, 5.0, 1_000_001);
        assert!(!stole);
        assert_eq!(s.current_target, Some(Endpoint::Node(1)));
    }

    #[test]
    fn preempt_accepted_after_commitment_with_dominance() {
        let mut s = IntentionState::default();
        s.try_commit(Endpoint::Node(1), None, 0.5, 1_000_000);
        let after = 1_000_000 + INTENTION_COMMITMENT_MIN_US + 1;
        // 0.6 is < 1.5×0.5=0.75 — should be rejected even after commitment open.
        let weak = s.try_commit(Endpoint::Node(2), None, 0.6, after);
        assert!(!weak);
        // 0.8 ≥ 0.75 — should be accepted.
        let strong = s.try_commit(Endpoint::Node(3), None, 0.8, after);
        assert!(strong);
        assert_eq!(s.current_target, Some(Endpoint::Node(3)));
    }

    #[test]
    fn pressure_rises_while_unsatisfied_and_decays_when_satisfied() {
        let mut s = IntentionState::default();
        s.try_commit(Endpoint::Node(1), None, 0.5, 1_000_000);
        for _ in 0..20 {
            s.tick(1_000_000);
        }
        let high = s.intention_pressure;
        assert!(high > 0.3, "pressure should have risen, got {}", high);
        s.update_satisfaction(0.95);
        for _ in 0..20 {
            s.tick(1_000_000);
        }
        assert!(
            s.intention_pressure < high,
            "pressure should decay once satisfied"
        );
    }

    #[test]
    fn pressure_decays_to_zero_when_target_absent() {
        let mut s = IntentionState::default();
        s.intention_pressure = 0.8;
        for _ in 0..200 {
            s.tick(1_000);
        }
        assert!(s.intention_pressure < 0.01);
    }

    #[test]
    fn serialize_roundtrip_preserves_all_fields() {
        let mut s = IntentionState::default();
        s.try_commit(Endpoint::Group(42), Some(99), 0.7, 5_000_000);
        s.update_satisfaction(0.4);
        for _ in 0..5 {
            s.tick(5_000_000);
        }
        let bytes = s.serialize();
        let back = IntentionState::deserialize(&bytes).expect("roundtrip");
        assert_eq!(back, s);
    }

    #[test]
    fn deserialize_rejects_short() {
        assert!(IntentionState::deserialize(&[0u8; 10]).is_none());
    }

    #[test]
    fn deserialize_rejects_unknown_kind() {
        let mut bytes = [0u8; IntentionState::SNAPSHOT_BYTES];
        bytes[0] = 9;
        assert!(IntentionState::deserialize(&bytes).is_none());
    }

    #[test]
    fn relation_endpoint_roundtrips() {
        let mut s = IntentionState::default();
        s.try_commit(Endpoint::Relation(7), Some(7), 0.3, 0);
        let back = IntentionState::deserialize(&s.serialize()).unwrap();
        assert_eq!(back.current_target, Some(Endpoint::Relation(7)));
    }
}
