use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct LoopbackFileBridge {
    out_path: PathBuf,
    in_path: PathBuf,
    frame_len: usize,
    offset: u64,
    pending: VecDeque<Vec<u8>>,
}

impl LoopbackFileBridge {
    pub fn new(out_path: PathBuf, in_path: PathBuf, frame_len: usize) -> Self {
        Self {
            out_path,
            in_path,
            frame_len: frame_len.max(1),
            offset: 0,
            pending: VecDeque::new(),
        }
    }

    pub fn tick(&mut self) -> bool {
        self.refill();
        let Some(bytes) = self.pending.pop_front() else {
            return false;
        };
        let ret = transform_bytes(&bytes);
        let Ok(mut input) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.in_path)
        else {
            return false;
        };
        input.write_all(&ret).is_ok()
    }

    fn refill(&mut self) {
        let Ok(mut file) = File::open(&self.out_path) else {
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
            self.pending.push_back(chunk.to_vec());
        }
    }
}

fn transform_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .enumerate()
        .map(|(i, byte)| byte.rotate_left(1) ^ (0x5a_u8.wrapping_add(i as u8)))
        .collect()
}
