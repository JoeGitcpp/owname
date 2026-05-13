// Action-consequence loop substrate.
//
// Implements the foundation from:
//   docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md
//
// Numeric/time-only. Records write epochs, consequence windows, and
// pending/paired relationships. Does not classify meaning, infer targets,
// compare self vs other, or perform imitation.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::evolution_priors::BiasPriors;
use crate::substrate_pressure::{compute_pressure, PressureSample};
use crate::viewport::Viewport;
use crate::CONSEQUENCE_WINDOW_CAP;
use crate::INGRESS_ACCUMULATION_THRESHOLD;
use crate::PAIR_HISTORY_CAP;
use crate::TEMPORAL_PAIR_MAX_DELAY_US;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConsequenceModalityKind {
    #[default]
    GenericIngress,
    FramedByteReturn,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConsequencePairStatus {
    #[default]
    Pending,
    Paired,
}

/// Minimal lifecycle phase for a consequence window.
///
/// `Open`   — window is actively forming; end_t_us may still advance.
/// `Closed` — window has been finalized; extend calls are rejected.
///
/// No result, success, target, or meaning semantics. Phase is a substrate-level
/// formation state, not a consequence evaluation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConsequenceWindowPhase {
    #[default]
    Open,
    Closed,
}

impl ConsequenceWindowPhase {
    pub fn as_u64(self) -> u64 {
        match self {
            ConsequenceWindowPhase::Open => 0,
            ConsequenceWindowPhase::Closed => 1,
        }
    }
    pub fn from_u64(v: u64) -> Self {
        match v {
            1 => ConsequenceWindowPhase::Closed,
            _ => ConsequenceWindowPhase::Open,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequenceWindow {
    /// Stable monotonic ID assigned at creation. Never zero; never reused.
    /// Parallel to WriteEpochRecord::write_seq. Used by ActionConsequencePair
    /// so pair references remain valid after Vec compaction.
    /// Vec-position order and window_id order are not guaranteed to match
    /// under concurrent creation. Use window_id, not Vec index, as the stable key.
    pub window_id: u64,
    pub modality_kind: ConsequenceModalityKind,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
    /// Formation lifecycle phase. Open = still accumulating; Closed = finalized.
    pub phase: ConsequenceWindowPhase,
    /// Substrate time of last phase or boundary update.
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequencePair {
    /// Monotonic pair handle assigned when a write/window pair is recorded.
    /// Engineering locator only; not target, result, success, or meaning.
    pub pair_seq: u64,
    pub write_seq: u64,
    /// Stable window identifier. Matches ActionConsequenceWindow::window_id.
    /// This is NOT a Vec position — it remains valid across Vec compaction.
    pub window_id: u64,
    pub delay_us: u64,
    pub status: ConsequencePairStatus,
    pub pair_t_us: u64,
    /// Numeric similarity between write epoch magnitude and
    /// consequence window magnitude. [0,1]. Engineering approximation
    /// of curve alignment — not success/failure semantics.
    pub curve_similarity: f64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequenceState {
    pub n_windows: u32,
    pub windows: Vec<ActionConsequenceWindow>,
    pub n_pairs: u32,
    pub total_pairs: u64,
    pub total_framed_byte_pairs: u64,
    pub total_other_pairs: u64,
    pub pairs: Vec<ActionConsequencePair>,
}

#[derive(Clone, Debug)]
pub struct ActionConsequenceWindowEvent {
    pub modality_kind: ConsequenceModalityKind,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
}

fn pair_retention_pressure(
    pair: &ActionConsequencePair,
    typical_delay_us: u64,
    birth_sensitivity: &BiasPriors,
) -> f64 {
    let delay_stability = if typical_delay_us == 0 {
        0.5
    } else {
        let delta = pair.delay_us.abs_diff(typical_delay_us) as f64;
        (1.0 - delta / typical_delay_us.max(1) as f64).clamp(0.0, 1.0)
    };
    compute_pressure(
        PressureSample {
            alignment: pair.curve_similarity,
            support: if pair.status == ConsequencePairStatus::Paired {
                1.0
            } else {
                0.0
            },
            deviation: 1.0 - pair.curve_similarity,
            temporal_gap: 1.0 - delay_stability,
            recurrence: pair.curve_similarity,
            ..Default::default()
        },
        &birth_sensitivity.pressure_weights,
    )
}

fn evict_pairs_by_retention_pressure(
    pairs: &mut Vec<ActionConsequencePair>,
    birth_sensitivity: &BiasPriors,
) {
    while pairs.len() > PAIR_HISTORY_CAP {
        let typical_delay_us = {
            let mut delays: Vec<u64> = pairs.iter().map(|p| p.delay_us).collect();
            delays.sort_unstable();
            delays[delays.len() / 2]
        };
        let idx = pairs
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let a_pressure = pair_retention_pressure(a, typical_delay_us, birth_sensitivity);
                let b_pressure = pair_retention_pressure(b, typical_delay_us, birth_sensitivity);
                a_pressure
                    .partial_cmp(&b_pressure)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.pair_t_us.cmp(&b.pair_t_us))
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        pairs.remove(idx);
    }
}

impl Viewport {
    pub fn read_action_consequence(&self) -> ActionConsequenceState {
        self.action_consequence.read().unwrap().clone()
    }

    /// Append a consequence observation window in the Open phase.
    ///
    /// Legacy static-record API. Kept for backward compatibility with existing
    /// tests. The lifecycle APIs (`begin_action_consequence_window`,
    /// `extend_action_consequence_window`, `close_action_consequence_window`)
    /// are preferred for new code.
    pub fn push_action_consequence_window(&self, ev: ActionConsequenceWindowEvent) {
        let window_id = self
            .action_consequence_window_seq
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let t_us = self.clock_us.load(Ordering::Relaxed);
        let mut s = self.action_consequence.write().unwrap();
        s.windows.push(ActionConsequenceWindow {
            window_id,
            modality_kind: ev.modality_kind,
            start_t_us: ev.start_t_us,
            end_t_us: ev.end_t_us,
            magnitude: ev.magnitude,
            phase: ConsequenceWindowPhase::Open,
            last_update_t_us: t_us,
        });
        // Eviction: keep within CONSEQUENCE_WINDOW_CAP. Evict from the front,
        // oldest first. No result/meaning judgment — purely by append order.
        if s.windows.len() > CONSEQUENCE_WINDOW_CAP {
            let evict_n = s.windows.len() - CONSEQUENCE_WINDOW_CAP;
            s.windows.drain(0..evict_n);
        }
        // Single authoritative assignment after eviction.
        s.n_windows = s.windows.len() as u32;
    }

    /// Begin a new consequence window in the `Open` phase. Returns the window
    /// index. The window's `last_update_t_us` is set from `ev.start_t_us` so
    /// that quiescence tracking starts from the event time, not from the
    /// viewport internal clock (which may be zero in unit tests).
    ///
    /// No target, result, or meaning inference; only substrate-level lifecycle.
    pub fn begin_action_consequence_window(&self, ev: ActionConsequenceWindowEvent) -> u32 {
        let window_id = self
            .action_consequence_window_seq
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let mut s = self.action_consequence.write().unwrap();
        s.windows.push(ActionConsequenceWindow {
            window_id,
            modality_kind: ev.modality_kind,
            start_t_us: ev.start_t_us,
            end_t_us: ev.end_t_us,
            magnitude: ev.magnitude,
            phase: ConsequenceWindowPhase::Open,
            last_update_t_us: ev.start_t_us,
        });
        // Eviction: keep within CONSEQUENCE_WINDOW_CAP. Evict from the front,
        // oldest first. No result/meaning judgment — purely by append order.
        if s.windows.len() > CONSEQUENCE_WINDOW_CAP {
            let evict_n = s.windows.len() - CONSEQUENCE_WINDOW_CAP;
            s.windows.drain(0..evict_n);
        }
        // Single authoritative assignment after eviction.
        s.n_windows = s.windows.len() as u32;
        // Return the Vec index of the newly appended window. Computed after
        // eviction so it is never stale even when the cap is saturated.
        (s.windows.len() - 1) as u32
    }

    /// Extend an Open window: advance `end_t_us` and record `last_update_t_us`.
    ///
    /// Returns `true` iff the window was found and is still Open.
    /// Returns `false` (no-op) when:
    /// - `window_id` is not found in the windows list, or
    /// - the window is already Closed.
    pub fn extend_action_consequence_window(
        &self,
        window_id: u64,
        new_end_t_us: u64,
        t_us: u64,
    ) -> bool {
        let mut s = self.action_consequence.write().unwrap();
        let idx = match s.windows.iter().position(|w| w.window_id == window_id) {
            Some(i) => i,
            None => return false,
        };
        if s.windows[idx].phase == ConsequenceWindowPhase::Closed {
            return false;
        }
        s.windows[idx].end_t_us = new_end_t_us;
        s.windows[idx].last_update_t_us = t_us;
        true
    }

    /// Finalize an Open window, transitioning it to `Closed`.
    ///
    /// Records `t_us` as `last_update_t_us`. Returns `true` on success.
    /// Returns `false` when `window_id` is not found in the windows list
    /// (already-Closed windows are re-closable safely — this is idempotent
    /// on phase).
    pub fn close_action_consequence_window(&self, window_id: u64, t_us: u64) -> bool {
        let mut s = self.action_consequence.write().unwrap();
        let idx = match s.windows.iter().position(|w| w.window_id == window_id) {
            Some(i) => i,
            None => return false,
        };
        s.windows[idx].phase = ConsequenceWindowPhase::Closed;
        s.windows[idx].last_update_t_us = t_us;
        true
    }

    /// Engineering helper: record a numeric ingress change in the shared substrate.
    ///
    /// Finds the most-recently-created Open window and extends it; if none is
    /// Open, begins a new one. Returns the stable window_id.
    ///
    /// This is a foundation-layer accumulation primitive, not consequence
    /// classification. It does not infer targets, results, or success/failure
    /// semantics. The engineering use of `t_us` as a time boundary marker is a
    /// substrate approximation, not a fixed theoretical consequence time window.
    pub fn accumulate_ingress_change(&self, t_us: u64, magnitude: f64) -> u64 {
        {
            let mut s = self.action_consequence.write().unwrap();
            for w in s.windows.iter_mut().rev() {
                if w.phase == ConsequenceWindowPhase::Open {
                    w.end_t_us = t_us;
                    w.last_update_t_us = t_us;
                    return w.window_id;
                }
            }
        }
        // No Open window — begin a new one.
        let window_id = self
            .action_consequence_window_seq
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let mut s = self.action_consequence.write().unwrap();
        s.windows.push(ActionConsequenceWindow {
            window_id,
            modality_kind: ConsequenceModalityKind::GenericIngress,
            start_t_us: t_us,
            end_t_us: t_us,
            magnitude,
            phase: ConsequenceWindowPhase::Open,
            last_update_t_us: t_us,
        });
        // Eviction: keep within CONSEQUENCE_WINDOW_CAP. Evict from the front,
        // oldest first. No result/meaning judgment — purely by append order.
        if s.windows.len() > CONSEQUENCE_WINDOW_CAP {
            let evict_n = s.windows.len() - CONSEQUENCE_WINDOW_CAP;
            s.windows.drain(0..evict_n);
        }
        // Single authoritative assignment after eviction.
        s.n_windows = s.windows.len() as u32;
        window_id
    }

    /// Live temporal bridge: scan the real write_epoch_history for the nearest
    /// legal predecessor of the most-recent consequence window, and if one
    /// exists, record a new `Paired` temporal link.
    ///
    /// Selection is substrate-time only: from all recorded write epochs, the
    /// scan keeps those whose `start_t_us` strictly precedes the window's
    /// `start_t_us` and whose delay to the window is at most
    /// `TEMPORAL_PAIR_MAX_DELAY_US`; among those, the one with the smallest
    /// delay is chosen. Append order in the ledger does not decide the
    /// winner — substrate time does.
    ///
    /// This is a substrate-level engineering approximation of "write before
    /// consequence within a plausible time envelope". It does NOT:
    /// - classify consequence meaning
    /// - infer output targets
    /// - evaluate success or failure
    /// - compare self vs other (imitation)
    ///
    /// `TEMPORAL_PAIR_MAX_DELAY_US` is an engineering knob, not a fixed
    /// theoretical consequence time window.
    ///
    /// Returns `true` iff a new pair was recorded.
    pub fn try_auto_pair_latest_epoch_to_window(&self, _now_t_us: u64) -> bool {
        // Step 1: read the most-recent consequence window (read lock released
        // before Step 2).
        let (window_id, window_start_t_us) = {
            let ac = self.action_consequence.read().unwrap();
            if ac.windows.is_empty() {
                return false;
            }
            let last = ac.windows.last().unwrap();
            (last.window_id, last.start_t_us)
        };

        // Step 2: scan the real write epoch ledger for the legal predecessor
        // with the smallest non-negative delay to the window, still within
        // the engineering time window. Selection is strictly by substrate
        // time (`epoch.start_t_us`); the latest-appended entry has no
        // privilege over earlier entries.
        let selected = {
            let ao = self.action_output.read().unwrap();
            let mut best: Option<(u64, u64)> = None; // (write_seq, delay_us)
            for ep in ao.write_epoch_history.iter() {
                // Temporal ordering: epoch must strictly precede the window.
                if ep.start_t_us >= window_start_t_us {
                    continue;
                }
                let delay_us = window_start_t_us - ep.start_t_us;
                // Within the engineering time window.
                if delay_us > TEMPORAL_PAIR_MAX_DELAY_US {
                    continue;
                }
                match best {
                    None => best = Some((ep.write_seq, delay_us)),
                    Some((_, cur_delay)) if delay_us < cur_delay => {
                        best = Some((ep.write_seq, delay_us));
                    }
                    _ => {}
                }
            }
            best
        };

        let (write_seq, delay_us) = match selected {
            Some(pair) => pair,
            None => return false,
        };

        // Step 3: duplicate guard — same (write_seq, window_id) pair must
        // not be recorded twice.
        {
            let ac = self.action_consequence.read().unwrap();
            if ac
                .pairs
                .iter()
                .any(|p| p.write_seq == write_seq && p.window_id == window_id)
            {
                return false;
            }
        }

        // Step 4: look up the closed write epoch boundary magnitude and
        // consequence window magnitude for similarity. The write-side value
        // comes from the epoch ledger, not the live channel, so later Adjust
        // events cannot rewrite the evidence used for this pair. The
        // consequence side also carries the return modality for cumulative
        // observer traces.
        let (write_mag, consequence_mag, modality_kind) = {
            let ao = self.action_output.read().unwrap();
            let write_mag = ao
                .write_epoch_history
                .iter()
                .find(|ep| ep.write_seq == write_seq)
                .map(|ep| ep.start_magnitude)
                .unwrap_or(0.0);
            let ac = self.action_consequence.read().unwrap();
            let consequence_mag = ac
                .windows
                .iter()
                .find(|w| w.window_id == window_id)
                .map(|w| (w.magnitude, w.modality_kind))
                .unwrap_or((0.0, ConsequenceModalityKind::GenericIngress));
            (write_mag, consequence_mag.0, consequence_mag.1)
        };
        let curve_similarity = 1.0 - (write_mag - consequence_mag).abs();

        // Step 5: record the pair under the write lock, with a final duplicate
        // guard to remain correct under concurrent callers.
        let pair_t_us = self.clock_us.load(Ordering::Relaxed);
        let mut ac = self.action_consequence.write().unwrap();
        if !ac.windows.iter().any(|w| w.window_id == window_id) {
            return false;
        }
        if ac
            .pairs
            .iter()
            .any(|p| p.write_seq == write_seq && p.window_id == window_id)
        {
            return false;
        }
        let pair_seq = self
            .action_consequence_pair_seq
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        ac.pairs.push(ActionConsequencePair {
            pair_seq,
            write_seq,
            window_id,
            delay_us,
            status: ConsequencePairStatus::Paired,
            pair_t_us,
            curve_similarity: curve_similarity.clamp(0.0, 1.0),
        });
        evict_pairs_by_retention_pressure(&mut ac.pairs, &self.birth_sensitivity);
        ac.n_pairs = ac.pairs.len() as u32;
        ac.total_pairs = pair_seq;
        bump_pair_source_totals(&mut ac, modality_kind);
        true
    }

    /// Engineering helper: close any Open window whose `last_update_t_us` is
    /// more than `quiescence_us` µs behind `now_t_us`.
    ///
    /// `quiescence_us` is an engineering approximation of the quiescence gap
    /// that signals the natural end of an ingress-change episode. It is NOT a
    /// fixed theoretical consequence boundary; it is a substrate knob that
    /// should be formed and revised through exploration history.
    ///
    /// Does not produce consequence pairs, target inference, or result
    /// classification.
    pub fn tick_ingress_accumulation(&self, now_t_us: u64, quiescence_us: u64) {
        let mut s = self.action_consequence.write().unwrap();
        for w in s.windows.iter_mut() {
            if w.phase == ConsequenceWindowPhase::Open
                && now_t_us.saturating_sub(w.last_update_t_us) >= quiescence_us
            {
                w.phase = ConsequenceWindowPhase::Closed;
                w.last_update_t_us = now_t_us;
            }
        }
    }

    /// Explicitly pair a write epoch (by write_seq) with a window (by stable
    /// window_id) at a given delay. Pairing is numeric-only; no consequence
    /// classification.
    ///
    /// Returns `true` iff the pair was recorded. Returns `false` (and records
    /// nothing) when:
    /// - `write_seq == 0` (null sentinel), or
    /// - `window_id == 0` (null sentinel), or
    /// - `write_seq` is not present in the write_epoch_history ledger of
    ///   action_output (i.e. no Begin event with that seq was ever recorded), or
    /// - `window_id` is not present in the current windows list.
    ///
    /// Validation is against the real epoch ledger, not just the max_issued
    /// counter, so a seq that was never produced by a Begin event is always
    /// rejected even if it happens to fall within the counter range.
    ///
    /// `pair_t_us` is set to the viewport's shared substrate clock (`clock_us`)
    /// at call time.
    pub fn compute_typical_consequence_delay(&self) -> u64 {
        let ac = self.action_consequence.read().unwrap();
        if ac.pairs.is_empty() {
            return 0;
        }

        let recent: Vec<u64> = ac.pairs.iter().rev().take(16).map(|p| p.delay_us).collect();

        if recent.is_empty() {
            return 0;
        }

        let mut sorted = recent.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    pub fn pair_action_consequence(&self, write_seq: u64, window_id: u64, delay_us: u64) -> bool {
        // write_seq 0 is the null sentinel.
        if write_seq == 0 {
            return false;
        }
        // window_id 0 is the null sentinel.
        if window_id == 0 {
            return false;
        }

        // Validate against the real epoch ledger, not just the counter.
        {
            let ao = self.action_output.read().unwrap();
            if !ao
                .write_epoch_history
                .iter()
                .any(|r| r.write_seq == write_seq)
            {
                return false;
            }
        }

        // Substrate time at pairing — comes from the body's shared clock.
        let pair_t_us = self.clock_us.load(Ordering::Relaxed);

        // Look up the closed write epoch boundary magnitude and consequence
        // window magnitude for curve similarity computation. The consequence
        // side also carries the return modality for cumulative observer traces.
        let (write_mag, consequence_mag, modality_kind) = {
            let ao = self.action_output.read().unwrap();
            let write_mag = ao
                .write_epoch_history
                .iter()
                .find(|ep| ep.write_seq == write_seq)
                .map(|ep| ep.start_magnitude)
                .unwrap_or(0.0);
            let ac_read = self.action_consequence.read().unwrap();
            let consequence_mag = ac_read
                .windows
                .iter()
                .find(|w| w.window_id == window_id)
                .map(|w| (w.magnitude, w.modality_kind))
                .unwrap_or((0.0, ConsequenceModalityKind::GenericIngress));
            (write_mag, consequence_mag.0, consequence_mag.1)
        };
        let curve_similarity = 1.0 - (write_mag - consequence_mag).abs();

        let mut s = self.action_consequence.write().unwrap();

        // Validate window_id is present in the windows list.
        if !s.windows.iter().any(|w| w.window_id == window_id) {
            return false;
        }

        // Duplicate guard.
        if s.pairs
            .iter()
            .any(|p| p.write_seq == write_seq && p.window_id == window_id)
        {
            return false;
        }

        let pair_seq = self
            .action_consequence_pair_seq
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        s.pairs.push(ActionConsequencePair {
            pair_seq,
            write_seq,
            window_id,
            delay_us,
            status: ConsequencePairStatus::Paired,
            pair_t_us,
            curve_similarity: curve_similarity.clamp(0.0, 1.0),
        });
        // Eviction: keep within PAIR_HISTORY_CAP. Retention pressure is computed
        // from numeric write-return alignment only; no target/result semantics.
        evict_pairs_by_retention_pressure(&mut s.pairs, &self.birth_sensitivity);
        s.n_pairs = s.pairs.len() as u32;
        s.total_pairs = pair_seq;
        bump_pair_source_totals(&mut s, modality_kind);
        true
    }
}

fn bump_pair_source_totals(
    state: &mut ActionConsequenceState,
    modality_kind: ConsequenceModalityKind,
) {
    match modality_kind {
        ConsequenceModalityKind::FramedByteReturn => {
            state.total_framed_byte_pairs = state.total_framed_byte_pairs.saturating_add(1);
        }
        ConsequenceModalityKind::GenericIngress => {
            state.total_other_pairs = state.total_other_pairs.saturating_add(1);
        }
    }
}

/// VIB ingress consequence: if VIB energy exceeds the accumulation threshold,
/// record the change in the shared action-consequence substrate.
///
/// This is a substrate-level accumulation primitive — it does not infer
/// targets, results, or success/failure semantics.
pub fn tick_vib_ingress_consequence(vp: &Arc<Viewport>, t: u64) {
    let vib_energy = *vp.ambient.vib.event_energy.read().unwrap();
    if vib_energy > INGRESS_ACCUMULATION_THRESHOLD {
        vp.accumulate_ingress_change(t, vib_energy.min(1.0));
    }
}

/// Unified ingress accumulation ticker: close quiescent windows across all
/// modalities and attempt auto-pairing for any newly closed windows.
///
/// Called from a periodic ticker in the body process (e.g. 10 Hz). This is
/// the single authoritative path for quiescence-based window closure,
/// replacing per-adapter accumulation ticks.
///
/// `quiescence_us` is an engineering approximation of the quiescence gap
/// that signals the natural end of an ingress-change episode.
pub fn tick_all_ingress_accumulation(vp: &Arc<Viewport>, now_t_us: u64, quiescence_us: u64) {
    vp.tick_ingress_accumulation(now_t_us, quiescence_us);
    vp.try_auto_pair_latest_epoch_to_window(now_t_us);
}
