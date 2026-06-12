//! vocal_target_recall — Phase 2 increment 1 (2026-06-12): content-addressed
//! motor recall, the foundation of the sequence layer.
//!
//! Why this exists (measured): the acoustic servo proved it CAN reduce shape
//! distance on a held target (tests/phase0_vocal_loop_diagnosis.rs), but in the
//! live nursery — whose curriculum cycles the SAME vowels repeatedly — it never
//! helped, because the 90 ms hill-climb cold-starts on every presentation and
//! the target moves on before it converges. The body kept re-solving a problem
//! it had already solved seconds ago.
//!
//! This memory stores, keyed by the TARGET's 64-mel fingerprint, the servo
//! correction (F1/F2 offset) that achieved a good fit. When a familiar target
//! recurs, the servo is WARM-STARTED to the remembered correction instead of
//! climbing from zero — turning "static-target capability" into "fast matching
//! on recurring targets". It is content-addressed recall (target → motor); the
//! transition/sequence links and eligibility traces build on top of it.
//!
//! No labels, no reward: the stored fit is a prediction-error-derived quantity;
//! recall is nearest-neighbour by spectral shape distance. Pure substrate.

use crate::vocal_self_analysis::{spectral_shape_distance_mel64, BANDS};

/// Max stored targets. Small: a developmental vocabulary of vowel/tone shapes,
/// not a database. Least-recently-useful is evicted at the cap.
pub const RECALL_CAP: usize = 16;
/// A target within this mel64 shape distance of a stored key counts as "the
/// same sound" for recall. Measured: spectral_shape_distance_mel64 is small —
/// clearly-distinct vowels sit ≈0.06 apart, a re-presented identical target ≈0
/// — so the match radius must be tight (0.04) to recall the same sound without
/// collapsing neighbours.
pub const RECALL_MATCH_DIST: f64 = 0.04;
/// Floor for memorising a correction. NOT an absolute "good fit" gate — the live
/// self↔target distance has a high floor (≈0.48) dominated by voice-QUALITY
/// mismatch (the body's glottal source vs the caregiver's timbre), which the
/// formant servo cannot close. So we keep the BEST-SO-FAR correction per target
/// (see `store`) and only skip obvious pre-voicing garbage below this floor.
pub const RECALL_STORE_MIN_FIT: f64 = 0.30;

#[derive(Clone, Debug, PartialEq)]
pub struct RecallEntry {
    pub target_mel64: [f64; BANDS],
    pub f1_off: f64,
    pub f2_off: f64,
    /// Best fit (1 - shape_dist) this correction has achieved for this target.
    pub fit: f64,
    pub last_update_t_us: u64,
}

