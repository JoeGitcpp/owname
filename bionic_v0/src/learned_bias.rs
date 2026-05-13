use crate::evolution_priors::{BiasAxis, BiasPriors};
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

pub const LEARNED_BIAS_MAGIC: u64 = 0xB10B10_0000_0042;
pub const LEARNED_BIAS_VERSION: u64 = 3;
pub const LEARNED_BIAS_CURVE_CAP: usize = 128;
pub const LEARNED_BIAS_EVENT_CAP: usize = 256;

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
        }
    }
}

fn unit(x: f64) -> f64 {
    if !x.is_finite() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
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

    Ok(LearnedBiasState {
        n_curves: curves.len() as u32,
        n_recent_events: recent_events.len() as u32,
        total_events,
        curves,
        recent_events,
        last_update_t_us,
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
