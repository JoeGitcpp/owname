#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalReflexState {
    pub reflex_pressure: f64,
    pub fast_gain: f64,
    pub familiarity: f64,
    pub suppression_strength: f64,
    pub prior_drive: f64,
    pub burst_count: u32,
    pub bypass_count: u32,
    pub quiet_regulation_successes: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalReflexDrive {
    pub drive_pressure: f64,
    pub energy: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalReflexBurst {
    pub breath: f64,
    pub pitch: f64,
    pub resonance: f64,
    pub noise: f64,
}

impl VocalReflexState {
    pub fn tick_drive(&mut self, now_us: u64, drive: VocalReflexDrive) -> Option<VocalReflexBurst> {
        let drive_pressure = drive.drive_pressure.clamp(0.0, 1.0);
        let sudden_delta = (drive_pressure - self.prior_drive).max(0.0);
        let regulating_pressure = self.familiarity * 0.35 + self.suppression_strength * 0.45;
        let mut reflex_pressure = drive_pressure * 0.75 + sudden_delta * 0.35 - regulating_pressure
            + self.fast_gain * 0.15;

        let bypass = drive_pressure > 0.78 && self.suppression_strength > 0.40;
        if bypass {
            reflex_pressure = reflex_pressure.max(0.56);
            self.bypass_count = self.bypass_count.saturating_add(1);
        }

        self.reflex_pressure = reflex_pressure.clamp(0.0, 1.0);
        self.fast_gain = (self.fast_gain * 0.82 + sudden_delta * 0.24).clamp(0.0, 1.0);
        self.prior_drive = self.prior_drive * 0.65 + drive_pressure * 0.35;
        self.last_update_t_us = now_us;

        if self.reflex_pressure <= 0.40 {
            return None;
        }

        self.burst_count = self.burst_count.saturating_add(1);
        Some(VocalReflexBurst {
            breath: (self.reflex_pressure * (0.45 + 0.35 * drive.energy)).clamp(0.0, 1.0),
            pitch: (0.46 + sudden_delta * 0.34 + self.reflex_pressure * 0.12).clamp(0.0, 1.0),
            resonance: (0.42 + drive_pressure * 0.22).clamp(0.0, 1.0),
            noise: (self.reflex_pressure * 0.55 + sudden_delta * 0.25).clamp(0.0, 1.0),
        })
    }

    pub fn observe_closure(&mut self, now_us: u64, closure_support: f64, vocalized: bool) {
        let closure = closure_support.clamp(0.0, 1.0);
        if vocalized {
            self.familiarity = (self.familiarity + closure * 0.10).clamp(0.0, 1.0);
            self.fast_gain = (self.fast_gain * (1.0 - closure * 0.30)).clamp(0.0, 1.0);
        } else {
            self.quiet_regulation_successes = self.quiet_regulation_successes.saturating_add(1);
            self.suppression_strength =
                (self.suppression_strength + closure * 0.18).clamp(0.0, 1.0);
            self.familiarity = (self.familiarity + closure * 0.13).clamp(0.0, 1.0);
            self.fast_gain = (self.fast_gain * (1.0 - closure * 0.45)).clamp(0.0, 1.0);
        }
        self.reflex_pressure = (self.reflex_pressure * (1.0 - closure * 0.35)).clamp(0.0, 1.0);
        self.last_update_t_us = now_us;
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        let fast_decay = 0.5_f64.powf(elapsed_us as f64 / 1_200_000.0);
        let slow_decay = 0.5_f64.powf(elapsed_us as f64 / 8_000_000.0);
        self.reflex_pressure *= fast_decay;
        self.fast_gain *= fast_decay;
        self.suppression_strength *= slow_decay;
        self.familiarity *= slow_decay;
        self.last_update_t_us = now_us;
    }
}
