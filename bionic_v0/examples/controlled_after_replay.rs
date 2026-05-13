use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

fn write_observed_snapshot(vp: &Arc<Viewport>, paths: &SnapshotPaths, t_us: u64) {
    vp.clock_us.store(t_us, Ordering::Relaxed);
    write_snapshot(vp, paths).expect("write controlled replay snapshot");
    thread::sleep(Duration::from_millis(20));
}

fn drive_vocal_after_segment(vp: &Viewport, start_t_us: u64, from_step: u64, to_step: u64) {
    let step_us = 1_000;
    for step in from_step..=to_step {
        let t_us = start_t_us + step * step_us;
        let fraction = step as f64 / 900.0;
        let target_energy = (0.80 + 1.20 * fraction).clamp(0.0, 2.0);
        vp.observe_vocal_target(t_us, target_energy, 0.35, 0.60, 0.20);
        vp.tick_vocal_imitation_loop(t_us);
    }
}

fn pair_latest_closed_epoch(vp: &Viewport, window_t_us: u64) -> u64 {
    let out = vp.read_action_output();
    let epoch = out
        .write_epoch_history
        .last()
        .expect("controlled replay must create a write epoch");
    let window_id = vp.accumulate_ingress_change(window_t_us, epoch.start_magnitude);
    assert!(vp.try_auto_pair_latest_epoch_to_window(window_t_us));
    vp.tick_life_closure_producers(window_t_us + 1_000);
    window_id
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runtime_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("bionic_v0_controlled_after_replay"));
    std::fs::create_dir_all(&runtime_dir).expect("create controlled replay runtime dir");
    let paths = SnapshotPaths::new(&runtime_dir);

    let vp = Arc::new(Viewport::new());
    let start_t_us = 1_000_000;
    vp.start_us.store(start_t_us, Ordering::Relaxed);

    drive_vocal_after_segment(&vp, start_t_us, 0, 600);
    let after_600_t_us = start_t_us + 610_000;
    let first_window = pair_latest_closed_epoch(&vp, after_600_t_us);
    for offset in 0..20 {
        write_observed_snapshot(&vp, &paths, after_600_t_us + offset * 20_000);
    }
    vp.close_action_consequence_window(first_window, after_600_t_us + 450_000);

    drive_vocal_after_segment(&vp, start_t_us, 601, 900);
    let after_900_t_us = start_t_us + 910_000;
    pair_latest_closed_epoch(&vp, after_900_t_us);
    for offset in 0..40 {
        write_observed_snapshot(&vp, &paths, after_900_t_us + offset * 20_000);
    }
}
