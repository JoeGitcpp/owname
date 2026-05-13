use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::clock::{now_us, spawn_ticker};
use crate::code_ingress_adapter::{CarrierFrame, CodeIngressAdapter, FramedByteCarrier};
use crate::viewport::Viewport;

pub const LIVE_BYTE_FRAME_LEN: usize = 8;
pub const LIVE_BYTE_STREAM_ID: u64 = 2;
pub const LIVE_BYTE_INGRESS_HZ: u32 = 20;

pub struct LiveByteFileCarrier {
    path: PathBuf,
    stream_context_id: u64,
    frame_len: usize,
    offset: u64,
    frames: VecDeque<Vec<u8>>,
}

impl LiveByteFileCarrier {
    pub fn new(path: PathBuf, stream_context_id: u64, frame_len: usize) -> Self {
        Self {
            path,
            stream_context_id,
            frame_len: frame_len.max(1),
            offset: 0,
            frames: VecDeque::new(),
        }
    }

    fn refill(&mut self) {
        let Ok(mut file) = File::open(&self.path) else {
            return;
        };
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }

        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_err() || bytes.is_empty() {
            return;
        }

        self.offset = self.offset.saturating_add(bytes.len() as u64);
        for chunk in bytes.chunks(self.frame_len) {
            self.frames.push_back(chunk.to_vec());
        }
    }
}

impl FramedByteCarrier for LiveByteFileCarrier {
    fn try_recv_frame(&mut self) -> Option<CarrierFrame> {
        if self.frames.is_empty() {
            self.refill();
        }

        self.frames.pop_front().map(|bytes| CarrierFrame {
            stream_context_id: self.stream_context_id,
            bytes,
            framing_validity: 1.0,
        })
    }
}

pub fn spawn_live_byte_ingress(
    vp: Arc<Viewport>,
    stop: Arc<AtomicBool>,
    path: PathBuf,
) -> JoinHandle<()> {
    let carrier = LiveByteFileCarrier::new(path, LIVE_BYTE_STREAM_ID, LIVE_BYTE_FRAME_LEN);
    let mut adapter = CodeIngressAdapter::new(carrier);
    spawn_ticker(LIVE_BYTE_INGRESS_HZ, stop, move |_| {
        adapter.tick(&vp, now_us());
    })
}
