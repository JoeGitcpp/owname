//! vocal_self_analysis — the body's own EARS for its own voice.
//!
//! The body synthesizes vocal PCM (vocal_synthesis.rs) but historically discarded
//! it: imitation compared 2 derived scalars (pitch_hint, resonance_hint) of the
//! TARGET against 2 motor params. That parametric proxy cannot resolve real
//! phonemes (P2 measurement: 14 Mandarin vowels collapse to ~4 cells).
//!
//! This module gives the body a PCM→mel analyzer so it can hear its OWN output
//! the SAME way it hears the target (the browser/gateway runs perception.ts;
//! this is a faithful Rust port of gateway/audio_to_bands.mjs, itself a replica
//! of site/src/lib/perception.ts). Self and target then live in one band space,
//! and their band-wise distance is a genuine perceptual prediction error — more
//! substrate-faithful than the programmer-chosen scalar gap, not less.
//!
//! Pipeline (identical to perception.ts readAudio):
//!   Blackman(2048) → FFT → |X|/N → EMA(0.6) → dB[-100,-30] → byte/255
//!   → 64 mel triangular bands (80–20000 Hz) ×1.4 clamp → downsample 64→16 VIB.
//!
//! Sample-rate note: mel band `b` always covers a FIXED Hz range (the bin mapping
//! adjusts for sample rate), so self bands (44100 Hz) and target bands (browser
//! ~48000 Hz) are directly comparable band-for-band. No resampling needed.

use crate::VIB_BANDS;

pub const FFT_SIZE: usize = 2048;
pub const NUM_BINS: usize = FFT_SIZE / 2; // 1024
pub const BANDS: usize = 64;
pub const MEL_LOW_HZ: f64 = 80.0;
pub const MEL_HIGH_HZ: f64 = 20000.0;
pub const SMOOTHING: f64 = 0.6;
pub const MIN_DB: f64 = -100.0;
pub const MAX_DB: f64 = -30.0;

/// AGC: the self-ear normalizes each analysis window's RMS to this level before
/// the dB conversion, so the body's full-amplitude self-voice lands in the dB
/// window's sensitive (non-saturating) region. Without this, every band pins to
/// 1.0 (a flat wall — no perceivable formant shape; measured root cause of the
/// body never acoustically tracking the target). Below AGC_FLOOR → treated as
/// silence (we don't amplify the noise floor of a quiet window).
pub const AGC_TARGET_RMS: f64 = 0.02;
pub const AGC_FLOOR: f64 = 1e-4;

#[inline]
fn clamp01(x: f64) -> f64 { if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x } }

// ── Blackman window (Web Audio AnalyserNode default) ──────────────────────────
pub fn blackman(n: usize) -> Vec<f64> {
    let (a0, a1, a2) = (0.42, 0.5, 0.08);
    (0..n)
        .map(|i| {
            let x = (2.0 * std::f64::consts::PI * i as f64) / n as f64;
            a0 - a1 * x.cos() + a2 * (2.0 * x).cos()
        })
        .collect()
}

// ── Radix-2 iterative FFT (in place, decimation-in-time) ──────────────────────
pub fn fft_in_place(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    debug_assert_eq!(n, im.len());
    debug_assert!(n.is_power_of_two(), "fft length must be power of 2");
    // bit-reversal permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        let half = len / 2;
        let mut i = 0usize;
        while i < n {
            let (mut cr, mut ci) = (1.0_f64, 0.0_f64);
            for k in 0..half {
                let a = i + k;
                let b = i + k + half;
                let vr = re[b] * cr - im[b] * ci;
                let vi = re[b] * ci + im[b] * cr;
                let (ur, ui) = (re[a], im[a]);
                re[a] = ur + vr;
                im[a] = ui + vi;
                re[b] = ur - vr;
                im[b] = ui - vi;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
            i += len;
        }
        len <<= 1;
    }
}

