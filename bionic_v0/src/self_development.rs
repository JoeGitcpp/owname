use crate::action_consequence::{
    ActionConsequencePair, ActionConsequenceWindow, ConsequenceModalityKind, ConsequencePairStatus,
};
use crate::action_output::WriteEpochRecord;
use crate::substrate::{ema, unit};
use crate::viewport::Viewport;
use crate::{CONSEQUENCE_WINDOW_CAP, PAIR_HISTORY_CAP, WRITE_EPOCH_HISTORY_CAP};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelfDevelopmentState {
    pub prediction_gap: f64,
    pub prediction_improvement: f64,
    /// Slow EMA of prediction_gap (α=0.04). Used for two-timescale trend detection:
    /// trend = slow − fast; positive only while gap is persistently declining.
    pub prediction_gap_slow: f64,
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
    // Layer 1: self-vocal boundary
    pub self_vocal_correlation: f64,
    pub external_vocal_correlation: f64,
    pub self_other_separation: f64,
    // Layer 2: body state awareness
    pub body_state_awareness: f64,
    pub fatigue_pressure: f64,
    pub health_surprise: f64,
    // Layer 3: agency pressure
    pub agency_pressure: f64,
    pub self_caused_event_ratio: f64,
    pub external_event_ratio: f64,
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
    self_vocal_correlation: f64,
    external_vocal_correlation: f64,
    phot_health_ok: f64,
    total_updates: u64,
    paired_write_ratio: f64,
}

impl SelfDevelopmentState {
    fn update(&mut self, sample: SelfDevelopmentSample, now_us: u64) {
        let gap = unit(sample.prediction_gap);
        // Two-timescale trend: slow EMA captures history; (slow − fast) is positive
        // only while gap is persistently declining. For pure noise the slow and fast
        // EMAs converge to the same mean, so the signed trend oscillates around 0 and
        // the clamped output stays near 0. A noisy-TV region therefore reads ≈ 0 here
        // even when its instantaneous surprise is high.
        let trend = self.prediction_gap_slow - gap;
        let raw_pi = (1.0 - 0.12) * self.prediction_improvement + 0.12 * trend;
        self.prediction_improvement = raw_pi.clamp(0.0, 1.0);
        self.prediction_gap = gap;
        self.prediction_gap_slow = ema(self.prediction_gap_slow, gap, 0.04);

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
            unit(self.repetition_pressure * (1.0 - 0.55 * self.familiarity_pressure));
        self.output_suppression = ema(self.output_suppression, suppression_input, 0.35);

        self.exploratory_pressure = unit(
            gap.max(sample.field_pressure) * (1.0 - 0.20 * self.familiarity_pressure),
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
                && self.repetition_pressure < 0.35,
        );

        if self.output_suppression > 0.50 {
            self.suppression_events = self.suppression_events.saturating_add(1);
        }
        if sample.reentry_score > 0.03 && self.familiarity_pressure > 0.25 {
            self.stable_reentry_count = self.stable_reentry_count.saturating_add(1);
        }

        // Layer 1: self-vocal boundary
        self.self_vocal_correlation =
            ema(self.self_vocal_correlation, sample.self_vocal_correlation, 0.22);
        self.external_vocal_correlation =
            ema(self.external_vocal_correlation, sample.external_vocal_correlation, 0.18);
        let sep = self.self_vocal_correlation + self.external_vocal_correlation;
        let raw_sep = if sep > 1e-9 {
            self.self_vocal_correlation / sep
        } else {
            0.0
        };
        self.self_other_separation = ema(self.self_other_separation, raw_sep, 0.30);

        // Layer 2: body state awareness
        let health_now = sample.phot_health_ok;
        let health_prev = (self.body_state_awareness * 0.5 + 0.5).clamp(0.0, 1.0);
        self.health_surprise =
            ema(self.health_surprise, (health_now - health_prev).abs(), 0.40);
        self.body_state_awareness = ema(
            self.body_state_awareness,
            unit(1.0 - health_now + self.health_surprise * 0.6),
            0.25,
        );
        self.fatigue_pressure = ema(
            self.fatigue_pressure,
            unit(sample.total_updates as f64 / 10_000.0),
            0.12,
        );

        // Layer 3: agency
        let agency_input =
            unit(sample.paired_write_ratio * (1.0 - 0.3 * sample.prediction_gap));
        self.agency_pressure = ema(self.agency_pressure, agency_input, 0.28);
        let total_events = sample
            .paired_write_ratio
            .max(1.0 - sample.paired_write_ratio)
            .max(1e-9);
        self.self_caused_event_ratio = ema(
            self.self_caused_event_ratio,
            sample.paired_write_ratio / total_events,
            0.30,
        );
        self.external_event_ratio = ema(
            self.external_event_ratio,
            unit(1.0 - sample.paired_write_ratio) / total_events,
            0.30,
        );

        self.total_updates = self.total_updates.saturating_add(1);
        self.last_update_t_us = now_us;
    }
}

