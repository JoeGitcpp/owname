use crate::substrate::unit;
use crate::evolution_priors::{BiasAxis, BiasPriors};
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

pub const LEARNED_BIAS_MAGIC: u64 = 0xB10B10_0000_0042;
pub const LEARNED_BIAS_VERSION: u64 = 5;
pub const LEARNED_BIAS_CURVE_CAP: usize = 128;
pub const LEARNED_BIAS_EVENT_CAP: usize = 256;
/// B2 机制A: action_suppression 的纯时间半衰期（5 分钟）。
/// 远长于 5s 内生节律与单次闭环间隔（单个闭环间隔内衰减 <1%，
/// 在场安静的沉积仍稳定积累）；但有限——长期没有闭环刺激时
/// suppression 松弛回低位，结构上消除单调累积锁死（0×x=0）。
/// 无语义的数值松弛：非惩罚、非 reward、不带正负价。
pub const SUPPRESSION_HALF_LIFE_US: f64 = 300_000_000.0;
/// vocal_efficacy 的纯时间半衰期（30 分钟）。刻意比 suppression（5 分钟）长
/// 一个量级：持续的真实声学拟合（每个真实闭环沉积 delta_eff）能稳住高
/// efficacy，但一旦停止真实拟合，efficacy 在 ~30 分钟尺度松弛，使它表征
/// **近期真实发声能力**而非"曾经叫过"的单调棘轮。无语义的数值松弛：
/// 非惩罚、非 reward、不带正负价。单个闭环间隔内衰减极小（在场持续沉积
/// 仍稳定爬升），但有限——长期无真实拟合时松弛，消除棘轮锁死的退化不动点。
pub const EFFICACY_HALF_LIFE_US: f64 = 1_800_000_000.0;
/// prediction_familiarity 的纯时间半衰期（2 分钟）。意图：familiarity 应
/// 表征**近期学分密度**（最近 ~2 分钟有无持续的预测性闭环），而非"曾经见过"
/// 的单调棘轮。与 MATURITY_ADVANCE_MIN_INTERVAL_US 配合（每秒最多一次积累
/// tick）：在密集 nursery 下（~6 证据闭环/秒 → 1 tick/秒）平衡点 ≈ 0.5–0.7，
/// 使 exploration_drive = (1−fam)*0.60 ≈ 0.18–0.30 维持内生 babble 不熄；
/// 沉默 ~2 分钟后松弛到低位，babble 基线重新抬头。无语义的数值松弛：非惩罚。
pub const FAMILIARITY_HALF_LIFE_US: f64 = 120_000_000.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BiasLocator {
    #[default]
    Global,
    Band(u16),
    Slot(u16),
    Edge(u16),
    Channel(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LearnedBiasEventInput {
    pub axis: BiasAxis,
    pub locator: BiasLocator,
    pub requested_delta: f64,
    pub support: f64,
    pub deviation: f64,
    pub evidence_ref: u64,
    pub event_t_us: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LearnedBiasEvent {
    pub axis: BiasAxis,
    pub locator: BiasLocator,
    pub requested_delta: f64,
    pub applied_delta: f64,
    pub support: f64,
    pub deviation: f64,
    pub evidence_ref: u64,
    pub event_t_us: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LearnedBiasCurve {
    pub axis: BiasAxis,
    pub locator: BiasLocator,
    pub delta_value: f64,
    pub delta_ema: f64,
    pub stability: f64,
    pub support_count: u32,
    pub deviation_count: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LearnedBiasState {
    pub curves: Vec<LearnedBiasCurve>,
    pub recent_events: Vec<LearnedBiasEvent>,
    pub n_curves: u32,
    pub n_recent_events: u32,
    pub total_events: u64,
    pub last_update_t_us: u64,
    pub prediction_familiarity: f64,
    pub action_suppression: f64,
    /// 改造1: voicing effectiveness — accumulated when voicing brought real
    /// external closure. RAISES the urge to voice (lowers the gate): "voicing
    /// proved useful → 爱叫". Independent of prediction_familiarity, which damps
    /// self-excitation. Newborn 0.0; 死后保留 (serialized v4+).
    pub vocal_efficacy: f64,
    /// A3: developmental clock m∈[0,1]（Viewport::developmental_maturity 的
    /// checkpoint 镜像）。真身在 Viewport 上；save 时由
    /// `Viewport::read_learned_bias` 装入，boot 时经
    /// `Viewport::restore_maturity` 回填。新生 0.0（Default 守住）；
    /// 死后保留 (serialized v5+)。
    pub developmental_maturity: f64,
    /// B2 机制A: 上次 suppression 半衰的时刻（µs）。
    /// 刻意**不持久化**（不入 learned_bias.bin、不动 LEARNED_BIAS_VERSION）：
    /// 重启后该字段为 0，被 decay 当作"尚未起表"——首拍用 now_us 起表、不衰减
    /// （重启即续航，不伪造遗忘）。遗忘只按真实流逝时间发生：长期无闭环刺激时
    /// suppression 仍按 τ 松弛，结构上消除单调累积锁死（0×x=0）。工程时戳。
    pub last_suppression_decay_us: u64,
    /// vocal_efficacy 时间遗忘的上次半衰时刻（µs）。与 last_suppression_decay_us
    /// 同款：刻意**不持久化**（不入 learned_bias.bin、不动 LEARNED_BIAS_VERSION）。
    /// 重启后为 0 → 被 decay 当作"尚未起表"，首拍用 now_us 起表、不衰减（续航）。
    /// efficacy 仍只由真实声学闭环沉积、按真实流逝时间松弛。工程时戳。
    pub last_efficacy_decay_us: u64,
    /// prediction_familiarity 的纯时间松弛时刻（µs）。不持久化：重启后为 0 →
    /// 被 decay 当作"尚未起表"，首拍用 now_us 起表、不衰减（续航）。按真实流逝
    /// 时间松弛。工程时戳。
    pub last_familiarity_decay_us: u64,
    /// 上次 familiarity 积累 tick 的时刻（µs）。配合 FAMILIARITY_ADVANCE_MIN_INTERVAL_US
    /// 限速：nursery 密集闭环下至多 1 tick/秒，防止几十秒内钉死 1.0。不持久化。
    pub last_familiarity_advance_us: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LearnedBiasSummary {
    pub n_curves: u32,
    pub n_recent_events: u32,
    pub total_events: u64,
    pub max_abs_delta: f64,
    pub protected_axis_curves: u32,
    pub ordinary_axis_curves: u32,
    pub last_update_t_us: u64,

    /// 预测熟悉度。VPC 低层 predictor 在 closure 事件中缓慢积累。
    /// 高 = 对行为后果有稳定预期，反射抑制增强。
    /// 初始值 0.0（新生儿），死后保留。
    pub prediction_familiarity: f64,

    /// 行动抑制强度。VPC 低层 predictor 在"安静+观察到闭环"中积累。
    /// 高 = 更倾向于等待，需要更高的内部压力才触发行动。
    /// 初始值 0.0（新生儿），死后保留。
    pub action_suppression: f64,

    /// 改造1: 发声有效性。发声带来真实外部闭环时积累。
    /// 高 = 发声证明有用 → 降低发声门槛(更爱叫)。初始 0.0,死后保留。
    pub vocal_efficacy: f64,
}

pub struct LearnedBiasCheckpointPaths {
    pub final_path: PathBuf,
    pub tmp_path: PathBuf,
}

impl LearnedBiasCheckpointPaths {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let d = dir.as_ref().to_path_buf();
        Self {
            final_path: d.join("learned_bias.bin"),
            tmp_path: d.join("learned_bias.bin.tmp"),
        }
    }
}

impl LearnedBiasState {
    pub fn apply_event(&mut self, priors: &BiasPriors, input: LearnedBiasEventInput) {
        let support = unit(input.support);
        let deviation = unit(input.deviation);
        let applied_delta = priors.clamp_modification(input.axis, input.requested_delta);
        let signed_update = applied_delta * support - applied_delta.abs() * deviation;

        let ev = LearnedBiasEvent {
            axis: input.axis,
            locator: input.locator,
            requested_delta: input.requested_delta,
            applied_delta,
            support,
            deviation,
            evidence_ref: input.evidence_ref,
            event_t_us: input.event_t_us,
        };
        self.recent_events.push(ev);
        evict_front_to_cap(&mut self.recent_events, LEARNED_BIAS_EVENT_CAP);
        self.n_recent_events = self.recent_events.len() as u32;
        self.total_events = self.total_events.saturating_add(1);
        self.last_update_t_us = input.event_t_us;

        if signed_update == 0.0 {
            return;
        }

        match self
            .curves
            .iter_mut()
            .find(|c| c.axis == input.axis && c.locator == input.locator)
        {
            Some(curve) => {
                curve.delta_value += signed_update;
                curve.delta_ema = 0.9 * curve.delta_ema + 0.1 * signed_update;
                if support > 0.0 {
                    curve.support_count = curve.support_count.saturating_add(1);
                }
                if deviation > 0.0 {
                    curve.deviation_count = curve.deviation_count.saturating_add(1);
                }
                curve.stability = update_stability(curve.stability, support, deviation);
                curve.last_update_t_us = input.event_t_us;
            }
            None => {
                if self.curves.len() >= LEARNED_BIAS_CURVE_CAP {
                    self.curves.remove(0);
                }
                self.curves.push(LearnedBiasCurve {
                    axis: input.axis,
                    locator: input.locator,
                    delta_value: signed_update,
                    delta_ema: signed_update,
                    stability: update_stability(0.0, support, deviation),
                    support_count: if support > 0.0 { 1 } else { 0 },
                    deviation_count: if deviation > 0.0 { 1 } else { 0 },
                    last_update_t_us: input.event_t_us,
                });
            }
        }
        self.n_curves = self.curves.len() as u32;
    }

    pub fn effective_scalar(
        &self,
        axis: BiasAxis,
        locator: BiasLocator,
        base: f64,
        min: f64,
        max: f64,
    ) -> f64 {
        let delta = self
            .curves
            .iter()
            .find(|c| c.axis == axis && c.locator == locator)
            .map(|c| c.delta_value)
            .unwrap_or(0.0);
        (base + delta).clamp(min, max)
    }

    pub fn summary(&self) -> LearnedBiasSummary {
        let mut max_abs_delta = 0.0f64;
        let mut protected_axis_curves = 0u32;
        let mut ordinary_axis_curves = 0u32;

        for curve in self.curves.iter() {
            max_abs_delta = max_abs_delta.max(curve.delta_value.abs());
            if bias_axis_is_protected(curve.axis) {
                protected_axis_curves = protected_axis_curves.saturating_add(1);
            } else {
                ordinary_axis_curves = ordinary_axis_curves.saturating_add(1);
            }
        }

        LearnedBiasSummary {
            n_curves: self.curves.len() as u32,
            n_recent_events: self.recent_events.len() as u32,
            total_events: self.total_events,
            max_abs_delta,
            protected_axis_curves,
            ordinary_axis_curves,
            last_update_t_us: self.last_update_t_us,
            prediction_familiarity: self.prediction_familiarity,
            action_suppression: self.action_suppression,
            vocal_efficacy: self.vocal_efficacy,
        }
    }

    /// B2 机制A: action_suppression 的纯时间半衰。
    /// 无语义数值松弛——非惩罚、非 reward；suppression 仍只由闭环事件
    /// 沉积，这里只补一条遗忘律，使它成为"近期未发声闭环的滑动沉积"
    /// 而非永久单调累积。τ 远大于单次闭环间隔（在场安静仍稳定沉积），
    /// 但有限（长期无闭环时松弛），消除自举死锁的退化不动点。
    /// 重启后 last_suppression_decay_us=0（不持久化）被当作"尚未起表"：首拍用
    /// now_us 起表、不衰减（续航，不把 epoch µs 当 elapsed 伪造一次归零）。
    pub fn decay_vocal_suppression(&mut self, now_us: u64) {
        // 重启即续航：last_suppression_decay_us 不持久化 → 重启后为 0。
        // now_us 是 epoch µs（≈1.75e15），若与 0 求差会得到数百万个半衰期、
        // 把 suppression 一拍归零——而 developmental_maturity 已被回填并继续
        // 棘轮上升，造成"成熟度爬升、底层能力被凭空清零"的不诚实状态。
        // 把 0 视作"尚未起表"：用 now_us 起表、本拍不衰减（无 elapsed 可言）。
        // 稳态衰减完全不变：起表后每拍 elapsed 仍是真实 tick 间隔。
        if self.last_suppression_decay_us == 0 {
            self.last_suppression_decay_us = now_us;
            return;
        }
        let elapsed = now_us.saturating_sub(self.last_suppression_decay_us);
        if elapsed == 0 {
            return;
        }
        let k = 0.5_f64.powf(elapsed as f64 / SUPPRESSION_HALF_LIFE_US);
        self.action_suppression *= k;
        self.last_suppression_decay_us = now_us;
    }

    /// vocal_efficacy 的纯时间半衰。无语义数值松弛——非惩罚、非 reward；
    /// efficacy 仍只由真实声学闭环沉积（observe_vocal_reflex_closure 的
    /// delta_eff），这里只补一条遗忘律，使它成为"近期真实发声能力的滑动
    /// 沉积"而非永久单调棘轮（打破钉死 1.0 的幻觉收敛）。τ 远大于单次闭环
    /// 间隔（持续真实拟合仍稳定爬升），但有限（长期无真实拟合时松弛）。
    /// 重启后 last_efficacy_decay_us=0（不持久化）被当作"尚未起表"：首拍用 now_us
    /// 起表、不衰减（续航）。逐行仿 decay_vocal_suppression。
    pub fn decay_vocal_efficacy(&mut self, now_us: u64) {
        // 重启即续航（同 decay_vocal_suppression）：last_efficacy_decay_us=0 表示
        // 重启后尚未起表，用 now_us 起表、本拍不衰减，避免 epoch µs 与 0 求差把
        // efficacy 一拍归零。诚实门不受影响：efficacy 仍只由真实声学闭环沉积，
        // 这里只是不再用"自纪元以来的墙钟"伪造一次巨额遗忘。稳态衰减不变。
        if self.last_efficacy_decay_us == 0 {
            self.last_efficacy_decay_us = now_us;
            return;
        }
        let elapsed = now_us.saturating_sub(self.last_efficacy_decay_us);
        if elapsed == 0 {
            return;
        }
        let k = 0.5_f64.powf(elapsed as f64 / EFFICACY_HALF_LIFE_US);
        self.vocal_efficacy *= k;
        self.last_efficacy_decay_us = now_us;
    }

    /// prediction_familiarity 的纯时间半衰（τ = FAMILIARITY_HALF_LIFE_US = 2 min）。
    /// 与 FAMILIARITY_ADVANCE_MIN_INTERVAL_US（1s 积累限速）配合，使平衡点在
    /// 密集 nursery 下落在 ~0.5–0.7，让 exploration_drive 维持非零 babble 基线。
    /// 逐行仿 decay_vocal_suppression；不持久化时戳设计同款。
    pub fn decay_prediction_familiarity(&mut self, now_us: u64) {
        // 重启即续航（同 decay_vocal_suppression）：last_familiarity_decay_us=0 表示
        // 重启后尚未起表，用 now_us 起表、本拍不衰减，避免 epoch µs 与 0 求差把
        // familiarity 一拍归零。稳态衰减不变；与限速积累配合的平衡点（~0.5–0.7）
        // 在续航后维持，不再因重启被错误重置到 0。
        if self.last_familiarity_decay_us == 0 {
            self.last_familiarity_decay_us = now_us;
            return;
        }
        let elapsed = now_us.saturating_sub(self.last_familiarity_decay_us);
        if elapsed == 0 {
            return;
        }
        let k = 0.5_f64.powf(elapsed as f64 / FAMILIARITY_HALF_LIFE_US);
        self.prediction_familiarity *= k;
        self.last_familiarity_decay_us = now_us;
    }
}


fn update_stability(current: f64, support: f64, deviation: f64) -> f64 {
    (current + support * 0.1 - deviation * 0.15).clamp(0.0, 1.0)
}

fn evict_front_to_cap<T>(items: &mut Vec<T>, cap: usize) {
    if items.len() > cap {
        let n = items.len() - cap;
        items.drain(0..n);
    }
}

pub fn bias_axis_is_protected(axis: BiasAxis) -> bool {
    matches!(axis, BiasAxis::PressureMultiplier)
}

pub fn write_learned_bias_checkpoint(
    state: &LearnedBiasState,
    paths: &LearnedBiasCheckpointPaths,
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    write_u64(&mut buf, LEARNED_BIAS_MAGIC);
    write_u64(&mut buf, LEARNED_BIAS_VERSION);
    write_u32(&mut buf, state.curves.len() as u32);
    for curve in state.curves.iter() {
        write_curve(&mut buf, curve);
    }
    write_u32(&mut buf, state.recent_events.len() as u32);
    for ev in state.recent_events.iter() {
        write_event(&mut buf, ev);
    }
    write_u64(&mut buf, state.total_events);
    write_u64(&mut buf, state.last_update_t_us);
    // v4: vocal biases now actually persist (死后保留). Before v4 these were
    // dropped on save and reset to 0 on load — a refactor gap, fixed here.
    write_f64(&mut buf, state.prediction_familiarity);
    write_f64(&mut buf, state.action_suppression);
    write_f64(&mut buf, state.vocal_efficacy);
    // v5: developmental clock persists（跨重启继承; A3）.
    write_f64(&mut buf, state.developmental_maturity);

    {
        let mut f = File::create(&paths.tmp_path)?;
        f.write_all(&buf)?;
        f.sync_data()?;
    }
    std::fs::rename(&paths.tmp_path, &paths.final_path)?;
    Ok(())
}

pub fn read_learned_bias_checkpoint(path: &Path) -> std::io::Result<LearnedBiasState> {
    let bytes = std::fs::read(path)?;
    let mut cur = 0usize;
    let magic = read_u64(&bytes, &mut cur)?;
    if magic != LEARNED_BIAS_MAGIC {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let version = read_u64(&bytes, &mut cur)?;
    if version != LEARNED_BIAS_VERSION {
        return Err(Error::from(ErrorKind::InvalidData));
    }

    let n_curves = read_u32(&bytes, &mut cur)? as usize;
    if n_curves > LEARNED_BIAS_CURVE_CAP {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let mut curves = Vec::with_capacity(n_curves);
    for _ in 0..n_curves {
        curves.push(read_curve(&bytes, &mut cur)?);
    }

    let n_events = read_u32(&bytes, &mut cur)? as usize;
    if n_events > LEARNED_BIAS_EVENT_CAP {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let mut recent_events = Vec::with_capacity(n_events);
    for _ in 0..n_events {
        recent_events.push(read_event(&bytes, &mut cur)?);
    }

    let total_events = read_u64(&bytes, &mut cur)?;
    let last_update_t_us = read_u64(&bytes, &mut cur)?;
    // v4: vocal biases persist. (version gate already enforced == 4 above.)
    let prediction_familiarity = read_f64(&bytes, &mut cur)?;
    let action_suppression = read_f64(&bytes, &mut cur)?;
    let vocal_efficacy = read_f64(&bytes, &mut cur)?;
    // v5: developmental clock persists. (version gate already enforced == 5.)
    let developmental_maturity = read_f64(&bytes, &mut cur)?;

    Ok(LearnedBiasState {
        n_curves: curves.len() as u32,
        n_recent_events: recent_events.len() as u32,
        total_events,
        curves,
        recent_events,
        last_update_t_us,
        prediction_familiarity,
        action_suppression,
        vocal_efficacy,
        developmental_maturity,
        // B2: 刻意不持久化。重启后从 0 起 → 被首拍 decay 当作"尚未起表"：用
        // now_us 起表、不衰减（续航）。此后只按真实流逝时间松弛，不再用 epoch µs
        // 与 0 求差伪造一次归零（否则 maturity 续航而底层能力被凭空清零）。
        last_suppression_decay_us: 0,
        // 同款不持久化：重启后从 0 起 → 首拍起表、不衰减（续航），此后按真实
        // 流逝时间松弛。
        last_efficacy_decay_us: 0,
        // 同款不持久化：重启后从 0 → 首拍起表、不衰减（续航）+ 积累限速重置，
        // 此后按真实流逝时间松弛。
        last_familiarity_decay_us: 0,
        last_familiarity_advance_us: 0,
    })
}

fn write_curve(buf: &mut Vec<u8>, curve: &LearnedBiasCurve) {
    write_u64(buf, axis_to_u64(curve.axis));
    write_locator(buf, curve.locator);
    write_f64(buf, curve.delta_value);
    write_f64(buf, curve.delta_ema);
    write_f64(buf, curve.stability);
    write_u32(buf, curve.support_count);
    write_u32(buf, curve.deviation_count);
    write_u64(buf, curve.last_update_t_us);
}

fn read_curve(bytes: &[u8], cur: &mut usize) -> std::io::Result<LearnedBiasCurve> {
    Ok(LearnedBiasCurve {
        axis: axis_from_u64(read_u64(bytes, cur)?)?,
        locator: read_locator(bytes, cur)?,
        delta_value: read_f64(bytes, cur)?,
        delta_ema: read_f64(bytes, cur)?,
        stability: read_f64(bytes, cur)?,
        support_count: read_u32(bytes, cur)?,
        deviation_count: read_u32(bytes, cur)?,
        last_update_t_us: read_u64(bytes, cur)?,
    })
}

fn write_event(buf: &mut Vec<u8>, ev: &LearnedBiasEvent) {
    write_u64(buf, axis_to_u64(ev.axis));
    write_locator(buf, ev.locator);
    write_f64(buf, ev.requested_delta);
    write_f64(buf, ev.applied_delta);
    write_f64(buf, ev.support);
    write_f64(buf, ev.deviation);
    write_u64(buf, ev.evidence_ref);
    write_u64(buf, ev.event_t_us);
}

fn read_event(bytes: &[u8], cur: &mut usize) -> std::io::Result<LearnedBiasEvent> {
    Ok(LearnedBiasEvent {
        axis: axis_from_u64(read_u64(bytes, cur)?)?,
        locator: read_locator(bytes, cur)?,
        requested_delta: read_f64(bytes, cur)?,
        applied_delta: read_f64(bytes, cur)?,
        support: read_f64(bytes, cur)?,
        deviation: read_f64(bytes, cur)?,
        evidence_ref: read_u64(bytes, cur)?,
        event_t_us: read_u64(bytes, cur)?,
    })
}

fn write_locator(buf: &mut Vec<u8>, locator: BiasLocator) {
    match locator {
        BiasLocator::Global => {
            write_u64(buf, 0);
            write_u64(buf, 0);
        }
        BiasLocator::Band(v) => {
            write_u64(buf, 1);
            write_u64(buf, v as u64);
        }
        BiasLocator::Slot(v) => {
            write_u64(buf, 2);
            write_u64(buf, v as u64);
        }
        BiasLocator::Edge(v) => {
            write_u64(buf, 3);
            write_u64(buf, v as u64);
        }
        BiasLocator::Channel(v) => {
            write_u64(buf, 4);
            write_u64(buf, v as u64);
        }
    }
}

fn read_locator(bytes: &[u8], cur: &mut usize) -> std::io::Result<BiasLocator> {
    let kind = read_u64(bytes, cur)?;
    let value = read_u64(bytes, cur)?;
    let value_u16 = u16::try_from(value).map_err(|_| Error::from(ErrorKind::InvalidData))?;
    match kind {
        0 => Ok(BiasLocator::Global),
        1 => Ok(BiasLocator::Band(value_u16)),
        2 => Ok(BiasLocator::Slot(value_u16)),
        3 => Ok(BiasLocator::Edge(value_u16)),
        4 => Ok(BiasLocator::Channel(value_u16)),
        _ => Err(Error::from(ErrorKind::InvalidData)),
    }
}

fn axis_to_u64(axis: BiasAxis) -> u64 {
    match axis {
        BiasAxis::ErrGain => 0,
        BiasAxis::RecurrenceGain => 1,
        BiasAxis::AmbiguityGain => 2,
        BiasAxis::AlignmentGain => 3,
        BiasAxis::SupportGain => 4,
        BiasAxis::DeviationGain => 5,
        BiasAxis::TemporalGapGain => 6,
        BiasAxis::CrossChannelGain => 7,
        BiasAxis::PressureMultiplier => 8,
    }
}

fn axis_from_u64(v: u64) -> std::io::Result<BiasAxis> {
    match v {
        0 => Ok(BiasAxis::ErrGain),
        1 => Ok(BiasAxis::RecurrenceGain),
        2 => Ok(BiasAxis::AmbiguityGain),
        3 => Ok(BiasAxis::AlignmentGain),
        4 => Ok(BiasAxis::SupportGain),
        5 => Ok(BiasAxis::DeviationGain),
        6 => Ok(BiasAxis::TemporalGapGain),
        7 => Ok(BiasAxis::CrossChannelGain),
        8 => Ok(BiasAxis::PressureMultiplier),
        _ => Err(Error::from(ErrorKind::InvalidData)),
    }
}

fn write_u64(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_f64(buf: &mut Vec<u8>, value: f64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn read_u64(bytes: &[u8], cur: &mut usize) -> std::io::Result<u64> {
    let raw = read_exact::<8>(bytes, cur)?;
    Ok(u64::from_le_bytes(raw))
}

fn read_u32(bytes: &[u8], cur: &mut usize) -> std::io::Result<u32> {
    let raw = read_exact::<4>(bytes, cur)?;
    Ok(u32::from_le_bytes(raw))
}

fn read_f64(bytes: &[u8], cur: &mut usize) -> std::io::Result<f64> {
    let raw = read_exact::<8>(bytes, cur)?;
    Ok(f64::from_le_bytes(raw))
}

fn read_exact<const N: usize>(bytes: &[u8], cur: &mut usize) -> std::io::Result<[u8; N]> {
    if bytes.len().saturating_sub(*cur) < N {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes[*cur..*cur + N]);
    *cur += N;
    Ok(out)
}
