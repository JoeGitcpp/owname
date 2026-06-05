use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{VIB_BANDS, VIB_BAND_CUTOFFS};

/// Splits audio into VIB_BANDS energy bands using 1st-order IIR LP filters.
/// Sends [f64; VIB_BANDS] RMS measurements at ~100 Hz (10ms windows).
pub struct VibSource {
    rx: Receiver<[f64; VIB_BANDS]>,
    _stream: cpal::Stream,
}

impl VibSource {
    pub fn try_new(
        fault: Arc<AtomicBool>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (rx, stream) = Self::try_new_parts(fault)?;
        Ok(VibSource {
            rx,
            _stream: stream,
        })
    }

    /// Build the mic pipeline and return `(receiver, stream)` so the caller
    /// owns both pieces directly. The stream must be kept alive for samples
    /// to keep flowing; dropping it stops the mic.
    pub fn try_new_parts(
        fault: Arc<AtomicBool>,
    ) -> Result<(Receiver<[f64; VIB_BANDS]>, cpal::Stream), Box<dyn std::error::Error + Send + Sync>>
    {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("no default audio input device")?;
        let supported = device.default_input_config()?;

        let sample_rate = supported.sample_rate().0 as usize;
        let channels = supported.channels() as usize;
        let mono_window = (sample_rate / 100).max(1);

        let (tx, rx) = channel::<[f64; VIB_BANDS]>();
        let fmt = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let stream = make_stream(
            &device,
            &config,
            fmt,
            tx,
            fault,
            channels,
            sample_rate,
            mono_window,
        )?;
        stream.play()?;
        Ok((rx, stream))
    }

    pub fn try_recv(&self) -> Option<[f64; VIB_BANDS]> {
        self.rx.try_recv().ok()
    }
}

// ---------------------------------------------------------------------------
// IIR 1st-order LP filter bank
// ---------------------------------------------------------------------------

// VIB_BANDS bands are produced by a cascade of (VIB_BANDS - 1) 1st-order IIR
// low-pass filters whose cutoffs are VIB_BAND_CUTOFFS. Band 0 is the lowest LP
// output, each interior band is the difference between successive LP outputs,
// and the top band is the residual high-pass (x − last LP output).
const VIB_LP_STAGES: usize = VIB_BANDS - 1;

struct BandFilter {
    alpha: [f64; VIB_LP_STAGES],
    lp: [f64; VIB_LP_STAGES],
    sum_sq: [f64; VIB_BANDS],
    cnt: usize,
    mono_window: usize,
}

impl BandFilter {
    fn new(sample_rate: usize, mono_window: usize) -> Self {
        let sr = sample_rate as f64;
        let pi2 = 2.0 * std::f64::consts::PI;
        let alpha = VIB_BAND_CUTOFFS.map(|fc| {
            let rc = 1.0 / (pi2 * fc);
            let dt = 1.0 / sr;
            dt / (rc + dt)
        });
        Self {
            alpha,
            lp: [0.0; VIB_LP_STAGES],
            sum_sq: [0.0; VIB_BANDS],
            cnt: 0,
            mono_window,
        }
    }

    fn push(&mut self, x: f64) -> Option<[f64; VIB_BANDS]> {
        // Advance each LP stage.
        for i in 0..VIB_LP_STAGES {
            let a = self.alpha[i];
            self.lp[i] = a * x + (1.0 - a) * self.lp[i];
        }

        // Band signals: successive differences of LP outputs, plus the
        // low-pass floor (band 0) and high-pass residual (top band).
        self.sum_sq[0] += self.lp[0] * self.lp[0];
        for i in 1..VIB_LP_STAGES {
            let b = self.lp[i] - self.lp[i - 1];
            self.sum_sq[i] += b * b;
        }
        let top = x - self.lp[VIB_LP_STAGES - 1];
        self.sum_sq[VIB_BANDS - 1] += top * top;
        self.cnt += 1;

        if self.cnt >= self.mono_window {
            let n = self.cnt as f64;
            let out = self.sum_sq.map(|s| (s / n).sqrt());
            self.sum_sq = [0.0; VIB_BANDS];
            self.cnt = 0;
            Some(out)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Stream construction for each cpal sample format
// ---------------------------------------------------------------------------

fn make_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    fmt: cpal::SampleFormat,
    tx: std::sync::mpsc::Sender<[f64; VIB_BANDS]>,
    fault: Arc<AtomicBool>,
    channels: usize,
    sample_rate: usize,
    mono_window: usize,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>> {
    match fmt {
        cpal::SampleFormat::F32 => {
            let mut filter = BandFilter::new(sample_rate, mono_window);
            let mut ch_idx = 0usize;
            let fault_cb = fault.clone();
            Ok(device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for &s in data {
                        if ch_idx == 0 {
                            if let Some(bands) = filter.push(s as f64) {
                                let _ = tx.send(bands);
                            }
                        }
                        ch_idx = (ch_idx + 1) % channels;
                    }
                },
                move |_: cpal::StreamError| {
                    fault_cb.store(true, Ordering::Relaxed);
                },
                None,
            )?)
        }
        cpal::SampleFormat::I16 => {
            let mut filter = BandFilter::new(sample_rate, mono_window);
            let mut ch_idx = 0usize;
            let fault_cb = fault.clone();
            Ok(device.build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    for &s in data {
                        if ch_idx == 0 {
                            if let Some(bands) = filter.push(s as f64 / 32768.0) {
                                let _ = tx.send(bands);
                            }
                        }
                        ch_idx = (ch_idx + 1) % channels;
                    }
                },
                move |_: cpal::StreamError| {
                    fault_cb.store(true, Ordering::Relaxed);
                },
                None,
            )?)
        }
        cpal::SampleFormat::U16 => {
            let mut filter = BandFilter::new(sample_rate, mono_window);
            let mut ch_idx = 0usize;
            let fault_cb = fault.clone();
            Ok(device.build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    for &s in data {
                        if ch_idx == 0 {
                            if let Some(bands) = filter.push((s as f64 - 32768.0) / 32768.0) {
                                let _ = tx.send(bands);
                            }
                        }
                        ch_idx = (ch_idx + 1) % channels;
                    }
                },
                move |_: cpal::StreamError| {
                    fault_cb.store(true, Ordering::Relaxed);
                },
                None,
            )?)
        }
        _ => Err(Box::new(std::io::Error::from(
            std::io::ErrorKind::Unsupported,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_filter_emits_only_after_configured_window_and_resets() {
        let mut filter = BandFilter::new(100, 3);

        assert!(filter.push(0.0).is_none());
        assert!(filter.push(0.0).is_none());

        let out = filter.push(0.0);
        assert!(out.is_some());
        let out = out.unwrap();
        assert!(out.iter().all(|band| band.abs() < 1e-12));

        assert!(
            filter.push(0.0).is_none(),
            "filter must reset its sample count after emitting a frame"
        );
    }

    #[test]
    fn band_filter_outputs_finite_non_negative_band_energy() {
        let mut filter = BandFilter::new(48_000, 4);

        assert!(filter.push(1.0).is_none());
        assert!(filter.push(-1.0).is_none());
        assert!(filter.push(0.5).is_none());
        let out = filter.push(-0.25);
        assert!(out.is_some());
        let out = out.unwrap();

        assert_eq!(out.len(), VIB_BANDS);
        assert!(
            out.iter().all(|band| band.is_finite() && *band >= 0.0),
            "RMS bands must stay finite and non-negative: {:?}",
            out
        );
        assert!(
            out.iter().any(|band| *band > 0.0),
            "non-zero waveform must produce at least one non-zero band"
        );
    }
}
