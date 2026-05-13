// Framed-byte egress substrate.
//
// Body-side digital disturbance write-out. This module records framed byte
// traces, action-output write handles, ordinary return windows, and numeric
// correction pressure. It does not define external carrier interpretation.

use crate::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use crate::viewport::Viewport;

const TRACE_CAP: usize = 16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FramedByteEgressPhase {
    #[default]
    Idle,
    Active,
    Decaying,
}

impl FramedByteEgressPhase {
    pub fn as_u8(self) -> u8 {
        match self {
            FramedByteEgressPhase::Idle => 0,
            FramedByteEgressPhase::Active => 1,
            FramedByteEgressPhase::Decaying => 2,
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => FramedByteEgressPhase::Active,
            2 => FramedByteEgressPhase::Decaying,
            _ => FramedByteEgressPhase::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramedByteFrameTrace {
    pub frame_seq: u64,
    pub stream_id: u64,
    pub write_seq: u64,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub byte_len: u32,
    pub byte_checksum: u64,
    pub pressure: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FramedByteReturnTrace {
    pub return_seq: u64,
    pub stream_id: u64,
    pub window_id: u64,
    pub return_t_us: u64,
    pub byte_len: u32,
    pub byte_checksum: u64,
    pub magnitude: f64,
    pub correction_deviation: f64,
    pub write_pressure: f64,
}

#[derive(Clone, Debug)]
pub struct FramedByteEgressState {
    pub phase: FramedByteEgressPhase,
    pub active_stream_id: u64,
    pub current_write_seq: u64,
    pub next_frame_seq: u64,
    pub next_return_seq: u64,
    pub byte_cursor: u64,
    pub output_pressure: f64,
    pub projection_confidence: f64,
    pub correction_deviation: f64,
    pub correction_count: u32,
    pub last_frame_seq: u64,
    pub last_return_window_id: u64,
    pub last_update_t_us: u64,
    pub n_frames: u32,
    pub n_returns: u32,
    pub total_frames: u64,
    pub total_returns: u64,
    pub frame_traces: Vec<FramedByteFrameTrace>,
    pub return_traces: Vec<FramedByteReturnTrace>,
}

impl Default for FramedByteEgressState {
    fn default() -> Self {
        Self {
            phase: FramedByteEgressPhase::Idle,
            active_stream_id: 0,
            current_write_seq: 0,
            next_frame_seq: 0,
            next_return_seq: 0,
            byte_cursor: 0,
            output_pressure: 0.0,
            projection_confidence: 0.5,
            correction_deviation: 0.0,
            correction_count: 0,
            last_frame_seq: 0,
            last_return_window_id: 0,
            last_update_t_us: 0,
            n_frames: 0,
            n_returns: 0,
            total_frames: 0,
            total_returns: 0,
            frame_traces: Vec::new(),
            return_traces: Vec::new(),
        }
    }
}

impl FramedByteEgressState {
    fn push_frame(
        &mut self,
        stream_id: u64,
        write_seq: u64,
        t_us: u64,
        bytes: &[u8],
        pressure: f64,
    ) -> FramedByteFrameTrace {
        self.next_frame_seq += 1;
        let trace = FramedByteFrameTrace {
            frame_seq: self.next_frame_seq,
            stream_id,
            write_seq,
            start_t_us: t_us,
            end_t_us: t_us,
            byte_len: bytes.len() as u32,
            byte_checksum: byte_checksum(bytes),
            pressure,
        };
        self.phase = FramedByteEgressPhase::Active;
        self.active_stream_id = stream_id;
        self.current_write_seq = write_seq;
        self.byte_cursor = self.byte_cursor.saturating_add(bytes.len() as u64);
        self.output_pressure = pressure;
        self.last_frame_seq = trace.frame_seq;
        self.last_update_t_us = t_us;
        self.frame_traces.push(trace);
        if self.frame_traces.len() > TRACE_CAP {
            let evict_n = self.frame_traces.len() - TRACE_CAP;
            self.frame_traces.drain(0..evict_n);
        }
        self.n_frames = self.frame_traces.len() as u32;
        self.total_frames = self.next_frame_seq;
        trace
    }

    fn push_return(
        &mut self,
        stream_id: u64,
        window_id: u64,
        t_us: u64,
        bytes: &[u8],
        magnitude: f64,
        deviation: f64,
    ) -> FramedByteReturnTrace {
        self.next_return_seq += 1;
        let deviation = deviation.clamp(0.0, 1.0);
        let magnitude = magnitude.clamp(0.0, 1.0);
        let trace = FramedByteReturnTrace {
            return_seq: self.next_return_seq,
            stream_id,
            window_id,
            return_t_us: t_us,
            byte_len: bytes.len() as u32,
            byte_checksum: byte_checksum(bytes),
            magnitude,
            correction_deviation: deviation,
            write_pressure: self.output_pressure,
        };
        let delta = (0.5 - deviation) * 0.2;
        self.projection_confidence = (self.projection_confidence + delta).clamp(0.0, 1.0);
        self.correction_deviation = deviation;
        self.correction_count = self.correction_count.saturating_add(1);
        self.last_return_window_id = window_id;
        self.last_update_t_us = t_us;
        self.return_traces.push(trace);
        if self.return_traces.len() > TRACE_CAP {
            let evict_n = self.return_traces.len() - TRACE_CAP;
            self.return_traces.drain(0..evict_n);
        }
        self.n_returns = self.return_traces.len() as u32;
        self.total_returns = self.next_return_seq;
        trace
    }
}

pub fn byte_checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |acc, b| {
        acc.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(*b)
    })
}

impl Viewport {
    pub fn read_framed_byte_egress(&self) -> FramedByteEgressState {
        self.framed_byte_egress.read().unwrap().clone()
    }

    pub fn begin_framed_byte_egress(
        &self,
        now_t_us: u64,
        stream_id: u64,
        bytes: &[u8],
        pressure: f64,
    ) -> FramedByteFrameTrace {
        let pressure = pressure.clamp(0.0, 1.0);
        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::FramedByteEgress,
            feedback: ActionOutputFeedback::Begin,
            magnitude: pressure,
            event_t_us: now_t_us,
        });
        let write_seq = self
            .read_action_output()
            .channels
            .iter()
            .find(|ch| ch.channel_kind == OutputChannelKind::FramedByteEgress)
            .map(|ch| ch.current_write_seq)
            .unwrap_or(0);
        let mut st = self.framed_byte_egress.write().unwrap();
        st.push_frame(stream_id, write_seq, now_t_us, bytes, pressure)
    }

