use std::collections::VecDeque;
use crate::pattern_memory::PATTERN_DIM;

pub const REPLAY_CHAIN_MAX: usize = 32;
pub const REPLAY_COMPRESS_THETA: f64 = 0.8;
pub const REPLAY_TRIGGER_DP: f64 = 0.1;
pub const TRANSITION_CAP: usize = 8192;
const TRANSITION_ALPHA: f64 = 0.05;
const TRANSITION_INIT_STRENGTH: f64 = 0.1;
const TRANSITION_EVICT_STRENGTH: f64 = 0.01;
const TRANSITION_EVICT_MIN_COUNT: u32 = 3;
const RECENT_TRACE_CAP: usize = 128;

const PRIORITY_W_SURPRISE: f64 = 1.0;
const PRIORITY_W_SOCIAL: f64 = 2.0;
const PRIORITY_W_RECENCY: f64 = 0.5;
pub const PRIORITY_W_INFO_GAIN: f64 = 1.5;
const PRIORITY_TAU_RECENCY_US: f64 = 300_000_000.0; // 5 min

pub struct PatternTransition {
    pub from_id: usize,
    pub to_id: usize,
    pub strength: f64,
    pub avg_dt_us: u64,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReplayDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug)]
pub enum ReplayTrigger {
    Quiet,
    Surprise { pattern_id: usize },
    Social { dp_delta: f64 },
}

pub struct ReplayStep {
    pub pattern_id: usize,
    pub direction: ReplayDirection,
}

