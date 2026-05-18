#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AuditoryVisualState {
    pub predictable_return_strength: f64,
    pub average_return_delay_us: f64,
    pub return_closure_count: u32,
    pub cross_channel_predictability: f64,
    pub alignment_pressure: f64,
    pub drift_pressure: f64,
    pub aligned_event_count: u32,
    pub drift_event_count: u32,
    pub expressive_pressure: f64,
    pub expressive_pressure_peak: f64,
    pub expression_write_epochs: u32,
    pub expression_closure_count: u32,
    pub mismatch_pressure: f64,
    pub pair_cursor: u64,
    pub fusion_cursor_t_us: u64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReturnSample {
    pub delay_us: u64,
    pub curve_similarity: f64,
    pub return_magnitude: f64,
    pub write_magnitude: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChannelSample {
    pub alignment: f64,
    pub confidence: f64,
    pub pressure: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExpressionDrive {
    pub breath: f64,
    pub pitch: f64,
    pub resonance: f64,
    pub pressure: f64,
}

impl AuditoryVisualState {
    pub fn observe_self_return(&mut self, now_us: u64, sample: ReturnSample) {
        let delay_support = 1.0 - (sample.delay_us as f64 / 240_000.0).clamp(0.0, 1.0);
        let magnitude_gap = (sample.return_magnitude - sample.write_magnitude).abs();
        let magnitude_support = (1.0 - magnitude_gap).clamp(0.0, 1.0);
        let support = (delay_support * 0.45
            + sample.curve_similarity.clamp(0.0, 1.0) * 0.35
            + magnitude_support * 0.20)
            .clamp(0.0, 1.0);

        self.predictable_return_strength =
            (self.predictable_return_strength * 0.70 + support * 0.30).clamp(0.0, 1.0);
        self.average_return_delay_us = if self.return_closure_count == 0 {
            sample.delay_us as f64
        } else {
            self.average_return_delay_us * 0.75 + sample.delay_us as f64 * 0.25
        };
        self.return_closure_count = self.return_closure_count.saturating_add(1);
        self.mismatch_pressure =
            (self.mismatch_pressure * 0.65 + (1.0 - support) * 0.35).clamp(0.0, 1.0);
        self.last_update_t_us = now_us;
    }

    pub fn observe_channel_alignment(&mut self, now_us: u64, sample: ChannelSample) {
        let alignment = sample.alignment.clamp(0.0, 1.0);
        let confidence = sample.confidence.clamp(0.0, 1.0);
        let pressure = sample.pressure.clamp(0.0, 1.0);
        let support = (alignment * 0.55 + confidence * 0.25 + pressure * 0.20).clamp(0.0, 1.0);

        if alignment >= 0.45 && confidence > 0.05 {
            self.aligned_event_count = self.aligned_event_count.saturating_add(1);
            self.cross_channel_predictability =
                (self.cross_channel_predictability * 0.68 + support * 0.32).clamp(0.0, 1.0);
            self.alignment_pressure =
                (self.alignment_pressure * 0.60 + support * 0.40).clamp(0.0, 1.0);
            self.expressive_pressure =
                (self.expressive_pressure * 0.72 + support * 0.34).clamp(0.0, 1.0);
            self.drift_pressure *= 0.82;
        } else {
            let drift = (1.0 - alignment).max(pressure * 0.35).clamp(0.0, 1.0);
            self.drift_event_count = self.drift_event_count.saturating_add(1);
            self.drift_pressure = (self.drift_pressure * 0.70 + drift * 0.30).clamp(0.0, 1.0);
            self.cross_channel_predictability *= 0.94;
            self.alignment_pressure *= 0.88;
        }

        self.expressive_pressure_peak = self.expressive_pressure_peak.max(self.expressive_pressure);
        self.last_update_t_us = now_us;
    }

    pub fn drive_expression(&mut self, now_us: u64) -> Option<ExpressionDrive> {
        let pressure = self
            .expressive_pressure
            .max(self.alignment_pressure * 0.85)
            .clamp(0.0, 1.0);
        if pressure <= 0.35 {
            return None;
        }
        self.expression_write_epochs = self.expression_write_epochs.saturating_add(1);
        self.expressive_pressure_peak = self.expressive_pressure_peak.max(pressure);
        self.last_update_t_us = now_us;
        Some(ExpressionDrive {
            breath: (0.28 + pressure * 0.52).clamp(0.0, 1.0),
            pitch: (0.38 + self.cross_channel_predictability * 0.24).clamp(0.0, 1.0),
            resonance: (0.44 + self.predictable_return_strength * 0.20).clamp(0.0, 1.0),
            pressure,
        })
    }

    pub fn observe_expression_closure(&mut self, now_us: u64, support: f64) {
        let support = support.clamp(0.0, 1.0);
        self.expression_closure_count = self.expression_closure_count.saturating_add(1);
        self.expressive_pressure =
            (self.expressive_pressure * (1.0 - support * 0.45)).clamp(0.0, 1.0);
        self.mismatch_pressure = (self.mismatch_pressure * (1.0 - support * 0.30)).clamp(0.0, 1.0);
        self.last_update_t_us = now_us;
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        let fast = 0.5_f64.powf(elapsed_us as f64 / 2_000_000.0);
        let slow = 0.5_f64.powf(elapsed_us as f64 / 8_000_000.0);
        self.alignment_pressure *= fast;
        self.drift_pressure *= fast;
        self.expressive_pressure *= fast;
        self.mismatch_pressure *= fast;
        self.predictable_return_strength *= slow;
        self.cross_channel_predictability *= slow;
        self.last_update_t_us = now_us;
    }
}
