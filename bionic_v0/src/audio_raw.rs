use crate::AUDIO_RAW_CAPACITY;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Ring buffer for raw audio waveform samples.
/// Stores 60 seconds of f32 samples at 48 kHz.
/// Writer: AudioAdapter (cpal mic callback).
/// Reader: future audio predictors, snapshot observer.
pub struct AudioRawBuffer {
    samples: RwLock<Vec<f32>>,
    write_pos: AtomicU64,
    total_samples: AtomicU64,
    last_sample_clock_us: AtomicU64,
}

impl AudioRawBuffer {
    pub fn new() -> Self {
        Self {
            samples: RwLock::new(vec![0.0_f32; AUDIO_RAW_CAPACITY]),
            write_pos: AtomicU64::new(0),
            total_samples: AtomicU64::new(0),
            last_sample_clock_us: AtomicU64::new(0),
        }
    }

    pub fn write_batch(&self, batch: &[f32], clock_us: u64) {
        let n = batch.len().min(AUDIO_RAW_CAPACITY);
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let mut samples = self.samples.write().unwrap();
        if pos + n <= AUDIO_RAW_CAPACITY {
            samples[pos..pos + n].copy_from_slice(&batch[..n]);
        } else {
            let first = AUDIO_RAW_CAPACITY - pos;
            samples[pos..].copy_from_slice(&batch[..first]);
            samples[..n - first].copy_from_slice(&batch[first..n]);
        }
        self.write_pos
            .store(((pos + n) % AUDIO_RAW_CAPACITY) as u64, Ordering::Relaxed);
        self.total_samples.fetch_add(n as u64, Ordering::Relaxed);
        self.last_sample_clock_us
            .store(clock_us, Ordering::Relaxed);
    }

    pub fn total_samples(&self) -> u64 {
        self.total_samples.load(Ordering::Relaxed)
    }

    pub fn last_sample_clock_us(&self) -> u64 {
        self.last_sample_clock_us.load(Ordering::Relaxed)
    }

    pub fn read_recent(&self, n: usize) -> Vec<f32> {
        let n = n.min(AUDIO_RAW_CAPACITY);
        let total = self.total_samples.load(Ordering::Relaxed) as usize;
        if total == 0 {
            return vec![0.0; n];
        }
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let start = if total < n {
            0
        } else {
            (pos + AUDIO_RAW_CAPACITY - n) % AUDIO_RAW_CAPACITY
        };
        let samples = self.samples.read().unwrap();
        if start + n <= AUDIO_RAW_CAPACITY {
            samples[start..start + n].to_vec()
        } else {
            let mut v = Vec::with_capacity(n);
            let first = AUDIO_RAW_CAPACITY - start;
            v.extend_from_slice(&samples[start..]);
            v.extend_from_slice(&samples[..n - first]);
            v
        }
    }
}