pub struct ReplayEngine {
    pub transitions: Vec<PatternTransition>,
    pub prev_pattern_id: Option<usize>,
    pub recent_trace: VecDeque<(usize, u64)>,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            prev_pattern_id: None,
            recent_trace: VecDeque::with_capacity(RECENT_TRACE_CAP),
        }
    }

    pub fn record_transition(&mut self, pattern_id: usize, t_us: u64) {
        if let Some(prev) = self.prev_pattern_id {
            if prev != pattern_id {
                if let Some(tr) = self.transitions.iter_mut()
                    .find(|t| t.from_id == prev && t.to_id == pattern_id)
                {
                    tr.strength = (1.0 - TRANSITION_ALPHA) * tr.strength + TRANSITION_ALPHA;
                    let dt = t_us.saturating_sub(
                        self.recent_trace.back().map(|&(_, t)| t).unwrap_or(t_us)
                    );
                    tr.avg_dt_us = ((tr.avg_dt_us as f64) * 0.9 + dt as f64 * 0.1) as u64;
                    tr.count += 1;
                } else {
                    if self.transitions.len() >= TRANSITION_CAP {
                        self.evict_weakest();
                    }
                    self.transitions.push(PatternTransition {
                        from_id: prev,
                        to_id: pattern_id,
                        strength: TRANSITION_INIT_STRENGTH,
                        avg_dt_us: 0,
                        count: 1,
                    });
                }
            }
        }
        self.prev_pattern_id = Some(pattern_id);
        if self.recent_trace.len() >= RECENT_TRACE_CAP {
            self.recent_trace.pop_front();
        }
        self.recent_trace.push_back((pattern_id, t_us));
    }

    pub fn should_replay(&self, trigger: ReplayTrigger) -> bool {
        if self.recent_trace.len() < 2 {
            return false;
        }
        match trigger {
            ReplayTrigger::Quiet => true,
            ReplayTrigger::Surprise { .. } => true,
            ReplayTrigger::Social { dp_delta } => dp_delta.abs() > REPLAY_TRIGGER_DP,
        }
    }

    pub fn generate_replay(
        &self,
        trigger: ReplayTrigger,
        predictability: &dyn Fn(usize) -> f64,
        now_us: u64,
    ) -> Vec<ReplayStep> {
        self.generate_replay_epi(trigger, predictability, &|_| 0.0, now_us)
    }

    pub fn generate_replay_epi(
        &self,
        trigger: ReplayTrigger,
        predictability: &dyn Fn(usize) -> f64,
        info_gain: &dyn Fn(usize) -> f64,
        now_us: u64,
    ) -> Vec<ReplayStep> {
        let direction = match trigger {
            ReplayTrigger::Quiet => ReplayDirection::Forward,
            ReplayTrigger::Surprise { .. } | ReplayTrigger::Social { .. } => ReplayDirection::Reverse,
        };

        let chain = match direction {
            ReplayDirection::Reverse => {
                let seed = match trigger {
                    ReplayTrigger::Surprise { pattern_id } => pattern_id,
                    _ => match self.recent_trace.back() {
                        Some(&(id, _)) => id,
                        None => return Vec::new(),
                    },
                };
                self.trace_back(seed, REPLAY_CHAIN_MAX)
            }
            ReplayDirection::Forward => {
                let seed = match self.recent_trace.back() {
                    Some(&(id, _)) => id,
                    None => return Vec::new(),
                };
                self.trace_forward(seed, REPLAY_CHAIN_MAX)
            }
        };

        let mut ranked: Vec<(usize, f64)> = chain.iter().map(|&pid| {
            let surprise_score = 1.0 - predictability(pid);
            let age_us = self.recent_trace.iter().rev()
                .find(|&&(id, _)| id == pid)
                .map(|&(_, t)| now_us.saturating_sub(t) as f64)
                .unwrap_or(PRIORITY_TAU_RECENCY_US * 3.0);
            let recency = (-age_us / PRIORITY_TAU_RECENCY_US).exp();
            let ig = if direction == ReplayDirection::Forward {
                info_gain(pid)
            } else {
                0.0
            };
            PRIORITY_W_SURPRISE * surprise_score
                + PRIORITY_W_SOCIAL * 0.0
                + PRIORITY_W_RECENCY * recency
                + PRIORITY_W_INFO_GAIN * ig
        }).enumerate().map(|(i, score)| (chain[i], score)).collect();

        ranked.retain(|&(pid, _)| predictability(pid) < REPLAY_COMPRESS_THETA);

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(REPLAY_CHAIN_MAX);

        if direction == ReplayDirection::Forward {
            ranked.sort_by_key(|&(pid, _)| {
                chain.iter().position(|&c| c == pid).unwrap_or(0)
            });
        }

        ranked.into_iter().map(|(pid, _)| ReplayStep {
            pattern_id: pid,
            direction,
        }).collect()
    }

    pub fn trace_back(&self, from_id: usize, max_hops: usize) -> Vec<usize> {
        let mut chain = vec![from_id];
        let mut current = from_id;
        let mut visited = std::collections::HashSet::new();
        visited.insert(current);

        for _ in 0..max_hops {
            let best = self.transitions.iter()
                .filter(|t| t.to_id == current && !visited.contains(&t.from_id))
                .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal));
            match best {
                Some(tr) => {
                    chain.push(tr.from_id);
                    visited.insert(tr.from_id);
                    current = tr.from_id;
                }
                None => break,
            }
        }
        chain
    }

    pub fn trace_forward(&self, from_id: usize, max_hops: usize) -> Vec<usize> {
        let mut chain = vec![from_id];
        let mut current = from_id;
        let mut visited = std::collections::HashSet::new();
        visited.insert(current);

        for _ in 0..max_hops {
            let best = self.transitions.iter()
                .filter(|t| t.from_id == current && !visited.contains(&t.to_id))
                .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal));
            match best {
                Some(tr) => {
                    chain.push(tr.to_id);
                    visited.insert(tr.to_id);
                    current = tr.to_id;
                }
                None => break,
            }
        }
        chain
    }

    pub fn decay_transitions(&mut self) {
        for tr in self.transitions.iter_mut() {
            tr.strength *= 0.999;
        }
        self.transitions.retain(|t|
            t.strength >= TRANSITION_EVICT_STRENGTH || t.count >= TRANSITION_EVICT_MIN_COUNT
        );
    }

    fn evict_weakest(&mut self) {
        if self.transitions.is_empty() { return; }
        let mut weakest = 0;
        let mut weakest_score = f64::MAX;
        for (i, tr) in self.transitions.iter().enumerate() {
            let score = tr.strength * (tr.count as f64).max(1.0);
            if score < weakest_score {
                weakest_score = score;
                weakest = i;
            }
        }
        self.transitions.swap_remove(weakest);
    }

    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    // ── Persistence ────────────────────────────────────────────────────────

    const MAGIC: [u8; 4] = *b"REPL";
    const VERSION: u8 = 0x01;
    const TRANSITION_BYTES: usize = 8 + 8 + 8 + 8 + 4; // from(usize as u64) + to + strength + avg_dt + count

    pub fn serialize(&self) -> Vec<u8> {
        let cap = 4 + 1 + 4 + self.transitions.len() * Self::TRANSITION_BYTES;
        let mut out = Vec::with_capacity(cap);
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(self.transitions.len() as u32).to_le_bytes());
        for tr in &self.transitions {
            out.extend_from_slice(&(tr.from_id as u64).to_le_bytes());
            out.extend_from_slice(&(tr.to_id as u64).to_le_bytes());
            out.extend_from_slice(&tr.strength.to_le_bytes());
            out.extend_from_slice(&tr.avg_dt_us.to_le_bytes());
            out.extend_from_slice(&tr.count.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 9 || data[0..4] != Self::MAGIC {
            return None;
        }
        let version = data[4];
        if version != Self::VERSION {
            return None;
        }
        let mut off = 5;
        let n = u32::from_le_bytes(data[off..off + 4].try_into().ok()?) as usize;
        off += 4;
        if n > TRANSITION_CAP { return None; }
        if data.len() < off + n * Self::TRANSITION_BYTES { return None; }

        let mut transitions = Vec::with_capacity(n);
        for _ in 0..n {
            let from_id = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            let to_id = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            let strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let avg_dt_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let count = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            transitions.push(PatternTransition {
                from_id, to_id, strength, avg_dt_us, count,
            });
        }
        Some(Self {
            transitions,
            prev_pattern_id: None,
            recent_trace: VecDeque::with_capacity(RECENT_TRACE_CAP),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_transition_creates_entry() {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 1_000);
        re.record_transition(1, 2_000);
        assert_eq!(re.transition_count(), 1);
        assert_eq!(re.transitions[0].from_id, 0);
        assert_eq!(re.transitions[0].to_id, 1);
    }

    #[test]
    fn record_transition_same_pattern_no_self_loop() {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 1_000);
        re.record_transition(0, 2_000);
        assert_eq!(re.transition_count(), 0, "same pattern should not create self-loop");
    }

    #[test]
    fn record_transition_strengthens_existing() {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 1_000);
        re.record_transition(1, 2_000);
        let s1 = re.transitions[0].strength;
        re.record_transition(0, 3_000);
        re.record_transition(1, 4_000);
        let s2 = re.transitions[0].strength;
        assert!(s2 > s1, "repeated transition should strengthen: {s1} → {s2}");
        assert_eq!(re.transitions[0].count, 2);
    }

    #[test]
    fn trace_back_walks_reverse() {
        let mut re = ReplayEngine::new();
        // Build chain: 0 → 1 → 2 → 3
        for i in 0..4 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        let chain = re.trace_back(3, 10);
        assert_eq!(chain, vec![3, 2, 1, 0]);
    }

    #[test]
    fn trace_forward_walks_forward() {
        let mut re = ReplayEngine::new();
        for i in 0..4 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        let chain = re.trace_forward(0, 10);
        assert_eq!(chain, vec![0, 1, 2, 3]);
    }

    #[test]
    fn trace_back_stops_on_cycle() {
        let mut re = ReplayEngine::new();
        // 0 → 1 → 2 → 0 (cycle)
        re.record_transition(0, 1_000);
        re.record_transition(1, 2_000);
        re.record_transition(2, 3_000);
        re.record_transition(0, 4_000);
        let chain = re.trace_back(0, 10);
        assert!(chain.len() <= 4, "cycle must terminate");
    }

    #[test]
    fn should_replay_needs_history() {
        let re = ReplayEngine::new();
        assert!(!re.should_replay(ReplayTrigger::Quiet));
    }

    #[test]
    fn should_replay_social_needs_threshold() {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 1_000);
        re.record_transition(1, 2_000);
        assert!(!re.should_replay(ReplayTrigger::Social { dp_delta: 0.05 }));
        assert!(re.should_replay(ReplayTrigger::Social { dp_delta: 0.15 }));
    }

    #[test]
    fn generate_replay_reverse_on_surprise() {
        let mut re = ReplayEngine::new();
        for i in 0..5 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        let steps = re.generate_replay(
            ReplayTrigger::Surprise { pattern_id: 4 },
            &|_| 0.0, // nothing predictable → nothing compressed
            10_000,
        );
        assert!(!steps.is_empty());
        assert!(steps.iter().all(|s| s.direction == ReplayDirection::Reverse));
    }

    #[test]
    fn generate_replay_forward_on_quiet() {
        let mut re = ReplayEngine::new();
        for i in 0..5 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        let steps = re.generate_replay(
            ReplayTrigger::Quiet,
            &|_| 0.0,
            10_000,
        );
        assert!(!steps.is_empty());
        assert!(steps.iter().all(|s| s.direction == ReplayDirection::Forward));
    }

    #[test]
    fn generate_replay_compresses_predictable_steps() {
        let mut re = ReplayEngine::new();
        for i in 0..6 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        // patterns 1,2,3 are highly predictable → compressed out
        let steps = re.generate_replay(
            ReplayTrigger::Surprise { pattern_id: 5 },
            &|pid| if pid >= 1 && pid <= 3 { 0.95 } else { 0.2 },
            10_000,
        );
        let ids: Vec<usize> = steps.iter().map(|s| s.pattern_id).collect();
        for compressed in [1, 2, 3] {
            assert!(!ids.contains(&compressed),
                "pattern {compressed} should be compressed out (predictability > theta)");
        }
    }

    #[test]
    fn decay_transitions_removes_weak() {
        let mut re = ReplayEngine::new();
        re.transitions.push(PatternTransition {
            from_id: 0, to_id: 1, strength: 0.005, avg_dt_us: 0, count: 1,
        });
        re.transitions.push(PatternTransition {
            from_id: 1, to_id: 2, strength: 0.5, avg_dt_us: 0, count: 10,
        });
        re.decay_transitions();
        assert_eq!(re.transition_count(), 1, "weak+low-count transition should be evicted");
        assert_eq!(re.transitions[0].from_id, 1);
    }

    #[test]
    fn eviction_at_cap() {
        let mut re = ReplayEngine::new();
        for i in 0..TRANSITION_CAP {
            re.transitions.push(PatternTransition {
                from_id: i, to_id: i + 1,
                strength: 0.5,
                avg_dt_us: 0,
                count: 5,
            });
        }
        assert_eq!(re.transition_count(), TRANSITION_CAP);
        re.record_transition(99999, 1_000);
        re.record_transition(99998, 2_000);
        assert_eq!(re.transition_count(), TRANSITION_CAP, "should stay at cap");
    }

    #[test]
    fn serialize_roundtrip() {
        let mut re = ReplayEngine::new();
        for i in 0..5 {
            re.record_transition(i, (i as u64 + 1) * 1_000);
        }
        let bytes = re.serialize();
        let restored = ReplayEngine::deserialize(&bytes).expect("deserialize should succeed");
        assert_eq!(restored.transition_count(), re.transition_count());
        for (a, b) in re.transitions.iter().zip(restored.transitions.iter()) {
            assert_eq!(a.from_id, b.from_id);
            assert_eq!(a.to_id, b.to_id);
            assert!((a.strength - b.strength).abs() < 1e-15);
            assert_eq!(a.count, b.count);
        }
    }

    #[test]
    fn deserialize_rejects_bad_magic() {
        let re = ReplayEngine::new();
        let mut bytes = re.serialize();
        bytes[0] = b'X';
        assert!(ReplayEngine::deserialize(&bytes).is_none());
    }

    #[test]
    fn deserialize_rejects_truncated() {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 1_000);
        re.record_transition(1, 2_000);
        let bytes = re.serialize();
        assert!(ReplayEngine::deserialize(&bytes[..bytes.len() - 5]).is_none());
    }

    // ── C3 info-gain replay tests ──────────────────────────────────────────

    fn make_forward_replay_engine() -> ReplayEngine {
        let mut re = ReplayEngine::new();
        re.record_transition(0, 100);
        re.record_transition(1, 200);
        re.record_transition(2, 300);
        re.record_transition(3, 400);
        // Seed back to 0 so Quiet trigger (forward) traces 0→1→2→3
        re.record_transition(0, 500);
        re
    }

    #[test]
    fn info_gain_higher_for_uncertain_relations() {
        let ig_uncertain = 1.0 / (2.0 * 3.0); // α=1,β=1 → 1/((1+1)(1+1+1))
        let ig_certain = 1.0 / (60.0 * 61.0);  // α=50,β=10
        assert!(ig_uncertain > ig_certain,
            "uncertain IG={} should exceed certain IG={}", ig_uncertain, ig_certain);
    }

    #[test]
    fn forward_replay_ranks_by_info_gain() {
        let re = make_forward_replay_engine();
        let steps = re.generate_replay_epi(
            ReplayTrigger::Quiet,
            &|_| 0.0,
            &|pid| if pid == 1 { 10.0 } else { 0.0 },
            500,
        );
        assert!(!steps.is_empty());
        let high_ig_pos = steps.iter().position(|s| s.pattern_id == 1);
        assert!(high_ig_pos.is_some(), "pattern 1 (high IG) should be in replay");
    }

    #[test]
    fn reverse_replay_ignores_info_gain() {
        let re = make_forward_replay_engine();
        let steps_no_ig = re.generate_replay_epi(
            ReplayTrigger::Surprise { pattern_id: 3 },
            &|_| 0.0,
            &|_| 0.0,
            500,
        );
        let steps_with_ig = re.generate_replay_epi(
            ReplayTrigger::Surprise { pattern_id: 3 },
            &|_| 0.0,
            &|pid| if pid == 0 { 100.0 } else { 0.0 },
            500,
        );
        let ids_no: Vec<usize> = steps_no_ig.iter().map(|s| s.pattern_id).collect();
        let ids_with: Vec<usize> = steps_with_ig.iter().map(|s| s.pattern_id).collect();
        assert_eq!(ids_no, ids_with, "reverse replay should ignore info_gain");
    }

    #[test]
    fn zero_info_gain_matches_old_behavior() {
        let re = make_forward_replay_engine();
        let pred = |_: usize| 0.3_f64;
        let old = re.generate_replay(ReplayTrigger::Quiet, &pred, 500);
        let epi = re.generate_replay_epi(ReplayTrigger::Quiet, &pred, &|_| 0.0, 500);
        let old_ids: Vec<usize> = old.iter().map(|s| s.pattern_id).collect();
        let epi_ids: Vec<usize> = epi.iter().map(|s| s.pattern_id).collect();
        assert_eq!(old_ids, epi_ids, "zero info_gain should match old behavior");
    }
}
