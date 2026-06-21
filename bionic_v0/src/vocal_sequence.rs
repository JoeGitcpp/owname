//! vocal_sequence — Phase 2 increment 2 (2026-06-15): the sequence/temporal layer.
//!
//! Why this exists (measured): vocal_efficacy plateaus at ~0.23 because the whole
//! vocal path is SINGLE-FRAME. The efficacy EMA is fed c = closure_support *
//! (1 - shape_dist), where shape_dist is the per-frame spectral distance between
//! ONE self mel64 frame and ONE target mel64 frame. That distance has a structural
//! ~0.48 floor dominated by glottal voice-QUALITY mismatch the formant servo
//! cannot close (see vocal_target_recall.rs:32-37) → fit ≤ ~0.52 → with
//! closure_support<1 the EMA fixed point lands ~0.23. Nothing represents ORDER
//! over time, so multi-phoneme/syllable SEQUENCES can be neither produced nor
//! scored. vocal_target_recall declared itself "the foundation of the sequence
//! layer" (its entries are content-addressed target→motor nodes); this module
//! adds the EDGES between those nodes, plus a DTW utterance-alignment fit that
//! credits getting the ORDER right — a signal that can exceed the single-frame
//! timbre floor.
//!
//! Numeric/local-learning only — transition COUNT EMAs + eligibility traces, a
//! textbook DTW. No backprop, no labels, no global gradients (respects the
//! emergence contract). The DTW cell cost is the SAME strict full-mel64 metric as
//! the per-frame credit path, so alignment cannot hand out cheap timbre credit.
//!
//! Rollout is staged: Stage 0 = struct + persistence + merge (this file);
//! Stage 1 = observe_transition + utterance buffering (viewport.rs::tick_vocal_drive_field);
//! Stage 2 = seq_fit feeds efficacy EMA (viewport.rs::observe_vocal_reflex_closure);
//! Stage 3 = predict_next anticipation for servo warm-start (viewport.rs::tick_vocal_drive_field).

use crate::vocal_self_analysis::{spectral_shape_distance_mel64, BANDS};
use std::collections::VecDeque;

/// Max stored transition edges (≤ RECALL_CAP² conceptually; sparse in practice).
pub const SEQ_EDGE_CAP: usize = 256;
/// Two voiced targets farther apart than this in real time are NOT one utterance.
pub const SEQ_MAX_INTERONSET_US: u64 = 600_000;
/// Alignment-buffer depth (~480 ms at the 50 Hz vocal tick).
pub const SEQ_UTT_CAP: usize = 24;
/// Eligibility-trace time constant (multi-step credit decays over ~0.4 s).
pub const SEQ_ELIG_TAU_US: f64 = 400_000.0;
/// Silence gap (no voiced output) that closes an utterance for DTW scoring.
pub const SEQ_UTTERANCE_GAP_US: u64 = 300_000;
/// developmental_maturity floor below which Stage-3 anticipation stays OFF, so a
/// young body babbles/imitates freely (anneal-in; avoids convergence collapse).
pub const SEQ_ANTICIPATE_MATURITY_FLOOR: f64 = 0.5;
/// Vocal silence gap after which the quiet-period replay tick computes a
/// rollout from `last_voiced_idx` (Q1c "old impressions actively reproduce in
/// prediction"). 1.5 s — shorter than the average inter-utterance pause for
/// active babble (so we don't interrupt) and longer than within-utterance
/// gaps (so we don't fire mid-word).
pub const SEQ_QUIET_REPLAY_GAP_US: u64 = 1_500_000;
/// Steps of look-ahead the quiet-period replay walks via `rollout`.
pub const SEQ_QUIET_REPLAY_STEPS: usize = 4;
/// EMA rate for an edge's traversal count. Seeded at this value on first sight so
/// a fresh edge is NOT 0 (avoids the 0×x bootstrap deadlock when it later biases
/// production multiplicatively).
const SEQ_COUNT_ALPHA: f64 = 0.2;
/// Minimum frames per side for a meaningful DTW alignment.
const SEQ_MIN_UTT: usize = 3;
/// Age-protection window for evict_weakest (Q1b long-lived impression): an edge
/// that has survived `SEQ_AGE_PROTECT_US` since its last traversal earns the
/// full `SEQ_AGE_PROTECT_MAX_BONUS` boost to its eviction-defence score, so a
/// rare-but-learned old edge (e.g. a sparsely-rehearsed word) is not silently
/// squeezed out by a flurry of high-frequency new edges. 6 h ≈ a sleep/reset
/// boundary — long enough to span sessions, short enough that genuinely cold
/// edges still age out. The bonus saturates so very ancient edges remain
/// evictable rather than un-killable.
pub const SEQ_AGE_PROTECT_US: f64 = 6.0 * 3600.0 * 1_000_000.0;
const SEQ_AGE_PROTECT_MAX_BONUS: f64 = 0.30;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeqEdge {
    /// Source / destination recall-entry indices (the nodes live in vocal_target_recall).
    pub from: u16,
    pub to: u16,
    /// EMA of "this transition happened" in [0,1] — the order-strength of from→to.
    pub count_ema: f64,
    /// Learned inter-onset interval (µs) for from→to.
    pub dt_ema_us: f64,
    /// Eligibility trace: 1.0 right after traversal, decays with SEQ_ELIG_TAU_US.
    pub elig: f64,
    pub last_t_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VocalSequence {
    /// Sparse directed transition table over recall-entry indices.
    pub edges: Vec<SeqEdge>,
    /// Rolling self / target utterance buffers for DTW alignment (transient — not
    /// serialized; rebuilt from live voicing each session).
    pub self_utt: VecDeque<[f64; BANDS]>,
    pub target_utt: VecDeque<[f64; BANDS]>,
    /// Last voiced recall index (-1 = none yet). Seeds the next edge's `from`.
    pub last_voiced_idx: i64,
    pub last_voiced_t_us: u64,
    /// Observer-only high-water mark of seq_fit (report ALONGSIDE the EMA so a
    /// lagging EMA isn't misread as no progress). Not serialized.
    pub best_seq_fit: f64,
}

impl Default for VocalSequence {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            self_utt: VecDeque::new(),
            target_utt: VecDeque::new(),
            last_voiced_idx: -1, // sentinel: derive(Default) would give 0 = a real node
            last_voiced_t_us: 0,
            best_seq_fit: 0.0,
        }
    }
}

