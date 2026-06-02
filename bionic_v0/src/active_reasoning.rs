use crate::substrate::unit;
use crate::action_consequence::ConsequencePairStatus;
use crate::viewport::Viewport;
use crate::ACTIVE_REASONING_GAP_TRACE_CAP;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveReasoningGapTrace {
    pub t_us: u64,
    pub gap_pressure: f64,
    pub closure_score: f64,
    pub reentry_score: f64,
    pub active_cycles: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveReasoningGapState {
    pub gap_pressure: f64,
    pub drive_score: f64,
    pub persistence_score: f64,
    pub closure_score: f64,
    pub reentry_score: f64,
    pub focus_slot_id: i8,
    pub focus_edge_id: i16,
    pub active_cycles: u32,
    pub last_pair_seq: u64,
    pub last_update_t_us: u64,
    pub history_len: u8,
    pub history_head: usize,
    pub total_samples: u64,
    pub history: [ActiveReasoningGapTrace; ACTIVE_REASONING_GAP_TRACE_CAP],
}

impl Default for ActiveReasoningGapState {
    fn default() -> Self {
        Self {
            gap_pressure: 0.0,
            drive_score: 0.0,
            persistence_score: 0.0,
            closure_score: 0.0,
            reentry_score: 0.0,
            focus_slot_id: -1,
            focus_edge_id: -1,
            active_cycles: 0,
            last_pair_seq: 0,
            last_update_t_us: 0,
            history_len: 0,
            history_head: 0,
            total_samples: 0,
            history: [ActiveReasoningGapTrace::default(); ACTIVE_REASONING_GAP_TRACE_CAP],
        }
    }
}

impl ActiveReasoningGapState {
    pub fn recent_trace(&self, index: usize) -> Option<ActiveReasoningGapTrace> {
        if index >= self.history_len as usize {
            return None;
        }
        let idx = (self.history_head + ACTIVE_REASONING_GAP_TRACE_CAP - 1 - index)
            % ACTIVE_REASONING_GAP_TRACE_CAP;
        Some(self.history[idx])
    }
}

impl Viewport {
    pub fn refresh_active_reasoning_gap(&self, now_us: u64) {
        let rs = self.read_global_reasoning();
        let memory = self.read_global_reasoning_memory_summary();
        let action_consequence = self.read_action_consequence();

        let newest_pair = action_consequence
            .pairs
            .iter()
            .filter(|pair| pair.status == ConsequencePairStatus::Paired)
            .max_by_key(|pair| pair.pair_seq)
            .map(|pair| (pair.pair_seq, pair.curve_similarity))
            .unwrap_or((0, 0.0));

        let mut state = self.active_reasoning_gap.write().unwrap();
        let closure_signal = if newest_pair.0 > state.last_pair_seq {
            unit(newest_pair.1)
        } else {
            0.0
        };

        let drive = unit(
            rs.global_surprise_load
                .max(rs.network_instability_score * 0.5)
                .max(memory.novelty_score),
        );

        let focus_continues = rs.routing_focus_slot_ids[0] >= 0
            && rs.routing_focus_slot_ids[0] == state.focus_slot_id
            && rs.routing_focus_edge_ids[0] == state.focus_edge_id;
        let reentry = if drive <= 0.05
            && focus_continues
            && state.gap_pressure > 0.0
            && memory.pattern_recurrence_score > 0.5
            && memory.persistence_score > 0.2
        {
            unit(
                memory.pattern_recurrence_score
                    * memory.persistence_score
                    * memory.dominant_focus_persistence.max(0.25),
            )
        } else {
            0.0
        };

        let carried = state.gap_pressure * 0.82;
        let mut gap_pressure = carried.max(drive).max(reentry);
        if closure_signal > 0.0 {
            gap_pressure *= 1.0 - 0.70 * closure_signal;
        }
        gap_pressure = unit(gap_pressure);

        if drive > 0.05 || reentry > 0.05 || gap_pressure > 0.05 {
            state.active_cycles = state.active_cycles.saturating_add(1);
        } else {
            state.active_cycles = 0;
        }

        state.gap_pressure = gap_pressure;
        state.drive_score = drive;
        state.persistence_score = unit(state.active_cycles as f64 / 6.0);
        state.closure_score = unit((state.closure_score * 0.72).max(closure_signal));
        state.reentry_score = unit(reentry);
        state.focus_slot_id = rs.routing_focus_slot_ids[0];
        state.focus_edge_id = rs.routing_focus_edge_ids[0];
        if newest_pair.0 > state.last_pair_seq {
            state.last_pair_seq = newest_pair.0;
        }
        state.last_update_t_us = now_us;
        let trace = ActiveReasoningGapTrace {
            t_us: now_us,
            gap_pressure: state.gap_pressure,
            closure_score: state.closure_score,
            reentry_score: state.reentry_score,
            active_cycles: state.active_cycles,
        };
        state.push_trace(trace);
    }

    pub fn read_active_reasoning_gap(&self) -> ActiveReasoningGapState {
        *self.active_reasoning_gap.read().unwrap()
    }
}

impl ActiveReasoningGapState {
    fn push_trace(&mut self, trace: ActiveReasoningGapTrace) {
        self.history[self.history_head] = trace;
        self.history_head = (self.history_head + 1) % ACTIVE_REASONING_GAP_TRACE_CAP;
        if (self.history_len as usize) < ACTIVE_REASONING_GAP_TRACE_CAP {
            self.history_len += 1;
        }
        self.total_samples = self.total_samples.saturating_add(1);
    }
}

