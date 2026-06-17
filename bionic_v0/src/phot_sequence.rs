//! phot_sequence — visual impression / temporal-context layer (Q2/3 of the
//! 2026-06-18 impression-memory design pass). The symmetric counterpart to
//! `vocal_sequence`: same three layers (short-window impression buffer,
//! per-edge eligibility trace, persistent transition edges), now over visual
//! scene-fingerprint nodes instead of vocal_target_recall indices. The user-
//! facing motivation is "刚刚看到的画面应该留点印象": the body should know
//! which scene-shape it was just seeing, and a re-encountered shape should be
//! recognised as a repeat rather than re-discovered from zero.
//!
//! Node model. The acoustic layer keys edges on a stable `recall_idx` because
//! vocal_target_recall hands out content-addressed indices. The visual side
//! has no such address book — Gaussians live in a slot pool with implicit
//! reuse on death+rebirth. We mint a `stable_id: u64` per Gaussian at birth
//! (a monotonic counter on the field; see phot_gaussian.rs) so that
//! "is this the same primitive across frames" becomes well-defined; and we
//! summarise the whole active-set into a 64-bit `scene_fingerprint(field)`
//! which IS the edge node. A scene that re-appears hashes to the same node
//! and reinforces the existing edges into/out of it — the "compression" the
//! user asked for ("下次见到同一场景应被压缩 直接理解").
//!
//! Numeric/local-learning only — count EMAs + eligibility traces + FNV-1a
//! hashing, no backprop. The fingerprint buffer is a `VecDeque<u64>` (not raw
//! pixels) so persistence is cheap (8 bytes/frame × 18 ≈ 144 B vs ~1.8 MB
//! per-frame splat).

use crate::phot_gaussian::GaussianField;
use std::collections::VecDeque;

/// Max persistent transition edges (parallel to SEQ_EDGE_CAP=256 on the
/// acoustic side; visual scenes turn over more slowly per second, but
/// long-tail rare scenes still benefit from headroom).
pub const GAUSS_SEQ_EDGE_CAP: usize = 512;
/// Eligibility-trace τ. At 30 Hz a saccade-to-saccade gap is ~300-500 ms;
/// 600 ms covers ~18 frames before eligibility halves. Larger than the
/// acoustic 400 ms because visual transitions are intrinsically slower.
pub const GAUSS_SEQ_ELIG_TAU_US: f64 = 600_000.0;
/// Two scene nodes farther apart than this in real time do NOT form an edge.
/// 800 ms — slightly larger than vocal's 600 ms to match visual's longer
/// natural transition pace (object appears / hand moves / face turns).
pub const GAUSS_SEQ_MAX_INTERONSET_US: u64 = 800_000;
/// Impression-buffer depth (~600 ms at the 30 Hz visual tick), mirrors the
/// acoustic short-window buffer that "remembers" what was just seen.
pub const GAUSS_UTT_CAP: usize = 18;
/// Debounce: ignore scene-changes faster than this so jitter / noise frames
/// don't pollute the impression and edge table. 33 ms ≈ one camera frame.
pub const GAUSS_SEQ_MIN_INTER_NODE_US: u64 = 33_000;
/// Age-protection window for the visual evict_weakest, identical role to
/// SEQ_AGE_PROTECT_US on the acoustic side.
pub const GAUSS_AGE_PROTECT_US: f64 = 6.0 * 3600.0 * 1_000_000.0;
const GAUSS_AGE_PROTECT_MAX_BONUS: f64 = 0.30;
const GAUSS_COUNT_ALPHA: f64 = 0.2;
/// Position quantisation when folding a Gaussian's μ into the fingerprint:
/// coarse enough that small jitter on the same primitive collapses to the
/// same bucket, fine enough that a moved primitive hashes differently.
const GAUSS_FP_POS_QUANT: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotSeqEdge {
    /// 64-bit scene fingerprint at the previous / next node.
    pub from_hash: u64,
    pub to_hash: u64,
    /// EMA in [0,1] of how often this transition has been observed.
    pub count_ema: f64,
    /// Learned inter-node interval (µs) for from→to.
    pub dt_ema_us: f64,
    /// Eligibility trace: 1.0 right after traversal, decays with GAUSS_SEQ_ELIG_TAU_US.
    pub elig: f64,
    pub last_t_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhotSequence {
    /// Sparse directed transition table over scene-fingerprint nodes.
    pub edges: Vec<PhotSeqEdge>,
    /// Rolling impression buffer: the last GAUSS_UTT_CAP scene fingerprints
    /// the body saw, with timestamps. This IS the "刚刚看到的画面留有印象"
    /// surface — observers / downstream learners can read it to know what
    /// scene-shapes have been active in the recent window.
    pub fingerprints: VecDeque<u64>,
    pub fingerprint_t_us: VecDeque<u64>,
    /// Cursor on the most recent committed scene node — seeds the from-side
    /// of the next edge. 0 = no node committed yet (sentinel; valid scene
    /// hashes are forced non-zero).
    pub last_node_hash: u64,
    pub last_node_t_us: u64,
}

impl Default for PhotSequence {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            fingerprints: VecDeque::new(),
            fingerprint_t_us: VecDeque::new(),
            last_node_hash: 0,
            last_node_t_us: 0,
        }
    }
}

