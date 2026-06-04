// Derived extraction buffer — NOT an independent object representation.
// This module computes numeric traces (transition energy, distribution skew,
// sequence recurrence) from raw byte frames.  Those traces are the SOURCE
// from which viewport::observe_code_ingress_frame projects onto the five
// tier-0 CODE axes (CODE_POS / CODE_SPAN / CODE_STRUCT_ORDER_POS /
// CODE_STRUCT_ORDER_ORIENT / CODE_STRUCT_ORDER_PERIOD).  The CODE-axis stream
// is the single authoritative representation of digital objects.
// CodeIngressState must not be consulted as a substitute for those axes.
use crate::{
    CODE_INGRESS_BYTE_SAMPLE_CAP, CODE_INGRESS_FRAME_HISTORY_CAP,
    CODE_INGRESS_PROJECTION_HISTORY_CAP, CODE_INGRESS_SEQUENCE_WINDOW_CAP,
};

pub const CODE_DECODER_BYTE_UNIT: u8 = 0;
pub const CODE_DECODER_BINARY_FRAGMENT: u8 = 1;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeIngressState {
    pub stream_count: u32,
    pub recent_frames: Vec<CodeIngressFrameTrace>,
    pub projection_candidates: Vec<CodeProjectionCandidate>,
    pub sequence_windows: Vec<CodeSequenceWindowTrace>,
    pub last_update_t_us: u64,
    next_frame_seq: u64,
    next_projection_ref: u64,
    next_window_id: u64,
}

pub struct CodeIngressFrameInput<'a> {
    pub stream_context_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub bytes: &'a [u8],
    pub framing_validity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeIngressFrameTrace {
    pub frame_seq: u64,
    pub stream_context_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub byte_len: u32,
    pub byte_hash: u64,
    pub byte_transition_energy: f64,
    pub byte_distribution_skew: f64,
    pub framing_validity: f64,
    pub evidence_ref: u64,
    pub byte_sample: [u8; CODE_INGRESS_BYTE_SAMPLE_CAP],
    pub byte_sample_len: u8,
}

