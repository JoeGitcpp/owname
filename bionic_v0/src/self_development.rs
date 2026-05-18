use crate::viewport::Viewport;
use crate::{CONSEQUENCE_WINDOW_CAP, PAIR_HISTORY_CAP, WRITE_EPOCH_HISTORY_CAP};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelfDevelopmentState {
    pub prediction_gap: f64,
    pub prediction_improvement: f64,
    pub stable_recurrence: f64,
    pub familiarity_pressure: f64,
    pub repetition_pressure: f64,
    pub exploratory_pressure: f64,
    pub output_suppression: f64,
    pub solitary_stability: f64,
    pub quiet_replay_pressure: f64,
    pub room_familiarity: f64,
    pub solitary_reentry_score: f64,
    pub solitary_growth_score: f64,
    pub solitary_growth_gate_pass: u8,
    pub total_updates: u64,
    pub suppression_events: u32,
    pub stable_reentry_count: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SelfDevelopmentSample {
    prediction_gap: f64,
    field_pressure: f64,
    pattern_recurrence: f64,
    routing_stability: f64,
    focus_persistence: f64,
    closure_score: f64,
    reentry_score: f64,
    write_load: f64,
    return_window_load: f64,
    pair_load: f64,
    average_pair_similarity: f64,
}

impl SelfDevelopmentState {
    fn update(&mut self, sample: SelfDevelopmentSample, now_us: u64) {
        let gap = unit(sample.prediction_gap);
        let instant_improvement = unit((self.prediction_gap - gap).max(0.0));
        self.prediction_improvement = ema(self.prediction_improvement, instant_improvement, 0.25);
        self.prediction_gap = gap;

        let recurrence = unit(
            sample.pattern_recurrence
                * sample.routing_stability.max(0.10)
                * sample.focus_persistence.max(0.10),
        );
        self.stable_recurrence = ema(self.stable_recurrence, recurrence, 0.18);

        let closure_support = unit(
            sample
                .closure_score
                .max(sample.average_pair_similarity)
                .max(self.prediction_improvement),
        );
        let familiarity_input =
            unit(recurrence * (0.62 + 0.38 * closure_support) * (1.0 - 0.45 * gap));
        self.familiarity_pressure = ema(self.familiarity_pressure, familiarity_input, 0.18);

        let loop_load = sample
            .write_load
            .max(sample.return_window_load)
            .max(sample.pair_load);
        let weak_closure = unit(1.0 - closure_support);
        let repetition_input = unit(loop_load * weak_closure * (1.0 - 0.35 * recurrence));
        self.repetition_pressure = ema(self.repetition_pressure, repetition_input, 0.30);

        let suppression_input =
            unit(self.repetition_pressure * (1.0 - 0.25 * self.familiarity_pressure));
        self.output_suppression = ema(self.output_suppression, suppression_input, 0.35);

        self.exploratory_pressure = unit(
            gap.max(sample.field_pressure)
                * (1.0 - 0.75 * self.output_suppression)
                * (1.0 - 0.20 * self.familiarity_pressure),
        );

        let quiet_load = unit(1.0 - loop_load);
        let quiet_gap = unit(1.0 - gap);
        let solitary_input = unit(recurrence * quiet_gap * quiet_load);
        self.solitary_stability = ema(self.solitary_stability, solitary_input, 0.16);

        let quiet_replay_input = unit(
            self.solitary_stability
                * memory_like(sample.pattern_recurrence, sample.focus_persistence)
                * (1.0 - self.repetition_pressure),
        );
        self.quiet_replay_pressure = ema(self.quiet_replay_pressure, quiet_replay_input, 0.14);

        let room_input = unit(
            self.familiarity_pressure
                .max(self.solitary_stability)
                .max(self.quiet_replay_pressure * 0.85),
        );
        self.room_familiarity = ema(self.room_familiarity, room_input, 0.18);

        let reentry_input = unit(sample.reentry_score * self.room_familiarity.max(0.05));
        self.solitary_reentry_score = unit((self.solitary_reentry_score * 0.82).max(reentry_input));

        self.solitary_growth_score = unit(
            self.solitary_stability * 0.34
                + self.quiet_replay_pressure * 0.22
                + self.room_familiarity * 0.24
                + self.familiarity_pressure * 0.20
                - self.repetition_pressure * 0.35,
        );
        self.solitary_growth_gate_pass = u8::from(
            self.solitary_growth_score > 0.35
                && self.room_familiarity > 0.30
                && self.repetition_pressure < 0.35
                && self.output_suppression < 0.45,
        );

        if self.output_suppression > 0.50 {
            self.suppression_events = self.suppression_events.saturating_add(1);
        }
        if sample.reentry_score > 0.03 && self.familiarity_pressure > 0.25 {
            self.stable_reentry_count = self.stable_reentry_count.saturating_add(1);
        }

        self.total_updates = self.total_updates.saturating_add(1);
        self.last_update_t_us = now_us;
    }
}

impl Viewport {
    pub fn tick_self_development_from_substrate(&self, now_us: u64) {
        let reasoning = self.read_global_reasoning();
        let memory = self.read_global_reasoning_memory_summary();
        let active_gap = self.read_active_reasoning_gap();
        let evidence = self.read_evidence_tension();
        let field = self.read_prediction_pressure_field();
        let action_output = self.read_action_output();
        let action_consequence = self.read_action_consequence();

        let relation_surprise = self
            .read_global_relations()
            .edges
            .iter()
            .filter(|edge| edge.active)
            .map(|edge| edge.pred.relation_surprise)
            .fold(0.0_f64, f64::max);

        let average_pair_similarity = if action_consequence.pairs.is_empty() {
            0.0
        } else {
            let n = action_consequence.pairs.len().min(16);
            let sum: f64 = action_consequence
                .pairs
                .iter()
                .rev()
                .take(n)
                .map(|pair| unit(pair.curve_similarity))
                .sum();
            sum / n as f64
        };

        let sample = SelfDevelopmentSample {
            prediction_gap: reasoning
                .global_surprise_load
                .max(relation_surprise)
                .max(active_gap.gap_pressure)
                .max(evidence.unresolved_pressure),
            field_pressure: field.dominant_pressure,
            pattern_recurrence: memory.pattern_recurrence_score,
            routing_stability: memory.routing_stability_score,
            focus_persistence: memory.dominant_focus_persistence,
            closure_score: active_gap.closure_score,
            reentry_score: active_gap.reentry_score,
            write_load: ratio(action_output.n_write_epochs, WRITE_EPOCH_HISTORY_CAP),
            return_window_load: ratio(action_consequence.n_windows, CONSEQUENCE_WINDOW_CAP),
            pair_load: ratio(action_consequence.n_pairs, PAIR_HISTORY_CAP),
            average_pair_similarity,
        };

        self.self_development
            .write()
            .unwrap()
            .update(sample, now_us);
    }

    pub fn read_self_development(&self) -> SelfDevelopmentState {
        *self.self_development.read().unwrap()
    }
}

fn ratio(value: u32, cap: usize) -> f64 {
    if cap == 0 {
        0.0
    } else {
        unit(value as f64 / cap as f64)
    }
}

fn ema(previous: f64, next: f64, alpha: f64) -> f64 {
    unit((1.0 - alpha) * previous + alpha * next)
}

fn memory_like(recurrence: f64, focus_persistence: f64) -> f64 {
    unit((unit(recurrence) + unit(focus_persistence)) * 0.5)
}

fn unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
