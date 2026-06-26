//! Speaker probe — isolate "no sound": is it the SPEAKER device or the BRAIN?
//!
//! Forces the vocal organ to hold breath and pushes it straight to the cpal
//! speaker path (`spawn_vocal_output`), bypassing all brain gating. If you hear
//! a steady ~430 Hz infant-voice tone, the audio device + output path work and
//! the silence in a live run is the BRAIN not driving breath. If this is ALSO
//! silent, the problem is the cpal output device (routing/volume/permission).
//!
//! Run from a terminal with audio:
//!   cargo run --release --bin vocal_speaker_probe

use bionic_v0::vocal_adapter::spawn_vocal_output;
use bionic_v0::viewport::Viewport;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let vp = Arc::new(Viewport::new());
    let stop = Arc::new(AtomicBool::new(false));

    // Start the real cpal speaker stream (same path the body uses).
    let h = spawn_vocal_output(vp.clone(), stop.clone());

    eprintln!("[probe] forcing voicing for 5s — you should hear a steady tone…");
    let start = Instant::now();
    let mut t_us: u64 = 0;
    while start.elapsed() < Duration::from_secs(5) {
        // Directly hold the organ Active with strong breath. Sweep the vowel
        // (resonance) so it is unmistakably a synthesized voice, not a click.
        let r = 0.3 + 0.4 * ((t_us as f64 / 400_000.0).sin() * 0.5 + 0.5);
        vp.vocal_organ
            .write()
            .unwrap()
            .begin_voicing(t_us, 0.9, 0.45, r);
        t_us += 50_000;
        std::thread::sleep(Duration::from_millis(50));
    }

    stop.store(true, Ordering::Relaxed);
    let _ = h.join();
    eprintln!("[probe] done. Heard a tone? → speakers OK (brain was the issue). Silent? → audio device issue.");
}
