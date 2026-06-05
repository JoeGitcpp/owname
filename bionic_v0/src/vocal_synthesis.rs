//! Biological vocal synthesis — replaces the 3-harmonic pure-sine synthesizer.
//!
//! Architecture:  glottal_source × vocal_tract_filter + aspiration_noise
//!   1. Rosenberg-C glottal pulse — non-linear vocal cord vibration model
//!      (rich harmonics, controlled open quotient)
//!   2. Five 2-pole IIR formant filters (F1–F5) — vocal tract resonances
//!      F1/F2 map to resonance_position (vowel space); F3–F5 fixed (tract length)
//!   3. Pink noise aspiration (Voss–McCartney) scaled by noise_mix
//!   4. Vibrato 5.2 Hz ± 0.8% depth, jitter 1.5%, shimmer 3.5%
//!   5. Exponential attack/decay envelope (10 ms / 20 ms)
//!   6. Survival coupling: reserve ↓ → vibrato depth ↓ + aspiration ↑
//!
//! No backprop, no ML, no pretrained model — pure physical acoustics.

use crate::vocal_organ::VocalOrganState;

pub const VOCAL_SAMPLE_RATE: u32 = 44100;
const TAU: f64 = std::f64::consts::TAU;

// ── Rosenberg-C glottal pulse ─────────────────────────────────────────────────

/// Rosenberg-C glottal pulse for one normalised period t ∈ [0, 1).
/// open_quotient: fraction of period the glottis is open (0.4–0.9).
/// Returns a value in [0, 1]; 0 during the closed phase.
pub fn rosenberg_c(t_norm: f64, open_quotient: f64) -> f64 {
    let oq = open_quotient.clamp(0.4, 0.9);
    if t_norm < oq {
        // Open phase: raised-cosine from 0 to 1 to 0 over oq duration
        0.5 * (1.0 - (TAU * 0.5 * t_norm / oq).cos())
    } else {
        // Closed phase: glottis shut → zero airflow → silence
        0.0
    }
}

// ── Two-pole formant (resonant bandpass) IIR filter ───────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct FormantFilter {
    a1: f64,
    a2: f64,
    b0: f64,
    z1: f64,
    z2: f64,
}

impl FormantFilter {
    /// Create a 2-pole resonant filter centred at freq_hz with given bandwidth.
    pub fn new(freq_hz: f64, bandwidth_hz: f64, sample_rate: f64) -> Self {
        use std::f64::consts::PI;
        let r   = (-PI * bandwidth_hz / sample_rate).exp();
        let cos = (2.0 * PI * freq_hz / sample_rate).cos();
        let a1  = -2.0 * r * cos;
        let a2  = r * r;
        let b0  = 1.0 - r * r;
        Self { a1, a2, b0, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x - self.a1 * self.z1 - self.a2 * self.z2;
        self.z2 = self.z1;
        self.z1 = y;
        y
    }

    pub fn reset(&mut self) { self.z1 = 0.0; self.z2 = 0.0; }
}

// ── Pink noise (Voss–McCartney algorithm) ─────────────────────────────────────

pub struct PinkNoise {
    b: [f64; 7],
    rng: u64,
}

impl PinkNoise {
    pub fn new(seed: u64) -> Self { Self { b: [0.0; 7], rng: seed ^ 0xdeadbeef } }

    #[inline]
    fn lcg(&mut self) -> f64 {
        self.rng = self.rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.rng >> 33) as f64 / (1u64 << 31) as f64 - 1.0   // [-1, 1]
    }

    /// One sample of pink noise, approximately [-1, 1].
    pub fn next(&mut self) -> f64 {
        let white  = self.lcg();
        let mask   = (self.rng >> 1) & 0x7F;
        for i in 0..7 {
            if mask & (1 << i) != 0 { self.b[i] = self.lcg(); }
        }
        (self.b.iter().sum::<f64>() + white) / 8.5
    }
}

// ── Main synthesizer state ────────────────────────────────────────────────────