fn compute_vocal_boundary(
    write_epochs: &[WriteEpochRecord],
    windows: &[ActionConsequenceWindow],
    pairs: &[ActionConsequencePair],
) -> (f64, f64) {
    let write_seqs: std::collections::BTreeSet<u64> =
        write_epochs.iter().map(|ep| ep.write_seq).collect();

    let mut self_corr = 0.0_f64;
    let mut ext_corr = 0.0_f64;

    let paired_window_ids: std::collections::BTreeSet<u64> = pairs
        .iter()
        .filter(|p| p.status == ConsequencePairStatus::Paired)
        .map(|p| p.window_id)
        .collect();

    for pair in pairs {
        if pair.status != ConsequencePairStatus::Paired {
            continue;
        }
        let window = match windows.iter().find(|w| w.window_id == pair.window_id) {
            Some(w) => w,
            None => continue,
        };
        if window.modality_kind != ConsequenceModalityKind::GenericIngress {
            continue;
        }
        if !write_seqs.contains(&pair.write_seq) {
            continue;
        }

        let delay_norm = unit(1.0 - (pair.delay_us as f64 / 500_000.0).min(1.0));
        let score = delay_norm * unit(pair.curve_similarity) * unit(window.magnitude);
        self_corr = self_corr.max(score);
    }

    for window in windows {
        if window.modality_kind == ConsequenceModalityKind::GenericIngress
            && !paired_window_ids.contains(&window.window_id)
        {
            ext_corr = ext_corr.max(unit(window.magnitude));
        }
    }

    (unit(self_corr), unit(ext_corr))
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

        let (self_vocal_corr, external_vocal_corr) = compute_vocal_boundary(
            &action_output.write_epoch_history,
            &action_consequence.windows,
            &action_consequence.pairs,
        );

        let phot_ok = match self.read_phot_health() {
            crate::viewport::HealthState::Ok => 1.0_f64,
            crate::viewport::HealthState::Quiet => 0.5_f64,
            _ => 0.0_f64,
        };

        let total_writes = action_output.n_write_epochs.max(1) as f64;
        let paired_writes = action_consequence
            .pairs
            .iter()
            .filter(|p| p.status == ConsequencePairStatus::Paired)
            .map(|p| p.write_seq)
            .collect::<std::collections::BTreeSet<u64>>()
            .len() as f64;
        let paired_write_ratio = unit(paired_writes / total_writes);

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
            self_vocal_correlation: self_vocal_corr,
            external_vocal_correlation: external_vocal_corr,
            phot_health_ok: phot_ok,
            total_updates: self.read_self_development().total_updates,
            paired_write_ratio,
        };

        self.self_development
            .write()
            .unwrap()
            .update(sample, now_us);
        // Stage 0: mirror learning_progress scalar onto its shared interoceptive
        // axis. prediction_improvement = EMA of d(gap)/dt downward component —
        // the substrate form of "prediction is getting better". No label "good/bad";
        // it is a substrate quantity that the prediction engine will now predict.
        let mut lp = self.self_development.read().unwrap().prediction_improvement;
        if let Some(crate::cognitive_pipeline::CognitiveQuerySummary::Separated { .. }) =
            self.read_cognitive_output()
        {
            lp = (lp + 0.1).min(1.0);
        }
        self.push_observation(crate::INTERO_LEARNING_PROGRESS, now_us, lp);
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

fn memory_like(recurrence: f64, focus_persistence: f64) -> f64 {
    unit((unit(recurrence) + unit(focus_persistence)) * 0.5)
}
