use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::code_ingress::CodeIngressFrameTrace;
use crate::evolution_priors::BiasAxis;
use crate::learned_bias::BiasLocator;
use crate::viewport::Viewport;

pub const LIVE_BYTE_EGRESS_STREAM_ID: u64 = 3;
pub const LIVE_BYTE_EGRESS_FRAME_LEN: usize = 8;
const MIN_EGRESS_PRESSURE: f64 = 0.0001;
const REENTRY_GAIN_RECENT_US: u64 = 30_000_000;
const REENTRY_GAIN_FULL_IDLE_US: u64 = 300_000_000;
const REENTRY_GAIN_MIN_FRACTION: f64 = 0.25;
const HISTORICAL_CHANNEL_GAIN_CAP: f64 = 0.10;

#[derive(Debug)]
pub struct LiveByteClosureDriver {
    out_path: PathBuf,
    output_code_frame_cursor: u64,
    return_code_frame_cursor: u64,
    paired_egress_frame_cursor: u64,
}

impl LiveByteClosureDriver {
    pub fn new(out_path: PathBuf) -> Self {
        Self {
            out_path,
            output_code_frame_cursor: 0,
            return_code_frame_cursor: 0,
            paired_egress_frame_cursor: 0,
        }
    }

    pub fn tick(&mut self, vp: &Viewport, now_t_us: u64) -> bool {
        let paired = self.pair_later_code_frame(vp, now_t_us);
        let wrote = self.write_from_fresh_code_pressure(vp, now_t_us);
        paired || wrote
    }

    fn write_from_fresh_code_pressure(&mut self, vp: &Viewport, now_t_us: u64) -> bool {
        let Some(frame) = self.latest_fresh_code_frame(vp) else {
            return false;
        };
        let pressure = vp
            .read_evidence_tension()
            .unresolved_pressure
            .clamp(0.0, 1.0)
            .max(MIN_EGRESS_PRESSURE + 0.01); // always allow exploration
        let pressure = learned_output_pressure(vp, pressure, now_t_us);
        let bytes = derive_output_bytes(&frame, pressure);
        let Ok(mut out) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.out_path)
        else {
            return false;
        };

        vp.begin_framed_byte_egress(now_t_us, LIVE_BYTE_EGRESS_STREAM_ID, &bytes, pressure);
        if out.write_all(&bytes).is_err() {
            return false;
        }
        self.output_code_frame_cursor = frame.frame_seq;
        true
    }

    fn pair_later_code_frame(&mut self, vp: &Viewport, now_t_us: u64) -> bool {
        let egress = vp.read_framed_byte_egress();
        if egress.last_frame_seq == 0 || egress.last_frame_seq <= self.paired_egress_frame_cursor {
            return false;
        }

        let Some(frame) = self.first_later_code_frame(vp, egress.last_update_t_us) else {
            return false;
        };
        let bytes = frame_sample(&frame);
        if bytes.is_empty() {
            return false;
        }

        let deviation = (frame.byte_transition_energy - egress.output_pressure)
            .abs()
            .clamp(0.0, 1.0);
        let magnitude = frame.byte_transition_energy.clamp(0.0, 1.0);
        vp.observe_framed_byte_return(
            now_t_us,
            egress.active_stream_id,
            &bytes,
            magnitude,
            deviation,
        );
        self.return_code_frame_cursor = frame.frame_seq;
        self.paired_egress_frame_cursor = egress.last_frame_seq;
        true
    }

    fn latest_fresh_code_frame(&self, vp: &Viewport) -> Option<CodeIngressFrameTrace> {
        vp.read_code_ingress()
            .recent_frames
            .into_iter()
            .filter(|frame| frame.frame_seq > self.output_code_frame_cursor)
            .last()
    }

    fn first_later_code_frame(
        &self,
        vp: &Viewport,
        egress_t_us: u64,
    ) -> Option<CodeIngressFrameTrace> {
        vp.read_code_ingress()
            .recent_frames
            .into_iter()
            .filter(|frame| {
                frame.frame_seq > self.return_code_frame_cursor && frame.t_start_us > egress_t_us
            })
            .next()
    }
}

fn derive_output_bytes(frame: &CodeIngressFrameTrace, pressure: f64) -> Vec<u8> {
    let scaled_pressure = (pressure.clamp(0.0, 1.0) * 65_535.0) as u64;
    let mut seed = frame.byte_hash ^ frame.evidence_ref.rotate_left(11) ^ scaled_pressure;
    let sample = frame_sample(frame);
    let mut out = Vec::with_capacity(LIVE_BYTE_EGRESS_FRAME_LEN);

    for i in 0..LIVE_BYTE_EGRESS_FRAME_LEN {
        seed = seed
            .wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add((i as u64) + 1);
        let base = (seed >> ((i % 8) * 8)) as u8;
        let observed = sample.get(i % sample.len().max(1)).copied().unwrap_or(0);
        out.push(base.rotate_left(1) ^ observed ^ 0xa5);
    }

    out
}

fn learned_output_pressure(vp: &Viewport, pressure: f64, now_t_us: u64) -> f64 {
    let gain = learned_channel_gain(vp, now_t_us);
    (pressure * gain).clamp(0.0, 1.0)
}

fn learned_channel_gain(vp: &Viewport, now_t_us: u64) -> f64 {
    let state = vp.read_learned_bias();
    let locator = BiasLocator::Channel(LIVE_BYTE_EGRESS_STREAM_ID as u16);
    state
        .curves
        .iter()
        .find(|curve| curve.axis == BiasAxis::CrossChannelGain && curve.locator == locator)
        .map(|curve| {
            let idle_fraction = reentry_gain_fraction(now_t_us, curve.last_update_t_us);
            let transient_gain = curve.delta_ema * curve.stability;
            let historical_gain = curve.delta_value.tanh() * HISTORICAL_CHANNEL_GAIN_CAP;
            (1.0 + (transient_gain + historical_gain) * idle_fraction).clamp(0.25, 2.0)
        })
        .unwrap_or(1.0)
}

fn reentry_gain_fraction(now_t_us: u64, last_update_t_us: u64) -> f64 {
    let elapsed_us = now_t_us.saturating_sub(last_update_t_us);
    if elapsed_us <= REENTRY_GAIN_RECENT_US {
        return 1.0;
    }
    if elapsed_us >= REENTRY_GAIN_FULL_IDLE_US {
        return REENTRY_GAIN_MIN_FRACTION;
    }

    let span = (REENTRY_GAIN_FULL_IDLE_US - REENTRY_GAIN_RECENT_US) as f64;
    let age = (elapsed_us - REENTRY_GAIN_RECENT_US) as f64 / span;
    1.0 - age * (1.0 - REENTRY_GAIN_MIN_FRACTION)
}

fn frame_sample(frame: &CodeIngressFrameTrace) -> Vec<u8> {
    let len = usize::from(frame.byte_sample_len).min(frame.byte_sample.len());
    frame.byte_sample[..len].to_vec()
}
