#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalReflexState {
    pub reflex_pressure: f64,
    pub fast_gain: f64,
    pub prior_drive: f64,
    pub burst_count: u32,
    pub bypass_count: u32,
    pub quiet_regulation_successes: u32,
    pub last_update_t_us: u64,
    /// Monotonic pair_seq cursor for tick_vocal_reflex_from_pairs.
    /// Engineering locator only; keeps closure processing idempotent.
    pub last_pair_cursor: u64,
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
    pub fn tick_drive(
        &mut self,
        now_us: u64,
        drive: VocalReflexDrive,
        prediction_familiarity: f64,
        action_suppression: f64,
        vocal_efficacy: f64,
    ) -> Option<VocalReflexBurst> {
        let drive_pressure = drive.drive_pressure.clamp(0.0, 1.0);
        let sudden_delta = (drive_pressure - self.prior_drive).max(0.0);
        let regulating_pressure = prediction_familiarity * 0.35 + action_suppression * 0.45;
        let mut reflex_pressure = drive_pressure * 0.75 + sudden_delta * 0.35 - regulating_pressure
            + self.fast_gain * 0.15;

        // Refactor (plan 2026-05-28 Step2) moved suppression into learned_bias
        // and lowered per-closure delta from ~0.10 to ≤0.01 (a slow 死后保留
        // sediment). The bypass threshold must track that smaller scale, else
        // it would need ~50 closures to ever trigger — the urgent-pressure
        // safety bypass would effectively never fire. Scaled 0.40 → 0.04.
        let bypass = drive_pressure > 0.78 && action_suppression > 0.04;
        if bypass {
            reflex_pressure = reflex_pressure.max(0.56);
            self.bypass_count = self.bypass_count.saturating_add(1);
        }

        self.reflex_pressure = reflex_pressure.clamp(0.0, 1.0);
        self.fast_gain = (self.fast_gain * 0.82 + sudden_delta * 0.24).clamp(0.0, 1.0);
        self.prior_drive = self.prior_drive * 0.65 + drive_pressure * 0.35;
        self.last_update_t_us = now_us;

        // 改造1: the voicing gate is LEARNED, not the hardcoded 0.40 wall.
        // It lowers as vocal_efficacy grows — i.e. as voicing has proven to
        // bring real external closure, the body gets easier to voice (爱叫).
        // A newborn (efficacy=0) keeps the 0.40 baseline (backward-compatible).
        // This is independent of prediction_familiarity/action_suppression,
        // which still RAISE the bar to damp self-excitation/habituation (闭嘴).
        let gate = (0.40 - vocal_efficacy.clamp(0.0, 1.0) * 0.30).clamp(0.10, 0.40);
        if self.reflex_pressure <= gate {
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

    /// Returns (delta_prediction_familiarity, delta_action_suppression)
    /// to be applied to LearnedBias by the caller.
    pub fn observe_closure(&mut self, now_us: u64, closure_support: f64, vocalized: bool) -> (f64, f64) {
        let closure = closure_support.clamp(0.0, 1.0);
        let mut delta_fam = 0.0;
        let mut delta_sup = 0.0;

        if vocalized {
            delta_fam = closure * 0.01;
            self.fast_gain = (self.fast_gain * (1.0 - closure * 0.30)).clamp(0.0, 1.0);
        } else {
            self.quiet_regulation_successes = self.quiet_regulation_successes.saturating_add(1);
            delta_sup = closure * 0.01;
            delta_fam = closure * 0.01;
            self.fast_gain = (self.fast_gain * (1.0 - closure * 0.45)).clamp(0.0, 1.0);
        }

        self.reflex_pressure = (self.reflex_pressure * (1.0 - closure * 0.35)).clamp(0.0, 1.0);
        self.last_update_t_us = now_us;

        (delta_fam, delta_sup)
    }

    /// Endogenous baseline rhythm: slow oscillation that drives exploratory
    /// vocalization in the absence of external stimuli. Modulated by
    /// prediction_familiarity and action_suppression so that experienced
    /// systems rely less on the baseline and more on learned regulation.
    ///
    /// Returns a pressure value in [0.0, 0.12] — just enough to cross the
    /// reflex threshold (0.40) when combined with other drive sources or
    /// when the newborn has near-zero regulating pressure.
    pub fn endogenous_baseline(
        now_us: u64,
        prediction_familiarity: f64,
        action_suppression: f64,
    ) -> f64 {
        // Slow oscillation: ~0.04 Hz (25-second period).
        // This is the "wave" — a substrate-level numerical rhythm that,
        // in the absence of external stimuli, creates periodic internal
        // activation. Not a meaning-bearing clock; purely a numerical carrier.
        //
        // Modulo first to keep the f64 argument within precision range.
        // UNIX epoch µs (~1.7e15) would otherwise lose 6+ decimal digits
        // in f64, randomising the sine phase.
        let period_us: u64 = 25_000_000;
        let phase = ((now_us % period_us) as f64 / period_us as f64)
            * 2.0 * std::f64::consts::PI;
        let rhythm = phase.sin() * 0.5 + 0.5; // [0, 1]

        // Exploration drive: maximal for newborns, minimal for experienced.
        // 0.60 at peak → reflex_pressure ≈ 0.60 * 0.75 = 0.45, crossing the
        // 0.40 burst threshold with comfortable margin for a faint exploratory
        // vocalization (breath ≈ 0.26). Fully experienced systems (familiarity=1.0)
        // drop to zero.
        let exploration_drive = (1.0 - prediction_familiarity) * 0.60;

        // Rhythm shaped by exploration drive.
        let baseline = rhythm * exploration_drive;

        // Dampened by learned suppression. High suppression → quieter baseline.
        baseline * (1.0 - action_suppression * 0.55)
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
        self.last_update_t_us = now_us;
    }
}