impl Default for RecallEntry {
    // [f64; 64] has no derived Default (arrays > 32), so spell it out.
    fn default() -> Self {
        Self {
            target_mel64: [0.0; BANDS],
            f1_off: 0.0,
            f2_off: 0.0,
            fit: 0.0,
            last_update_t_us: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalTargetRecall {
    pub entries: Vec<RecallEntry>,
}

impl VocalTargetRecall {
    /// Nearest stored entry to `target` within RECALL_MATCH_DIST. Returns
    /// (index, distance). None if the target is novel (nothing close enough).
    pub fn recall(&self, target: &[f64; BANDS]) -> Option<(usize, f64)> {
        if target.iter().sum::<f64>() <= 1e-6 {
            return None; // no fingerprint → can't key
        }
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.entries.iter().enumerate() {
            let d = spectral_shape_distance_mel64(target, &e.target_mel64);
            if d <= RECALL_MATCH_DIST && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best
    }

    /// Memorise that (f1_off, f2_off) achieved `fit` for `target`. If a near
    /// entry exists, refine it only when the new fit is at least as good (keep
    /// the best-known correction); otherwise insert, evicting the
    /// least-recently-updated entry at the cap.
    pub fn store(
        &mut self,
        now_us: u64,
        target: &[f64; BANDS],
        f1_off: f64,
        f2_off: f64,
        fit: f64,
    ) {
        if fit < RECALL_STORE_MIN_FIT || target.iter().sum::<f64>() <= 1e-6 {
            return;
        }
        if let Some((idx, _)) = self.recall(target) {
            if fit >= self.entries[idx].fit {
                let e = &mut self.entries[idx];
                e.f1_off = f1_off;
                e.f2_off = f2_off;
                e.fit = fit;
                e.last_update_t_us = now_us;
                // Blend the key toward the observed target so it tracks drift.
                for (k, &t) in e.target_mel64.iter_mut().zip(target.iter()) {
                    *k = *k * 0.8 + t * 0.2;
                }
            }
            return;
        }
        if self.entries.len() >= RECALL_CAP {
            // Evict least-recently-updated.
            if let Some((lru, _)) = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.last_update_t_us)
            {
                self.entries.swap_remove(lru);
            }
        }
        self.entries.push(RecallEntry {
            target_mel64: *target,
            f1_off,
            f2_off,
            fit,
            last_update_t_us: now_us,
        });
    }

    // ── Persistence ──────────────────────────────────────────────────────────
    // Mirrors vocal_memory.bin so learned target→correction mappings ACCUMULATE
    // across nursery restarts instead of being re-solved from zero every session
    // (the project's standing "doesn't grow across sessions" pain). Layout:
    //   [4] magic b"VTRC"  [1] version  [8] n_entries u64-le
    //   per entry: [64×8] target_mel64 f64-le, [8] f1_off, [8] f2_off, [8] fit,
    //              [8] last_update_t_us u64-le   (= 544 bytes)
    const MAGIC: [u8; 4] = *b"VTRC";
    const VERSION: u8 = 0x01;
    const ENTRY_BYTES: usize = BANDS * 8 + 4 * 8;

    pub fn serialize(&self) -> Vec<u8> {
        let n = self.entries.len();
        let mut out = Vec::with_capacity(4 + 1 + 8 + n * Self::ENTRY_BYTES);
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(n as u64).to_le_bytes());
        for e in &self.entries {
            for v in &e.target_mel64 {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&e.f1_off.to_le_bytes());
            out.extend_from_slice(&e.f2_off.to_le_bytes());
            out.extend_from_slice(&e.fit.to_le_bytes());
            out.extend_from_slice(&e.last_update_t_us.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 13 || data[0..4] != Self::MAGIC || data[4] != Self::VERSION {
            return None;
        }
        let n = (u64::from_le_bytes(data[5..13].try_into().ok()?) as usize).min(RECALL_CAP);
        if data.len() < 13 + n * Self::ENTRY_BYTES {
            return None;
        }
        let mut off = 13usize;
        let mut rd_f64 = |off: &mut usize| {
            let v = f64::from_le_bytes(data[*off..*off + 8].try_into().unwrap());
            *off += 8;
            v
        };
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            let mut mel = [0.0; BANDS];
            for m in mel.iter_mut() {
                *m = rd_f64(&mut off);
            }
            let f1_off = rd_f64(&mut off);
            let f2_off = rd_f64(&mut off);
            let fit = rd_f64(&mut off);
            // u64 field: read the raw bytes as u64, NOT via the f64 reader.
            let last_update_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            entries.push(RecallEntry {
                target_mel64: mel,
                f1_off,
                f2_off,
                fit,
                last_update_t_us,
            });
        }
        Some(VocalTargetRecall { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mel_with_peak(peak_bin: usize) -> [f64; BANDS] {
        let mut m = [0.0; BANDS];
        for (i, v) in m.iter_mut().enumerate() {
            let d = i as f64 - peak_bin as f64;
            *v = (-(d * d) / 18.0).exp(); // gaussian bump → a "formant"
        }
        m
    }

    #[test]
    fn novel_target_has_no_recall() {
        let mem = VocalTargetRecall::default();
        assert!(mem.recall(&mel_with_peak(20)).is_none());
    }

    #[test]
    fn stores_and_recalls_matching_target() {
        let mut mem = VocalTargetRecall::default();
        let a = mel_with_peak(15);
        mem.store(1_000, &a, 0.10, 0.30, 0.85);
        let (idx, dist) = mem.recall(&a).expect("should recall the stored target");
        assert_eq!(idx, 0);
        assert!(dist < 1e-9);
        assert!((mem.entries[idx].f2_off - 0.30).abs() < 1e-9);
    }

    #[test]
    fn distinct_targets_recall_distinct_offsets() {
        let mut mem = VocalTargetRecall::default();
        let a = mel_with_peak(12);
        let b = mel_with_peak(40); // far enough → distinct key
        mem.store(1, &a, 0.0, 0.20, 0.80);
        mem.store(2, &b, 0.0, -0.30, 0.80);
        assert_eq!(mem.entries.len(), 2, "distinct targets must not collapse");
        let (ia, _) = mem.recall(&a).unwrap();
        let (ib, _) = mem.recall(&b).unwrap();
        assert!((mem.entries[ia].f2_off - 0.20).abs() < 1e-9);
        assert!((mem.entries[ib].f2_off + 0.30).abs() < 1e-9);
    }

    #[test]
    fn low_fit_is_not_stored() {
        let mut mem = VocalTargetRecall::default();
        mem.store(1, &mel_with_peak(20), 0.1, 0.1, 0.20); // below RECALL_STORE_MIN_FIT
        assert!(mem.entries.is_empty(), "a correction that didn't work must not be memorised");
    }

    #[test]
    fn serialize_roundtrip_preserves_entries() {
        let mut mem = VocalTargetRecall::default();
        mem.store(1, &mel_with_peak(12), 0.10, 0.30, 0.85);
        mem.store(2, &mel_with_peak(40), -0.05, 0.22, 0.78);
        let back = VocalTargetRecall::deserialize(&mem.serialize()).expect("roundtrip");
        assert_eq!(back.entries.len(), 2);
        for (a, b) in mem.entries.iter().zip(back.entries.iter()) {
            assert!((a.f1_off - b.f1_off).abs() < 1e-12);
            assert!((a.f2_off - b.f2_off).abs() < 1e-12);
            assert!((a.fit - b.fit).abs() < 1e-12);
            assert_eq!(a.last_update_t_us, b.last_update_t_us);
            for (x, y) in a.target_mel64.iter().zip(b.target_mel64.iter()) {
                assert!((x - y).abs() < 1e-12);
            }
        }
        // A recalled target still resolves to the same correction after reload.
        let (idx, _) = back.recall(&mel_with_peak(12)).expect("recall after reload");
        assert!((back.entries[idx].f2_off - 0.30).abs() < 1e-9);
    }

    #[test]
    fn deserialize_rejects_corrupt() {
        assert!(VocalTargetRecall::deserialize(&[]).is_none());
        assert!(VocalTargetRecall::deserialize(b"JUNKjunkjunk").is_none());
        // A header claiming entries but with a truncated body must be rejected.
        let mut mem = VocalTargetRecall::default();
        mem.store(1, &mel_with_peak(20), 0.1, 0.2, 0.9);
        let bytes = mem.serialize();
        assert!(VocalTargetRecall::deserialize(&bytes[..bytes.len() - 1]).is_none());
        // The empty (header-only) blob round-trips to empty.
        let empty = VocalTargetRecall::deserialize(&VocalTargetRecall::default().serialize());
        assert!(empty.map(|m| m.entries.is_empty()).unwrap_or(false));
    }

    #[test]
    fn refine_keeps_best_fit() {
        let mut mem = VocalTargetRecall::default();
        let a = mel_with_peak(25);
        mem.store(1, &a, 0.0, 0.10, 0.75);
        mem.store(2, &a, 0.0, 0.22, 0.90); // better fit → adopt
        let (idx, _) = mem.recall(&a).unwrap();
        assert!((mem.entries[idx].f2_off - 0.22).abs() < 1e-9);
        mem.store(3, &a, 0.0, 0.99, 0.60); // worse fit → ignore
        let (idx, _) = mem.recall(&a).unwrap();
        assert!((mem.entries[idx].f2_off - 0.22).abs() < 1e-9, "worse fit must not overwrite");
    }
}
