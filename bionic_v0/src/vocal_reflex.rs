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
        endogenous_floor: f64,
    ) -> Option<VocalReflexBurst> {
        let drive_pressure = drive.drive_pressure.clamp(0.0, 1.0);
        let sudden_delta = (drive_pressure - self.prior_drive).max(0.0);
        let regulating_pressure = prediction_familiarity * 0.35 + action_suppression * 0.45;
        let mut reflex_pressure = drive_pressure * 0.75 + sudden_delta * 0.35 - regulating_pressure
            + self.fast_gain * 0.15;

        // Post-regulation babble floor: the endogenous baseline (rhythm ×
        // exploration_drive, already maturity-annealed) survives regulation.
        // Without this, regulating_pressure (dominated by pred_fam × 0.35 ≈
        // 0.35) kills all spontaneous drive (peak ~0.18), making the system
        // 100% reactive / 0% spontaneous.
        reflex_pressure = reflex_pressure.max(endogenous_floor);

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
    /// `damp_floor`: multiplicative floor for the suppression damping term,
    /// precomputed by the caller from the developmental clock via
    /// lib.rs::suppression_damp_floor (continuous anneal: widest at birth,
    /// narrowing to the term's own natural lower bound 0.45 where it never
    /// binds). Passing SUPPRESSION_DAMP_FLOOR reproduces the newborn law.
    pub fn endogenous_baseline(
        now_us: u64,
        prediction_familiarity: f64,
        action_suppression: f64,
        damp_floor: f64,
        babble_floor: f64,
    ) -> f64 {
        // Slow oscillation: ~0.04 Hz (25-second period).
        // This is the "wave" — a substrate-level numerical rhythm that,
        // in the absence of external stimuli, creates periodic internal
        // activation. Not a meaning-bearing clock; purely a numerical carrier.
        //
        // Modulo first to keep the f64 argument within precision range.
        // UNIX epoch µs (~1.7e15) would otherwise lose 6+ decimal digits
        // in f64, randomising the sine phase.
        // Reduced from 25 s → 5 s: faster heartbeat gives the newborn body many
        // more opportunities per minute to attempt a vocalization and hear itself
        // back, accelerating the L1 babble→self-play bootstrap.  Once the
        // phoneme pool is stable, `exploration_drive` falls toward 0 anyway.
        let period_us: u64 = 5_000_000;
        let phase = ((now_us % period_us) as f64 / period_us as f64)
            * 2.0 * std::f64::consts::PI;
        let rhythm = phase.sin() * 0.5 + 0.5; // [0, 1]

        // Exploration drive: maximal for newborns, minimal for experienced.
        // 0.60 at peak → reflex_pressure ≈ 0.60 * 0.75 = 0.45, crossing the
        // 0.40 burst threshold with comfortable margin for a faint exploratory
        // vocalization (breath ≈ 0.26). Fully experienced systems (familiarity=1.0)
        // drop to zero.
        // 可疑-1 fix: floor the exploratory drive so familiarity→1 can no longer
        // zero spontaneous babble (the bursts=0-all-call gap). The (1−familiarity)
        // term still QUIETS a mastered region (Moulin-Frier "redirect, don't go
        // globally silent"); `.max(babble_floor)` installs the Warlaumont/SAC
        // always-on floor under it. babble_floor is annealed by the developmental
        // clock by the caller (wide at birth → thin adult floor), like damp_floor.
        let exploration_drive = ((1.0 - prediction_familiarity) * 0.60).max(babble_floor);

        // Rhythm shaped by exploration drive.
        let baseline = rhythm * exploration_drive;

        // Dampened by learned suppression. High suppression → quieter baseline.
        // B2 机制B: 阻尼项加乘性地板——消除乘性退化不动点,非发声靶子;
        // familiarity→1 时 exploration_drive→0 仍让 baseline 整体归零。
        // 地板值由调用方按发育时钟连续退火传入(幼态宽=自举逃逸,
        // 成熟收窄到 0.45 后地板不再起作用,回到原始阻尼数学)。
        baseline * (1.0 - action_suppression * 0.55).max(damp_floor)
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
