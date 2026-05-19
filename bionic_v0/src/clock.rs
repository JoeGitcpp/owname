use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::viewport::Viewport;

pub struct ClockHandles {
    pub threads: Vec<JoinHandle<()>>,
}

pub fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

pub fn spawn_master_clock(
    vp: Arc<Viewport>,
    stop: Arc<std::sync::atomic::AtomicBool>,
) -> JoinHandle<()> {
    spawn_master_clock_hz(vp, stop, 1_000)
}

pub fn spawn_master_clock_hz(
    vp: Arc<Viewport>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    rate_hz: u32,
) -> JoinHandle<()> {
    let period = Duration::from_micros(1_000_000u64 / rate_hz.max(1) as u64);
    thread::spawn(move || {
        let start_wall = now_us();
        vp.start_us.store(start_wall, Ordering::Relaxed);
        let t0 = Instant::now();
        let mut next = t0 + period;
        while !stop.load(Ordering::Relaxed) {
            vp.master_tick.fetch_add(1, Ordering::Relaxed);
            vp.clock_us.store(
                start_wall + t0.elapsed().as_micros() as u64,
                Ordering::Relaxed,
            );
            let now = Instant::now();
            if next > now {
                thread::sleep(next - now);
                next += period;
            } else {
                next = now + period;
            }
        }
    })
}

pub fn spawn_ticker<F>(
    rate_hz: u32,
    stop: Arc<std::sync::atomic::AtomicBool>,
    mut f: F,
) -> JoinHandle<()>
where
    F: FnMut(u64) + Send + 'static,
{
    let period = Duration::from_micros(1_000_000u64 / rate_hz as u64);
    thread::spawn(move || {
        let t0 = Instant::now();
        let mut next = t0 + period;
        let mut local_tick: u64 = 0;
        while !stop.load(Ordering::Relaxed) {
            f(local_tick);
            local_tick = local_tick.wrapping_add(1);
            let now = Instant::now();
            if next > now {
                thread::sleep(next - now);
                next += period;
            } else {
                next = now + period;
            }
        }
    })
}