impl VocalSequence {
    /// Record that the body just committed recall entry `cur_idx` (≥0). A genuine
    /// CHANGE between two known nodes within the inter-onset window is an edge:
    /// bump (prev→cur)'s count EMA, learn its timing, and set its eligibility high
    /// (so when the NEXT step lands well-fit, credit can flow up the recent chain).
    pub fn observe_transition(&mut self, now_us: u64, cur_idx: i64) {
        if cur_idx < 0 {
            return;
        }
        let prev = self.last_voiced_idx;
        if prev == cur_idx {
            return; // same target still held — onset clock unchanged, so this is
                    // safe to call every voiced tick; only a CHANGE records an edge.
        }
        if prev >= 0 {
            let dt = now_us.saturating_sub(self.last_voiced_t_us);
            if dt <= SEQ_MAX_INTERONSET_US {
                let (from, to) = (prev as u16, cur_idx as u16);
                if let Some(e) = self.edges.iter_mut().find(|e| e.from == from && e.to == to) {
                    e.count_ema += SEQ_COUNT_ALPHA * (1.0 - e.count_ema);
                    e.dt_ema_us = e.dt_ema_us * 0.8 + dt as f64 * 0.2;
                    e.elig = 1.0;
                    e.last_t_us = now_us;
                } else {
                    if self.edges.len() >= SEQ_EDGE_CAP {
                        self.evict_weakest(now_us);
                    }
                    self.edges.push(SeqEdge {
                        from,
                        to,
                        count_ema: SEQ_COUNT_ALPHA, // seeded ≠ 0 (no bootstrap deadlock)
                        dt_ema_us: dt as f64,
                        elig: 1.0,
                        last_t_us: now_us,
                    });
                }
            }
        }
        self.last_voiced_idx = cur_idx;
        self.last_voiced_t_us = now_us;
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

    /// Composite eviction-defence score: count_ema plus an age bonus that
    /// saturates at `SEQ_AGE_PROTECT_MAX_BONUS` once an edge crosses
    /// `SEQ_AGE_PROTECT_US`. This is what Q1b changes — pure count_ema would
    /// silently squeeze out rare-but-learned old edges (think a sparsely
    /// rehearsed word) the moment 256 high-frequency new edges arrive. The
    /// bonus is capped so very ancient edges still age out gracefully rather
    /// than becoming permanently un-evictable.
    fn eviction_defence(e: &SeqEdge, now_us: u64) -> f64 {
        let age = now_us.saturating_sub(e.last_t_us) as f64;
        let bonus = (age / SEQ_AGE_PROTECT_US).min(1.0) * SEQ_AGE_PROTECT_MAX_BONUS;
        e.count_ema + bonus
    }

    pub fn push_self(&mut self, frame: &[f64; BANDS]) {
        if self.self_utt.len() >= SEQ_UTT_CAP {
            self.self_utt.pop_front();
        }
        self.self_utt.push_back(*frame);
    }

    pub fn push_target(&mut self, frame: &[f64; BANDS]) {
        if self.target_utt.len() >= SEQ_UTT_CAP {
            self.target_utt.pop_front();
        }
        self.target_utt.push_back(*frame);
    }

    /// Most-likely successor entry for `from_idx` and its learned inter-onset
    /// interval (µs). None if no edge is known. Callers gate its INFLUENCE by
    /// developmental_maturity so it anneals in and never smothers early babble.
    pub fn predict_next(&self, from_idx: i64) -> Option<(u16, f64)> {
        if from_idx < 0 {
            return None;
        }
        let from = from_idx as u16;
        self.edges
            .iter()
            .filter(|e| e.from == from)
            .max_by(|a, b| a.count_ema.partial_cmp(&b.count_ema).unwrap())
            .map(|e| (e.to, e.dt_ema_us))
    }

    /// Multi-step rollout: walk the argmax(count_ema) chain forward up to
    /// `k_steps` steps from `from_idx`, returning the sequence of
    /// (successor_idx, learned inter-onset µs) pairs. This is the Q1c
    /// long-horizon prediction surface — predict_next is single-step and good
    /// for servo warm-start; rollout exposes the full chain so attention /
    /// planning layers (or quiet-period replay below) can see what the body
    /// "expects to say next" beyond one phoneme. Cycle-safe: a vertex is
    /// followed only once, so a stable A↔B oscillation does not run forever.
    pub fn rollout(&self, from_idx: i64, k_steps: usize) -> Vec<(u16, f64)> {
        if from_idx < 0 || k_steps == 0 {
            return Vec::new();
        }
        let mut out: Vec<(u16, f64)> = Vec::with_capacity(k_steps);
        let mut visited: Vec<u16> = Vec::with_capacity(k_steps + 1);
        let mut cursor = from_idx as u16;
        visited.push(cursor);
        for _ in 0..k_steps {
            let next = self
                .edges
                .iter()
                .filter(|e| e.from == cursor)
                .max_by(|a, b| a.count_ema.partial_cmp(&b.count_ema).unwrap());
            let Some(e) = next else { break };
            if visited.contains(&e.to) {
                break;
            }
            out.push((e.to, e.dt_ema_us));
            visited.push(e.to);
            cursor = e.to;
        }
        out
    }

    /// Quiet-period replay chain: the body's planned next-K utterance based on
    /// the persistent edge table, queried from `last_voiced_idx`. Returns empty
    /// when no cursor is known (cold start) or no edge follows the cursor.
    /// Pure read — does not mutate the sequence (so age-protection's
    /// `last_t_us` clock is not falsely refreshed by internal rehearsal). The
    /// Viewport tick is what decides when this should fire (after a vocal
    /// silence gap) and what to do with the chain.
    pub fn quiet_replay_chain(&self, k_steps: usize) -> Vec<(u16, f64)> {
        self.rollout(self.last_voiced_idx, k_steps)
    }

    /// DTW alignment fit between the buffered self utterance and target utterance,
    /// in [0,1] (1 = perfectly alignable shapes). Uses the SAME strict full-mel64
    /// metric as the per-frame credit path as the cell cost, so it cannot inflate
    /// efficacy with coarse/timbre matches. None if either buffer is too short.
    pub fn seq_fit(&self) -> Option<f64> {
        let n = self.self_utt.len();
        let m = self.target_utt.len();
        if n < SEQ_MIN_UTT || m < SEQ_MIN_UTT {
            return None;
        }
        let s: Vec<&[f64; BANDS]> = self.self_utt.iter().collect();
        let t: Vec<&[f64; BANDS]> = self.target_utt.iter().collect();
        let inf = f64::INFINITY;
        // Rolling two-row DTW, O(n*m) time / O(m) space.
        let mut prev = vec![inf; m + 1];
        let mut cur = vec![inf; m + 1];
        prev[0] = 0.0;
        for i in 1..=n {
            cur[0] = inf;
            for j in 1..=m {
                let cost = spectral_shape_distance_mel64(s[i - 1], t[j - 1]);
                // F-fix (Giorgino 2009): symmetric2 step pattern — the diagonal
                // move carries weight 2, the horizontal/vertical moves weight 1.
                // With this weighting EVERY complete (0,0)→(n,m) path has total
                // step-weight EXACTLY n+m, so the (n+m) divisor below is the exact
                // average-per-step normalizer. The old symmetric1 (every move adds
                // bare cost, divide by n+m) under-counted the path length — a
                // diagonal advanced BOTH indices for one cost — so the distance was
                // ~halved and seq_fit was biased HIGH, one-directionally inflating
                // the max(per_frame_fit, seq_fit) credit.
                let diag = prev[j - 1] + 2.0 * cost;
                let up = prev[j] + cost;
                let left = cur[j - 1] + cost;
                cur[j] = diag.min(up).min(left);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        // Normalize by the warping-path step-weight (n+m), EXACT under symmetric2
        // → a true average-per-step shape distance, utterance-length-invariant.
        let dist = prev[m] / (n + m) as f64;
        Some((1.0 - dist).clamp(0.0, 1.0))
    }

    /// Close the current utterance: stamp the best seq_fit seen, clear the buffers.
    pub fn end_utterance(&mut self) {
        if let Some(f) = self.seq_fit() {
            if f > self.best_seq_fit {
                self.best_seq_fit = f;
            }
        }
        self.self_utt.clear();
        self.target_utt.clear();
    }

    /// Refresh eligibility traces as a pure function of time-since-traversal
    /// (idempotent — safe to call every tick).
    pub fn decay_eligibility(&mut self, now_us: u64) {
        for e in self.edges.iter_mut() {
            let dt = now_us.saturating_sub(e.last_t_us) as f64;
            e.elig = (-dt / SEQ_ELIG_TAU_US).exp();
        }
    }

    /// Distributed-training merge: fold another life's transitions into this one,
    /// keeping the STRONGER edge per (from,to) and unioning novel edges (respecting
    /// the cap by evicting the weakest). Mirrors VocalTargetRecall::merge.
    pub fn merge(&mut self, other: &VocalSequence) {
        for oe in &other.edges {
            if let Some(e) = self
                .edges
                .iter_mut()
                .find(|e| e.from == oe.from && e.to == oe.to)
            {
                if oe.count_ema > e.count_ema {
                    e.count_ema = oe.count_ema;
                    e.dt_ema_us = oe.dt_ema_us;
                    e.last_t_us = e.last_t_us.max(oe.last_t_us);
                }
            } else if self.edges.len() < SEQ_EDGE_CAP {
                self.edges.push(*oe);
            } else {
                // Only displace our weakest edge if the incoming one is stronger.
                if let Some((min_i, min_c)) = self
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
    }

    // ── Persistence ──────────────────────────────────────────────────────────
    // Sidecar vocal_sequence.bin (NOT the global snapshot): bumping SNAPSHOT_VERSION
    // would reject every existing snapshot.bin and reset the whole lineage's
    // maturity. Mirrors vocal_target_recall.bin's self-versioned sidecar.
    //
    // Layout v1 (legacy — edges only, still accepted):
    //   [4] magic b"VSEQ"  [1] version=0x01  [8] n_edges u64-le
    //   per edge: 36 bytes (see EDGE_BYTES).
    //   Transient utterance buffers and last_voiced cursor are NOT persisted.
    //
    // Layout v2 (current — long-lived impression: persists the short-term DTW
    // buffers and the recent-voicing cursor so the body resumes its acoustic
    // impression instead of restarting it cold each session — see Q1a in the
    // 2026-06-18 impression-memory design discussion). After the v1 edge block:
    //   [8] last_voiced_idx_p1 u64-le (= last_voiced_idx + 1; 0 = sentinel "none")
    //   [8] last_voiced_t_us u64-le
    //   [8] best_seq_fit f64-le
    //   [8] n_self_utt u64-le  (≤ SEQ_UTT_CAP)
    //   per self frame: BANDS × 8 bytes (f64 mel band)
    //   [8] n_target_utt u64-le  (≤ SEQ_UTT_CAP)
    //   per target frame: BANDS × 8 bytes
    const MAGIC: [u8; 4] = *b"VSEQ";
    const VERSION_V1: u8 = 0x01;
    const VERSION: u8 = 0x02;
    const EDGE_BYTES: usize = 2 + 2 + 8 + 8 + 8 + 8;
    const FRAME_BYTES: usize = BANDS * 8;

    pub fn serialize(&self) -> Vec<u8> {
        let n = self.edges.len();
        let ns = self.self_utt.len().min(SEQ_UTT_CAP);
        let nt = self.target_utt.len().min(SEQ_UTT_CAP);
        let mut out = Vec::with_capacity(
            4 + 1
                + 8
                + n * Self::EDGE_BYTES
                + 8 + 8 + 8
                + 8 + ns * Self::FRAME_BYTES
                + 8 + nt * Self::FRAME_BYTES,
        );
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(n as u64).to_le_bytes());
        for e in &self.edges {
            out.extend_from_slice(&e.from.to_le_bytes());
            out.extend_from_slice(&e.to.to_le_bytes());
            out.extend_from_slice(&e.count_ema.to_le_bytes());
            out.extend_from_slice(&e.dt_ema_us.to_le_bytes());
            out.extend_from_slice(&e.elig.to_le_bytes());
            out.extend_from_slice(&e.last_t_us.to_le_bytes());
        }
        // v2 transient block. last_voiced_idx is i64 in memory but we map -1 →
        // 0 sentinel via "plus one" so the on-disk form stays unsigned (avoids
        // a signed/unsigned cast hazard if the file is hand-inspected).
        let last_voiced_idx_p1: u64 = if self.last_voiced_idx < 0 {
            0
        } else {
            (self.last_voiced_idx as u64) + 1
        };
        out.extend_from_slice(&last_voiced_idx_p1.to_le_bytes());
        out.extend_from_slice(&self.last_voiced_t_us.to_le_bytes());
        out.extend_from_slice(&self.best_seq_fit.to_le_bytes());
        out.extend_from_slice(&(ns as u64).to_le_bytes());
        for frame in self.self_utt.iter().take(ns) {
            for v in frame.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out.extend_from_slice(&(nt as u64).to_le_bytes());
        for frame in self.target_utt.iter().take(nt) {
            for v in frame.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 13 || data[0..4] != Self::MAGIC {
            return None;
        }
        let version = data[4];
        if version != Self::VERSION_V1 && version != Self::VERSION {
            return None;
        }
        let n = (u64::from_le_bytes(data[5..13].try_into().ok()?) as usize).min(SEQ_EDGE_CAP);
        if data.len() < 13 + n * Self::EDGE_BYTES {
            return None;
        }
        let mut off = 13usize;
        let mut edges = Vec::with_capacity(n);
        for _ in 0..n {
            let from = u16::from_le_bytes(data[off..off + 2].try_into().ok()?);
            off += 2;
            let to = u16::from_le_bytes(data[off..off + 2].try_into().ok()?);
            off += 2;
            let count_ema = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let dt_ema_us = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let elig = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let last_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            edges.push(SeqEdge {
                from,
                to,
                count_ema,
                dt_ema_us,
                elig,
                last_t_us,
            });
        }
        // v1: stop here; transient state initialises to Default sentinels.
        if version == Self::VERSION_V1 {
            return Some(VocalSequence {
                edges,
                ..Default::default()
            });
        }
        // v2: strict parse of transient block. Any underrun rejects the whole
        // file (matches the v1 behaviour of `deserialize_rejects_corrupt`: we
        // do not know whether a short tail is a bona fide truncation or a
        // corruption that also damaged the edges; safest to reload from
        // Default rather than commit partial state).
        let need_header = 8 + 8 + 8 + 8;
        if data.len() < off + need_header {
            return None;
        }
        let last_voiced_idx_p1 = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let last_voiced_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let best_seq_fit = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let ns_raw = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
        off += 8;
        if ns_raw > SEQ_UTT_CAP {
            return None;
        }
        if data.len() < off + ns_raw * Self::FRAME_BYTES + 8 {
            return None;
        }
        let mut self_utt: VecDeque<[f64; BANDS]> = VecDeque::with_capacity(ns_raw);
        for _ in 0..ns_raw {
            let mut frame = [0.0f64; BANDS];
            for v in frame.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            self_utt.push_back(frame);
        }
        let nt_raw = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
        off += 8;
        if nt_raw > SEQ_UTT_CAP {
            return None;
        }
        if data.len() < off + nt_raw * Self::FRAME_BYTES {
            return None;
        }
        let mut target_utt: VecDeque<[f64; BANDS]> = VecDeque::with_capacity(nt_raw);
        for _ in 0..nt_raw {
            let mut frame = [0.0f64; BANDS];
            for v in frame.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            target_utt.push_back(frame);
        }
        let last_voiced_idx: i64 = if last_voiced_idx_p1 == 0 {
            -1
        } else {
            (last_voiced_idx_p1 - 1) as i64
        };
        Some(VocalSequence {
            edges,
            self_utt,
            target_utt,
            last_voiced_idx,
            last_voiced_t_us,
            best_seq_fit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mel_with_peak(peak_bin: usize) -> [f64; BANDS] {
        let mut m = [0.0; BANDS];
        for (i, v) in m.iter_mut().enumerate() {
            let d = i as f64 - peak_bin as f64;
            *v = (-(d * d) / 18.0).exp();
        }
        m
    }

    #[test]
    fn learns_and_predicts_a_transition() {
        let mut seq = VocalSequence::default();
        seq.observe_transition(1_000, 0); // first node, no edge yet
        seq.observe_transition(200_000, 1); // 0→1 within window
        seq.observe_transition(400_000, 2); // 1→2
        seq.observe_transition(600_000, 1); // 2→1
        // 0→1 traversed once; predict_next(0) must be 1.
        let (to, dt) = seq.predict_next(0).expect("edge 0→1 learned");
        assert_eq!(to, 1);
        assert!(dt > 0.0);
        assert!(seq.predict_next(99).is_none(), "unknown source → no prediction");
    }

    #[test]
    fn stable_ticks_do_not_reset_the_onset_clock() {
        // Stage 1 calls observe_transition every voiced tick. Holding the same
        // target for many ticks must NOT reset the inter-onset clock, or the
        // learned dt would collapse to one tick. Hold idx 0 up to ~0.5 s, then
        // change to 1: the learned dt must reflect the held time, not a tick.
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        for t in 1..500 {
            seq.observe_transition(t * 1000, 0); // 1 kHz of stable-target ticks
        }
        seq.observe_transition(500_000, 1);
        let (to, dt) = seq.predict_next(0).expect("edge 0→1");
        assert_eq!(to, 1);
        assert!(dt >= 400_000.0, "onset clock reset on stable ticks; dt={dt}");
        assert_eq!(seq.edges.len(), 1, "stable ticks must not spawn spurious edges");
    }

    #[test]
    fn rollout_walks_argmax_chain_and_stops_on_cycle() {
        // Build a chain 0→1→2→3 with monotonically strong counts plus a weak
        // 1→9 distractor that must lose to 1→2 at every junction.
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        seq.observe_transition(100_000, 1);
        seq.observe_transition(200_000, 2);
        seq.observe_transition(300_000, 3);
        // Re-traverse to bias 0→1→2→3 heavier than any distractor we add.
        for k in 1..5 {
            seq.observe_transition(k * 500_000, 0);
            seq.observe_transition(k * 500_000 + 100_000, 1);
            seq.observe_transition(k * 500_000 + 200_000, 2);
            seq.observe_transition(k * 500_000 + 300_000, 3);
        }
        // Weak distractor 1→9 (observed once).
        seq.observe_transition(10_000_000, 1);
        seq.observe_transition(10_100_000, 9);
        let chain = seq.rollout(0, 4);
        assert!(chain.len() >= 3, "should produce at least 0→1→2→3, got {chain:?}");
        let tos: Vec<u16> = chain.iter().map(|(t, _)| *t).collect();
        assert_eq!(&tos[..3], &[1u16, 2, 3], "argmax chain must follow heavy path");

        // Cycle-safety: A→B→A would otherwise loop. Build it explicitly.
        let mut osc = VocalSequence::default();
        osc.observe_transition(0, 0);
        osc.observe_transition(100_000, 1);
        osc.observe_transition(200_000, 0); // 1→0 edge
        osc.observe_transition(300_000, 1); // 0→1 edge again
        let chain = osc.rollout(0, 20);
        assert!(chain.len() < 20, "must not loop forever on a 2-cycle");
    }

    #[test]
    fn quiet_replay_chain_uses_last_voiced_cursor() {
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        seq.observe_transition(100_000, 1);
        seq.observe_transition(200_000, 2);
        // Cursor lives on whatever was observed last (= 2). No outgoing edge
        // from 2 → empty chain. Set the cursor manually to a node WITH an
        // outgoing edge and re-check.
        let chain = seq.quiet_replay_chain(3);
        assert!(chain.is_empty(), "node 2 has no outgoing edge → empty replay");
        seq.last_voiced_idx = 0; // pretend we just landed on 0 in silence
        let chain = seq.quiet_replay_chain(3);
        assert!(!chain.is_empty(), "replay from a node with outgoing edges must yield a chain");
        assert_eq!(chain[0].0, 1, "argmax successor of 0 is 1");
    }

    #[test]
    fn no_edge_across_too_large_a_gap() {
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        seq.observe_transition(SEQ_MAX_INTERONSET_US + 1, 1); // gap too large
        assert!(seq.predict_next(0).is_none(), "no edge when inter-onset exceeds the window");
    }

    #[test]
    fn dtw_alignment_beats_per_frame_fit() {
        // The ceiling-break mechanism, numerically: a self utterance with the SAME
        // shapes in the SAME order but DIFFERENT timing (a time-warp) aligns nearly
        // perfectly under DTW, while a naive per-frame comparison of the misaligned
        // frames scores much lower. This is exactly why a sequence-alignment credit
        // can exceed the single-frame timbre floor.
        let mut seq = VocalSequence::default();
        // target: hold@10 ×2, hold@20 ×2, hold@30 ×2  (6 frames)
        for &p in &[10usize, 10, 20, 20, 30, 30] {
            seq.push_target(&mel_with_peak(p));
        }
        // self: same shapes, faster — 10,20,30 (3 frames) → a time-warp of target.
        for &p in &[10usize, 20, 30] {
            seq.push_self(&mel_with_peak(p));
        }
        let seq_fit = seq.seq_fit().expect("buffers long enough");

        // Naive per-frame fit over the overlapping prefix (no alignment).
        let s: Vec<_> = seq.self_utt.iter().collect();
        let t: Vec<_> = seq.target_utt.iter().collect();
        let k = s.len().min(t.len());
        let per_frame: f64 = (0..k)
            .map(|i| 1.0 - spectral_shape_distance_mel64(s[i], t[i]))
            .sum::<f64>()
            / k as f64;

        assert!(
            seq_fit > per_frame + 0.1,
            "DTW alignment ({seq_fit:.3}) must beat per-frame fit ({per_frame:.3}) on a time-warped utterance"
        );
    }

    #[test]
    fn seq_fit_symmetric2_true_average_per_step() {
        // Identical utterances → perfect alignment → fit ≈ 1.
        let mut same = VocalSequence::default();
        for _ in 0..4 {
            same.push_self(&mel_with_peak(20));
            same.push_target(&mel_with_peak(20));
        }
        let f_same = same.seq_fit().unwrap();
        assert!(f_same > 0.999, "identical utterances must align perfectly, got {f_same}");

        // Uniform mismatch: every aligned cell costs the SAME c. Under symmetric2
        // the normalized DTW distance is exactly c (a TRUE average-per-step), so
        // seq_fit = 1−c. The old symmetric1 normalization halved the path length
        // and reported ≈ 1−c/2 — inflated. Assert the honest (lower) value.
        let c = spectral_shape_distance_mel64(&mel_with_peak(10), &mel_with_peak(30));
        assert!(c > 0.1, "precondition: distinct peaks must differ enough, c={c}");
        let mut diff = VocalSequence::default();
        for _ in 0..4 {
            diff.push_self(&mel_with_peak(10));
            diff.push_target(&mel_with_peak(30));
        }
        let f_diff = diff.seq_fit().unwrap();
        let honest = 1.0 - c;
        let inflated = 1.0 - c / 2.0;
        assert!(
            (f_diff - honest).abs() < 0.05,
            "seq_fit ({f_diff:.4}) must be the true 1−c ({honest:.4}), not the \
             symmetric1-inflated 1−c/2 ({inflated:.4})"
        );
        assert!(
            f_diff < inflated - 0.05,
            "seq_fit ({f_diff:.4}) must be markedly below the inflated value ({inflated:.4})"
        );
    }

    #[test]
    fn seq_fit_none_when_too_short() {
        let mut seq = VocalSequence::default();
        seq.push_self(&mel_with_peak(10));
        seq.push_target(&mel_with_peak(10));
        assert!(seq.seq_fit().is_none(), "below SEQ_MIN_UTT → no score");
    }

    #[test]
    fn serialize_roundtrip_preserves_edges_and_v2_transient_state() {
        // v2 sidecar persists the recent-voicing cursor AND the rolling DTW
        // buffers (Q1a long-lived impression) so a restarted body resumes its
        // acoustic short-term memory instead of starting cold every session.
        let mut seq = VocalSequence::default();
        seq.observe_transition(1_000, 0);
        seq.observe_transition(200_000, 1);
        seq.observe_transition(400_000, 2);
        seq.push_self(&mel_with_peak(10));
        seq.push_self(&mel_with_peak(20));
        seq.push_target(&mel_with_peak(15));
        seq.best_seq_fit = 0.42;
        let back = VocalSequence::deserialize(&seq.serialize()).expect("roundtrip");
        assert_eq!(back.edges.len(), seq.edges.len());
        for (a, b) in seq.edges.iter().zip(back.edges.iter()) {
            assert_eq!(a.from, b.from);
            assert_eq!(a.to, b.to);
            assert!((a.count_ema - b.count_ema).abs() < 1e-12);
            assert!((a.dt_ema_us - b.dt_ema_us).abs() < 1e-12);
            assert_eq!(a.last_t_us, b.last_t_us);
        }
        assert_eq!(back.last_voiced_idx, 2);
        assert_eq!(back.last_voiced_t_us, 400_000);
        assert!((back.best_seq_fit - 0.42).abs() < 1e-12);
        assert_eq!(back.self_utt.len(), 2);
        assert_eq!(back.target_utt.len(), 1);
        // Mel frames survive intact bit-for-bit (BANDS f64s × 3 frames here).
        for (a, b) in seq.self_utt.iter().zip(back.self_utt.iter()) {
            for (x, y) in a.iter().zip(b.iter()) {
                assert!((x - y).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn deserialize_accepts_legacy_v1_with_empty_transient_state() {
        // Bodies persisted under v1 (edges-only) must still load — bumping
        // the sidecar version cannot reset existing lineages. The transient
        // buffers come back empty (which IS the v1 semantic: it did not record
        // them); the edges are preserved.
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        seq.observe_transition(100_000, 1);
        // Hand-craft a v1 blob with the same edges by capturing the v2 output
        // up to the edge tail and pinning the version byte to 0x01.
        let v2 = seq.serialize();
        let n_edges = seq.edges.len();
        let edge_tail = 13 + n_edges * VocalSequence::EDGE_BYTES;
        let mut v1 = v2[..edge_tail].to_vec();
        v1[4] = VocalSequence::VERSION_V1;
        let back = VocalSequence::deserialize(&v1).expect("v1 still loads");
        assert_eq!(back.edges.len(), seq.edges.len());
        assert_eq!(back.last_voiced_idx, -1, "v1 has no cursor");
        assert!(back.self_utt.is_empty(), "v1 has no transient buffer");
        assert!(back.target_utt.is_empty(), "v1 has no transient buffer");
    }

    #[test]
    fn deserialize_rejects_corrupt() {
        assert!(VocalSequence::deserialize(&[]).is_none());
        assert!(VocalSequence::deserialize(b"JUNKjunkjunk").is_none());
        let mut seq = VocalSequence::default();
        seq.observe_transition(0, 0);
        seq.observe_transition(100_000, 1);
        let bytes = seq.serialize();
        assert!(VocalSequence::deserialize(&bytes[..bytes.len() - 1]).is_none());
        // Empty (header-only) round-trips to empty.
        let empty = VocalSequence::deserialize(&VocalSequence::default().serialize());
        assert!(empty.map(|s| s.edges.is_empty()).unwrap_or(false));
    }

    #[test]
    fn rare_old_edge_survives_eviction_under_age_protection() {
        // The shape of the Q1b protection bug pre-fix: 256 cap fills with
        // high-frequency new edges, a rare-but-learned old edge gets squeezed
        // out by pure count_ema sorting. With age-protection it survives,
        // because the bonus offsets its lower count_ema.
        let mut seq = VocalSequence::default();
        let now_us = SEQ_AGE_PROTECT_US as u64 + 1_000_000; // fully past the bonus window
        for i in 0..SEQ_EDGE_CAP {
            seq.edges.push(SeqEdge {
                from: i as u16,
                to: (i + 1) as u16,
                count_ema: 0.50,
                dt_ema_us: 100_000.0,
                elig: 1.0,
                last_t_us: now_us, // fresh — earns NO age bonus
            });
        }
        // Inject a rare old edge at t=0; count_ema=0.25 (one exposure) but age
        // is fully past SEQ_AGE_PROTECT_US so it earns the full +0.30 bonus.
        // Effective defence: 0.25 + 0.30 = 0.55, beats the fresh 0.50 cohort.
        seq.edges.push(SeqEdge {
            from: 10_000,
            to: 10_001,
            count_ema: 0.25,
            dt_ema_us: 100_000.0,
            elig: 0.0,
            last_t_us: 0,
        });
        seq.evict_weakest(now_us);
        assert!(
            seq.edges.iter().any(|e| e.from == 10_000 && e.to == 10_001),
            "rare old edge must survive — age bonus offsets the count_ema gap"
        );
        // And the edge we DID evict came from the fresh cohort, not the old one.
        assert_eq!(seq.edges.len(), SEQ_EDGE_CAP);
    }

    #[test]
    fn very_weak_old_edge_still_evictable() {
        // The age bonus saturates at SEQ_AGE_PROTECT_MAX_BONUS (0.30) — a truly
        // cold edge with count_ema near zero cannot become un-killable. This
        // protects against the opposite failure mode: lifelong ghost edges.
        let mut seq = VocalSequence::default();
        let now_us = SEQ_AGE_PROTECT_US as u64 * 1_000; // very far past
        for i in 0..SEQ_EDGE_CAP {
            seq.edges.push(SeqEdge {
                from: i as u16,
                to: (i + 1) as u16,
                count_ema: 0.50,
                dt_ema_us: 100_000.0,
                elig: 1.0,
                last_t_us: now_us,
            });
        }
        seq.edges.push(SeqEdge {
            from: 10_000,
            to: 10_001,
            count_ema: 0.05, // near zero
            dt_ema_us: 100_000.0,
            elig: 0.0,
            last_t_us: 0,
        });
        seq.evict_weakest(now_us);
        assert!(
            !seq.edges.iter().any(|e| e.from == 10_000 && e.to == 10_001),
            "ghost edge with 0.05 + 0.30 = 0.35 < 0.50 must still be evictable"
        );
    }

    #[test]
    fn merge_keeps_stronger_edge_and_unions() {
        // Mainline: 0→1 traversed once (weak).
        let mut mainline = VocalSequence::default();
        mainline.observe_transition(0, 0);
        mainline.observe_transition(100_000, 1);
        // Branch: 0→1 traversed many times (stronger) AND learned 1→2.
        let mut branch = VocalSequence::default();
        branch.observe_transition(0, 0);
        for i in 1..6 {
            branch.observe_transition(i * 100_000, 1);
            branch.observe_transition(i * 100_000 + 50_000, 0);
        }
        branch.observe_transition(700_000, 1);
        branch.observe_transition(800_000, 2);
        let before = mainline
            .edges
            .iter()
            .find(|e| e.from == 0 && e.to == 1)
            .unwrap()
            .count_ema;
        mainline.merge(&branch);
        let after = mainline
            .edges
            .iter()
            .find(|e| e.from == 0 && e.to == 1)
            .unwrap()
            .count_ema;
        assert!(after > before, "0→1 adopts the branch's stronger count");
        assert!(
            mainline.edges.iter().any(|e| e.from == 1 && e.to == 2),
            "novel 1→2 edge absorbed from branch"
        );
    }
}
