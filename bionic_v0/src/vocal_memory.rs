#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalPrototype {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub stability: f64,
    pub reuse_success: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalMemoryState {
    pub prototypes: Vec<VocalPrototype>,
    pub last_update_t_us: u64,
}

impl VocalMemoryState {
    pub fn record_success(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.last_update_t_us = now_us;

        let matching_idx = self.prototypes.iter().position(|p| {
            let breath_diff = (p.breath_drive - breath).abs();
            let pitch_diff = (p.pitch_tension - pitch).abs();
            let resonance_diff = (p.resonance_position - resonance).abs();
            breath_diff < 0.1 && pitch_diff < 0.1 && resonance_diff < 0.1
        });

        if let Some(idx) = matching_idx {
            self.prototypes[idx].stability += 0.1;
            self.prototypes[idx].reuse_success += 1;
            self.prototypes[idx].last_update_t_us = now_us;
        } else {
            // Birth above the cull floor (0.05) with headroom: at the 10 s
            // half-life, 0.2 survives ~20 s of true silence (two half-lives)
            // before crossing 0.05, and a reinforcement (+0.1) buys more.
            // Born at 0.1 a prototype sat exactly one half-life from the floor
            // and was culled after a single quiet gap — see tick_decay.
            self.prototypes.push(VocalPrototype {
                breath_drive: breath,
                pitch_tension: pitch,
                resonance_position: resonance,
                stability: 0.2,
                reuse_success: 1,
                last_update_t_us: now_us,
            });
        }

        if self.prototypes.len() > 8 {
            self.prototypes
                .sort_by(|a, b| b.stability.partial_cmp(&a.stability).unwrap());
            self.prototypes.truncate(8);
        }
    }

    /// Distributed-training merge: fold another life's motor prototypes into
    /// this one (mainline ← branch). A prototype near an existing one (same
    /// proximity bin as record_success) reinforces it — stability and
    /// reuse_success ADD, so a gesture independently confirmed by many branches
    /// becomes the most stable; novel prototypes are absorbed, then the pool is
    /// capped to the 8 most stable. Last-writer-wins is avoided: evidence sums.
    pub fn merge(&mut self, other: &VocalMemoryState) {
        for p in &other.prototypes {
            let matching_idx = self.prototypes.iter().position(|q| {
                (q.breath_drive - p.breath_drive).abs() < 0.1
                    && (q.pitch_tension - p.pitch_tension).abs() < 0.1
                    && (q.resonance_position - p.resonance_position).abs() < 0.1
            });
            if let Some(idx) = matching_idx {
                self.prototypes[idx].stability += p.stability;
                self.prototypes[idx].reuse_success += p.reuse_success;
                self.prototypes[idx].last_update_t_us =
                    self.prototypes[idx].last_update_t_us.max(p.last_update_t_us);
            } else {
                self.prototypes.push(p.clone());
            }
            self.last_update_t_us = self.last_update_t_us.max(p.last_update_t_us);
        }
        if self.prototypes.len() > 8 {
            self.prototypes
                .sort_by(|a, b| b.stability.partial_cmp(&a.stability).unwrap());
            self.prototypes.truncate(8);
        }
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        for p in &mut self.prototypes {
            let decay_factor = 0.5_f64.powf(elapsed_us as f64 / 10_000_000.0);
            p.stability *= decay_factor;
        }
        self.prototypes.retain(|p| p.stability > 0.05);
        // Advance the decay clock so the next tick decays only the NEW elapsed
        // span. Without this the state clock stayed pinned at the last
        // record_success, so every quiet tick re-applied an ever-growing gap as
        // a multiplicative decay — annihilating fresh prototypes within a
        // fraction of a second instead of over the real 10 s half-life.
        self.last_update_t_us = now_us;
    }

