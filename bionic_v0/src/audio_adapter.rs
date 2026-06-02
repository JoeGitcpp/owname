use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::audio_raw::AudioRawBuffer;
use crate::clock::now_us;
use crate::{AUDIO_RAW_BATCH_INTERVAL_US, AUDIO_RAW_BATCH_SAMPLES};

pub struct AudioAdapter {
    _stream: Option<cpal::Stream>,
    fault: Arc<AtomicBool>,
}

impl AudioAdapter {
    /// Create an AudioAdapter that feeds raw samples into the given buffer.
    /// Returns None if no input device is available.
    pub fn new(audio_buf: Arc<AudioRawBuffer>) -> Option<Self> {
        let fault = Arc::new(AtomicBool::new(false));
        let host = cpal::default_host();
        let device = host.default_input_device()?;
        let config = device.default_input_config().ok()?;
        let stream_config: cpal::StreamConfig = config.into();

        let buf = audio_buf.clone();
        let fault_clone = fault.clone();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Accumulate samples until we have a full batch
                    // For simplicity, just write directly (batch boundaries are approximate)
                    let now = now_us();
                    buf.write_batch(data, now);
                },
                move |_err| {
                    fault_clone.store(true, Ordering::Relaxed);
                },
                None,
            )
            .ok()?;

        stream.play().ok()?;

        Some(Self {
            _stream: Some(stream),
            fault,
        })
    }
}
