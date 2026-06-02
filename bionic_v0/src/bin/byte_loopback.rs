use std::path::PathBuf;
use std::time::{Duration, Instant};

use bionic_v0::live_byte_closure::LIVE_BYTE_EGRESS_FRAME_LEN;
use bionic_v0::live_byte_loopback::LoopbackFileBridge;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runtime_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("runtime"));
    let duration_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let runtime_dir = PathBuf::from(runtime_dir);
    let live_dir = runtime_dir.join("live_bytes");
    let _ = std::fs::create_dir_all(&live_dir);

    let mut bridge = LoopbackFileBridge::new(
        live_dir.join("out.frames"),
        live_dir.join("in.frames"),
        LIVE_BYTE_EGRESS_FRAME_LEN,
    );

    if duration_secs > 0 {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(duration_secs) {
            let _ = bridge.tick();
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        loop {
            let _ = bridge.tick();
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