// ── Mel band bin boundaries (identical to perception.ts) ──────────────────────
fn hz_to_mel(hz: f64) -> f64 { 2595.0 * (1.0 + hz / 700.0).log10() }
fn mel_to_hz(mel: f64) -> f64 { 700.0 * (10f64.powf(mel / 2595.0) - 1.0) }

pub fn build_mel_band_boundaries(num_bands: usize, num_bins: usize, sample_rate: f64) -> Vec<usize> {
    let mel_low = hz_to_mel(MEL_LOW_HZ);
    let mel_high = hz_to_mel(MEL_HIGH_HZ);
    let hz_per_bin = sample_rate / (2.0 * num_bins as f64);
    (0..=num_bands + 1)
        .map(|i| {
            let mel = mel_low + (i as f64 / (num_bands + 1) as f64) * (mel_high - mel_low);
            (mel_to_hz(mel) / hz_per_bin).round() as usize
        })
        .collect()
}

/// Downsample 64 mel bands → 16 VIB bands by averaging groups of 4
/// (identical to net_adapter.rs incoming-band downsample, so self and target
/// undergo the same reduction).
pub fn downsample_to_vib(bands64: &[f64; BANDS]) -> [f64; VIB_BANDS] {
    let per = BANDS / VIB_BANDS; // 4
    let mut out = [0.0; VIB_BANDS];
    for (b, slot) in out.iter_mut().enumerate() {
        let mut s = 0.0;
        for k in 0..per {
            s += bands64[b * per + k];
        }
        *slot = s / per as f64;
    }
    out
}

/// Energy-weighted spectral center of a VIB band vector → [0,1] (matches the
/// body's pitch_hint derivation in tick_vocal_target_from_vib).
pub fn vib_spectral_center(bands: &[f64; VIB_BANDS]) -> f64 {
    let sum: f64 = bands.iter().sum();
    if sum <= f64::EPSILON {
        return 0.0;
    }
    let denom = (VIB_BANDS - 1) as f64;
    bands
        .iter()
        .enumerate()
        .map(|(i, v)| v * (i as f64 / denom))
        .sum::<f64>()
        / sum
}

/// High/low band ratio of a VIB band vector → [0,1] (matches resonance_hint).
pub fn vib_resonance(bands: &[f64; VIB_BANDS]) -> f64 {
    let sum: f64 = bands.iter().sum();
    if sum <= f64::EPSILON {
        return 0.0;
    }
    let high: f64 = bands.iter().enumerate().filter(|(i, _)| *i >= VIB_BANDS / 2).map(|(_, v)| *v).sum();
    high / sum
}

/// Normalized band-wise distance between two VIB band vectors → [0,1].
/// Mean absolute difference (L1/n): self vs target perceptual error.
pub fn spectral_distance(a: &[f64; VIB_BANDS], b: &[f64; VIB_BANDS]) -> f64 {
    let s: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    clamp01(s / VIB_BANDS as f64)
}

/// Shape-based spectral distance in [0,1] — compares the band-energy
/// DISTRIBUTION (formant pattern), not absolute level. Each vector is normalized
/// to unit sum, then total-variation distance (½·L1) between the two
/// distributions: 0 = identical shape, 1 = disjoint. Loudness-invariant, so the
/// acoustic servo targets vowel identity rather than volume — the right error
/// for imitation (measurement showed raw level distance is dominated by a
/// loudness/breadth mismatch, masking the shape error).
pub fn spectral_shape_distance(a: &[f64; VIB_BANDS], b: &[f64; VIB_BANDS]) -> f64 {
    let sa: f64 = a.iter().sum();
    let sb: f64 = b.iter().sum();
    if sa <= f64::EPSILON || sb <= f64::EPSILON {
        return 1.0; // one side silent → maximally far in shape
    }
    let tv: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x / sa - y / sb).abs())
        .sum::<f64>()
        * 0.5;
    clamp01(tv)
}

