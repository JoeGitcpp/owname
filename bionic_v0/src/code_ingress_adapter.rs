use std::collections::VecDeque;
use std::sync::Arc;

use crate::code_ingress::CodeIngressFrameInput;
use crate::viewport::Viewport;

pub struct CarrierFrame {
    pub stream_context_id: u64,
    pub bytes: Vec<u8>,
    pub framing_validity: f64,
}

pub trait FramedByteCarrier {
    fn try_recv_frame(&mut self) -> Option<CarrierFrame>;
}

pub struct CodeIngressAdapter<C> {
    carrier: C,
}

impl<C: FramedByteCarrier> CodeIngressAdapter<C> {
    pub fn new(carrier: C) -> Self {
        Self { carrier }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        while let Some(frame) = self.carrier.try_recv_frame() {
            vp.observe_code_ingress_frame(CodeIngressFrameInput {
                stream_context_id: frame.stream_context_id,
                t_start_us: now_us,
                t_end_us: now_us,
                bytes: &frame.bytes,
                framing_validity: frame.framing_validity,
            });
        }
    }
}

pub struct ScriptedCodeIngressCarrier {
    stream_context_id: u64,
    frames: VecDeque<Vec<u8>>,
}

impl ScriptedCodeIngressCarrier {
    pub fn new(stream_context_id: u64, frames: Vec<Vec<u8>>) -> Self {
        Self {
            stream_context_id,
            frames: frames.into(),
        }
    }
}

impl FramedByteCarrier for ScriptedCodeIngressCarrier {
    fn try_recv_frame(&mut self) -> Option<CarrierFrame> {
        self.frames.pop_front().map(|bytes| CarrierFrame {
            stream_context_id: self.stream_context_id,
            bytes,
            framing_validity: 1.0,
        })
    }
}
