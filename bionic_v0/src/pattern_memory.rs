use crate::feature_extraction::N_POOLS;

pub const PATTERN_DIM: usize = N_POOLS; // 32
pub const PATTERN_CAP: usize = 2048;

const THETA_COMPLETE_INIT: f64 = 0.85;
const THETA_COMPLETE_MATURE: f64 = 0.75;
const THETA_SEPARATE_INIT: f64 = 0.40;
const THETA_SEPARATE_MATURE: f64 = 0.55;
const ALPHA_FAST: f64 = 0.1;
const DEFAULT_TEMPERATURE: f64 = 1.0;
const CONSOLIDATE_INTERVAL: u32 = 10_000;
const CONSOLIDATE_MERGE_THRESH: f64 = 0.90;
const CONSOLIDATE_DECAY: f64 = 0.999;

#[derive(Clone)]
pub struct PatternEntry {
    pub pattern: [f64; PATTERN_DIM],
    pub access_count: u32,
    pub last_access_t_us: u64,
    pub strength: f64,
}

impl PatternEntry {
    fn new(pattern: [f64; PATTERN_DIM], t_us: u64) -> Self {
        Self {
            pattern,
            access_count: 1,
            last_access_t_us: t_us,
            strength: 1.0,
        }
    }
}

pub enum PatternQueryResult {
    Completed {
        pattern: [f64; PATTERN_DIM],
        confidence: f64,
        index: usize,
        /// L4 match error: retrieved_pattern[i] - query[i].
        /// Positive = L4 expected more than L3 sent; drives FiLM weight update.
        residual: [f64; PATTERN_DIM],
    },
    Separated { new_id: usize },
    Ambiguous { top_candidates: Vec<(usize, f64)> },
    Empty,
}

pub struct PatternMemory {
    pub entries: Vec<PatternEntry>,
    pub theta_complete: f64,
    pub theta_separate: f64,
    pub temperature: f64,
    tick_counter: u32,
}