// ── SelfEar: stateful analyzer over a sliding PCM window ───────────────────────
/// Holds the EMA (temporal smoothing) state and a sliding FFT_SIZE window.
/// `observe(samples)` appends new PCM and returns the current 16 VIB bands.
pub struct SelfEar {
    win_fn: Vec<f64>,        // Blackman coefficients
    mel_bins: Vec<usize>,    // BANDS+2 boundaries for this sample rate
    ring: Vec<f32>,          // last FFT_SIZE samples (sliding)
    smoothed: Vec<f64>,      // EMA magnitude state, NUM_BINS
    re: Vec<f64>,
    im: Vec<f64>,
}

impl SelfEar {
    pub fn new(sample_rate: f64) -> Self {
        SelfEar {
            win_fn: blackman(FFT_SIZE),
            mel_bins: build_mel_band_boundaries(BANDS, NUM_BINS, sample_rate),
            ring: vec![0.0; FFT_SIZE],
            smoothed: vec![0.0; NUM_BINS],
            re: vec![0.0; FFT_SIZE],
            im: vec![0.0; FFT_SIZE],
        }
    }

    /// Slide `samples` into the ring (keep last FFT_SIZE), analyze, return 16 VIB bands.
    pub fn observe(&mut self, samples: &[f32]) -> [f64; VIB_BANDS] {
        // slide window: drop oldest, append newest (zero-stays if samples<FFT_SIZE)
        if samples.len() >= FFT_SIZE {
            self.ring.copy_from_slice(&samples[samples.len() - FFT_SIZE..]);
        } else {
            let keep = FFT_SIZE - samples.len();
            self.ring.copy_within(FFT_SIZE - keep.., 0);  // shift survivors to front
            self.ring[keep..].copy_from_slice(samples);
        }
        self.bands64_to_vib()
    }

