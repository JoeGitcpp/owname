use crate::evolution_priors::BiasPriors;
use crate::substrate_pressure::{compute_pressure, PressureSample};
use crate::{
    EVIDENCE_TENSION_CLOSURE_ALPHA, EVIDENCE_TENSION_DECAY_ALPHA, EVIDENCE_TENSION_HISTORY_CAP,
    EVIDENCE_TENSION_PRESSURE_SCALE,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvidenceTensionTrace {
    pub t_us: u64,
    pub tension: f64,
    pub unresolved_pressure: f64,
    pub closure_support: f64,
    pub pair_cursor: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceTensionState {
    pub current_tension: f64,
    pub unresolved_pressure: f64,
    pub last_update_t_us: u64,
    pub last_pair_cursor: u64,
    pub history: [EvidenceTensionTrace; EVIDENCE_TENSION_HISTORY_CAP],
    pub history_head: usize,
    pub history_len: usize,
}

impl Default for EvidenceTensionState {
    fn default() -> Self {
        Self {
            current_tension: 0.0,
            unresolved_pressure: 0.0,
            last_update_t_us: 0,
            last_pair_cursor: 0,
            history: [EvidenceTensionTrace::default(); EVIDENCE_TENSION_HISTORY_CAP],
            history_head: 0,
            history_len: 0,
        }
    }
}

impl EvidenceTensionState {
    pub fn absorb_features(&mut self, sample: PressureSample, priors: &BiasPriors, t_us: u64) {
        let tension = compute_pressure(sample, &priors.pressure_weights);
        let normalized = (tension / EVIDENCE_TENSION_PRESSURE_SCALE).clamp(0.0, 1.0);
        if normalized >= self.unresolved_pressure {
            self.unresolved_pressure = normalized;
        } else {
            self.unresolved_pressure = self.unresolved_pressure
                * (1.0 - EVIDENCE_TENSION_DECAY_ALPHA)
                + normalized * EVIDENCE_TENSION_DECAY_ALPHA;
        }
        self.current_tension = tension;
        self.last_update_t_us = t_us;
        self.push_trace(t_us, tension, 0.0, self.last_pair_cursor);
    }

    pub fn absorb_return_support(&mut self, support: f64, pair_cursor: u64, t_us: u64) {
        if pair_cursor <= self.last_pair_cursor {
            return;
        }
        let closure = support.clamp(0.0, 1.0);
        self.unresolved_pressure *= 1.0 - closure * EVIDENCE_TENSION_CLOSURE_ALPHA;
        self.current_tension = self.unresolved_pressure * EVIDENCE_TENSION_PRESSURE_SCALE;
        self.last_pair_cursor = pair_cursor;
        self.last_update_t_us = t_us;
        self.push_trace(t_us, self.current_tension, closure, pair_cursor);
    }

    fn push_trace(&mut self, t_us: u64, tension: f64, closure_support: f64, pair_cursor: u64) {
        self.history[self.history_head] = EvidenceTensionTrace {
            t_us,
            tension,
            unresolved_pressure: self.unresolved_pressure,
            closure_support: closure_support.clamp(0.0, 1.0),
            pair_cursor,
        };
        self.history_head = (self.history_head + 1) % EVIDENCE_TENSION_HISTORY_CAP;
        if self.history_len < EVIDENCE_TENSION_HISTORY_CAP {
            self.history_len += 1;
        }
    }
}
