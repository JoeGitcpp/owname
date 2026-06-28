/// B2 机制B(B1路线): endogenous baseline 的 suppression 乘性阻尼地板。
/// 消除乘性退化不动点(0×x=0)——不是发声靶子:它只保证"内生探索节律"
/// 这一输入项不会被同一回路沉积的抑制乘性压塌;suppression 仍经
/// tick_drive 的 regulating_pressure 加性地全额起作用(单重阻尼,
/// 不再双重)。取 0.96: 地板恰在历史死锁起点 S*≈0.0717 处接管
/// (1-0.55·0.0717≈0.96),低于该点原阻尼律不受影响。
/// 成熟体 prediction_familiarity→1 时 exploration_drive→0,
/// baseline 整体仍自然归零——地板不会逼成熟体发声。
///
/// B2 增强: 此常量现为发育时钟退火的 m=0 端点(新生最宽)。实际地板由
/// 调用方经 lib.rs::suppression_damp_floor(发育时钟) 连续线性收窄,
/// 在 m=1 处到 0.45 = 1−0.55·1.0——即地板退到乘性项自身的自然下限,
/// 成熟体逐位回到原始无地板阻尼律。连续数值调制,非资格门、非靶子。
pub const SUPPRESSION_DAMP_FLOOR: f64 = 0.96;

/// Fatigue rise per burst: determines how fast the negative-feedback
/// pole accumulates. Two quick bursts → fatigue ≈ 0.24, suppression
/// ≈ 0.24×0.45 = 0.11, enough to push a newborn (margin ≈ 0.10) below
/// the gate → silence until decay. Tuned for 2-3 burst clusters.
pub const FATIGUE_RISE_PER_BURST: f64 = 0.12;
/// Fatigue suppression gain: how strongly fatigue reduces reflex_pressure.
pub const FATIGUE_SUPPRESSION_GAIN: f64 = 0.45;
/// Fatigue decay per tick (at ~50 Hz). Half-life ≈ 3s:
/// 0.5^(1/150) ≈ 0.9954. Creates a ~5-8s natural oscillation period.
pub const FATIGUE_DECAY_PER_TICK: f64 = 0.9954;

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
    /// Vocal fatigue: accumulates on burst, decays during silence.
    /// Creates a limit-cycle oscillation (burst cluster → silence → burst)
    /// replacing the hardcoded sine rhythm.
    pub vocal_fatigue: f64,
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
        endogenous_floor: f64,
    ) -> Option<VocalReflexBurst> {
        let drive_pressure = drive.drive_pressure.clamp(0.0, 1.0);
        let sudden_delta = (drive_pressure - self.prior_drive).max(0.0);
        let regulating_pressure = prediction_familiarity * 0.35 + action_suppression * 0.45;
        let mut reflex_pressure = drive_pressure * 0.75 + sudden_delta * 0.35 - regulating_pressure
            + self.fast_gain * 0.15;

        // Endogenous floor: prevents regulation from crushing drive to zero.
        reflex_pressure = reflex_pressure.max(endogenous_floor);

        // Fatigue: negative feedback that creates natural burst-pause oscillation.
        // Accumulates on burst (positive feedback, fast), decays during silence
        // (negative feedback, slow). The ratio of rise/decay time constants
        // determines the oscillation period (~5-8 seconds).
        reflex_pressure -= self.vocal_fatigue * FATIGUE_SUPPRESSION_GAIN;
        self.vocal_fatigue *= FATIGUE_DECAY_PER_TICK;

        let bypass = drive_pressure > 0.78 && action_suppression > 0.04;
        if bypass {
            reflex_pressure = reflex_pressure.max(0.56);
            self.bypass_count = self.bypass_count.saturating_add(1);
        }

        self.reflex_pressure = reflex_pressure.clamp(0.0, 1.0);
        self.fast_gain = (self.fast_gain * 0.82 + sudden_delta * 0.24).clamp(0.0, 1.0);
        self.prior_drive = self.prior_drive * 0.65 + drive_pressure * 0.35;
        self.last_update_t_us = now_us;

        let gate = (0.40 - vocal_efficacy.clamp(0.0, 1.0) * 0.30).clamp(0.10, 0.40);
        if self.reflex_pressure <= gate {
            return None;
        }

        self.burst_count = self.burst_count.saturating_add(1);
        self.vocal_fatigue = (self.vocal_fatigue + FATIGUE_RISE_PER_BURST).min(1.0);
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

    /// Endogenous baseline: steady-state drive level for exploratory
    /// vocalization. Maximal for newborns, decays with familiarity.
    /// No hardcoded rhythm — the burst-pause oscillation emerges from
    /// vocal_fatigue dynamics (positive feedback on burst, exponential
    /// decay during silence → limit cycle).
    pub fn endogenous_baseline(
        _now_us: u64,
        prediction_familiarity: f64,
        action_suppression: f64,
        damp_floor: f64,
        babble_floor: f64,
    ) -> f64 {
        let exploration_drive = ((1.0 - prediction_familiarity) * 0.60).max(babble_floor);
        exploration_drive * (1.0 - action_suppression * 0.55).max(damp_floor)
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