    fn bands64_to_vib(&mut self) -> [f64; VIB_BANDS] {
        // AGC: normalize the window RMS into the dB analyzer's sensitive range.
        // The body's loud synthesis otherwise saturates every band to 1.0, erasing
        // all formant shape. Silence (RMS < floor) stays silent (gain 0).
        let rms = (self.ring.iter().map(|s| { let v = *s as f64; v * v }).sum::<f64>()
            / FFT_SIZE as f64)
            .sqrt();
        let gain = if rms > AGC_FLOOR { AGC_TARGET_RMS / rms } else { 0.0 };
        // windowed (and AGC-scaled) FFT input
        for i in 0..FFT_SIZE {
            self.re[i] = self.ring[i] as f64 * gain * self.win_fn[i];
            self.im[i] = 0.0;
        }
        fft_in_place(&mut self.re, &mut self.im);
        // magnitude → EMA → dB → byte/255 → freq vals
        let span = MAX_DB - MIN_DB;
        let mut freq = [0.0f64; NUM_BINS];
        for k in 0..NUM_BINS {
            let mag = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt() / FFT_SIZE as f64;
            let sm = SMOOTHING * self.smoothed[k] + (1.0 - SMOOTHING) * mag;
            self.smoothed[k] = sm;
            let db = if sm > 0.0 { 20.0 * sm.log10() } else { f64::NEG_INFINITY };
            let mut byte = ((255.0 * (db - MIN_DB)) / span).floor();
            if !byte.is_finite() || byte < 0.0 {
                byte = 0.0;
            } else if byte > 255.0 {
                byte = 255.0;
            }
            freq[k] = byte / 255.0;
        }
        // 64 mel triangular bands
        let mut bands64 = [0.0f64; BANDS];
        if self.mel_bins.len() == BANDS + 2 {
            for b in 0..BANDS {
                let lo = self.mel_bins[b];
                let hi = self.mel_bins[b + 2];
                let center = self.mel_bins[b + 1];
                let mut band_energy = 0.0;
                let mut band_count = 0.0;
                let mut i = lo;
                while i < hi && i < NUM_BINS {
                    let w = if i <= center {
                        if center > lo { (i - lo) as f64 / (center - lo) as f64 } else { 1.0 }
                    } else if hi > center {
                        (hi - i) as f64 / (hi - center) as f64
                    } else {
                        1.0
                    };
                    band_energy += freq[i] * w;
                    band_count += w;
                    i += 1;
                }
                bands64[b] = if band_count > 0.0 { clamp01((band_energy / band_count) * 1.4) } else { 0.0 };
            }
        }
        downsample_to_vib(&bands64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f64, dur_sec: f64, sr: f64, amp: f64) -> Vec<f32> {
        let n = (dur_sec * sr).round() as usize;
        (0..n)
            .map(|i| (amp * (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin()) as f32)
            .collect()
    }

    // settle the EMA by feeding the same windowed signal repeatedly
    fn settle_center(freq: f64, sr: f64) -> ([f64; VIB_BANDS], f64) {
        let mut ear = SelfEar::new(sr);
        let s = sine(freq, FFT_SIZE as f64 / sr, sr, 0.8); // exactly one FFT window
        let mut bands = [0.0; VIB_BANDS];
        for _ in 0..40 {
            bands = ear.observe(&s);
        }
        let c = vib_spectral_center(&bands);
        (bands, c)
    }

    #[test]
    fn fft_cosine_peaks_at_its_bin() {
        let n = 64usize;
        let bin = 5usize;
        let mut re: Vec<f64> = (0..n).map(|i| (2.0 * std::f64::consts::PI * bin as f64 * i as f64 / n as f64).cos()).collect();
        let mut im = vec![0.0; n];
        fft_in_place(&mut re, &mut im);
        let mag = |k: usize| (re[k] * re[k] + im[k] * im[k]).sqrt();
        assert!(mag(bin) > mag(bin - 1) * 5.0);
        assert!(mag(bin) > mag(bin + 1) * 5.0);
        assert!(mag(bin) > mag(10));
    }

    #[test]
    fn blackman_shape() {
        let w = blackman(FFT_SIZE);
        assert!(w[0] < 0.01);
        assert!(w[FFT_SIZE / 2] > 0.9);
        for i in 1..FFT_SIZE / 2 {
            assert!((w[i] - w[FFT_SIZE - i]).abs() < 1e-9);
        }
    }

    #[test]
    fn mel_bins_monotonic_in_range() {
        let bins = build_mel_band_boundaries(BANDS, NUM_BINS, 44100.0);
        assert_eq!(bins.len(), BANDS + 2);
        for i in 1..bins.len() {
            assert!(bins[i] >= bins[i - 1]);
        }
        assert!(bins[bins.len() - 1] <= NUM_BINS);
    }

    #[test]
    fn silence_gives_zero_bands() {
        let mut ear = SelfEar::new(44100.0);
        let bands = ear.observe(&vec![0.0f32; FFT_SIZE]);
        for b in bands {
            assert_eq!(b, 0.0);
        }
    }

    #[test]
    fn bands_in_range_and_length() {
        let mut ear = SelfEar::new(44100.0);
        let s = sine(1000.0, 0.1, 44100.0, 0.8);
        let bands = ear.observe(&s);
        assert_eq!(bands.len(), VIB_BANDS);
        for b in bands {
            assert!((0.0..=1.0).contains(&b), "band out of range: {b}");
        }
    }

    // The behavioral keystone (mirrors JS test A6): higher pitch → higher center.
    #[test]
    fn higher_pitch_higher_center() {
        let (_b_lo, c_lo) = settle_center(300.0, 44100.0);
        let (_b_hi, c_hi) = settle_center(4000.0, 44100.0);
        assert!(c_lo > 0.0 && c_hi > 0.0, "degenerate: {c_lo} {c_hi}");
        assert!(c_hi > c_lo + 0.05, "4kHz center ({c_hi}) should exceed 300Hz ({c_lo})");
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let s = sine(700.0, FFT_SIZE as f64 / 44100.0, 44100.0, 0.6);
        let mut e1 = SelfEar::new(44100.0);
        let mut e2 = SelfEar::new(44100.0);
        for _ in 0..10 {
            let a = e1.observe(&s);
            let b = e2.observe(&s);
            assert_eq!(a, b);
        }
    }

    // Two different sounds → measurably different bands (discrimination).
    #[test]
    fn distinct_sounds_distinct_bands() {
        let (b_lo, _) = settle_center(400.0, 44100.0);
        let (b_hi, _) = settle_center(3000.0, 44100.0);
        let d = spectral_distance(&b_lo, &b_hi);
        assert!(d > 0.01, "distinct sounds should differ in band space: d={d}");
    }

    #[test]
    fn spectral_distance_zero_for_identical() {
        let (b, _) = settle_center(500.0, 44100.0);
        assert_eq!(spectral_distance(&b, &b), 0.0);
    }

    #[test]
    fn shape_distance_zero_for_identical_and_scaled() {
        let mut a = [0.0; VIB_BANDS];
        a[2] = 0.3; a[3] = 0.6; a[7] = 0.1;
        assert!(spectral_shape_distance(&a, &a) < 1e-12);
        // loudness-invariant: scaling one side does not change shape distance
        let mut a2 = a;
        for v in a2.iter_mut() { *v *= 3.7; }
        assert!(spectral_shape_distance(&a, &a2) < 1e-9, "shape distance must be scale-invariant");
    }

    #[test]
    fn shape_distance_one_for_disjoint_and_silent() {
        let mut lo = [0.0; VIB_BANDS];
        lo[0] = 1.0; lo[1] = 1.0;
        let mut hi = [0.0; VIB_BANDS];
        hi[14] = 1.0; hi[15] = 1.0;
        assert!(spectral_shape_distance(&lo, &hi) > 0.99, "disjoint shapes should be ~1");
        let silent = [0.0; VIB_BANDS];
        assert_eq!(spectral_shape_distance(&lo, &silent), 1.0);
    }

    // DIAGNOSTIC: does the vocal apparatus actually move its perceived spectrum
    // when motor params sweep? If yes → self_c being stuck in vivo is a DRIVE
    // problem (servo viable). If the range is tiny → synthesis maps params to a
    // narrow acoustic band and a servo can't help. Run: cargo test diag_ -- --nocapture
    #[test]
    fn diag_param_sweep_self_center() {
        use crate::vocal_organ::VocalOrganState;
        use crate::vocal_synthesis::{VocalSynthesizer, VOCAL_SAMPLE_RATE};
        let sr = VOCAL_SAMPLE_RATE as f64;
        let sweep = |set: &dyn Fn(f64) -> VocalOrganState| -> Vec<(f64, f64, f64)> {
            [0.0, 0.25, 0.5, 0.75, 1.0]
                .iter()
                .map(|&p| {
                    let organ = set(p);
                    let mut syn = VocalSynthesizer::new();
                    let mut ear = SelfEar::new(sr);
                    let mut bands = [0.0; VIB_BANDS];
                    for _ in 0..50 {
                        let pcm = syn.synthesize(&organ, 1.0, 512);
                        bands = ear.observe(&pcm);
                    }
                    (p, vib_spectral_center(&bands), vib_resonance(&bands))
                })
                .collect()
        };
        let reso_sweep = sweep(&|p| VocalOrganState { breath_drive: 0.7, pitch_tension: 0.4, resonance_position: p, ..Default::default() });
        let pitch_sweep = sweep(&|p| VocalOrganState { breath_drive: 0.7, pitch_tension: p, resonance_position: 0.5, ..Default::default() });
        println!("\n[diag] resonance sweep (pitch=0.4):");
        for (p, c, r) in &reso_sweep { println!("  reso={p:.2} self_c={c:.3} self_res={r:.3}"); }
        println!("[diag] pitch sweep (reso=0.5):");
        for (p, c, r) in &pitch_sweep { println!("  pitch={p:.2} self_c={c:.3} self_res={r:.3}"); }
        let rc: Vec<f64> = reso_sweep.iter().map(|(_, c, _)| *c).collect();
        let pc: Vec<f64> = pitch_sweep.iter().map(|(_, c, _)| *c).collect();
        let range = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
        println!("[diag] self_c range: reso-sweep={:.3}  pitch-sweep={:.3}", range(&rc), range(&pc));

        // Decisive: does the full-band SHAPE discriminate even if centroid doesn't?
        let bands_at = |pitch: f64, reso: f64| -> [f64; VIB_BANDS] {
            let organ = VocalOrganState { breath_drive: 0.7, pitch_tension: pitch, resonance_position: reso, ..Default::default() };
            let mut syn = VocalSynthesizer::new();
            let mut ear = SelfEar::new(sr);
            let mut b = [0.0; VIB_BANDS];
            for _ in 0..50 { let pcm = syn.synthesize(&organ, 1.0, 512); b = ear.observe(&pcm); }
            b
        };
        let lo_r = bands_at(0.4, 0.0);
        let hi_r = bands_at(0.4, 1.0);
        let lo_p = bands_at(0.0, 0.5);
        let hi_p = bands_at(1.0, 0.5);
        let fmt = |b: &[f64; VIB_BANDS]| b.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ");
        println!("[diag] bands reso=0.0: {}", fmt(&lo_r));
        println!("[diag] bands reso=1.0: {}", fmt(&hi_r));
        println!("[diag] shape_dist(reso 0↔1)  = {:.4}", spectral_shape_distance(&lo_r, &hi_r));
        println!("[diag] shape_dist(pitch 0↔1) = {:.4}", spectral_shape_distance(&lo_p, &hi_p));

        // FIX HYPOTHESIS: RMS-normalize self PCM into the target's gentle regime
        // (corpus peak≈0.7, RMS≈0.1) BEFORE the dB analysis, to avoid saturation.
        let bands_norm = |pitch: f64, reso: f64, target_rms: f64| -> [f64; VIB_BANDS] {
            let organ = VocalOrganState { breath_drive: 0.7, pitch_tension: pitch, resonance_position: reso, ..Default::default() };
            let mut syn = VocalSynthesizer::new();
            let mut ear = SelfEar::new(sr);
            let mut b = [0.0; VIB_BANDS];
            for _ in 0..50 {
                let mut pcm = syn.synthesize(&organ, 1.0, 512);
                let rms = (pcm.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>() / pcm.len() as f64).sqrt();
                if rms > 1e-6 { let g = (target_rms / rms) as f32; for s in pcm.iter_mut() { *s *= g; } }
                b = ear.observe(&pcm);
            }
            b
        };
        for trms in [0.10, 0.04, 0.02, 0.01] {
            let nlo = bands_norm(0.4, 0.0, trms);
            let nhi = bands_norm(0.4, 1.0, trms);
            let nlp = bands_norm(0.0, 0.5, trms);
            let nhp = bands_norm(1.0, 0.5, trms);
            println!("[diag] rms={:.2}: reso-shape={:.4} pitch-shape={:.4}  self_c[reso0={:.3} reso1={:.3}]  mean_band={:.2}",
                trms,
                spectral_shape_distance(&nlo, &nhi),
                spectral_shape_distance(&nlp, &nhp),
                vib_spectral_center(&nlo), vib_spectral_center(&nhi),
                nlo.iter().sum::<f64>() / VIB_BANDS as f64);
        }
    }

    #[test]
    fn shape_distance_orders_by_overlap() {
        // partial overlap should sit strictly between identical(0) and disjoint(1)
        let mut a = [0.0; VIB_BANDS];
        a[4] = 0.5; a[5] = 0.5;
        let mut b = [0.0; VIB_BANDS];
        b[5] = 0.5; b[6] = 0.5; // shares band 5
        let d = spectral_shape_distance(&a, &b);
        assert!(d > 0.0 && d < 1.0, "partial overlap distance out of range: {d}");
    }
}
