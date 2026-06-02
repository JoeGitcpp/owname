use crate::substrate::unit;
use crate::viewport::Viewport;

pub const PREDICTION_WAVE_GAP: u8 = 1;
pub const PREDICTION_WAVE_EVENT: u8 = 2;
pub const PREDICTION_WAVE_RELATION: u8 = 3;
pub const PREDICTION_WAVE_RESIDUE: u8 = 4;

const WAVE_CAP: usize = 16;
const WAVE_DECAY: f64 = 0.86;
const FOCUS_INERTIA: f64 = 0.12;
const MIN_SWITCH_MARGIN: f64 = 0.10;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PredictionWaveInput {
    pub source_kind: u8,
    pub slot_id: i8,
    pub edge_id: i16,
    pub window_start_us: u64,
    pub window_end_us: u64,
    pub amplitude: f64,
    pub resonance: f64,
    pub closure_affordance: f64,
    pub write_risk: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PredictionWaveTrace {
    pub source_kind: u8,
    pub slot_id: i8,
    pub edge_id: i16,
    pub window_start_us: u64,
    pub window_end_us: u64,
    pub amplitude: f64,
    pub persistence: f64,
    pub resonance: f64,
    pub closure_affordance: f64,
    pub write_risk: f64,
    pub inhibition: f64,
    pub last_update_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictionPressureFieldState {
    pub waves: Vec<PredictionWaveTrace>,
    pub waves_alive: u32,
    pub dominant_wave_index: i8,
    pub dominant_pressure: f64,
    pub focus_switches: u32,
    pub total_updates: u64,
    pub quiet_reentry_count: u32,
    pub event_override_count: u32,
    pub resonance_spike_count: u32,
    pub closure_reductions: u32,
    pub retained_unclosed_residue: u32,
    pub last_update_t_us: u64,
}

impl Default for PredictionPressureFieldState {
    fn default() -> Self {
        Self {
            waves: Vec::new(),
            waves_alive: 0,
            dominant_wave_index: -1,
            dominant_pressure: 0.0,
            focus_switches: 0,
            total_updates: 0,
            quiet_reentry_count: 0,
            event_override_count: 0,
            resonance_spike_count: 0,
            closure_reductions: 0,
            retained_unclosed_residue: 0,
            last_update_t_us: 0,
        }
    }
}

impl PredictionPressureFieldState {
    pub fn dominant_focus_is_single(&self) -> u8 {
        u8::from(self.dominant_wave_index >= 0 && self.dominant_pressure > 0.0)
    }

    pub fn focus_switches_bounded(&self, max_switches: u32) -> u8 {
        u8::from(self.focus_switches <= max_switches)
    }

    fn refresh(&mut self, now_us: u64, inputs: &[PredictionWaveInput]) {
        let previous_index = self.dominant_wave_index;
        let previous_pressure = self.dominant_pressure;
        let previous_source = self
            .waves
            .get(previous_index.max(0) as usize)
            .map(|wave| wave.source_kind)
            .unwrap_or_default();

        for wave in &mut self.waves {
            wave.amplitude = unit(wave.amplitude * WAVE_DECAY);
            if wave.amplitude > 0.02 {
                wave.persistence = unit(wave.persistence + 0.12);
            } else {
                wave.persistence = unit(wave.persistence * 0.5);
            }
            wave.inhibition = 0.0;
        }

        for input in inputs {
            self.apply_input(now_us, *input);
        }

        self.waves.retain(|wave| wave.amplitude > 0.01);
        self.waves_alive = self.waves.len() as u32;
        self.apply_inhibition();
        self.select_dominant(previous_index);

        let new_source = self
            .waves
            .get(self.dominant_wave_index.max(0) as usize)
            .map(|wave| wave.source_kind)
            .unwrap_or_default();
        if self.dominant_wave_index != previous_index && previous_index >= 0 {
            self.focus_switches = self.focus_switches.saturating_add(1);
        }
        if new_source == PREDICTION_WAVE_RESIDUE
            && self.dominant_pressure > previous_pressure
            && previous_pressure > 0.0
        {
            self.quiet_reentry_count = self.quiet_reentry_count.saturating_add(1);
        }
        if new_source == PREDICTION_WAVE_EVENT
            && previous_source != PREDICTION_WAVE_EVENT
            && self.dominant_pressure > previous_pressure
        {
            self.event_override_count = self.event_override_count.saturating_add(1);
        }
        if inputs
            .iter()
            .any(|input| input.source_kind == PREDICTION_WAVE_RELATION && input.resonance >= 0.90)
        {
            self.resonance_spike_count = self.resonance_spike_count.saturating_add(1);
        }
        if self
            .waves
            .iter()
            .any(|wave| wave.amplitude > 0.04 && wave.source_kind == PREDICTION_WAVE_RESIDUE)
        {
            self.retained_unclosed_residue = self.retained_unclosed_residue.saturating_add(1);
        }

        self.total_updates = self.total_updates.saturating_add(1);
        self.last_update_t_us = now_us;
    }

    fn apply_input(&mut self, now_us: u64, input: PredictionWaveInput) {
        let effective = unit(
            input.amplitude
                * (0.55 + 0.25 * unit(input.resonance) + 0.20 * unit(input.closure_affordance))
                * (1.0 - 0.45 * unit(input.write_risk)),
        );
        if let Some(wave) = self.waves.iter_mut().find(|wave| wave.matches(input)) {
            wave.amplitude = wave.amplitude.max(effective);
            wave.resonance = wave.resonance.max(unit(input.resonance));
            wave.closure_affordance = wave.closure_affordance.max(unit(input.closure_affordance));
            wave.write_risk = unit((wave.write_risk + input.write_risk) * 0.5);
            wave.last_update_us = now_us;
            return;
        }

        if self.waves.len() >= WAVE_CAP {
            self.waves.sort_by(|lhs, rhs| {
                lhs.amplitude
                    .partial_cmp(&rhs.amplitude)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.waves.remove(0);
        }
        self.waves.push(PredictionWaveTrace {
            source_kind: input.source_kind,
            slot_id: input.slot_id,
            edge_id: input.edge_id,
            window_start_us: input.window_start_us,
            window_end_us: input.window_end_us,
            amplitude: effective,
            persistence: 0.12,
            resonance: unit(input.resonance),
            closure_affordance: unit(input.closure_affordance),
            write_risk: unit(input.write_risk),
            inhibition: 0.0,
            last_update_us: now_us,
        });
    }

    fn apply_inhibition(&mut self) {
        let max_amp = self
            .waves
            .iter()
            .map(|wave| wave.amplitude)
            .fold(0.0, f64::max);
        for wave in &mut self.waves {
            wave.inhibition = unit((max_amp - wave.amplitude) * 0.35);
        }
    }

    fn select_dominant(&mut self, previous_index: i8) {
        if self.waves.is_empty() {
            self.dominant_wave_index = -1;
            self.dominant_pressure = 0.0;
            return;
        }

        let mut best_index = 0usize;
        let mut best_score = 0.0f64;
        for (idx, wave) in self.waves.iter().enumerate() {
            let mut score = wave.focus_score();
            if previous_index == idx as i8 {
                score += FOCUS_INERTIA;
            }
            if score > best_score {
                best_score = score;
                best_index = idx;
            }
        }

        if previous_index >= 0 && (previous_index as usize) < self.waves.len() {
            let previous_wave = self.waves[previous_index as usize];
            let previous_score = previous_wave.focus_score() + FOCUS_INERTIA;
            if best_index != previous_index as usize
                && best_score < previous_score + MIN_SWITCH_MARGIN
            {
                best_index = previous_index as usize;
                best_score = previous_score;
            }
        }

        self.dominant_wave_index = best_index as i8;
        self.dominant_pressure = unit(best_score);
    }

    fn apply_closure(&mut self, feedback: f64) {
        let idx = self.dominant_wave_index;
        if idx < 0 {
            return;
        }
        if let Some(wave) = self.waves.get_mut(idx as usize) {
            let before = wave.amplitude;
            wave.amplitude = unit(wave.amplitude * (1.0 - 0.65 * unit(feedback)));
            if wave.amplitude < before {
                self.closure_reductions = self.closure_reductions.saturating_add(1);
            }
        }
        let previous = self.dominant_wave_index;
        self.apply_inhibition();
        self.select_dominant(previous);
    }
}

impl PredictionWaveTrace {
    fn matches(&self, input: PredictionWaveInput) -> bool {
        self.source_kind == input.source_kind
            && self.slot_id == input.slot_id
            && self.edge_id == input.edge_id
            && self.window_start_us == input.window_start_us
            && self.window_end_us == input.window_end_us
    }

    fn focus_score(&self) -> f64 {
        unit(
            self.amplitude
                * (0.60 + 0.25 * self.persistence + 0.15 * self.resonance)
                * self.closure_affordance.max(0.20)
                * (1.0 - 0.40 * self.write_risk)
                * (1.0 - self.inhibition),
        )
    }
}

impl Viewport {
    pub fn refresh_prediction_pressure_field(&self, now_us: u64, inputs: &[PredictionWaveInput]) {
        self.prediction_pressure_field
            .write()
            .unwrap()
            .refresh(now_us, inputs);
    }

    pub fn tick_prediction_pressure_field_from_substrate(&self, now_us: u64) {
        let gap = self.read_active_reasoning_gap();
        let evidence = self.read_evidence_tension();
        let memory = self.read_global_reasoning_memory_summary();
        let relation = {
            let relations = self.global_relations.read().unwrap();
            relations
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.active)
                .max_by(|(_, lhs), (_, rhs)| {
                    lhs.pred
                        .relation_surprise
                        .partial_cmp(&rhs.pred.relation_surprise)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, edge)| {
                    (
                        idx as i16,
                        edge.src_slot_id as i8,
                        edge.pred.relation_surprise,
                        edge.relation_confidence,
                        edge.predictive_score,
                    )
                })
        };

        let mut inputs = Vec::with_capacity(4);
        if gap.gap_pressure > 0.03 {
            inputs.push(PredictionWaveInput {
                source_kind: PREDICTION_WAVE_GAP,
                slot_id: gap.focus_slot_id,
                edge_id: gap.focus_edge_id,
                window_start_us: gap.last_update_t_us,
                window_end_us: now_us,
                amplitude: gap.gap_pressure,
                resonance: unit(gap.persistence_score.max(gap.reentry_score)),
                closure_affordance: unit(gap.closure_score.max(gap.persistence_score * 0.5)),
                write_risk: unit(0.35 * (1.0 - gap.closure_score)),
            });
        }

        if evidence.unresolved_pressure > 0.03 {
            let tension_resonance = unit(evidence.current_tension / 4.0);
            inputs.push(PredictionWaveInput {
                source_kind: PREDICTION_WAVE_EVENT,
                slot_id: gap.focus_slot_id,
                edge_id: gap.focus_edge_id,
                window_start_us: evidence.last_update_t_us,
                window_end_us: now_us,
                amplitude: evidence.unresolved_pressure,
                resonance: tension_resonance,
                closure_affordance: unit(
                    gap.closure_score.max(memory.routing_stability_score * 0.5),
                ),
                write_risk: unit(0.65 * (1.0 - gap.closure_score)),
            });
        }

        if let Some((edge_id, slot_id, surprise, confidence, predictive)) = relation {
            if surprise > 0.03 {
                inputs.push(PredictionWaveInput {
                    source_kind: PREDICTION_WAVE_RELATION,
                    slot_id,
                    edge_id,
                    window_start_us: now_us.saturating_sub(20_000),
                    window_end_us: now_us,
                    amplitude: unit(surprise),
                    resonance: unit(confidence.max(memory.pattern_recurrence_score)),
                    closure_affordance: unit(predictive.max(memory.routing_stability_score)),
                    write_risk: unit(0.45 * (1.0 - confidence)),
                });
            }
        }

        let immediate_pressure = gap.gap_pressure.max(evidence.unresolved_pressure).max(
            relation
                .map(|(_, _, surprise, _, _)| unit(surprise))
                .unwrap_or(0.0),
        );
        let residue_amplitude = unit(
            memory.pattern_recurrence_score
                * memory.persistence_score
                * memory.dominant_focus_persistence.max(0.25)
                * (1.0 - immediate_pressure * 0.35),
        );
        if residue_amplitude > 0.03 {
            inputs.push(PredictionWaveInput {
                source_kind: PREDICTION_WAVE_RESIDUE,
                slot_id: gap.focus_slot_id,
                edge_id: gap.focus_edge_id,
                window_start_us: memory.last_update_t_us,
                window_end_us: now_us,
                amplitude: residue_amplitude,
                resonance: unit(
                    memory
                        .pattern_recurrence_score
                        .max(memory.dominant_focus_persistence),
                ),
                closure_affordance: unit(memory.routing_stability_score.max(0.25)),
                write_risk: 0.30,
            });
        }

        self.refresh_prediction_pressure_field(now_us, &inputs);
    }

    pub fn apply_prediction_field_closure(&self, feedback: f64) {
        self.prediction_pressure_field
            .write()
            .unwrap()
            .apply_closure(feedback);
    }

    pub fn read_prediction_pressure_field(&self) -> PredictionPressureFieldState {
        self.prediction_pressure_field.read().unwrap().clone()
    }
}