    pub fn adjust_framed_byte_egress(
        &self,
        now_t_us: u64,
        bytes: &[u8],
        pressure: f64,
    ) -> FramedByteFrameTrace {
        let pressure = pressure.clamp(0.0, 1.0);
        let (stream_id, write_seq) = {
            let st = self.framed_byte_egress.read().unwrap();
            (st.active_stream_id, st.current_write_seq)
        };
        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::FramedByteEgress,
            feedback: ActionOutputFeedback::Adjust,
            magnitude: pressure,
            event_t_us: now_t_us,
        });
        let mut st = self.framed_byte_egress.write().unwrap();
        st.push_frame(stream_id, write_seq, now_t_us, bytes, pressure)
    }

    pub fn stop_framed_byte_egress(&self, now_t_us: u64) {
        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::FramedByteEgress,
            feedback: ActionOutputFeedback::Stop,
            magnitude: 0.0,
            event_t_us: now_t_us,
        });
        let mut st = self.framed_byte_egress.write().unwrap();
        st.phase = FramedByteEgressPhase::Idle;
        st.output_pressure = 0.0;
        st.last_update_t_us = now_t_us;
    }

    pub fn observe_framed_byte_return(
        &self,
        now_t_us: u64,
        stream_id: u64,
        bytes: &[u8],
        magnitude: f64,
        deviation: f64,
    ) -> u64 {
        let idx = self.begin_action_consequence_window(ActionConsequenceWindowEvent {
            modality_kind: ConsequenceModalityKind::FramedByteReturn,
            start_t_us: now_t_us,
            end_t_us: now_t_us,
            magnitude: magnitude.clamp(0.0, 1.0),
        });
        let window_id = self
            .read_action_consequence()
            .windows
            .get(idx as usize)
            .map(|w| w.window_id)
            .unwrap_or(0);
        self.try_auto_pair_latest_epoch_to_window(now_t_us);
        let mut st = self.framed_byte_egress.write().unwrap();
        st.push_return(stream_id, window_id, now_t_us, bytes, magnitude, deviation);
        window_id
    }
}
