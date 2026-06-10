//! B1 robustness observation tests (推论3 工程保证).
//!
//! These do NOT verify a "watchdog feature"; they observe two factual
//! dynamics on the tier-0 thread-fault axis:
//!   1. a panicking tick is absorbed — the rhythm thread stays alive and the
//!      fault count grows monotonically (not silent, not half-dead);
//!   2. the new snapshot fields round-trip symmetrically.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bionic_v0::clock::spawn_ticker;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn panicking_tick_is_absorbed_and_lands_on_thread_fault_axis() {
    let vp = Arc::new(Viewport::new());
    let stop = Arc::new(AtomicBool::new(false));

    let (fault0, count0) = vp.thread_fault_status();
    assert!(!fault0);
    assert_eq!(count0, 0);

    // A ticker whose every tick panics. The payload must be dropped unread;
    // only the neutral count may grow.
    let handle = spawn_ticker(vp.clone(), 200, stop.clone(), move |tick| {
        if tick % 2 == 0 {
            panic!("intentional test panic");
        }
    });

    // Let several rhythm periods elapse.
    std::thread::sleep(Duration::from_millis(300));
    let (fault_mid, count_mid) = vp.thread_fault_status();
    assert!(fault_mid, "absorbed panic must set the thread_fault flag");
    assert!(count_mid > 0, "absorbed panics must be counted");

    // The rhythm must still be self-exciting: the thread is NOT finished
    // while stop is still false.
    assert!(
        !handle.is_finished(),
        "ticker thread must survive panicking ticks (rhythm not killed)"
    );

    // And the count keeps growing monotonically while the rhythm runs.
    std::thread::sleep(Duration::from_millis(300));
    let (_, count_late) = vp.thread_fault_status();
    assert!(
        count_late > count_mid,
        "fault count must grow monotonically while the panicking rhythm runs"
    );

    stop.store(true, Ordering::Relaxed);
    handle.join().expect("ticker joins cleanly after stop");
}

#[test]
fn spawn_ticker_rate_zero_does_not_panic() {
    let vp = Arc::new(Viewport::new());
    let stop = Arc::new(AtomicBool::new(false));
    // rate_hz = 0 used to divide by zero; now clamped to 1 Hz.
    let handle = spawn_ticker(vp.clone(), 0, stop.clone(), move |_| {});
    std::thread::sleep(Duration::from_millis(50));
    assert!(!handle.is_finished());
    stop.store(true, Ordering::Relaxed);
    handle.join().expect("ticker joins cleanly after stop");
}

#[test]
fn snapshot_round_trips_thread_fault_axis() {
    let vp = Arc::new(Viewport::new());

    // Non-zero fault state on the live viewport.
    vp.mark_thread_fault();
    vp.mark_thread_fault();
    vp.mark_thread_fault();
    let (fault, count) = vp.thread_fault_status();
    assert!(fault);
    assert_eq!(count, 3);

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_thread_fault_snap_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let re = read_snapshot(&paths.final_path).unwrap();

    assert!(re.thread_fault, "thread_fault flag must survive the round trip");
    assert_eq!(
        re.thread_fault_count, 3,
        "thread_fault_count must survive the round trip"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