impl PhotSequence {
    /// Record the body just observed scene fingerprint `cur_hash` at `now_us`.
    /// A genuine change between two known nodes within the inter-onset window
    /// is an edge: bump (prev→cur)'s count EMA, learn its timing, set its
    /// eligibility high. Idempotent within GAUSS_SEQ_MIN_INTER_NODE_US so a
    /// stable scene held across many frames does not record spurious edges.
    pub fn observe_scene(&mut self, now_us: u64, cur_hash: u64) {
        if cur_hash == 0 {
            return; // sentinel — empty scene / nothing to anchor on
        }
        if self.fingerprints.len() >= GAUSS_UTT_CAP {
            self.fingerprints.pop_front();
            self.fingerprint_t_us.pop_front();
        }
        self.fingerprints.push_back(cur_hash);
        self.fingerprint_t_us.push_back(now_us);

        let prev = self.last_node_hash;
        if prev == cur_hash {
            return; // same scene held — onset clock unchanged
        }
        // Debounce: too-fast changes are jitter, not transitions.
        if prev != 0 && now_us.saturating_sub(self.last_node_t_us) < GAUSS_SEQ_MIN_INTER_NODE_US {
            // Keep the impression buffer current but do NOT commit an edge.
            return;
        }
        if prev != 0 {
            let dt = now_us.saturating_sub(self.last_node_t_us);
            if dt <= GAUSS_SEQ_MAX_INTERONSET_US {
                if let Some(e) = self
                    .edges
                    .iter_mut()
                    .find(|e| e.from_hash == prev && e.to_hash == cur_hash)
                {
                    e.count_ema += GAUSS_COUNT_ALPHA * (1.0 - e.count_ema);
                    e.dt_ema_us = e.dt_ema_us * 0.8 + dt as f64 * 0.2;
                    e.elig = 1.0;
                    e.last_t_us = now_us;
                } else {
                    if self.edges.len() >= GAUSS_SEQ_EDGE_CAP {
                        self.evict_weakest(now_us);
                    }
                    self.edges.push(PhotSeqEdge {
                        from_hash: prev,
                        to_hash: cur_hash,
                        count_ema: GAUSS_COUNT_ALPHA,
                        dt_ema_us: dt as f64,
                        elig: 1.0,
                        last_t_us: now_us,
                    });
                }
            }
        }
        self.last_node_hash = cur_hash;
        self.last_node_t_us = now_us;
    }