    // ── Persistence ──────────────────────────────────────────────────────────
    // Binary layout:
    //   [4]  magic  b"VMEM"
    //   [1]  version  0x01
    //   [8]  n_prototypes  u64-le
    //   per prototype (48 bytes each):
    //     [8]  breath_drive     f64-le
    //     [8]  pitch_tension    f64-le
    //     [8]  resonance_pos    f64-le
    //     [8]  stability        f64-le
    //     [8]  reuse_success    u64-le
    //     [8]  last_update_t_us u64-le
    //   [8]  state.last_update_t_us  u64-le
    const MAGIC:   [u8; 4] = *b"VMEM";
    const VERSION: u8      = 0x01;
    const PROTO_BYTES: usize = 6 * 8; // 6 × f64/u64 = 48

    pub fn serialize(&self) -> Vec<u8> {
        let n = self.prototypes.len();
        let mut out = Vec::with_capacity(4 + 1 + 8 + n * Self::PROTO_BYTES + 8);
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(n as u64).to_le_bytes());
        for p in &self.prototypes {
            out.extend_from_slice(&p.breath_drive.to_le_bytes());
            out.extend_from_slice(&p.pitch_tension.to_le_bytes());
            out.extend_from_slice(&p.resonance_position.to_le_bytes());
            out.extend_from_slice(&p.stability.to_le_bytes());
            out.extend_from_slice(&(p.reuse_success as u64).to_le_bytes());
            out.extend_from_slice(&p.last_update_t_us.to_le_bytes());
        }
        out.extend_from_slice(&self.last_update_t_us.to_le_bytes());
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 4 + 1 + 8 { return None; }
        if data[0..4] != Self::MAGIC  { return None; }
        if data[4]     != Self::VERSION { return None; }

        let n = u64::from_le_bytes(data[5..13].try_into().ok()?) as usize;
        // Clamp to cap so corrupt files don't OOM.
        let n = n.min(8);
        let needed = 4 + 1 + 8 + n * Self::PROTO_BYTES + 8;
        if data.len() < needed { return None; }

        let mut off = 13usize;
        macro_rules! rd_f64 { () => {{
            let v = f64::from_le_bytes(data[off..off+8].try_into().ok()?);
            off += 8; v
        }}}
        macro_rules! rd_u64 { () => {{
            let v = u64::from_le_bytes(data[off..off+8].try_into().ok()?);
            off += 8; v
        }}}

        let mut prototypes = Vec::with_capacity(n);
        for _ in 0..n {
            prototypes.push(VocalPrototype {
                breath_drive:       rd_f64!(),
                pitch_tension:      rd_f64!(),
                resonance_position: rd_f64!(),
                stability:          rd_f64!(),
                reuse_success:      rd_u64!() as u32,
                last_update_t_us:   rd_u64!(),
            });
        }
        let last_update_t_us = rd_u64!();
        Some(VocalMemoryState { prototypes, last_update_t_us })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TDD-1b: empty state survives round-trip
    #[test]
    fn empty_roundtrip() {
        let state = VocalMemoryState::default();
        let bytes = state.serialize();
        let back  = VocalMemoryState::deserialize(&bytes).expect("deserialize empty");
        assert_eq!(back.prototypes.len(), 0);
        assert_eq!(back.last_update_t_us, 0);
    }

    // TDD-1a: state with multiple prototypes survives round-trip
    #[test]
    fn prototype_roundtrip() {
        let mut state = VocalMemoryState::default();
        state.record_success(1_000_000, 0.3, 0.5, 0.7);
        state.record_success(2_000_000, 0.1, 0.9, 0.2);
        state.record_success(3_000_000, 0.3, 0.5, 0.7); // same bin → reinforce
        let bytes = state.serialize();
        let back  = VocalMemoryState::deserialize(&bytes).expect("deserialize protos");
        assert_eq!(back.prototypes.len(), state.prototypes.len());
        for (a, b) in state.prototypes.iter().zip(back.prototypes.iter()) {
            assert!((a.breath_drive       - b.breath_drive      ).abs() < 1e-12);
            assert!((a.pitch_tension      - b.pitch_tension     ).abs() < 1e-12);
            assert!((a.resonance_position - b.resonance_position).abs() < 1e-12);
            assert!((a.stability          - b.stability         ).abs() < 1e-12);
            assert_eq!(a.reuse_success,    b.reuse_success);
            assert_eq!(a.last_update_t_us, b.last_update_t_us);
        }
        assert_eq!(back.last_update_t_us, state.last_update_t_us);
    }

