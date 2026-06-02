use crate::substrate::unit;
use crate::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_EVENT, PREDICTION_WAVE_RELATION, PREDICTION_WAVE_RESIDUE,
};
use crate::viewport::Viewport;

const TRACE_CAP: usize = 16;
const RELATION_RESONANCE_CAP: usize = 16;
const LOCAL_PROBE_CHAIN_CAP: usize = 16;
const CHAIN_MEMORY_CAP: usize = 16;
const TRACE_DECAY: f64 = 0.86;
const RESIDUE_DECAY: f64 = 0.92;
const ATTEMPT_DECAY: f64 = 0.74;
const CHAIN_MEMORY_DECAY: f64 = 0.965;
const CHAIN_MEMORY_FLOOR: f64 = 0.015;
const APERTURE_INERTIA: f64 = 0.12;
const APERTURE_SWITCH_MARGIN: f64 = 0.10;
const ECHO_LIMIT_RATIO: f64 = 0.25;
const ECHO_PROBE_GAIN: f64 = 0.35;
const LOW_MATCH_THRESHOLD: f64 = 0.45;
const HIGH_MATCH_THRESHOLD: f64 = 0.62;
const CHAIN_CLOSURE_THRESHOLD: f64 = 0.72;
const CHAIN_MISMATCH_THRESHOLD: f64 = 0.45;
const SOFT_ARBITRATION_SWITCH_MARGIN: f64 = 0.12;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveFocusTrace {
    pub source_kind: u8,
    pub slot_id: i8,
    pub edge_id: i16,
    pub pressure: f64,
    pub persistence: f64,
    pub resonance: f64,
    pub closure_affordance: f64,
    pub write_risk: f64,
    pub aperture_share: f64,
    pub echo_share: f64,
    pub residue_strength: f64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RelationResonanceTrace {
    pub edge_id: i16,
    pub slot_id: i8,
    pub attempt_pressure: f64,
    pub match_score: f64,
    pub residue_support: f64,
    pub spike_pressure: f64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LocalProbeChainTrace {
    pub pair_seq: u64,
    pub write_seq: u64,
    pub source_kind: u8,
    pub slot_id: i8,
    pub edge_id: i16,
    pub return_support: f64,
    pub closure_pressure: f64,
    pub mismatch_pressure: f64,
    pub next_probe_pressure: f64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChainMemoryTrace {
    pub source_kind: u8,
    pub slot_id: i8,
    pub edge_id: i16,
    pub sediment_strength: f64,
    pub closure_support: f64,
    pub mismatch_support: f64,
    pub continuity_score: f64,
    pub branch_pressure: f64,
    pub recurrence_count: u32,
    pub reentry_pressure: f64,
    pub span_us: u64,
    pub depth_score: f64,
    pub long_chain_score: f64,
    pub interruption_count: u32,
    pub recovery_count: u32,
    pub last_pair_seq: u64,
    pub first_update_t_us: u64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveSurfaceState {
    pub traces: Vec<ActiveFocusTrace>,
    pub relation_resonance_traces: Vec<RelationResonanceTrace>,
    pub local_probe_chain_traces: Vec<LocalProbeChainTrace>,
    pub chain_memory_traces: Vec<ChainMemoryTrace>,
    pub traces_alive: u32,
    pub relation_resonance_traces_alive: u32,
    pub chain_memory_traces_alive: u32,
    pub aperture_trace_index: i8,
    pub aperture_pressure: f64,
    pub echo_trace_index: i8,
    pub echo_pressure: f64,
    pub aperture_switches: u32,
    pub echo_present_count: u32,
    pub reentry_count: u32,
    pub aperture_displacements: u32,
    pub retained_displaced_residue: u32,
    pub closure_reductions: u32,
    pub low_match_decay_count: u32,
    pub sudden_match_count: u32,
    pub sudden_match_enters_surface_count: u32,
    pub wrong_match_leakage: f64,
    pub local_probe_chain_steps: u32,
    pub local_probe_chain_closures: u32,
    pub local_probe_chain_mismatches: u32,
    pub local_probe_chain_next_probe_count: u32,
    pub local_probe_chain_last_pair_seq: u64,
    pub local_probe_chain_last_update_t_us: u64,
    pub chain_memory_sediment_updates: u32,
    pub chain_memory_reentry_count: u32,
    pub chain_memory_continuity_steps: u32,
    pub chain_memory_parallel_branch_count: u32,
    pub chain_memory_dominant_chain_index: i8,
    pub chain_memory_dominant_continuity_score: f64,
    pub chain_memory_branch_pressure: f64,
    pub chain_memory_soft_arbitration_ticks: u32,
    pub chain_memory_soft_hold_count: u32,
    pub chain_memory_soft_switch_count: u32,
    pub chain_memory_soft_reentry_count: u32,
    pub chain_memory_competition_pressure: f64,
    pub chain_memory_challenger_pressure: f64,
    pub chain_memory_arbitration_margin: f64,
    pub chain_memory_dominant_slot_id: i8,
    pub chain_memory_dominant_edge_id: i16,
    pub chain_memory_previous_dominant_slot_id: i8,
    pub chain_memory_previous_dominant_edge_id: i16,
    pub chain_memory_long_chain_steps: u32,
    pub chain_memory_long_chain_span_us: u64,
    pub chain_memory_long_chain_score: f64,
    pub chain_memory_interruption_count: u32,
    pub chain_memory_recovery_count: u32,
    pub chain_memory_interruption_branch_pressure: f64,
    pub chain_memory_delayed_reentry_pressure: f64,
    pub chain_memory_last_pair_seq: u64,
    pub chain_memory_last_reentry_pressure: f64,
    pub chain_memory_last_update_t_us: u64,
    pub total_updates: u64,
    pub last_update_t_us: u64,
    pub last_relation_resonance_t_us: u64,
}

impl Default for ActiveSurfaceState {
    fn default() -> Self {
        Self {
            traces: Vec::new(),
            relation_resonance_traces: Vec::new(),
            local_probe_chain_traces: Vec::new(),
            chain_memory_traces: Vec::new(),
            traces_alive: 0,
            relation_resonance_traces_alive: 0,
            chain_memory_traces_alive: 0,
            aperture_trace_index: -1,
            aperture_pressure: 0.0,
            echo_trace_index: -1,
            echo_pressure: 0.0,
            aperture_switches: 0,
            echo_present_count: 0,
            reentry_count: 0,
            aperture_displacements: 0,
            retained_displaced_residue: 0,
            closure_reductions: 0,
            low_match_decay_count: 0,
            sudden_match_count: 0,
            sudden_match_enters_surface_count: 0,
            wrong_match_leakage: 0.0,
            local_probe_chain_steps: 0,
            local_probe_chain_closures: 0,
            local_probe_chain_mismatches: 0,
            local_probe_chain_next_probe_count: 0,
            local_probe_chain_last_pair_seq: 0,
            local_probe_chain_last_update_t_us: 0,
            chain_memory_sediment_updates: 0,
            chain_memory_reentry_count: 0,
            chain_memory_continuity_steps: 0,
            chain_memory_parallel_branch_count: 0,
            chain_memory_dominant_chain_index: -1,
            chain_memory_dominant_continuity_score: 0.0,
            chain_memory_branch_pressure: 0.0,
            chain_memory_soft_arbitration_ticks: 0,
            chain_memory_soft_hold_count: 0,
            chain_memory_soft_switch_count: 0,
            chain_memory_soft_reentry_count: 0,
            chain_memory_competition_pressure: 0.0,
            chain_memory_challenger_pressure: 0.0,
            chain_memory_arbitration_margin: 0.0,
            chain_memory_dominant_slot_id: -1,
            chain_memory_dominant_edge_id: -1,
            chain_memory_previous_dominant_slot_id: -1,
            chain_memory_previous_dominant_edge_id: -1,
            chain_memory_long_chain_steps: 0,
            chain_memory_long_chain_span_us: 0,
            chain_memory_long_chain_score: 0.0,
            chain_memory_interruption_count: 0,
            chain_memory_recovery_count: 0,
            chain_memory_interruption_branch_pressure: 0.0,
            chain_memory_delayed_reentry_pressure: 0.0,
            chain_memory_last_pair_seq: 0,
            chain_memory_last_reentry_pressure: 0.0,
            chain_memory_last_update_t_us: 0,
            total_updates: 0,
            last_update_t_us: 0,
            last_relation_resonance_t_us: 0,
        }
    }
}

impl ActiveSurfaceState {
    pub fn active_aperture_count(&self) -> u8 {
        u8::from(self.aperture_trace_index >= 0 && self.aperture_pressure > 0.0)
    }

    pub fn echo_focus_present(&self) -> u8 {
        u8::from(self.echo_trace_index >= 0 && self.echo_pressure > 0.0)
    }

    pub fn echo_pressure_below_aperture(&self) -> u8 {
        u8::from(
            self.echo_pressure <= self.aperture_pressure * ECHO_LIMIT_RATIO + 1e-9
                && self.echo_pressure <= self.aperture_pressure,
        )
    }

    pub fn non_aperture_focus_retained(&self) -> u8 {
        u8::from(
            self.traces.iter().enumerate().any(|(idx, trace)| {
                idx as i8 != self.aperture_trace_index && trace.pressure > 0.01
            }),
        )
    }

    pub fn aperture_switches_bounded(&self, max_switches: u32) -> u8 {
        u8::from(self.aperture_switches <= max_switches)
    }

    pub fn effective_probe_pressure(&self) -> f64 {
        unit(self.aperture_pressure + self.echo_pressure * ECHO_PROBE_GAIN)
    }

    pub fn high_match_spike_observed(&self) -> u8 {
        u8::from(self.sudden_match_count > 0)
    }

    pub fn sudden_match_enters_surface(&self) -> u8 {
        u8::from(self.sudden_match_enters_surface_count > 0)
    }

    fn refresh(
        &mut self,
        now_us: u64,
        waves: &[crate::prediction_pressure_field::PredictionWaveTrace],
    ) {
        let previous_key = self
            .traces
            .get(self.aperture_trace_index.max(0) as usize)
            .map(trace_key);
        let previous_source = self
            .traces
            .get(self.aperture_trace_index.max(0) as usize)
            .map(|trace| trace.source_kind)
            .unwrap_or_default();
        let previous_pressure = self.aperture_pressure;

        for trace in &mut self.traces {
            trace.pressure = unit(trace.pressure * TRACE_DECAY);
            trace.persistence = if trace.pressure > 0.02 {
                unit(trace.persistence + 0.08)
            } else {
                unit(trace.persistence * 0.5)
            };
            trace.residue_strength =
                unit(trace.residue_strength * RESIDUE_DECAY + trace.pressure * 0.04);
            trace.aperture_share = 0.0;
            trace.echo_share = 0.0;
        }

        for wave in waves {
            let projected = unit(
                wave.amplitude
                    * (0.60 + 0.25 * wave.persistence + 0.15 * wave.resonance)
                    * wave.closure_affordance.max(0.20)
                    * (1.0 - 0.40 * wave.write_risk)
                    * source_gain(wave.source_kind, wave.resonance),
            );
            self.absorb_wave(now_us, wave, projected);
        }

        self.traces
            .retain(|trace| trace.pressure > 0.01 || trace.residue_strength > 0.03);
        self.traces_alive = self.traces.len() as u32;
        self.select_aperture(previous_key, previous_source, previous_pressure);
        self.observe_relation_surface_entry();
        self.total_updates = self.total_updates.saturating_add(1);
        self.last_update_t_us = now_us;
    }

    fn absorb_wave(
        &mut self,
        now_us: u64,
        wave: &crate::prediction_pressure_field::PredictionWaveTrace,
        projected: f64,
    ) {
        if let Some(trace) = self.traces.iter_mut().find(|trace| {
            trace.source_kind == wave.source_kind
                && trace.slot_id == wave.slot_id
                && trace.edge_id == wave.edge_id
        }) {
            trace.pressure = trace.pressure.max(projected);
            trace.persistence = trace.persistence.max(wave.persistence);
            trace.resonance = trace.resonance.max(wave.resonance);
            trace.closure_affordance = trace.closure_affordance.max(wave.closure_affordance);
            trace.write_risk = unit((trace.write_risk + wave.write_risk) * 0.5);
            trace.residue_strength = trace.residue_strength.max(projected * 0.35);
            trace.last_update_t_us = now_us;
            return;
        }

        if self.traces.len() >= TRACE_CAP {
            self.traces.sort_by(|lhs, rhs| {
                (lhs.pressure + lhs.residue_strength * 0.5)
                    .total_cmp(&(rhs.pressure + rhs.residue_strength * 0.5))
            });
            self.traces.remove(0);
        }
        self.traces.push(ActiveFocusTrace {
            source_kind: wave.source_kind,
            slot_id: wave.slot_id,
            edge_id: wave.edge_id,
            pressure: projected,
            persistence: wave.persistence.max(0.10),
            resonance: wave.resonance,
            closure_affordance: wave.closure_affordance,
            write_risk: wave.write_risk,
            aperture_share: 0.0,
            echo_share: 0.0,
            residue_strength: projected * 0.35,
            last_update_t_us: now_us,
        });
    }

    fn select_aperture(
        &mut self,
        previous_key: Option<(u8, i8, i16)>,
        previous_source: u8,
        previous_pressure: f64,
    ) {
        if self.traces.is_empty() {
            self.aperture_trace_index = -1;
            self.aperture_pressure = 0.0;
            self.echo_trace_index = -1;
            self.echo_pressure = 0.0;
            return;
        }

        let previous_index = previous_key
            .and_then(|key| self.traces.iter().position(|trace| trace_key(trace) == key));
        let mut best_index = 0usize;
        let mut best_score = 0.0f64;
        for (idx, trace) in self.traces.iter().enumerate() {
            let mut score = trace.focus_score();
            if trace.source_kind == PREDICTION_WAVE_RESIDUE
                && trace.resonance >= 0.90
                && trace.pressure > 0.05
            {
                score += 0.18;
            }
            if Some(idx) == previous_index {
                score += APERTURE_INERTIA;
            }
            if score > best_score {
                best_score = score;
                best_index = idx;
            }
        }

        if let Some(prev_idx) = previous_index {
            let previous_score = self.traces[prev_idx].focus_score() + APERTURE_INERTIA;
            if best_index != prev_idx && best_score < previous_score + APERTURE_SWITCH_MARGIN {
                best_index = prev_idx;
                best_score = previous_score;
            }
        }

        let changed = previous_index != Some(best_index);
        if changed && previous_index.is_some() {
            self.aperture_switches = self.aperture_switches.saturating_add(1);
        }
        if self.traces[best_index].source_kind == PREDICTION_WAVE_RESIDUE
            && self.traces[best_index].residue_strength > 0.05
            && (changed || self.traces[best_index].pressure > previous_pressure)
        {
            self.reentry_count = self.reentry_count.saturating_add(1);
        }
        if changed
            && self.traces[best_index].source_kind != previous_source
            && self.traces[best_index].pressure > previous_pressure
        {
            self.aperture_displacements = self.aperture_displacements.saturating_add(1);
        }
        if let Some(prev_idx) = previous_index {
            if changed && prev_idx < self.traces.len() {
                self.traces[prev_idx].residue_strength = self.traces[prev_idx]
                    .residue_strength
                    .max(previous_pressure * 0.60);
                self.retained_displaced_residue = self.retained_displaced_residue.saturating_add(1);
            }
        }

        self.aperture_trace_index = best_index as i8;
        self.aperture_pressure = unit(best_score);
        self.assign_shares();
    }

    fn assign_shares(&mut self) {
        for trace in &mut self.traces {
            trace.aperture_share = 0.0;
            trace.echo_share = 0.0;
        }
        if self.aperture_trace_index < 0 {
            self.echo_trace_index = -1;
            self.echo_pressure = 0.0;
            return;
        }
        let aperture_idx = self.aperture_trace_index as usize;
        if let Some(trace) = self.traces.get_mut(aperture_idx) {
            trace.aperture_share = 1.0;
        }

        let echo = self
            .traces
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != aperture_idx)
            .map(|(idx, trace)| (idx, trace.focus_score()))
            .max_by(|(_, lhs), (_, rhs)| lhs.total_cmp(rhs));
        if let Some((echo_idx, echo_score)) = echo {
            let capped = unit(echo_score.min(self.aperture_pressure * ECHO_LIMIT_RATIO));
            self.echo_trace_index = echo_idx as i8;
            self.echo_pressure = capped;
            if let Some(trace) = self.traces.get_mut(echo_idx) {
                trace.echo_share = if self.aperture_pressure > 1e-9 {
                    unit(capped / self.aperture_pressure)
                } else {
                    0.0
                };
            }
            if capped > 0.0 {
                self.echo_present_count = self.echo_present_count.saturating_add(1);
            }
        } else {
            self.echo_trace_index = -1;
            self.echo_pressure = 0.0;
        }
    }

    fn apply_closure(&mut self, feedback: f64) {
        if self.aperture_trace_index < 0 {
            return;
        }
        let idx = self.aperture_trace_index as usize;
        if let Some(trace) = self.traces.get_mut(idx) {
            let before = trace.pressure;
            trace.pressure = unit(trace.pressure * (1.0 - 0.65 * unit(feedback)));
            trace.residue_strength = unit(trace.residue_strength * (1.0 - 0.25 * unit(feedback)));
            if trace.pressure < before {
                self.closure_reductions = self.closure_reductions.saturating_add(1);
            }
        }
        self.aperture_pressure = self
            .traces
            .get(idx)
            .map(|trace| trace.focus_score())
            .unwrap_or(0.0);
        self.assign_shares();
    }

    fn tick_probe_chain(&mut self, now_us: u64, pairs: &[ProbeChainPairInput]) {
        for pair in pairs {
            if pair.pair_seq <= self.local_probe_chain_last_pair_seq {
                continue;
            }
            self.local_probe_chain_last_pair_seq = pair.pair_seq;
            self.local_probe_chain_steps = self.local_probe_chain_steps.saturating_add(1);

            let support = unit(pair.curve_similarity);
            let closure_pressure = if support >= CHAIN_CLOSURE_THRESHOLD {
                unit(
                    (support - CHAIN_MISMATCH_THRESHOLD)
                        / (1.0 - CHAIN_MISMATCH_THRESHOLD).max(1e-9),
                )
            } else {
                0.0
            };
            let mismatch_pressure = if support < CHAIN_MISMATCH_THRESHOLD {
                unit((CHAIN_MISMATCH_THRESHOLD - support) / CHAIN_MISMATCH_THRESHOLD)
            } else {
                0.0
            };

            let mut source_kind = 0;
            let mut slot_id = -1;
            let mut edge_id = -1;
            let mut next_probe_pressure = 0.0;

            if self.aperture_trace_index >= 0 {
                let idx = self.aperture_trace_index as usize;
                if let Some(trace) = self.traces.get_mut(idx) {
                    source_kind = trace.source_kind;
                    slot_id = trace.slot_id;
                    edge_id = trace.edge_id;
                    if closure_pressure > 0.0 {
                        let before = trace.pressure;
                        trace.pressure = unit(trace.pressure * (1.0 - 0.58 * closure_pressure));
                        trace.residue_strength = unit(
                            trace.residue_strength.max(before * 0.35)
                                * (1.0 - 0.12 * closure_pressure),
                        );
                        if trace.pressure < before {
                            self.closure_reductions = self.closure_reductions.saturating_add(1);
                            self.local_probe_chain_closures =
                                self.local_probe_chain_closures.saturating_add(1);
                        }
                    } else if mismatch_pressure > 0.0 {
                        trace.pressure = unit(trace.pressure + 0.34 * mismatch_pressure);
                        trace.persistence = unit(trace.persistence + 0.12 * mismatch_pressure);
                        trace.residue_strength =
                            unit(trace.residue_strength + 0.28 * mismatch_pressure);
                        trace.write_risk = unit(trace.write_risk + 0.08 * mismatch_pressure);
                        next_probe_pressure = unit(trace.focus_score() + 0.30 * mismatch_pressure);
                        self.local_probe_chain_mismatches =
                            self.local_probe_chain_mismatches.saturating_add(1);
                        if next_probe_pressure > 0.0 {
                            self.local_probe_chain_next_probe_count =
                                self.local_probe_chain_next_probe_count.saturating_add(1);
                        }
                    }
                }
            }

            self.local_probe_chain_traces.push(LocalProbeChainTrace {
                pair_seq: pair.pair_seq,
                write_seq: pair.write_seq,
                source_kind,
                slot_id,
                edge_id,
                return_support: support,
                closure_pressure,
                mismatch_pressure,
                next_probe_pressure,
                last_update_t_us: now_us,
            });
            if self.local_probe_chain_traces.len() > LOCAL_PROBE_CHAIN_CAP {
                let evict_n = self.local_probe_chain_traces.len() - LOCAL_PROBE_CHAIN_CAP;
                self.local_probe_chain_traces.drain(0..evict_n);
            }
        }

        if self.aperture_trace_index >= 0 {
            let idx = self.aperture_trace_index as usize;
            self.aperture_pressure = self
                .traces
                .get(idx)
                .map(|trace| trace.focus_score())
                .unwrap_or(0.0);
            self.assign_shares();
        }
        if !pairs.is_empty() {
            self.local_probe_chain_last_update_t_us = now_us;
        }
    }

    fn tick_chain_memory(&mut self, now_us: u64) -> Vec<PredictionWaveInput> {
        for trace in &mut self.chain_memory_traces {
            trace.sediment_strength = unit(trace.sediment_strength * CHAIN_MEMORY_DECAY);
            trace.closure_support = unit(trace.closure_support * CHAIN_MEMORY_DECAY);
            trace.mismatch_support = unit(trace.mismatch_support * CHAIN_MEMORY_DECAY);
            trace.continuity_score = unit(trace.continuity_score * CHAIN_MEMORY_DECAY);
            trace.branch_pressure = unit(trace.branch_pressure * CHAIN_MEMORY_DECAY);
            trace.reentry_pressure = unit(trace.reentry_pressure * RESIDUE_DECAY);
        }

        let pending = self
            .local_probe_chain_traces
            .iter()
            .copied()
            .filter(|trace| trace.pair_seq > self.chain_memory_last_pair_seq)
            .collect::<Vec<_>>();
        for chain_trace in pending {
            self.chain_memory_last_pair_seq = chain_trace.pair_seq;
            self.chain_memory_sediment_updates =
                self.chain_memory_sediment_updates.saturating_add(1);
            let idx = self.ensure_chain_memory_trace_index(chain_trace);
            let memory = &mut self.chain_memory_traces[idx];
            let continued = memory.last_pair_seq > 0;
            let support_delta = unit(
                chain_trace.closure_pressure * 0.46
                    + chain_trace.mismatch_pressure * 0.52
                    + chain_trace.return_support * 0.08,
            );
            memory.sediment_strength = unit(memory.sediment_strength + support_delta);
            memory.closure_support =
                unit(memory.closure_support + chain_trace.closure_pressure * 0.62);
            memory.mismatch_support =
                unit(memory.mismatch_support + chain_trace.mismatch_pressure * 0.68);
            memory.recurrence_count = memory.recurrence_count.saturating_add(1);
            if continued {
                self.chain_memory_continuity_steps =
                    self.chain_memory_continuity_steps.saturating_add(1);
            }
            memory.continuity_score = unit(
                memory.continuity_score
                    + 0.30 * support_delta
                    + 0.10 * memory.recurrence_count.min(4) as f64
                    + if continued { 0.18 } else { 0.04 },
            );
            if memory.first_update_t_us == 0 {
                memory.first_update_t_us = now_us;
            }
            memory.span_us = now_us.saturating_sub(memory.first_update_t_us);
            memory.depth_score = unit(memory.recurrence_count as f64 / 12.0);
            memory.long_chain_score = chain_memory_long_score(memory);
            memory.last_pair_seq = chain_trace.pair_seq;
            memory.last_update_t_us = now_us;
        }

        self.chain_memory_traces.retain(|trace| {
            trace.sediment_strength > CHAIN_MEMORY_FLOOR
                || trace.closure_support > CHAIN_MEMORY_FLOOR
                || trace.mismatch_support > CHAIN_MEMORY_FLOOR
        });
        if self.chain_memory_traces.len() > CHAIN_MEMORY_CAP {
            self.chain_memory_traces.sort_by(|lhs, rhs| {
                lhs.sediment_strength
                    .total_cmp(&rhs.sediment_strength)
                    .then_with(|| lhs.last_update_t_us.cmp(&rhs.last_update_t_us))
            });
            while self.chain_memory_traces.len() > CHAIN_MEMORY_CAP {
                self.chain_memory_traces.remove(0);
            }
        }
        self.chain_memory_traces_alive = self.chain_memory_traces.len() as u32;
        self.refresh_chain_memory_branch_state();

        let best = self.chain_memory_traces.iter_mut().max_by(|lhs, rhs| {
            chain_memory_reentry_score(lhs).total_cmp(&chain_memory_reentry_score(rhs))
        });
        let Some(best) = best else {
            self.chain_memory_last_reentry_pressure = 0.0;
            self.chain_memory_last_update_t_us = now_us;
            return Vec::new();
        };

        let reentry_pressure = unit(chain_memory_reentry_score(best));
        best.reentry_pressure = reentry_pressure;
        self.chain_memory_last_reentry_pressure = reentry_pressure;
        self.chain_memory_last_update_t_us = now_us;
        if reentry_pressure <= 0.05 {
            return Vec::new();
        }

        self.chain_memory_reentry_count = self.chain_memory_reentry_count.saturating_add(1);
        vec![PredictionWaveInput {
            source_kind: PREDICTION_WAVE_RESIDUE,
            slot_id: best.slot_id,
            edge_id: best.edge_id,
            window_start_us: now_us,
            window_end_us: now_us + 10_000,
            amplitude: reentry_pressure,
            resonance: unit(
                best.sediment_strength + 0.25 * best.closure_support + 0.35 * best.mismatch_support,
            ),
            closure_affordance: best.closure_support.max(0.28),
            write_risk: unit(0.10 + 0.22 * best.mismatch_support),
        }]
    }

    fn tick_relation_resonance(
        &mut self,
        now_us: u64,
        relation_inputs: &[RelationResonanceInput],
    ) -> Vec<PredictionWaveInput> {
        for trace in &mut self.relation_resonance_traces {
            trace.attempt_pressure = unit(trace.attempt_pressure * ATTEMPT_DECAY);
            trace.spike_pressure = unit(trace.spike_pressure * ATTEMPT_DECAY);
            trace.match_score = unit(trace.match_score * 0.75);
            trace.residue_support = unit(trace.residue_support * RESIDUE_DECAY);
        }

        let mut wave_inputs = Vec::new();
        let mut leakage = 0.0_f64;
        for input in relation_inputs {
            let previous_pressure = self
                .relation_resonance_traces
                .iter()
                .find(|trace| trace.edge_id == input.edge_id)
                .map(|trace| trace.attempt_pressure)
                .unwrap_or(0.0);
            let trace_idx = self.ensure_relation_trace_index(*input);
            if input.match_score < LOW_MATCH_THRESHOLD {
                let trace = &mut self.relation_resonance_traces[trace_idx];
                trace.attempt_pressure = unit(trace.attempt_pressure * ATTEMPT_DECAY);
                trace.spike_pressure = unit(trace.spike_pressure * ATTEMPT_DECAY);
                self.low_match_decay_count = self.low_match_decay_count.saturating_add(1);
            } else {
                let trace = &mut self.relation_resonance_traces[trace_idx];
                trace.attempt_pressure = unit(trace.attempt_pressure.max(input.match_score * 0.72));
            }
            let trace = &mut self.relation_resonance_traces[trace_idx];
            trace.match_score = input.match_score;
            trace.residue_support = input.residue_support;
            trace.last_update_t_us = now_us;

            if input.match_score >= HIGH_MATCH_THRESHOLD {
                let spike =
                    unit(input.match_score * (0.70 + 0.30 * input.residue_support.max(0.20)));
                trace.spike_pressure = trace.spike_pressure.max(spike);
                if previous_pressure < HIGH_MATCH_THRESHOLD {
                    self.sudden_match_count = self.sudden_match_count.saturating_add(1);
                }
                let spike_pressure = trace.spike_pressure;
                wave_inputs.push(PredictionWaveInput {
                    source_kind: PREDICTION_WAVE_RELATION,
                    slot_id: input.slot_id,
                    edge_id: input.edge_id,
                    window_start_us: now_us,
                    window_end_us: now_us + 10_000,
                    amplitude: spike_pressure,
                    resonance: input.match_score,
                    closure_affordance: unit(
                        input.relation_confidence.max(input.routing_stability),
                    ),
                    write_risk: unit(0.35 * (1.0 - input.relation_confidence)),
                });
            } else {
                leakage = leakage.max(trace.attempt_pressure);
            }
        }

        self.relation_resonance_traces
            .retain(|trace| trace.attempt_pressure > 0.005 || trace.spike_pressure > 0.01);
        if self.relation_resonance_traces.len() > RELATION_RESONANCE_CAP {
            self.relation_resonance_traces.sort_by(|lhs, rhs| {
                (lhs.attempt_pressure + lhs.spike_pressure)
                    .total_cmp(&(rhs.attempt_pressure + rhs.spike_pressure))
            });
            while self.relation_resonance_traces.len() > RELATION_RESONANCE_CAP {
                self.relation_resonance_traces.remove(0);
            }
        }
        self.relation_resonance_traces_alive = self.relation_resonance_traces.len() as u32;
        self.wrong_match_leakage = if leakage <= 0.10 { 0.0 } else { unit(leakage) };
        self.last_relation_resonance_t_us = now_us;
        wave_inputs
    }

    fn observe_relation_surface_entry(&mut self) {
        let relation_entered = [self.aperture_trace_index, self.echo_trace_index]
            .into_iter()
            .filter(|idx| *idx >= 0)
            .any(|idx| {
                self.traces
                    .get(idx as usize)
                    .map(|trace| trace.source_kind == PREDICTION_WAVE_RELATION)
                    .unwrap_or(false)
            });
        let spike_alive = self
            .relation_resonance_traces
            .iter()
            .any(|trace| trace.spike_pressure > 0.05);
        if relation_entered && spike_alive {
            self.sudden_match_enters_surface_count =
                self.sudden_match_enters_surface_count.saturating_add(1);
        }
    }

    fn ensure_relation_trace_index(&mut self, input: RelationResonanceInput) -> usize {
        if let Some(idx) = self
            .relation_resonance_traces
            .iter()
            .position(|trace| trace.edge_id == input.edge_id)
        {
            return idx;
        }
        if self.relation_resonance_traces.len() >= RELATION_RESONANCE_CAP {
            self.relation_resonance_traces.sort_by(|lhs, rhs| {
                (lhs.attempt_pressure + lhs.spike_pressure)
                    .total_cmp(&(rhs.attempt_pressure + rhs.spike_pressure))
            });
            self.relation_resonance_traces.remove(0);
        }
        self.relation_resonance_traces.push(RelationResonanceTrace {
            edge_id: input.edge_id,
            slot_id: input.slot_id,
            attempt_pressure: input.match_score * 0.35,
            match_score: input.match_score,
            residue_support: input.residue_support,
            spike_pressure: 0.0,
            last_update_t_us: input.now_us,
        });
        self.relation_resonance_traces.len() - 1
    }

    fn ensure_chain_memory_trace_index(&mut self, input: LocalProbeChainTrace) -> usize {
        if let Some(idx) = self
            .chain_memory_traces
            .iter()
            .position(|trace| trace.slot_id == input.slot_id && trace.edge_id == input.edge_id)
        {
            return idx;
        }
        if self.chain_memory_traces.len() >= CHAIN_MEMORY_CAP {
            self.chain_memory_traces.sort_by(|lhs, rhs| {
                lhs.sediment_strength
                    .total_cmp(&rhs.sediment_strength)
                    .then_with(|| lhs.last_update_t_us.cmp(&rhs.last_update_t_us))
            });
            self.chain_memory_traces.remove(0);
        }
        self.chain_memory_traces.push(ChainMemoryTrace {
            source_kind: input.source_kind,
            slot_id: input.slot_id,
            edge_id: input.edge_id,
            sediment_strength: 0.0,
            closure_support: 0.0,
            mismatch_support: 0.0,
            continuity_score: 0.0,
            branch_pressure: 0.0,
            recurrence_count: 0,
            reentry_pressure: 0.0,
            span_us: 0,
            depth_score: 0.0,
            long_chain_score: 0.0,
            interruption_count: 0,
            recovery_count: 0,
            last_pair_seq: 0,
            first_update_t_us: input.last_update_t_us,
            last_update_t_us: input.last_update_t_us,
        });
        self.chain_memory_traces.len() - 1
    }

    fn refresh_chain_memory_branch_state(&mut self) {
        if self.chain_memory_traces.is_empty() {
            self.chain_memory_parallel_branch_count = 0;
            self.chain_memory_dominant_chain_index = -1;
            self.chain_memory_dominant_continuity_score = 0.0;
            self.chain_memory_branch_pressure = 0.0;
            self.chain_memory_competition_pressure = 0.0;
            self.chain_memory_challenger_pressure = 0.0;
            self.chain_memory_arbitration_margin = 0.0;
            self.chain_memory_dominant_slot_id = -1;
            self.chain_memory_dominant_edge_id = -1;
            self.chain_memory_long_chain_steps = 0;
            self.chain_memory_long_chain_span_us = 0;
            self.chain_memory_long_chain_score = 0.0;
            self.chain_memory_interruption_count = 0;
            self.chain_memory_recovery_count = 0;
            self.chain_memory_interruption_branch_pressure = 0.0;
            self.chain_memory_delayed_reentry_pressure = 0.0;
            return;
        }
        self.chain_memory_soft_arbitration_ticks =
            self.chain_memory_soft_arbitration_ticks.saturating_add(1);

        let arbitration_scores = self
            .chain_memory_traces
            .iter()
            .map(|trace| {
                chain_memory_dominance_score(trace)
                    + 0.34 * self.active_support_for_chain_memory_trace(trace)
            })
            .collect::<Vec<_>>();
        let absolute_strongest = self
            .chain_memory_traces
            .iter()
            .enumerate()
            .max_by(|(lhs_idx, _), (rhs_idx, _)| {
                arbitration_scores[*lhs_idx].total_cmp(&arbitration_scores[*rhs_idx])
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        let previous_idx = if self.chain_memory_dominant_slot_id >= 0 {
            self.chain_memory_traces.iter().position(|trace| {
                trace.slot_id == self.chain_memory_dominant_slot_id
                    && trace.edge_id == self.chain_memory_dominant_edge_id
            })
        } else {
            None
        };
        let previous_score = previous_idx
            .and_then(|idx| arbitration_scores.get(idx).copied())
            .unwrap_or(0.0);
        let strongest_challenger = previous_idx.and_then(|prev_idx| {
            self.chain_memory_traces
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != prev_idx)
                .max_by(|(lhs_idx, _), (rhs_idx, _)| {
                    arbitration_scores[*lhs_idx].total_cmp(&arbitration_scores[*rhs_idx])
                })
                .map(|(idx, _)| idx)
        });
        let sustained_challenger = strongest_challenger.filter(|idx| {
            self.chain_memory_traces[*idx].recurrence_count >= 3
                && arbitration_scores[*idx] > previous_score * 0.45
        });
        let reentry_challenger = strongest_challenger.filter(|idx| {
            let trace = &self.chain_memory_traces[*idx];
            trace.slot_id == self.chain_memory_previous_dominant_slot_id
                && trace.edge_id == self.chain_memory_previous_dominant_edge_id
                && trace.recurrence_count >= 3
                && self.active_support_for_chain_memory_trace(trace) > 0.10
                && arbitration_scores[*idx] > previous_score * 0.30
        });
        let promoted_challenger = reentry_challenger.or(sustained_challenger);
        let strongest = promoted_challenger.unwrap_or(absolute_strongest);
        let strongest_score = arbitration_scores.get(strongest).copied().unwrap_or(0.0);
        let strongest_trace = self.chain_memory_traces[strongest];
        let previous_key = (
            self.chain_memory_dominant_slot_id,
            self.chain_memory_dominant_edge_id,
        );
        let returning_to_previous = self.chain_memory_previous_dominant_slot_id >= 0
            && strongest_trace.slot_id == self.chain_memory_previous_dominant_slot_id
            && strongest_trace.edge_id == self.chain_memory_previous_dominant_edge_id
            && previous_key != (strongest_trace.slot_id, strongest_trace.edge_id);

        let dominant = match previous_idx {
            None => strongest,
            Some(prev_idx) if prev_idx == strongest => {
                self.chain_memory_soft_hold_count =
                    self.chain_memory_soft_hold_count.saturating_add(1);
                prev_idx
            }
            Some(prev_idx) => {
                let margin = strongest_score - previous_score;
                let previous_active_support = self
                    .chain_memory_traces
                    .get(prev_idx)
                    .map(|trace| self.active_support_for_chain_memory_trace(trace))
                    .unwrap_or(0.0);
                let strongest_active_support = self
                    .chain_memory_traces
                    .get(strongest)
                    .map(|trace| self.active_support_for_chain_memory_trace(trace))
                    .unwrap_or(0.0);
                let promoted_switch_allowed = promoted_challenger.is_some()
                    && strongest_active_support >= previous_active_support;
                let margin_switch_allowed = margin > SOFT_ARBITRATION_SWITCH_MARGIN
                    && (previous_active_support <= 0.10
                        || strongest_active_support >= previous_active_support);
                if promoted_switch_allowed || margin_switch_allowed {
                    self.chain_memory_soft_switch_count =
                        self.chain_memory_soft_switch_count.saturating_add(1);
                    if returning_to_previous {
                        self.chain_memory_soft_reentry_count =
                            self.chain_memory_soft_reentry_count.saturating_add(1);
                        if let Some(trace) = self.chain_memory_traces.get_mut(strongest) {
                            if trace.interruption_count > trace.recovery_count {
                                trace.recovery_count = trace.recovery_count.saturating_add(1);
                                self.chain_memory_delayed_reentry_pressure = self
                                    .chain_memory_delayed_reentry_pressure
                                    .max(trace.long_chain_score.max(trace.reentry_pressure));
                            }
                        }
                    }
                    self.chain_memory_previous_dominant_slot_id =
                        self.chain_memory_dominant_slot_id;
                    self.chain_memory_previous_dominant_edge_id =
                        self.chain_memory_dominant_edge_id;
                    strongest
                } else {
                    self.chain_memory_soft_hold_count =
                        self.chain_memory_soft_hold_count.saturating_add(1);
                    prev_idx
                }
            }
        };

        let dominant_score = arbitration_scores.get(dominant).copied().unwrap_or(0.0);
        let challenger_score = self
            .chain_memory_traces
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != dominant)
            .map(|(idx, _)| arbitration_scores[idx])
            .fold(0.0_f64, f64::max);
        self.chain_memory_competition_pressure = unit(dominant_score.max(challenger_score));
        self.chain_memory_challenger_pressure = unit(challenger_score);
        self.chain_memory_arbitration_margin = unit((dominant_score - challenger_score).abs());

        let mut branch_count = 0u32;
        let mut branch_pressure = 0.0f64;
        let mut interruption_count = 0u32;
        let mut recovery_count = 0u32;
        let mut interruption_branch_pressure = 0.0f64;
        let mut long_chain_steps = 0u32;
        let mut long_chain_span_us = 0u64;
        let mut long_chain_score = 0.0f64;
        for (idx, trace) in self.chain_memory_traces.iter_mut().enumerate() {
            long_chain_steps = long_chain_steps.max(trace.recurrence_count);
            long_chain_span_us = long_chain_span_us.max(trace.span_us);
            long_chain_score = long_chain_score.max(trace.long_chain_score);
            if idx == dominant {
                trace.branch_pressure = 0.0;
            } else {
                let pressure = unit(trace.sediment_strength * (0.55 + trace.mismatch_support));
                trace.branch_pressure = pressure;
                if pressure > 0.03 {
                    branch_count = branch_count.saturating_add(1);
                    branch_pressure = branch_pressure.max(pressure);
                    if trace.long_chain_score > 0.20 {
                        trace.interruption_count = trace.interruption_count.saturating_add(1);
                    }
                }
            }
            interruption_count = interruption_count.max(trace.interruption_count);
            recovery_count = recovery_count.max(trace.recovery_count);
            interruption_branch_pressure =
                interruption_branch_pressure.max(trace.branch_pressure * trace.long_chain_score);
        }
        self.chain_memory_parallel_branch_count = branch_count;
        self.chain_memory_dominant_chain_index = dominant as i8;
        self.chain_memory_dominant_continuity_score =
            self.chain_memory_traces[dominant].continuity_score;
        self.chain_memory_branch_pressure = branch_pressure;
        self.chain_memory_dominant_slot_id = self.chain_memory_traces[dominant].slot_id;
        self.chain_memory_dominant_edge_id = self.chain_memory_traces[dominant].edge_id;
        self.chain_memory_long_chain_steps = long_chain_steps;
        self.chain_memory_long_chain_span_us = long_chain_span_us;
        self.chain_memory_long_chain_score = long_chain_score;
        self.chain_memory_interruption_count = interruption_count;
        self.chain_memory_recovery_count = recovery_count;
        self.chain_memory_interruption_branch_pressure = interruption_branch_pressure;
    }

    fn active_support_for_chain_memory_trace(&self, memory: &ChainMemoryTrace) -> f64 {
        self.traces
            .iter()
            .filter(|trace| trace.slot_id == memory.slot_id && trace.edge_id == memory.edge_id)
            .map(|trace| {
                let source_alignment = if trace.source_kind == memory.source_kind {
                    1.0
                } else {
                    0.82
                };
                unit(trace.focus_score() * source_alignment)
            })
            .fold(0.0_f64, f64::max)
    }
}

impl ActiveFocusTrace {
    fn focus_score(&self) -> f64 {
        unit(
            self.pressure
                * (0.64 + 0.18 * self.persistence + 0.18 * self.resonance)
                * self.closure_affordance.max(0.20)
                * (1.0 - 0.35 * self.write_risk),
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RelationResonanceInput {
    edge_id: i16,
    slot_id: i8,
    match_score: f64,
    residue_support: f64,
    relation_confidence: f64,
    routing_stability: f64,
    now_us: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct ProbeChainPairInput {
    pair_seq: u64,
    write_seq: u64,
    curve_similarity: f64,
}

impl Viewport {
    pub fn refresh_active_surface(&self, now_us: u64) {
        let field = self.read_prediction_pressure_field();
        self.active_surface
            .write()
            .unwrap()
            .refresh(now_us, &field.waves);
    }

    pub fn apply_active_surface_closure(&self, feedback: f64) {
        self.active_surface.write().unwrap().apply_closure(feedback);
    }

    pub fn tick_active_surface_probe_chain(&self, now_us: u64) {
        let last_pair_seq = self
            .active_surface
            .read()
            .unwrap()
            .local_probe_chain_last_pair_seq;
        let pairs = self
            .read_action_consequence()
            .pairs
            .iter()
            .filter(|pair| pair.pair_seq > last_pair_seq)
            .map(|pair| ProbeChainPairInput {
                pair_seq: pair.pair_seq,
                write_seq: pair.write_seq,
                curve_similarity: pair.curve_similarity,
            })
            .collect::<Vec<_>>();
        if pairs.is_empty() {
            return;
        }
        self.active_surface
            .write()
            .unwrap()
            .tick_probe_chain(now_us, &pairs);
    }

    pub fn tick_active_surface_chain_memory(&self, now_us: u64) {
        let wave_inputs = self
            .active_surface
            .write()
            .unwrap()
            .tick_chain_memory(now_us);
        if !wave_inputs.is_empty() {
            self.refresh_prediction_pressure_field(now_us, &wave_inputs);
        }
    }

    pub fn tick_active_surface_relation_resonance(&self, now_us: u64) {
        let memory = self.read_global_reasoning_memory_summary();
        let traces = self.read_active_surface().traces;
        let relation_inputs = {
            let relations = self.global_relations.read().unwrap();
            relations
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.active)
                .map(|(idx, edge)| {
                    let edge_id = idx as i16;
                    let slot_id = edge.src_slot_id as i8;
                    let residue_support = traces
                        .iter()
                        .filter(|trace| {
                            trace.edge_id == edge_id
                                || trace.slot_id == edge.src_slot_id as i8
                                || trace.slot_id == edge.dst_slot_id as i8
                        })
                        .map(|trace| trace.residue_strength.max(trace.pressure * 0.25))
                        .fold(0.0_f64, f64::max);
                    let match_score = unit(
                        0.30 * edge.pred.relation_surprise
                            + 0.24 * edge.relation_confidence
                            + 0.18 * edge.predictive_score
                            + 0.14 * memory.pattern_recurrence_score
                            + 0.08 * memory.routing_stability_score
                            + 0.06 * residue_support,
                    );
                    RelationResonanceInput {
                        edge_id,
                        slot_id,
                        match_score,
                        residue_support,
                        relation_confidence: edge.relation_confidence,
                        routing_stability: memory.routing_stability_score,
                        now_us,
                    }
                })
                .collect::<Vec<_>>()
        };
        let wave_inputs = self
            .active_surface
            .write()
            .unwrap()
            .tick_relation_resonance(now_us, &relation_inputs);
        if !wave_inputs.is_empty() {
            self.refresh_prediction_pressure_field(now_us, &wave_inputs);
        }
    }

    pub fn read_active_surface(&self) -> ActiveSurfaceState {
        self.active_surface.read().unwrap().clone()
    }
}

fn trace_key(trace: &ActiveFocusTrace) -> (u8, i8, i16) {
    (trace.source_kind, trace.slot_id, trace.edge_id)
}

fn source_gain(source_kind: u8, resonance: f64) -> f64 {
    if source_kind == PREDICTION_WAVE_EVENT {
        1.30
    } else if source_kind == PREDICTION_WAVE_RESIDUE && resonance >= 0.90 {
        1.80
    } else {
        1.0
    }
}

fn chain_memory_reentry_score(trace: &ChainMemoryTrace) -> f64 {
    unit(
        trace.sediment_strength
            * (0.62
                + 0.22 * trace.closure_support
                + 0.34 * trace.mismatch_support
                + 0.18 * trace.continuity_score
                + 0.04 * trace.recurrence_count.min(4) as f64),
    )
}

fn chain_memory_dominance_score(trace: &ChainMemoryTrace) -> f64 {
    unit(
        trace.sediment_strength * 0.52
            + trace.continuity_score * 0.34
            + trace.reentry_pressure * 0.14
            + trace.long_chain_score * 0.08,
    )
}

fn chain_memory_long_score(trace: &ChainMemoryTrace) -> f64 {
    let span_score = unit(trace.span_us as f64 / 1_500_000.0);
    unit(
        trace.sediment_strength * 0.28
            + trace.continuity_score * 0.24
            + trace.depth_score * 0.28
            + span_score * 0.20,
    )
}

