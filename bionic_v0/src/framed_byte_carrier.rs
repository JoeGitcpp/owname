use std::collections::VecDeque;

use crate::framed_byte_egress::FramedByteFrameTrace;
use crate::viewport::Viewport;

#[derive(Clone, Debug, PartialEq)]
pub struct SandboxFramedByteReturn {
    pub stream_id: u64,
    pub bytes: Vec<u8>,
    pub magnitude: f64,
    pub deviation: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SandboxFramedByteCarrier {
    returns: VecDeque<SandboxFramedByteReturn>,
    pub accepted_frames: Vec<FramedByteFrameTrace>,
}

impl SandboxFramedByteCarrier {
    pub fn queue_return(&mut self, stream_id: u64, bytes: &[u8], magnitude: f64, deviation: f64) {
        self.returns.push_back(SandboxFramedByteReturn {
            stream_id,
            bytes: bytes.to_vec(),
            magnitude: magnitude.clamp(0.0, 1.0),
            deviation: deviation.clamp(0.0, 1.0),
        });
    }

    fn accept_frame(&mut self, frame: FramedByteFrameTrace) {
        self.accepted_frames.push(frame);
    }

    fn pop_return_for(&mut self, stream_id: u64) -> Option<SandboxFramedByteReturn> {
        let pos = self.returns.iter().position(|r| r.stream_id == stream_id)?;
        self.returns.remove(pos)
    }
}

pub fn tick_sandbox_framed_byte_loop(
    vp: &Viewport,
    carrier: &mut SandboxFramedByteCarrier,
    now_t_us: u64,
    stream_id: u64,
    bytes: &[u8],
    pressure: f64,
) {
    let frame = vp.begin_framed_byte_egress(now_t_us, stream_id, bytes, pressure);
    carrier.accept_frame(frame);

    if let Some(ret) = carrier.pop_return_for(stream_id) {
        vp.observe_framed_byte_return(
            now_t_us.saturating_add(1),
            ret.stream_id,
            &ret.bytes,
            ret.magnitude,
            ret.deviation,
        );
    }
}