    // TDD-1c: 8-prototype cap survives round-trip and all values are preserved exactly
    #[test]
    fn cap_and_stability_roundtrip() {
        let mut state = VocalMemoryState::default();
        // Push 10 distinct prototypes — cap should keep at most 8.
        for i in 0..10u64 {
            let pitch = i as f64 * 0.1;
            // Record each i+1 times so stability grows with i.
            for _ in 0..=(i as u32) {
                state.record_success(i * 1_000_000 + 1, 0.2, pitch, 0.4);
            }
        }
        assert!(state.prototypes.len() <= 8, "cap exceeded: {}", state.prototypes.len());

        let bytes = state.serialize();
        let back  = VocalMemoryState::deserialize(&bytes).expect("deserialize capped");
        // Round-trip preserves count and exact values in original order.
        assert_eq!(back.prototypes.len(), state.prototypes.len());
        for (a, b) in state.prototypes.iter().zip(back.prototypes.iter()) {
            assert!((a.pitch_tension - b.pitch_tension).abs() < 1e-12);
            assert!((a.stability     - b.stability    ).abs() < 1e-12);
            assert_eq!(a.reuse_success, b.reuse_success);
        }
        // All retained prototypes should have stability above the decay threshold.
        for p in &back.prototypes {
            assert!(p.stability > 0.05, "sub-threshold prototype: {}", p.stability);
        }
    }

    #[test]
    fn merge_reinforces_shared_and_absorbs_novel() {
        let mut mainline = VocalMemoryState::default();
        mainline.record_success(1, 0.3, 0.5, 0.7); // shared gesture, stability 0.1
        let mut branch = VocalMemoryState::default();
        branch.record_success(2, 0.3, 0.5, 0.7); // same gesture (reinforces)
        branch.record_success(2, 0.8, 0.2, 0.4); // novel gesture
        mainline.merge(&branch);
        assert_eq!(mainline.prototypes.len(), 2, "shared merged, novel absorbed");
        // The shared prototype's evidence summed (0.1 + 0.1) and reuse added.
        let shared = mainline.prototypes.iter()
            .find(|p| (p.pitch_tension - 0.5).abs() < 0.05).unwrap();
        assert!(shared.stability > 0.15, "shared gesture reinforced, got {}", shared.stability);
        assert!(shared.reuse_success >= 2, "reuse evidence summed, got {}", shared.reuse_success);
    }

    #[test]
    fn merge_caps_at_eight_most_stable() {
        let mut a = VocalMemoryState::default();
        let mut b = VocalMemoryState::default();
        for i in 0..6u64 { a.record_success(1, 0.1 * i as f64, 0.5, 0.4); }
        for i in 0..6u64 { b.record_success(2, 0.1 * i as f64, 0.2, 0.7); }
        a.merge(&b);
        assert!(a.prototypes.len() <= 8, "merge respects the 8-prototype cap, got {}", a.prototypes.len());
    }

    // TDD-1d: corrupt / truncated data returns None safely
    #[test]
    fn corrupt_data_returns_none() {
        assert!(VocalMemoryState::deserialize(&[]).is_none());
        assert!(VocalMemoryState::deserialize(b"JUNK").is_none());
        // Valid magic+version but truncated body.
        let good = VocalMemoryState::default().serialize();
        assert!(VocalMemoryState::deserialize(&good[..good.len()-1]).is_none());
    }
}
