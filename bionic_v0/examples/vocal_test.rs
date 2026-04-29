use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_adapter::{sample_vocal_waveform, spawn_vocal_output};

fn analyze_samples(samples: &[f64]) -> (f64, f64, f64) {
    let energy: f64 = samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64;
    let energy = energy.sqrt();

    let zero_crossings = samples
        .windows(2)
        .filter(|w| w[0].signum() != w[1].signum())
        .count();
    let avg_freq = zero_crossings as f64 / 2.0 / (samples.len() as f64 / 48000.0);

    (energy, avg_freq, zero_crossings as f64)
}

fn run_self_driven_test(
    vp: &Arc<Viewport>,
    name: &str,
    energy: f64,
    pitch: f64,
    resonance: f64,
    envelope: f64,
) {
    println!("=== Self-driven test: {} ===", name);
    println!(
        "Target: energy={}, pitch={}, resonance={}\n",
        energy, pitch, resonance
    );

    for tick in 1..=20 {
        vp.observe_vocal_target(tick * 100_000, energy, pitch, resonance, envelope);
        vp.refresh_probe_law(tick * 100_000);
        vp.tick_probe_output_from_law(tick * 100_000);
        vp.tick_vocal_imitation_loop(tick * 100_000);

        let organ = vp.read_vocal_organ();
        let imitation = vp.read_vocal_imitation();

        if tick <= 5 || tick % 5 == 0 || organ.phase.as_u8() == 1 {
            println!(
                "  tick {:2}: phase={:?}, breath={:.3}, pitch={:.3}, pressure={:.3}",
                tick,
                organ.phase,
                organ.breath_drive,
                organ.pitch_tension,
                imitation.imitation_pressure
            );
        }

        if organ.phase.as_u8() == 1 && tick > 5 {
            let samples = sample_vocal_waveform(vp, 2400);
            let (e, freq, _) = analyze_samples(&samples);
            if e > 0.01 {
                println!("         >>> AUDIO: energy={:.4}, freq={:.0}Hz", e, freq);
            }
            thread::sleep(Duration::from_millis(200));
        }
    }
    println!();
}

fn main() {
    println!("=== Vocal Self-Driven System Test ===\n");
    println!("Testing if the system can learn and produce various sounds through its own drive.\n");

    let vp = Arc::new(Viewport::new());
    let stop = Arc::new(AtomicBool::new(false));
    let output = spawn_vocal_output(vp.clone(), stop.clone());

    let sound_types = vec![
        ("Deep bass voice", 0.8, 0.08, 0.2, 0.3),
        ("Normal speaking voice", 0.7, 0.35, 0.5, 0.4),
        ("High pitched scream", 0.9, 0.85, 0.6, 0.5),
        ("Whisper (low energy)", 0.3, 0.4, 0.4, 0.2),
        ("Nasal tone", 0.6, 0.45, 0.95, 0.4),
        ("Muffled/closed", 0.6, 0.3, 0.05, 0.3),
    ];

    for (name, energy, pitch, resonance, envelope) in sound_types {
        run_self_driven_test(&vp, name, energy, pitch, resonance, envelope);
    }

    stop.store(true, Ordering::Relaxed);
    let _ = output.join();

    println!("=== Test Complete ===");
    println!("If you saw different phase transitions, the system is working!");
}