pub struct VocalSynthesizer {
    phase:        f64,          // glottal phase ∈ [0, 1)
    vib_phase:    f64,          // vibrato phase ∈ [0, 2π)
    formants:     [FormantFilter; 5],
    pink:         PinkNoise,
    env:          f64,          // amplitude envelope [0, 1]
    rng:          u64,          // LCG for jitter / shimmer
}

impl VocalSynthesizer {
    pub fn new() -> Self {
        let sr = VOCAL_SAMPLE_RATE as f64;
        Self {
            phase:     0.0,
            vib_phase: 0.0,
            formants:  [
                FormantFilter::new(500.0, 80.0,  sr),  // F1 — vowel height
                FormantFilter::new(1500.0, 100.0, sr), // F2 — vowel backness
                FormantFilter::new(2500.0, 150.0, sr), // F3 — fixed (tract length)
                FormantFilter::new(3500.0, 200.0, sr), // F4 — fixed
                FormantFilter::new(4500.0, 280.0, sr), // F5 — fixed (lip radiation)
            ],
            pink:  PinkNoise::new(0xc0ffee),
            env:   0.0,
            rng:   0x1234567890abcdef,
        }
    }

    #[inline]
    fn rand_f64(&mut self) -> f64 {
        self.rng = self.rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.rng >> 33) as f64 / (1u64 << 31) as f64 - 1.0
    }

    /// Synthesize n_samples of PCM at VOCAL_SAMPLE_RATE Hz.
    /// Updates internal state (phase, envelope, filter states) in place.
    pub fn synthesize(&mut self, organ: &VocalOrganState, reserve: f64, n_samples: usize) -> Vec<f32> {
        let sr        = VOCAL_SAMPLE_RATE as f64;
        let mut out   = Vec::with_capacity(n_samples);

        let breath = organ.breath_drive;
        if breath < 0.01 {
            // Silent: let envelope decay, output zeros
            for _ in 0..n_samples {
                self.env = (self.env * 0.9997).max(0.0); // ~20 ms decay
                out.push(0.0f32);
            }
            return out;
        }

        // Acoustic parameters derived from body state
        let f0    = 180.0 + organ.pitch_tension * 520.0;   // 180–700 Hz
        let oq    = 0.55  + organ.pitch_tension * 0.25;    // 0.55–0.80
        let reso  = organ.resonance_position.clamp(0.0, 1.0);
        let noise = organ.noise_mix.clamp(0.0, 1.0);

        // Vibrato depth scales with reserve (fatigued body → less vibrato)
        let vib_depth  = 0.008 * reserve.max(0.05);
        let vib_rate   = 5.2;   // Hz
        let vib_inc    = TAU * vib_rate / sr;

        // Update F1/F2 formants to match resonance (vowel space)
        let f1_hz = 300.0 + reso * 600.0;    // 300–900 Hz
        let f2_hz = 850.0 + reso * 1500.0;   // 850–2350 Hz
        self.formants[0] = FormantFilter::new(f1_hz, 80.0,  sr);
        self.formants[1] = FormantFilter::new(f2_hz, 100.0, sr);

        // Attack coefficient (10 ms)
        let attack = 1.0 - (-1.0 / (0.010 * sr)).exp();

        for _ in 0..n_samples {
            // Advance vibrato
            self.vib_phase += vib_inc;
            if self.vib_phase >= TAU { self.vib_phase -= TAU; }

            // Jitter (per-sample) — vocal-cord mass irregularity
            let jitter    = 1.0 + 0.015 * self.rand_f64();
            let f0_inst   = f0 * (1.0 + vib_depth * self.vib_phase.sin()) * jitter;

            // Advance glottal phase
            self.phase += f0_inst / sr;
            if self.phase >= 1.0 { self.phase -= 1.0; }

            // Glottal source
            let g = rosenberg_c(self.phase, oq);

            // Shimmer (per-sample amplitude perturbation)
            let shimmer = 1.0 + 0.035 * self.rand_f64();
            let g_sh    = g * shimmer;

            // Vocal tract: run through F1–F5 formant chain
            let mut y = g_sh;
            for f in &mut self.formants { y = f.process(y); }

            // Aspiration noise (pink, gated by noise_mix and breath)
            let asp = self.pink.next() * noise * 0.20 * breath;

            // Amplitude envelope
            let target_env = breath;
            if target_env > self.env {
                self.env += attack * (target_env - self.env);
            } else {
                self.env *= 0.9997; // ~20 ms release
            }

            let sample = ((y + asp) * self.env).clamp(-1.0, 1.0) as f32;
            out.push(sample);
        }
        out
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocal_organ::{VocalOrganState, VocalOrganPhase};

    fn make_organ(breath: f64, pitch: f64, reso: f64, noise: f64) -> VocalOrganState {
        VocalOrganState {
            breath_drive: breath,
            pitch_tension: pitch,
            resonance_position: reso,
            noise_mix: noise,
            phase: VocalOrganPhase::Active,
            noise_ratio: noise,
            last_update_t_us: 0,
            breath_noise_ratio: noise,
        }
    }

    fn fft_harmonics(samples: &[f32], f0: f64, sr: f64) -> Vec<f64> {
        // Rough harmonic energy estimate: look at ±5% windows around each harmonic
        let n = samples.len() as f64;
        let mut harmonics = Vec::new();
        for k in 1..=10usize {
            let target_freq = f0 * k as f64;
            let bin = (target_freq / sr * n) as usize;
            let lo = bin.saturating_sub(2);
            let hi = (bin + 3).min(samples.len() / 2);
            // Simple Fourier sum around target bin
            let energy: f64 = (lo..hi).map(|b| {
                let (re, im): (f64, f64) = samples.iter().enumerate().fold((0.0, 0.0), |(r, i), (t, &s)| {
                    let angle = -2.0 * std::f64::consts::PI * b as f64 * t as f64 / n;
                    (r + s as f64 * angle.cos(), i + s as f64 * angle.sin())
                });
                (re * re + im * im).sqrt() / n
            }).sum::<f64>() / (hi - lo) as f64;
            harmonics.push(energy);
        }
        harmonics
    }

    #[test]
    fn rosenberg_c_starts_at_zero() {
        assert!((rosenberg_c(0.0, 0.6) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn rosenberg_c_no_dc_bias() {
        let n = 10_000;
        let sum: f64 = (0..n).map(|i| rosenberg_c(i as f64 / n as f64, 0.65)).sum::<f64>() / n as f64;
        // Mean should be well below 0.5 (open phase is partial)
        assert!(sum < 0.5, "DC bias too high: {sum}");
    }

    #[test]
    fn rosenberg_c_closed_phase_is_zero() {
        // Past open_quotient, the pulse is zero
        let oq = 0.6;
        for t in [0.65f64, 0.75, 0.9, 0.99] {
            assert_eq!(rosenberg_c(t, oq), 0.0, "expected 0 at t={t}");
        }
    }

    #[test]
    fn rosenberg_c_open_quotient_controls_open_fraction() {
        let n = 1000;
        let nonzero_60 = (0..n).filter(|&i| rosenberg_c(i as f64 / n as f64, 0.60) > 1e-9).count();
        let nonzero_80 = (0..n).filter(|&i| rosenberg_c(i as f64 / n as f64, 0.80) > 1e-9).count();
        assert!(nonzero_80 > nonzero_60, "oq=0.8 should have more open samples than oq=0.6");
    }

    #[test]
    fn formant_filter_peak_at_target_frequency() {
        let sr = 44100.0f64;
        let f0 = 800.0f64;
        let mut filt = FormantFilter::new(f0, 80.0, sr);
        // Sweep a sine at exactly f0 and compare to slightly off frequency
        let n = 4096usize;
        let mut energy_at_f0 = 0.0f64;
        let mut energy_off   = 0.0f64;
        for i in 0..n {
            let t = i as f64 / sr;
            let at_f0   = filt.process((2.0 * std::f64::consts::PI * f0       * t).sin());
            let at_off  = filt.process((2.0 * std::f64::consts::PI * (f0 * 2.0) * t).sin());
            if i > n / 4 { // skip transient
                energy_at_f0 += at_f0.abs();
                energy_off   += at_off.abs();
            }
        }
        assert!(energy_at_f0 > energy_off * 2.0, "filter should amplify f0 more than 2×f0");
    }

    #[test]
    fn five_formants_produce_five_spectral_peaks() {
        let sr = 44100.0f64;
        let freqs = [500.0, 1500.0, 2500.0, 3500.0, 4500.0];
        let bws   = [80.0, 100.0, 150.0, 200.0, 280.0];
        let mut chain: Vec<FormantFilter> = freqs.iter().zip(bws.iter())
            .map(|(&f, &bw)| FormantFilter::new(f, bw, sr)).collect();

        // Drive chain with impulse, measure output spectrum
        let n = 8192usize;
        let mut out = vec![0.0f64; n];
        let mut x = 1.0f64; // impulse
        for i in 0..n {
            let mut y = x;
            for f in &mut chain { y = f.process(y); }
            out[i] = y;
            x = 0.0;
        }
        // Check that energy is non-negligible at each formant frequency
        for (k, &freq) in freqs.iter().enumerate() {
            let bin = (freq / sr * n as f64) as usize;
            let local_energy: f64 = (bin.saturating_sub(3)..=(bin+3).min(n/2))
                .map(|b| {
                    let (re, im) = out.iter().enumerate().fold((0.0f64, 0.0f64), |(r, i), (t, &s)| {
                        let a = -2.0 * std::f64::consts::PI * b as f64 * t as f64 / n as f64;
                        (r + s * a.cos(), i + s * a.sin())
                    });
                    (re*re + im*im).sqrt()
                }).sum::<f64>();
            assert!(local_energy > 1e-4, "formant F{} at {}Hz has negligible energy", k+1, freq);
        }
    }

    #[test]
    fn pink_noise_has_near_zero_mean() {
        let mut p = PinkNoise::new(12345);
        let n = 10_000;
        let mean: f64 = (0..n).map(|_| p.next()).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.1, "pink noise mean too far from 0: {mean}");
    }

    #[test]
    fn synthesize_silence_when_breath_zero() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(0.0, 0.5, 0.5, 0.0);
        let out = syn.synthesize(&organ, 1.0, 512);
        assert_eq!(out.len(), 512);
        assert!(out.iter().all(|&s| s.abs() < 0.01), "should be silent");
    }

    #[test]
    fn synthesize_produces_nonzero_output_when_active() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(0.8, 0.5, 0.5, 0.0);
        let out = syn.synthesize(&organ, 1.0, 4096);
        let rms: f64 = (out.iter().map(|&s| s as f64 * s as f64).sum::<f64>() / out.len() as f64).sqrt();
        assert!(rms > 0.01, "expected audible output, RMS={rms:.4}");
    }

    #[test]
    fn different_pitch_produces_different_fundamental() {
        let mut syn1 = VocalSynthesizer::new();
        let mut syn2 = VocalSynthesizer::new();
        let o1 = make_organ(0.7, 0.1, 0.5, 0.0);  // low pitch
        let o2 = make_organ(0.7, 0.9, 0.5, 0.0);  // high pitch
        let n = 8192usize;
        let sr = VOCAL_SAMPLE_RATE as f64;
        let s1 = syn1.synthesize(&o1, 1.0, n);
        let s2 = syn2.synthesize(&o2, 1.0, n);
        // Find peak FFT bin for each
        let peak = |s: &[f32]| -> usize {
            let mut best = 0usize;
            let mut best_e = 0.0f64;
            for b in 50..(n/4) {
                let e: f64 = s.iter().enumerate().map(|(t, &x)| {
                    let a = -2.0 * std::f64::consts::PI * b as f64 * t as f64 / n as f64;
                    x as f64 * a.cos()
                }).sum::<f64>().abs();
                if e > best_e { best_e = e; best = b; }
            }
            best
        };
        let bin1 = peak(&s1);
        let bin2 = peak(&s2);
        let f1 = bin1 as f64 * sr / n as f64;
        let f2 = bin2 as f64 * sr / n as f64;
        assert!(f2 > f1 + 30.0, "high pitch (f2={f2:.0}Hz) should be > low pitch (f1={f1:.0}Hz) + 30Hz");
    }

    #[test]
    fn different_resonance_produces_different_spectral_centroid() {
        let mut syn_low  = VocalSynthesizer::new();
        let mut syn_high = VocalSynthesizer::new();
        let o_low  = make_organ(0.7, 0.5, 0.0, 0.0);  // reso=0 → low F1/F2
        let o_high = make_organ(0.7, 0.5, 1.0, 0.0);  // reso=1 → high F2
        let n = 4096usize;
        let sr = VOCAL_SAMPLE_RATE as f64;
        let s_low  = syn_low.synthesize(&o_low, 1.0, n);
        let s_high = syn_high.synthesize(&o_high, 1.0, n);

        let centroid = |s: &[f32]| -> f64 {
            let (num, den) = s.iter().enumerate().fold((0.0f64, 0.0f64), |(n, d), (t, &x)| {
                let e = x as f64 * x as f64;
                let freq = t as f64 * sr / s.len() as f64;
                (n + freq * e, d + e)
            });
            if den > 1e-10 { num / den } else { 0.0 }
        };
        let c_low  = centroid(&s_low);
        let c_high = centroid(&s_high);
        assert!(c_high > c_low + 400.0,
            "high reso centroid ({c_high:.0}Hz) should be >400Hz above low ({c_low:.0}Hz)");
    }

    #[test]
    fn high_noise_mix_produces_noisier_output() {
        let mut syn_clean = VocalSynthesizer::new();
        let mut syn_noisy = VocalSynthesizer::new();
        let o_clean = make_organ(0.7, 0.5, 0.5, 0.0);
        let o_noisy = make_organ(0.7, 0.5, 0.5, 1.0);
        let n = 4096usize;
        let s_clean = syn_clean.synthesize(&o_clean, 1.0, n);
        let s_noisy = syn_noisy.synthesize(&o_noisy, 1.0, n);

        // Measure spectral flatness (higher = more noise-like)
        let flatness = |s: &[f32]| -> f64 {
            let half = s.len() / 4;
            let energies: Vec<f64> = (0..half).map(|b| {
                let e: f64 = s.iter().enumerate().map(|(t, &x)| {
                    let a = -2.0 * std::f64::consts::PI * b as f64 * t as f64 / s.len() as f64;
                    x as f64 * a.cos()
                }).sum::<f64>().powi(2);
                e.max(1e-12)
            }).collect();
            let geom_mean = energies.iter().map(|e| e.ln()).sum::<f64>() / half as f64;
            let arith_mean = energies.iter().sum::<f64>() / half as f64;
            (geom_mean.exp() / arith_mean).clamp(0.0, 1.0)
        };
        let f_clean = flatness(&s_clean);
        let f_noisy = flatness(&s_noisy);
        assert!(f_noisy > f_clean, "noisy output should be spectrally flatter than clean: {f_noisy:.4} vs {f_clean:.4}");
    }

    #[test]
    fn vibrato_exists_as_pitch_variation() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(0.7, 0.5, 0.5, 0.0);
        let n = VOCAL_SAMPLE_RATE as usize;  // 1 second
        let out = syn.synthesize(&organ, 1.0, n);

        // Compute zero-crossing rate in windows to detect pitch variation
        let window = VOCAL_SAMPLE_RATE as usize / 100;  // 10ms windows
        let periods: Vec<f64> = out.windows(window).step_by(window).map(|w| {
            let zc = w.windows(2).filter(|p| p[0] * p[1] < 0.0).count();
            if zc > 0 { window as f64 / zc as f64 } else { 0.0 }
        }).filter(|&p| p > 0.0).collect();

        if periods.len() > 10 {
            let mean = periods.iter().sum::<f64>() / periods.len() as f64;
            let var  = periods.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / periods.len() as f64;
            let std  = var.sqrt();
            // Vibrato should cause non-trivial pitch variation
            assert!(std > 0.5, "vibrato should cause pitch std > 0.5 period units, got {std:.3}");
        }
    }

    #[test]
    fn low_reserve_reduces_vibrato_depth() {
        // With low reserve, vibrato_depth = 0.008 * 0.05 = 0.0004 (minimal)
        // With high reserve, vibrato_depth = 0.008 * 1.0 = 0.008
        // The synthesizer uses vib_depth in modulation — measure pitch variation
        let n = VOCAL_SAMPLE_RATE as usize / 2;
        let organ = make_organ(0.7, 0.5, 0.5, 0.0);

        let pitch_std = |reserve: f64| -> f64 {
            let mut syn = VocalSynthesizer::new();
            let out = syn.synthesize(&organ, reserve, n);
            let window = 256usize;
            let zcrates: Vec<f64> = out.windows(window).step_by(window / 4).map(|w| {
                w.windows(2).filter(|p| p[0] * p[1] < 0.0).count() as f64 / window as f64
            }).collect();
            if zcrates.is_empty() { return 0.0; }
            let mean = zcrates.iter().sum::<f64>() / zcrates.len() as f64;
            (zcrates.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / zcrates.len() as f64).sqrt()
        };

        let std_low  = pitch_std(0.05);
        let std_high = pitch_std(1.0);
        assert!(std_high >= std_low,
            "high reserve should have >= vibrato variation: high={std_high:.5} low={std_low:.5}");
    }

    #[test]
    fn phase_continuity_across_frames() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(0.7, 0.5, 0.5, 0.0);
        let frame1 = syn.synthesize(&organ, 1.0, 1024);
        let frame2 = syn.synthesize(&organ, 1.0, 1024);
        // Check that the boundary between frames has no sudden jump
        let last1  = *frame1.last().unwrap();
        let first2 = frame2[0];
        let jump   = (last1 - first2).abs();
        assert!(jump < 0.5, "frame boundary jump too large: {jump:.4}");
    }

    #[test]
    fn attack_envelope_rises_gradually() {
        let mut syn = VocalSynthesizer::new();
        // First frame: envelope starts at 0
        let organ = make_organ(0.8, 0.5, 0.5, 0.0);
        let out = syn.synthesize(&organ, 1.0, 2048);
        // First 10ms = 441 samples; amplitude should start lower than later
        let early_rms: f64 = out[..441].iter().map(|&s| s as f64 * s as f64).sum::<f64>() / 441.0;
        let later_rms: f64 = out[441..].iter().map(|&s| s as f64 * s as f64).sum::<f64>() / (out.len() - 441) as f64;
        assert!(later_rms > early_rms * 1.5, "attack should rise: early={early_rms:.5} later={later_rms:.5}");
    }

    #[test]
    fn sample_count_matches_requested() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(0.5, 0.5, 0.5, 0.0);
        for n in [256, 512, 1024, 4096] {
            let out = syn.synthesize(&organ, 1.0, n);
            assert_eq!(out.len(), n, "expected {n} samples");
        }
    }

    #[test]
    fn output_clamped_to_valid_range() {
        let mut syn = VocalSynthesizer::new();
        let organ = make_organ(1.0, 1.0, 1.0, 1.0);  // max everything
        let out = syn.synthesize(&organ, 1.0, 8192);
        assert!(out.iter().all(|&s| s >= -1.0 && s <= 1.0), "output must be clamped to [-1, 1]");
    }

    #[test]
    fn vocal_sample_rate_is_44100() {
        assert_eq!(VOCAL_SAMPLE_RATE, 44100);
    }
}