impl PatternMemory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            theta_complete: THETA_COMPLETE_INIT,
            theta_separate: THETA_SEPARATE_INIT,
            temperature: DEFAULT_TEMPERATURE,
            tick_counter: 0,
        }
    }

    pub fn update_thresholds(&mut self, maturity: f64) {
        let m = maturity.clamp(0.0, 1.0);
        self.theta_complete = THETA_COMPLETE_INIT + (THETA_COMPLETE_MATURE - THETA_COMPLETE_INIT) * m;
        self.theta_separate = THETA_SEPARATE_INIT + (THETA_SEPARATE_MATURE - THETA_SEPARATE_INIT) * m;
    }

    pub fn query(&mut self, q: &[f64; PATTERN_DIM], t_us: u64) -> PatternQueryResult {
        if self.entries.is_empty() {
            let idx = self.insert_new(q, t_us);
            return PatternQueryResult::Separated { new_id: idx };
        }

        let (best_idx, best_sim) = self.nearest(q);

        if best_sim >= self.theta_complete {
            // Completion: update matched pattern (Hebbian fast write)
            let entry = &mut self.entries[best_idx];
            for d in 0..PATTERN_DIM {
                entry.pattern[d] = (1.0 - ALPHA_FAST) * entry.pattern[d] + ALPHA_FAST * q[d];
            }
            entry.access_count += 1;
            entry.last_access_t_us = t_us;
            entry.strength = (entry.strength + 0.01).min(1.0);

            let retrieved = self.hopfield_retrieve(q);
            let mut residual = [0.0f64; PATTERN_DIM];
            for d in 0..PATTERN_DIM {
                residual[d] = retrieved[d] - q[d];
            }
            PatternQueryResult::Completed {
                pattern: retrieved,
                confidence: best_sim,
                index: best_idx,
                residual,
            }
        } else if best_sim < self.theta_separate {
            // Separation: new pattern
            let idx = self.insert_new(q, t_us);
            PatternQueryResult::Separated { new_id: idx }
        } else {
            // Ambiguous zone
            let candidates = self.top_k(q, 3);
            PatternQueryResult::Ambiguous { top_candidates: candidates }
        }
    }

    pub fn tick_consolidation(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        if self.tick_counter % CONSOLIDATE_INTERVAL == 0 {
            self.consolidate();
        }
    }

    pub fn pattern_count(&self) -> usize {
        self.entries.len()
    }

    pub fn tick_count(&self) -> u32 {
        self.tick_counter
    }

    fn insert_new(&mut self, q: &[f64; PATTERN_DIM], t_us: u64) -> usize {
        if self.entries.len() >= PATTERN_CAP {
            self.evict_weakest();
        }
        let entry = PatternEntry::new(*q, t_us);
        self.entries.push(entry);
        self.entries.len() - 1
    }

    fn nearest(&self, q: &[f64; PATTERN_DIM]) -> (usize, f64) {
        let mut best_idx = 0;
        let mut best_sim = f64::NEG_INFINITY;
        for (i, entry) in self.entries.iter().enumerate() {
            let sim = cosine_similarity(q, &entry.pattern);
            if sim > best_sim {
                best_sim = sim;
                best_idx = i;
            }
        }
        (best_idx, best_sim)
    }

    fn top_k(&self, q: &[f64; PATTERN_DIM], k: usize) -> Vec<(usize, f64)> {
        let mut scored: Vec<(usize, f64)> = self.entries.iter().enumerate()
            .map(|(i, e)| (i, cosine_similarity(q, &e.pattern)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    fn hopfield_retrieve(&self, q: &[f64; PATTERN_DIM]) -> [f64; PATTERN_DIM] {
        // Modern Hopfield: softmax(q · P^T / τ√d) · P
        let inv_sqrt_d = 1.0 / (PATTERN_DIM as f64).sqrt();
        let inv_tau = 1.0 / self.temperature.max(0.01);

        let mut scores: Vec<f64> = self.entries.iter()
            .map(|e| {
                let dot: f64 = q.iter().zip(e.pattern.iter()).map(|(a, b)| a * b).sum();
                dot * inv_sqrt_d * inv_tau
            })
            .collect();

        // Stable softmax
        let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mut exp_sum = 0.0f64;
        for s in scores.iter_mut() {
            *s = (*s - max_score).exp();
            exp_sum += *s;
        }
        if exp_sum > 0.0 {
            for s in scores.iter_mut() {
                *s /= exp_sum;
            }
        }

        // Weighted retrieval
        let mut retrieved = [0.0f64; PATTERN_DIM];
        for (i, &w) in scores.iter().enumerate() {
            for d in 0..PATTERN_DIM {
                retrieved[d] += w * self.entries[i].pattern[d];
            }
        }
        retrieved
    }

    fn consolidate(&mut self) {
        // Decay all strengths
        for entry in self.entries.iter_mut() {
            entry.strength *= CONSOLIDATE_DECAY;
        }

        // Merge highly similar patterns
        let n = self.entries.len();
        if n < 2 { return; }

        let mut merged = vec![false; n];
        for i in 0..n {
            if merged[i] { continue; }
            for j in (i + 1)..n {
                if merged[j] { continue; }
                let sim = cosine_similarity(&self.entries[i].pattern, &self.entries[j].pattern);
                if sim >= CONSOLIDATE_MERGE_THRESH {
                    // Merge j into i, weighted by access count
                    let wi = self.entries[i].access_count as f64;
                    let wj = self.entries[j].access_count as f64;
                    let total = wi + wj;
                    if total > 0.0 {
                        for d in 0..PATTERN_DIM {
                            self.entries[i].pattern[d] =
                                (wi * self.entries[i].pattern[d] + wj * self.entries[j].pattern[d]) / total;
                        }
                    }
                    self.entries[i].access_count += self.entries[j].access_count;
                    self.entries[i].strength = self.entries[i].strength.max(self.entries[j].strength);
                    merged[j] = true;
                }
            }
        }

        // Remove merged entries (reverse order to preserve indices)
        let mut i = n;
        while i > 0 {
            i -= 1;
            if merged[i] {
                self.entries.swap_remove(i);
            }
        }
    }

    fn evict_weakest(&mut self) {
        if self.entries.is_empty() { return; }
        let mut weakest = 0;
        let mut weakest_score = f64::MAX;
        for (i, entry) in self.entries.iter().enumerate() {
            let score = entry.strength * (entry.access_count as f64).ln().max(0.1);
            if score < weakest_score {
                weakest_score = score;
                weakest = i;
            }
        }
        self.entries.swap_remove(weakest);
    }

    /// §7.4: Seed L4 from perturbation_graph FastPath signature (24-D → 32-D).
    /// Only inserts if no existing pattern is close enough (avoids duplicates).
    pub fn seed_from_fast_path(&mut self, sig: &[f64; 24], t_us: u64) -> bool {
        let mut pattern = [0.0f64; PATTERN_DIM];
        let copy_len = 24.min(PATTERN_DIM);
        pattern[..copy_len].copy_from_slice(&sig[..copy_len]);

        let norm: f64 = pattern.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-10 { return false; }
        for d in 0..PATTERN_DIM {
            pattern[d] /= norm;
        }

        if !self.entries.is_empty() {
            let (_, best_sim) = self.nearest(&pattern);
            if best_sim >= self.theta_separate {
                return false;
            }
        }

        self.insert_new(&pattern, t_us);
        true
    }

    // ── Persistence (hand-rolled binary, matches perturbation_graph pattern) ──

    const MAGIC: [u8; 4] = *b"PMEM";
    const VERSION: u8 = 0x01;

    /// Bytes per entry on disk: PATTERN_DIM * 8 (f64s) + 4 (u32 access_count)
    /// + 8 (u64 last_access_t_us) + 8 (f64 strength).
    const ENTRY_BYTES: usize = PATTERN_DIM * 8 + 4 + 8 + 8;

    pub fn serialize(&self) -> Vec<u8> {
        let cap = 4 + 1 // magic + version
            + 8 + 8 + 8  // theta_complete, theta_separate, temperature
            + 4           // n_entries (u32)
            + self.entries.len() * Self::ENTRY_BYTES;
        let mut out = Vec::with_capacity(cap);

        // Header
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);

        // Adaptive thresholds + temperature
        out.extend_from_slice(&self.theta_complete.to_le_bytes());
        out.extend_from_slice(&self.theta_separate.to_le_bytes());
        out.extend_from_slice(&self.temperature.to_le_bytes());

        // Entries
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            for &v in &entry.pattern {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&entry.access_count.to_le_bytes());
            out.extend_from_slice(&entry.last_access_t_us.to_le_bytes());
            out.extend_from_slice(&entry.strength.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        // Minimum: magic(4) + version(1) + 3 f64s(24) + n_entries u32(4) = 33
        if data.len() < 33 {
            return None;
        }
        if data[0..4] != Self::MAGIC {
            return None;
        }
        if data[4] != Self::VERSION {
            return None;
        }
        let mut off = 5usize;

        let theta_complete = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let theta_separate = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let temperature = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;

        let n_entries = u32::from_le_bytes(data[off..off + 4].try_into().ok()?) as usize;
        off += 4;

        if n_entries > PATTERN_CAP {
            return None;
        }
        if data.len() < off + n_entries * Self::ENTRY_BYTES {
            return None;
        }

        let mut entries = Vec::with_capacity(n_entries);
        for _ in 0..n_entries {
            let mut pattern = [0.0f64; PATTERN_DIM];
            for v in pattern.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            let access_count = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let last_access_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            entries.push(PatternEntry {
                pattern,
                access_count,
                last_access_t_us,
                strength,
            });
        }

        Some(Self {
            entries,
            theta_complete,
            theta_separate,
            temperature,
            tick_counter: 0,
        })
    }
}

fn cosine_similarity(a: &[f64; PATTERN_DIM], b: &[f64; PATTERN_DIM]) -> f64 {
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for d in 0..PATTERN_DIM {
        dot += a[d] * b[d];
        na += a[d] * a[d];
        nb += b[d] * b[d];
    }
    let denom = (na * nb).sqrt();
    if denom < 1e-12 { 0.0 } else { dot / denom }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pattern(val: f64) -> [f64; PATTERN_DIM] {
        [val; PATTERN_DIM]
    }

    fn make_pattern_varied(seed: u64) -> [f64; PATTERN_DIM] {
        let mut p = [0.0f64; PATTERN_DIM];
        let mut rng = seed;
        for d in 0..PATTERN_DIM {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            p[d] = (rng >> 33) as f64 / (1u64 << 31) as f64;
        }
        p
    }

    #[test]
    fn empty_memory_creates_first_pattern() {
        let mut pm = PatternMemory::new();
        let p = make_pattern(1.0);
        let result = pm.query(&p, 1000);
        assert!(matches!(result, PatternQueryResult::Separated { new_id: 0 }));
        assert_eq!(pm.pattern_count(), 1);
    }

    #[test]
    fn identical_input_completes() {
        let mut pm = PatternMemory::new();
        let p = make_pattern_varied(42);
        pm.query(&p, 1000);

        let result = pm.query(&p, 2000);
        match result {
            PatternQueryResult::Completed { confidence, .. } => {
                assert!(confidence > 0.99, "identical input should have confidence ~1.0, got {confidence}");
            }
            _ => panic!("identical input should complete, not separate/ambiguous"),
        }
    }

    #[test]
    fn distant_input_separates() {
        let mut pm = PatternMemory::new();
        // Construct two orthogonal patterns
        let mut p1 = [0.0f64; PATTERN_DIM];
        let mut p2 = [0.0f64; PATTERN_DIM];
        for d in 0..PATTERN_DIM / 2 {
            p1[d] = 1.0;
        }
        for d in PATTERN_DIM / 2..PATTERN_DIM {
            p2[d] = 1.0;
        }
        pm.query(&p1, 1000);

        let result = pm.query(&p2, 2000);
        match result {
            PatternQueryResult::Separated { .. } => {}
            _ => panic!("orthogonal input should separate"),
        }
        assert_eq!(pm.pattern_count(), 2);
    }

    #[test]
    fn completion_updates_pattern() {
        let mut pm = PatternMemory::new();
        let p = make_pattern_varied(42);
        pm.query(&p, 1000);
        let original = pm.entries[0].pattern;

        // Slight perturbation — should still complete
        let mut p2 = p;
        p2[0] += 0.01;
        p2[1] -= 0.01;
        pm.query(&p2, 2000);

        let updated = pm.entries[0].pattern;
        let mut changed = false;
        for d in 0..PATTERN_DIM {
            if (updated[d] - original[d]).abs() > 1e-10 {
                changed = true;
                break;
            }
        }
        assert!(changed, "completion should update the stored pattern via Hebbian fast write");
    }

    #[test]
    fn hopfield_retrieval_returns_stored_pattern() {
        let mut pm = PatternMemory::new();
        let p1 = make_pattern_varied(10);
        let p2 = make_pattern_varied(20);
        pm.query(&p1, 1000);
        pm.query(&p2, 2000);

        // Query with partial cue close to p1
        let mut cue = p1;
        cue[0] = 0.0; // corrupt one dimension
        cue[1] = 0.0;
        let retrieved = pm.hopfield_retrieve(&cue);

        let sim_p1 = cosine_similarity(&retrieved, &p1);
        let sim_p2 = cosine_similarity(&retrieved, &p2);
        assert!(sim_p1 > sim_p2, "retrieved should be closer to p1 ({sim_p1}) than p2 ({sim_p2})");
    }

    #[test]
    fn thresholds_shift_with_maturity() {
        let mut pm = PatternMemory::new();
        pm.update_thresholds(0.0);
        assert!((pm.theta_complete - THETA_COMPLETE_INIT).abs() < 1e-6);
        assert!((pm.theta_separate - THETA_SEPARATE_INIT).abs() < 1e-6);

        pm.update_thresholds(1.0);
        assert!((pm.theta_complete - THETA_COMPLETE_MATURE).abs() < 1e-6);
        assert!((pm.theta_separate - THETA_SEPARATE_MATURE).abs() < 1e-6);
    }

    #[test]
    fn consolidation_merges_similar() {
        let mut pm = PatternMemory::new();
        let p = make_pattern_varied(42);
        pm.query(&p, 1000);

        // Add very similar pattern directly
        let mut p2 = p;
        p2[0] += 0.001;
        let entry = PatternEntry::new(p2, 2000);
        pm.entries.push(entry);
        assert_eq!(pm.pattern_count(), 2);

        pm.consolidate();
        assert_eq!(pm.pattern_count(), 1, "very similar patterns should merge during consolidation");
    }

    #[test]
    fn eviction_at_capacity() {
        let mut pm = PatternMemory::new();
        // Fill to capacity with diverse patterns
        for i in 0..PATTERN_CAP {
            let p = make_pattern_varied(i as u64 * 1000 + 7);
            pm.insert_new(&p, i as u64 * 100);
        }
        assert_eq!(pm.pattern_count(), PATTERN_CAP);

        // One more should trigger eviction
        let p_new = make_pattern_varied(0xFFFF);
        pm.insert_new(&p_new, 999_999);
        assert_eq!(pm.pattern_count(), PATTERN_CAP, "should stay at cap after eviction + insert");
    }

    #[test]
    fn cosine_similarity_self_is_one() {
        let p = make_pattern_varied(42);
        let sim = cosine_similarity(&p, &p);
        assert!((sim - 1.0).abs() < 1e-10, "self-similarity should be 1.0, got {sim}");
    }

    #[test]
    fn cosine_similarity_orthogonal_is_zero() {
        let mut a = [0.0f64; PATTERN_DIM];
        let mut b = [0.0f64; PATTERN_DIM];
        a[0] = 1.0;
        b[1] = 1.0;
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10, "orthogonal similarity should be ~0, got {sim}");
    }

    // ── Serialization round-trip tests ─────────────────────────────────

    #[test]
    fn serialize_empty_round_trips() {
        let pm = PatternMemory::new();
        let bytes = pm.serialize();
        let restored = PatternMemory::deserialize(&bytes).expect("deserialize empty should succeed");
        assert_eq!(restored.pattern_count(), 0);
        assert!((restored.theta_complete - pm.theta_complete).abs() < 1e-15);
        assert!((restored.theta_separate - pm.theta_separate).abs() < 1e-15);
        assert!((restored.temperature - pm.temperature).abs() < 1e-15);
    }

    #[test]
    fn serialize_with_entries_round_trips() {
        let mut pm = PatternMemory::new();
        // Insert orthogonal patterns directly to guarantee multiple entries
        for i in 0..5 {
            let mut p = [0.0f64; PATTERN_DIM];
            p[i % PATTERN_DIM] = 1.0;
            pm.entries.push(PatternEntry {
                pattern: p,
                access_count: (i as u32) + 1,
                last_access_t_us: i as u64 * 100_000,
                strength: 0.5 + (i as f64) * 0.1,
            });
        }
        pm.update_thresholds(0.6);
        assert!(pm.pattern_count() >= 2, "should have accumulated patterns");

        let bytes = pm.serialize();
        let restored = PatternMemory::deserialize(&bytes).expect("deserialize should succeed");

        assert_eq!(restored.pattern_count(), pm.pattern_count());
        assert!((restored.theta_complete - pm.theta_complete).abs() < 1e-15);
        assert!((restored.theta_separate - pm.theta_separate).abs() < 1e-15);

        // Verify each entry matches exactly
        for (orig, rest) in pm.entries.iter().zip(restored.entries.iter()) {
            for d in 0..PATTERN_DIM {
                assert!((orig.pattern[d] - rest.pattern[d]).abs() < 1e-15,
                    "pattern dim {d} mismatch");
            }
            assert_eq!(orig.access_count, rest.access_count);
            assert_eq!(orig.last_access_t_us, rest.last_access_t_us);
            assert!((orig.strength - rest.strength).abs() < 1e-15);
        }
    }

    #[test]
    fn deserialize_rejects_bad_magic() {
        let pm = PatternMemory::new();
        let mut bytes = pm.serialize();
        bytes[0] = b'X'; // corrupt magic
        assert!(PatternMemory::deserialize(&bytes).is_none());
    }

    #[test]
    fn deserialize_rejects_truncated() {
        let mut pm = PatternMemory::new();
        let p = make_pattern_varied(42);
        pm.query(&p, 1000);
        let bytes = pm.serialize();
        // Truncate mid-entry
        let truncated = &bytes[..bytes.len() - 10];
        assert!(PatternMemory::deserialize(truncated).is_none());
    }

    #[test]
    fn deserialize_rejects_too_short() {
        assert!(PatternMemory::deserialize(&[]).is_none());
        assert!(PatternMemory::deserialize(&[0u8; 10]).is_none());
    }

    #[test]
    fn seed_from_fast_path_inserts_normalized() {
        let mut pm = PatternMemory::new();
        let mut sig = [0.0f64; 24];
        sig[0] = 3.0;
        sig[1] = 4.0;
        assert!(pm.seed_from_fast_path(&sig, 1_000_000));
        assert_eq!(pm.pattern_count(), 1);
        let p = &pm.entries[0].pattern;
        let norm: f64 = p.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "seeded pattern should be unit-normalized, got {norm}");
    }

    #[test]
    fn seed_from_fast_path_rejects_duplicate() {
        let mut pm = PatternMemory::new();
        let mut sig = [0.0f64; 24];
        sig[0] = 1.0;
        assert!(pm.seed_from_fast_path(&sig, 1_000_000));
        assert!(!pm.seed_from_fast_path(&sig, 2_000_000));
        assert_eq!(pm.pattern_count(), 1);
    }

    #[test]
    fn seed_from_fast_path_rejects_zero() {
        let mut pm = PatternMemory::new();
        let sig = [0.0f64; 24];
        assert!(!pm.seed_from_fast_path(&sig, 1_000_000));
        assert_eq!(pm.pattern_count(), 0);
    }
}
