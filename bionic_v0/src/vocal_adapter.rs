use std::sync::Arc;
use crate::viewport::Viewport;

pub fn sample_vocal_waveform(vp: &Arc<Viewport>, n_samples: usize) -> Vec<f64> {
    let organ = vp.read_vocal_organ();
    let mut samples = Vec::with_capacity(n_samples);
    
    if organ.breath_drive < 0.01 {
        for _ in 0..n_samples {
            samples.push(0.0);
        }
        return samples;
    }
    
    let pitch = 100.0 + organ.pitch_tension * 400.0;
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