use crate::viewport::Viewport;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub fn sample_vocal_waveform(vp: &Arc<Viewport>, n_samples: usize) -> Vec<f64> {
    let organ = vp.read_vocal_organ();
    let mut samples = Vec::with_capacity(n_samples);

    if organ.breath_drive < 0.01 {
        for _ in 0..n_samples {
            samples.push(0.0);
        }
        return samples;
    }

    // Infant voice range: 280–700 Hz (tension=0.35 → ~427 Hz, newborn-like)
    let pitch = 280.0 + organ.pitch_tension * 420.0;
    let resonance = organ.resonance_position * 3.0;

    for i in 0..n_samples {
        let t = i as f64 / n_samples as f64;
        let fundamental = (t * pitch * 2.0 * std::f64::consts::PI).sin();
        let harmonic1 = (t * pitch * 4.0 * std::f64::consts::PI).sin() * 0.3;
        let harmonic2 = (t * pitch * 6.0 * std::f64::consts::PI).sin() * 0.1 * resonance;
        let sample = organ.breath_drive * (fundamental + harmonic1 + harmonic2);
        samples.push(sample.clamp(-1.0, 1.0));
    }

    samples
}

pub fn spawn_vocal_output(vp: Arc<Viewport>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
            return;
        };
        let Ok(output_config) = device.default_output_config() else {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
            return;
        };
        let stream_config: StreamConfig = output_config.config();
        let channels = usize::from(stream_config.channels.max(1));
        let sample_rate_hz = stream_config.sample_rate.0 as f64;

        let err_fn = |_err: cpal::StreamError| {};

        let stream_result = match output_config.sample_format() {
            SampleFormat::F32 => {
                let vp_f32 = vp.clone();
                let mut phase = 0.0_f64;
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _| {
                        fill_output_buffer_f32(data, channels, sample_rate_hz, &vp_f32, &mut phase);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let vp_i16 = vp.clone();
                let mut phase = 0.0_f64;
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        fill_output_buffer_i16(data, channels, sample_rate_hz, &vp_i16, &mut phase);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let vp_u16 = vp.clone();
                let mut phase = 0.0_f64;
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _| {
                        fill_output_buffer_u16(data, channels, sample_rate_hz, &vp_u16, &mut phase);
                    },
                    err_fn,
                    None,
                )
            }
            _ => return,
        };

        let Ok(stream) = stream_result else {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
            return;
        };
        if stream.play().is_err() {
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
            return;
        }

        while !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(50));
        }
    })
}

fn next_wave_sample(vp: &Arc<Viewport>, phase: &mut f64, sample_rate_hz: f64) -> f32 {
    let organ = vp.read_vocal_organ();
    if organ.breath_drive < 0.01 {
        return 0.0;
    }

    // Use v2 formant synthesis when formant parameters differ from neutral defaults
    let use_v2 = (organ.formant_f1 - 500.0).abs() > 1.0
        || (organ.formant_f2 - 1500.0).abs() > 1.0
        || (organ.formant_f3 - 2500.0).abs() > 1.0
        || organ.voiced_ratio > 0.01;

    if use_v2 {
        return next_wave_sample_v2(&organ, phase, sample_rate_hz);
    }

    let pitch = 280.0 + organ.pitch_tension * 420.0;
    let resonance = organ.resonance_position * 3.0;
    let phase_step = pitch / sample_rate_hz;
    *phase = (*phase + phase_step).fract();

    let theta = *phase * 2.0 * std::f64::consts::PI;
    let fundamental = theta.sin();
    let harmonic1 = (theta * 2.0).sin() * 0.3;
    let harmonic2 = (theta * 3.0).sin() * 0.1 * resonance;
    let sample = organ.breath_drive * (fundamental + harmonic1 + harmonic2);
    sample.clamp(-1.0, 1.0) as f32
}

fn next_wave_sample_v2(organ: &crate::vocal_organ::VocalOrganState, phase: &mut f64, sample_rate_hz: f64) -> f32 {
    // Infant formant range: 300–700 Hz fundamental
    let pitch = 300.0 + organ.pitch_tension * 400.0;
    let phase_step = pitch / sample_rate_hz;
    *phase = (*phase + phase_step).fract();

    // Glottal source: pulse + noise
    let voiced_pulse = if *phase < 0.03 {
        (*phase / 0.03 * std::f64::consts::PI).sin()
    } else {
        0.0
    };
    let noise = (*phase * 44.0).sin() * (*phase * 337.0).cos() * 0.3;
    let source = organ.voiced_ratio * voiced_pulse
        + (1.0 - organ.voiced_ratio) * organ.breath_noise_ratio * noise;

    // 3 parallel formant resonators
    let resonance = |freq: f64, q: f64| -> f64 {
        let w = 2.0 * std::f64::consts::PI * freq / sample_rate_hz;
        let r = (-w / q).exp();
        1.0 / (1.0 - 2.0 * r * w.cos() + r * r).sqrt()
    };

    let f1_gain = resonance(organ.formant_f1.max(100.0), 8.0);
    let f2_gain = resonance(organ.formant_f2.max(200.0), 12.0);
    let f3_gain = resonance(organ.formant_f3.max(500.0), 15.0);
    let filter = (f1_gain * 1.2 + f2_gain * 0.8 + f3_gain * 0.4) / 3.0;

    let sample = organ.breath_drive * source * filter;
    sample.clamp(-1.0, 1.0) as f32
}

fn fill_output_buffer_f32(
    data: &mut [f32],
    channels: usize,
    sample_rate_hz: f64,
    vp: &Arc<Viewport>,
    phase: &mut f64,
) {
    for frame in data.chunks_mut(channels) {
        let s = next_wave_sample(vp, phase, sample_rate_hz);
        for out in frame.iter_mut() {
            *out = s;
        }
    }
}

fn fill_output_buffer_i16(
    data: &mut [i16],
    channels: usize,
    sample_rate_hz: f64,
    vp: &Arc<Viewport>,
    phase: &mut f64,
) {
    for frame in data.chunks_mut(channels) {
        let s = next_wave_sample(vp, phase, sample_rate_hz);
        let v = (s * i16::MAX as f32) as i16;
        for out in frame.iter_mut() {
            *out = v;
        }
    }
}

fn fill_output_buffer_u16(
    data: &mut [u16],
    channels: usize,
    sample_rate_hz: f64,
    vp: &Arc<Viewport>,
    phase: &mut f64,
) {
    for frame in data.chunks_mut(channels) {
        let s = next_wave_sample(vp, phase, sample_rate_hz);
        let normalized = ((s * 0.5) + 0.5).clamp(0.0, 1.0);
        let v = (normalized * u16::MAX as f32) as u16;
        for out in frame.iter_mut() {
            *out = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: a voiced organ MUST produce audible (non-zero) samples through
    /// the real cpal synth path. begin_voicing writes non-neutral formants, which
    /// routes the synthesizer to its v2 path; that path's glottal source is zero
    /// unless voiced_ratio/breath_noise_ratio are set. They were not, so the body
    /// was silent no matter how hard it breathed. This pins the fix.
    #[test]
    fn voiced_organ_produces_nonzero_waveform() {
        let vp = Arc::new(Viewport::new());
        vp.vocal_organ.write().unwrap().begin_voicing(0, 0.9, 0.5, 0.5);
        let mut phase = 0.0;
        let any_nonzero = (0..2000)
            .map(|_| next_wave_sample(&vp, &mut phase, 48_000.0))
            .any(|s| s.abs() > 1e-4);
        assert!(
            any_nonzero,
            "a voiced organ must produce audible (non-zero) samples"
        );
    }
}