    fn evict_weakest(&mut self, now_us: u64) {
        if let Some(min_i) = self
            .edges
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                Self::eviction_defence(a, now_us)
                    .partial_cmp(&Self::eviction_defence(b, now_us))
                    .unwrap()
            })
            .map(|(i, _)| i)
        {
            self.edges.swap_remove(min_i);
        }
    }

    fn eviction_defence(e: &PhotSeqEdge, now_us: u64) -> f64 {
        let age = now_us.saturating_sub(e.last_t_us) as f64;
        let bonus = (age / GAUSS_AGE_PROTECT_US).min(1.0) * GAUSS_AGE_PROTECT_MAX_BONUS;
        e.count_ema + bonus
    }

    /// Most-likely successor scene and its learned inter-node interval (µs).
    pub fn predict_next(&self, from_hash: u64) -> Option<(u64, f64)> {
        if from_hash == 0 {
            return None;
        }
        self.edges
            .iter()
            .filter(|e| e.from_hash == from_hash)
            .max_by(|a, b| a.count_ema.partial_cmp(&b.count_ema).unwrap())
            .map(|e| (e.to_hash, e.dt_ema_us))
    }

    /// Multi-step rollout — cycle-safe, mirrors vocal_sequence::rollout.
    pub fn rollout(&self, from_hash: u64, k_steps: usize) -> Vec<(u64, f64)> {
        if from_hash == 0 || k_steps == 0 {
            return Vec::new();
        }
        let mut out: Vec<(u64, f64)> = Vec::with_capacity(k_steps);
        let mut visited: Vec<u64> = Vec::with_capacity(k_steps + 1);
        let mut cursor = from_hash;
        visited.push(cursor);
        for _ in 0..k_steps {
            let next = self
                .edges
                .iter()
                .filter(|e| e.from_hash == cursor)
                .max_by(|a, b| a.count_ema.partial_cmp(&b.count_ema).unwrap());
            let Some(e) = next else { break };
            if visited.contains(&e.to_hash) {
                break;
            }
            out.push((e.to_hash, e.dt_ema_us));
            visited.push(e.to_hash);
            cursor = e.to_hash;
        }
        out
    }

    /// Refresh eligibility traces as a pure function of time-since-traversal.
    pub fn decay_eligibility(&mut self, now_us: u64) {
        for e in self.edges.iter_mut() {
            let dt = now_us.saturating_sub(e.last_t_us) as f64;
            e.elig = (-dt / GAUSS_SEQ_ELIG_TAU_US).exp();
        }
    }

    /// Distributed-training merge — same shape as VocalSequence::merge.
    pub fn merge(&mut self, other: &PhotSequence) {
        for oe in &other.edges {
            if let Some(e) = self
                .edges
                .iter_mut()
                .find(|e| e.from_hash == oe.from_hash && e.to_hash == oe.to_hash)
            {
                if oe.count_ema > e.count_ema {
                    e.count_ema = oe.count_ema;
                    e.dt_ema_us = oe.dt_ema_us;
                    e.last_t_us = e.last_t_us.max(oe.last_t_us);
                }
            } else if self.edges.len() < GAUSS_SEQ_EDGE_CAP {
                self.edges.push(*oe);
            } else if let Some((min_i, min_c)) = self
                .edges
                .iter()
                .enumerate()
                .map(|(i, e)| (i, e.count_ema))
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            {
                if min_c < oe.count_ema {
                    self.edges.swap_remove(min_i);
                    self.edges.push(*oe);
                }
            }
        }
    }

    // ── Persistence ──────────────────────────────────────────────────────────
    // Sidecar runtime/phot_sequence.bin (NOT the global snapshot — bumping
    // SNAPSHOT_VERSION would reject every existing snapshot.bin and reset
    // maturity). Mirrors the acoustic vocal_sequence.bin layout.
    //
    // Layout v1:
    //   [4] magic b"PSEQ"  [1] version=0x01  [8] n_edges u64-le
    //   per edge: [8 from_hash][8 to_hash][8 count_ema][8 dt_ema_us][8 elig]
    //             [8 last_t_us] = 48 bytes
    //   [8] last_node_hash u64-le
    //   [8] last_node_t_us u64-le
    //   [8] n_fingerprints u64-le (≤ GAUSS_UTT_CAP)
    //   per fp: [8 hash][8 t_us] = 16 bytes
    const MAGIC: [u8; 4] = *b"PSEQ";
    const VERSION: u8 = 0x01;
    const EDGE_BYTES: usize = 8 + 8 + 8 + 8 + 8 + 8;

    pub fn serialize(&self) -> Vec<u8> {
        let n = self.edges.len();
        let nfp = self.fingerprints.len().min(GAUSS_UTT_CAP);
        let mut out = Vec::with_capacity(
            4 + 1 + 8 + n * Self::EDGE_BYTES + 8 + 8 + 8 + nfp * 16,
        );
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(n as u64).to_le_bytes());
        for e in &self.edges {
            out.extend_from_slice(&e.from_hash.to_le_bytes());
            out.extend_from_slice(&e.to_hash.to_le_bytes());
            out.extend_from_slice(&e.count_ema.to_le_bytes());
            out.extend_from_slice(&e.dt_ema_us.to_le_bytes());
            out.extend_from_slice(&e.elig.to_le_bytes());
            out.extend_from_slice(&e.last_t_us.to_le_bytes());
        }
        out.extend_from_slice(&self.last_node_hash.to_le_bytes());
        out.extend_from_slice(&self.last_node_t_us.to_le_bytes());
        out.extend_from_slice(&(nfp as u64).to_le_bytes());
        for (h, t) in self
            .fingerprints
            .iter()
            .zip(self.fingerprint_t_us.iter())
            .take(nfp)
        {
            out.extend_from_slice(&h.to_le_bytes());
            out.extend_from_slice(&t.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 13 || data[0..4] != Self::MAGIC || data[4] != Self::VERSION {
            return None;
        }
        let n =
            (u64::from_le_bytes(data[5..13].try_into().ok()?) as usize).min(GAUSS_SEQ_EDGE_CAP);
        if data.len() < 13 + n * Self::EDGE_BYTES {
            return None;
        }
        let mut off = 13usize;
        let mut edges = Vec::with_capacity(n);
        for _ in 0..n {
            let from_hash = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let to_hash = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let count_ema = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let dt_ema_us = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let elig = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let last_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            edges.push(PhotSeqEdge {
                from_hash,
                to_hash,
                count_ema,
                dt_ema_us,
                elig,
                last_t_us,
            });
        }
        // Transient impression block (strict — any underrun rejects the file
        // rather than commit partial state; matches vocal_sequence v2 policy).
        if data.len() < off + 8 + 8 + 8 {
            return None;
        }
        let last_node_hash = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let last_node_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let nfp_raw = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
        off += 8;
        if nfp_raw > GAUSS_UTT_CAP {
            return None;
        }
        if data.len() < off + nfp_raw * 16 {
            return None;
        }
        let mut fingerprints: VecDeque<u64> = VecDeque::with_capacity(nfp_raw);
        let mut fingerprint_t_us: VecDeque<u64> = VecDeque::with_capacity(nfp_raw);
        for _ in 0..nfp_raw {
            let h = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let t = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            fingerprints.push_back(h);
            fingerprint_t_us.push_back(t);
        }
        Some(PhotSequence {
            edges,
            fingerprints,
            fingerprint_t_us,
            last_node_hash,
            last_node_t_us,
        })
    }
}