impl Default for CodeIngressFrameTrace {
    fn default() -> Self {
        Self {
            frame_seq: 0,
            stream_context_id: 0,
            t_start_us: 0,
            t_end_us: 0,
            byte_len: 0,
            byte_hash: 0,
            byte_transition_energy: 0.0,
            byte_distribution_skew: 0.0,
            framing_validity: 0.0,
            evidence_ref: 0,
            byte_sample: [0; CODE_INGRESS_BYTE_SAMPLE_CAP],
            byte_sample_len: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeProjectionCandidate {
    pub projection_ref: u64,
    pub frame_seq: u64,
    pub decoder_kind_code: u8,
    pub confidence: f64,
    pub validity: f64,
    pub error_magnitude: f64,
    pub code_unit_count: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeSequenceWindowTrace {
    pub window_id: u64,
    pub stream_context_id: u64,
    pub start_frame_seq: u64,
    pub end_frame_seq: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub recurrence_score: f64,
    pub boundary_confidence: f64,
    pub ambiguity_level: f64,
}

impl CodeIngressState {
    pub fn from_snapshot(
        stream_count: u32,
        recent_frames: Vec<CodeIngressFrameTrace>,
        projection_candidates: Vec<CodeProjectionCandidate>,
        sequence_windows: Vec<CodeSequenceWindowTrace>,
        last_update_t_us: u64,
    ) -> Self {
        let next_frame_seq = recent_frames
            .iter()
            .map(|frame| frame.frame_seq)
            .max()
            .unwrap_or(0);
        let next_projection_ref = projection_candidates
            .iter()
            .map(|projection| projection.projection_ref)
            .max()
            .unwrap_or(0);
        let next_window_id = sequence_windows
            .iter()
            .map(|window| window.window_id)
            .max()
            .unwrap_or(0);
        Self {
            stream_count,
            recent_frames,
            projection_candidates,
            sequence_windows,
            last_update_t_us,
            next_frame_seq,
            next_projection_ref,
            next_window_id,
        }
    }

    pub fn observe_frame(&mut self, input: CodeIngressFrameInput<'_>) {
        self.next_frame_seq = self.next_frame_seq.saturating_add(1);
        let frame_seq = self.next_frame_seq;
        let validity = clamp01(input.framing_validity);
        let byte_hash = hash_bytes(input.bytes);
        let evidence_ref = nonzero_ref(byte_hash ^ frame_seq.rotate_left(17), frame_seq);
        let mut byte_sample = [0u8; CODE_INGRESS_BYTE_SAMPLE_CAP];
        let sample_len = input.bytes.len().min(CODE_INGRESS_BYTE_SAMPLE_CAP);
        byte_sample[..sample_len].copy_from_slice(&input.bytes[..sample_len]);

        let trace = CodeIngressFrameTrace {
            frame_seq,
            stream_context_id: input.stream_context_id,
            t_start_us: input.t_start_us,
            t_end_us: input.t_end_us,
            byte_len: input.bytes.len().min(u32::MAX as usize) as u32,
            byte_hash,
            byte_transition_energy: transition_energy(input.bytes),
            byte_distribution_skew: distribution_skew(input.bytes),
            framing_validity: validity,
            evidence_ref,
            byte_sample,
            byte_sample_len: sample_len as u8,
        };

        push_bounded(
            &mut self.recent_frames,
            trace,
            CODE_INGRESS_FRAME_HISTORY_CAP,
        );
        self.observe_projection(
            frame_seq,
            CODE_DECODER_BYTE_UNIT,
            validity,
            input.bytes.len() as u32,
            input.t_end_us,
        );
        self.observe_projection(
            frame_seq,
            CODE_DECODER_BINARY_FRAGMENT,
            1.0,
            input.bytes.len() as u32,
            input.t_end_us,
        );
        self.observe_sequence_window(&input, frame_seq, validity);
        self.stream_count = count_streams(&self.recent_frames);
        self.last_update_t_us = input.t_end_us;
    }

    fn observe_projection(
        &mut self,
        frame_seq: u64,
        decoder_kind_code: u8,
        validity: f64,
        code_unit_count: u32,
        last_update_t_us: u64,
    ) {
        self.next_projection_ref = self.next_projection_ref.saturating_add(1);
        let confidence = clamp01(validity);
        push_bounded(
            &mut self.projection_candidates,
            CodeProjectionCandidate {
                projection_ref: self.next_projection_ref,
                frame_seq,
                decoder_kind_code,
                confidence,
                validity: confidence,
                error_magnitude: 1.0 - confidence,
                code_unit_count,
                last_update_t_us,
            },
            CODE_INGRESS_PROJECTION_HISTORY_CAP,
        );
    }

    fn observe_sequence_window(
        &mut self,
        input: &CodeIngressFrameInput<'_>,
        frame_seq: u64,
        validity: f64,
    ) {
        if let Some(last) = self.sequence_windows.last_mut() {
            if last.stream_context_id == input.stream_context_id {
                last.end_frame_seq = frame_seq;
                last.t_end_us = input.t_end_us;
                last.recurrence_score = clamp01(last.recurrence_score + 0.1);
                last.boundary_confidence = clamp01((last.boundary_confidence + validity) * 0.5);
                last.ambiguity_level = clamp01(1.0 - last.boundary_confidence);
                return;
            }
        }

        self.next_window_id = self.next_window_id.saturating_add(1);
        push_bounded(
            &mut self.sequence_windows,
            CodeSequenceWindowTrace {
                window_id: self.next_window_id,
                stream_context_id: input.stream_context_id,
                start_frame_seq: frame_seq,
                end_frame_seq: frame_seq,
                t_start_us: input.t_start_us,
                t_end_us: input.t_end_us,
                recurrence_score: 0.0,
                boundary_confidence: validity,
                ambiguity_level: 1.0 - validity,
            },
            CODE_INGRESS_SEQUENCE_WINDOW_CAP,
        );
    }
}

fn push_bounded<T>(items: &mut Vec<T>, item: T, cap: usize) {
    if cap == 0 {
        return;
    }
    if items.len() == cap {
        items.remove(0);
    }
    items.push(item);
}

fn clamp01(value: f64) -> f64 {
    if !value.is_finite() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn transition_energy(bytes: &[u8]) -> f64 {
    if bytes.len() < 2 {
        return 0.0;
    }
    let total: f64 = bytes
        .windows(2)
        .map(|w| (w[1] as f64 - w[0] as f64).abs() / 255.0)
        .sum();
    clamp01(total / (bytes.len() - 1) as f64)
}

fn distribution_skew(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mean = bytes.iter().map(|b| *b as f64).sum::<f64>() / bytes.len() as f64;
    let low = mean / 255.0;
    clamp01((low - 0.5).abs() * 2.0)
}

fn count_streams(frames: &[CodeIngressFrameTrace]) -> u32 {
    let mut streams = Vec::new();
    for frame in frames {
        if !streams.contains(&frame.stream_context_id) {
            streams.push(frame.stream_context_id);
        }
    }
    streams.len().min(u32::MAX as usize) as u32
}

fn nonzero_ref(candidate: u64, fallback: u64) -> u64 {
    if candidate == 0 {
        fallback.max(1)
    } else {
        candidate
    }
}