/// 64-bit fingerprint of the currently active set of Gaussians. Folds each
/// active primitive's stable_id and a coarse-quantised pose+color into an
/// FNV-1a hash. Re-occurrence of the same active set (same primitives, same
/// rough poses) maps to the same fingerprint — that IS how the impression
/// layer "compresses" a re-encountered scene into a known node.
///
/// Two stability invariants:
///  • Order-independence: the contribution per primitive is accumulated and
///    XOR'd in at the end, so a permutation of `gaussians[..]` (which the
///    field's birth/death cycle does produce) yields the same hash.
///  • Quantisation tolerance: μ at quant 1/16 and color at 1/16 means jitter
///    on a held primitive does not split the scene into a new node.
///
/// Returns 0 when there are NO active primitives (the sentinel for "empty
/// scene — no node to anchor on"), and never returns 0 otherwise (a non-empty
/// scene whose accumulated bits happen to fold to 0 is bumped to 1).
pub fn scene_fingerprint(field: &GaussianField) -> u64 {
    let mut acc: u64 = 0;
    let mut n_active: u32 = 0;
    for g in field.gaussians.iter() {
        if !g.active {
            continue;
        }
        n_active += 1;
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        h ^= g.stable_id;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        let qx = (g.mu_x * GAUSS_FP_POS_QUANT).round().clamp(0.0, 1.0e9) as i64;
        let qy = (g.mu_y * GAUSS_FP_POS_QUANT).round().clamp(0.0, 1.0e9) as i64;
        let qsx = (g.sx * GAUSS_FP_POS_QUANT).round().clamp(0.0, 1.0e9) as i64;
        let qsy = (g.sy * GAUSS_FP_POS_QUANT).round().clamp(0.0, 1.0e9) as i64;
        for q in [qx, qy, qsx, qsy] {
            h ^= q as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        for c in g.color.iter() {
            let qc = (c * GAUSS_FP_POS_QUANT).round().clamp(0.0, 1.0e9) as i64;
            h ^= qc as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        acc ^= h;
    }
    if n_active == 0 {
        return 0;
    }
    // Fold the active count too so a 3-Gaussian scene cannot accidentally
    // collide with a 7-Gaussian scene that XOR-cancels to the same acc.
    acc ^= (n_active as u64).wrapping_mul(0x0000_0100_0000_01b3);
    if acc == 0 {
        1
    } else {
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phot_gaussian::{Gaussian, GaussianField};

    fn gauss(stable_id: u64, mu_x: f64, mu_y: f64) -> Gaussian {
        Gaussian {
            mu_x,
            mu_y,
            sx: 0.05,
            sy: 0.05,
            color: [0.5, 0.5, 0.5],
            opacity: 1.0,
            active: true,
            inv_depth: 1.0,
            depth_observable: false,
            prev_mu_x: mu_x,
            stable_id,
        }
    }

    fn field_with(gaussians: Vec<Gaussian>) -> GaussianField {
        let mut f = GaussianField::new();
        for (i, g) in gaussians.into_iter().enumerate() {
            if i < f.gaussians.len() {
                f.gaussians[i] = g;
            }
        }
        f
    }

    #[test]
    fn empty_field_hashes_to_zero_sentinel() {
        let f = GaussianField::new();
        assert_eq!(scene_fingerprint(&f), 0);
    }

    #[test]
    fn same_scene_hashes_identically_under_jitter() {
        let f1 = field_with(vec![gauss(1, 0.300, 0.300), gauss(2, 0.700, 0.700)]);
        // Tiny jitter below the quant grid (1/16 = 0.0625) must NOT change hash.
        let f2 = field_with(vec![gauss(1, 0.301, 0.301), gauss(2, 0.701, 0.701)]);
        assert_eq!(scene_fingerprint(&f1), scene_fingerprint(&f2));
    }

    #[test]
    fn permutation_invariance() {
        // The field's birth/death cycle can reorder slots; hash must be
        // order-independent over the active set.
        let f1 = field_with(vec![gauss(1, 0.3, 0.3), gauss(2, 0.7, 0.7)]);
        let f2 = field_with(vec![gauss(2, 0.7, 0.7), gauss(1, 0.3, 0.3)]);
        assert_eq!(scene_fingerprint(&f1), scene_fingerprint(&f2));
    }

    #[test]
    fn distinct_active_count_separates_scenes() {
        // Active count is folded so 3 vs 7 primitives cannot XOR-collide.
        let f1 = field_with(vec![gauss(1, 0.3, 0.3)]);
        let f2 = field_with(vec![gauss(1, 0.3, 0.3), gauss(2, 0.7, 0.7)]);
        assert_ne!(scene_fingerprint(&f1), scene_fingerprint(&f2));
    }

    #[test]
    fn observe_scene_learns_a_transition_after_debounce() {
        let mut seq = PhotSequence::default();
        seq.observe_scene(0, 0xAAA);
        // Within debounce window — no edge.
        seq.observe_scene(GAUSS_SEQ_MIN_INTER_NODE_US - 1, 0xBBB);
        assert!(
            seq.edges.is_empty(),
            "transitions inside the debounce window must NOT commit edges"
        );
        // Past debounce window — commits.
        let mut seq = PhotSequence::default();
        seq.observe_scene(0, 0xAAA);
        seq.observe_scene(200_000, 0xBBB);
        assert_eq!(seq.edges.len(), 1);
        assert_eq!(seq.edges[0].from_hash, 0xAAA);
        assert_eq!(seq.edges[0].to_hash, 0xBBB);
    }

    #[test]
    fn no_edge_across_too_large_a_gap() {
        let mut seq = PhotSequence::default();
        seq.observe_scene(0, 0xAAA);
        seq.observe_scene(GAUSS_SEQ_MAX_INTERONSET_US + 1, 0xBBB);
        assert!(seq.edges.is_empty());
    }

    #[test]
    fn stable_scene_does_not_spawn_edges() {
        let mut seq = PhotSequence::default();
        for k in 0..100 {
            seq.observe_scene(k * 33_000, 0xAAA);
        }
        assert!(seq.edges.is_empty(), "holding one scene must not spawn self-loops");
    }

    #[test]
    fn rollout_walks_chain_and_stops_on_cycle() {
        let mut seq = PhotSequence::default();
        // Train A→B→C→D heavy.
        for k in 0..5 {
            let base = k * 1_000_000;
            seq.observe_scene(base, 0xAAA);
            seq.observe_scene(base + 200_000, 0xBBB);
            seq.observe_scene(base + 400_000, 0xCCC);
            seq.observe_scene(base + 600_000, 0xDDD);
        }
        let chain = seq.rollout(0xAAA, 4);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].0, 0xBBB);
        assert_eq!(chain[1].0, 0xCCC);
        assert_eq!(chain[2].0, 0xDDD);
    }

    #[test]
    fn serialize_roundtrip_preserves_state() {
        let mut seq = PhotSequence::default();
        seq.observe_scene(0, 0xAAA);
        seq.observe_scene(200_000, 0xBBB);
        seq.observe_scene(400_000, 0xCCC);
        let back = PhotSequence::deserialize(&seq.serialize()).expect("roundtrip");
        assert_eq!(back.edges.len(), seq.edges.len());
        for (a, b) in seq.edges.iter().zip(back.edges.iter()) {
            assert_eq!(a.from_hash, b.from_hash);
            assert_eq!(a.to_hash, b.to_hash);
            assert!((a.count_ema - b.count_ema).abs() < 1e-12);
        }
        assert_eq!(back.last_node_hash, 0xCCC);
        assert_eq!(back.fingerprints, seq.fingerprints);
    }

    #[test]
    fn deserialize_rejects_corrupt() {
        assert!(PhotSequence::deserialize(&[]).is_none());
        assert!(PhotSequence::deserialize(b"JUNKjunkjunk").is_none());
        let mut seq = PhotSequence::default();
        seq.observe_scene(0, 0xAAA);
        seq.observe_scene(200_000, 0xBBB);
        let bytes = seq.serialize();
        assert!(PhotSequence::deserialize(&bytes[..bytes.len() - 1]).is_none());
    }

    #[test]
    fn merge_keeps_stronger_edge_and_unions() {
        let mut a = PhotSequence::default();
        a.observe_scene(0, 0xAAA);
        a.observe_scene(200_000, 0xBBB);
        let mut b = PhotSequence::default();
        b.observe_scene(0, 0xAAA);
        for k in 1..6 {
            b.observe_scene(k * 200_000, 0xBBB);
            b.observe_scene(k * 200_000 + 100_000, 0xAAA);
        }
        b.observe_scene(2_000_000, 0xCCC); // novel edge BBB→CCC? actually AAA→CCC
        let before = a
            .edges
            .iter()
            .find(|e| e.from_hash == 0xAAA && e.to_hash == 0xBBB)
            .unwrap()
            .count_ema;
        a.merge(&b);
        let after = a
            .edges
            .iter()
            .find(|e| e.from_hash == 0xAAA && e.to_hash == 0xBBB)
            .unwrap()
            .count_ema;
        assert!(after > before, "merge adopts the stronger AAA→BBB count");
    }

    #[test]
    fn rare_old_edge_survives_eviction_under_age_protection() {
        let mut seq = PhotSequence::default();
        let now_us = GAUSS_AGE_PROTECT_US as u64 + 1_000_000;
        for i in 0..GAUSS_SEQ_EDGE_CAP {
            seq.edges.push(PhotSeqEdge {
                from_hash: (i as u64) * 2 + 1,
                to_hash: (i as u64) * 2 + 2,
                count_ema: 0.50,
                dt_ema_us: 100_000.0,
                elig: 1.0,
                last_t_us: now_us,
            });
        }
        seq.edges.push(PhotSeqEdge {
            from_hash: 0xDEAD,
            to_hash: 0xBEEF,
            count_ema: 0.25,
            dt_ema_us: 100_000.0,
            elig: 0.0,
            last_t_us: 0,
        });
        seq.evict_weakest(now_us);
        assert!(
            seq.edges
                .iter()
                .any(|e| e.from_hash == 0xDEAD && e.to_hash == 0xBEEF),
            "rare old edge must survive age-protected eviction"
        );
    }
}
