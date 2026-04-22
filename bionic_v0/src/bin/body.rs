use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bionic_v0::action_consequence::tick_all_ingress_accumulation;
use bionic_v0::adapter::{spawn_registry_driver, AdapterRegistry, RealPhotAdapter, VibAdapter};
use bionic_v0::clock::{now_us, spawn_master_clock, spawn_ticker};
use bionic_v0::highlevel::HighLevelPredictor;
use bionic_v0::phot_camera::{RealCameraFrameProvider, StubRealCameraFrameProvider};
use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use bionic_v0::{BINDING_REFRESH_HZ, PHOT_CONSEQUENCE_QUIESCENCE_US};

// Serial global refresh cycle terminates at reasoning history + snapshot.
// Per cleanup spec 2026-04-20, semantic-readiness and symbolizer are not part
// of the body main loop and must not be reintroduced here.
fn run_global_refresh_cycle(vp: &Arc<Viewport>, paths: &Arc<SnapshotPaths>, t: u64) {
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    vp.refresh_probe_law(t);
    vp.tick_probe_output_from_law(t);
    // Additional action-output triggers (Tasks 5-7 from body-side-write-trigger plan).
    // Each reads independently-refreshed substrate state and fires events
    // only when its engineering threshold is strictly exceeded.
    vp.tick_surprise_trigger(t);
    vp.tick_attention_trigger(t);
    vp.tick_relation_instability_trigger(t);
    if write_snapshot(vp, paths).is_err() {
        vp.mark_snapshot_fail();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runtime_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("runtime"));
    let duration_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let runtime_dir = PathBuf::from(runtime_dir);
    std::fs::create_dir_all(&runtime_dir).ok();
    let paths = Arc::new(SnapshotPaths::new(&runtime_dir));

    let vp = Arc::new(Viewport::new());
    let stop = Arc::new(AtomicBool::new(false));

    let h_master = spawn_master_clock(vp.clone(), stop.clone());

    // All sensory I/O goes through the registry. body.rs is oblivious to
    // which modalities are present — only that the registry exists.
    let h_adapters = {
        let vp_reg = vp.clone();
        spawn_registry_driver(vp.clone(), stop.clone(), move || {
            let mut reg = AdapterRegistry::new();
            // VIB: real mic. Owns its cpal stream inside this thread.
            reg.register(Box::new(VibAdapter::new(&vp_reg)))
                .expect("register vib");
            // PHOT: attempt to bind the default monocular camera. On success
            // the real hardware path is live; on failure phot_health surfaces
            // FAULT immediately via the stub — never a silent fallback to
            // synthetic data.
            // No human-readable diagnostics in the body process (cleanup spec
            // §4.5). Camera startup outcome is observable through phot_health
            // state alone: success → live; failure → StubRealCameraFrameProvider
            // drives phot_health = FAULT via the adapter path.
            let cam: Box<dyn bionic_v0::phot_camera::CameraFrameProvider> =
                match RealCameraFrameProvider::try_new() {
                    Ok(p) => Box::new(p),
                    Err(_) => Box::new(StubRealCameraFrameProvider::try_new()),
                };
            reg.register(Box::new(RealPhotAdapter::new(cam)))
                .expect("register phot");
            reg
        })
    };

    // Per-band AR-2 predictors, 100 Hz — purely consume viewport data, do not
    // own any input source, so they stay in body.rs.
    let predictors: Arc<Mutex<Vec<Ar2Predictor>>> =
        Arc::new(Mutex::new((0..VIB_BANDS).map(Ar2Predictor::new).collect()));
    let vp_pred = vp.clone();
    let pred_ref = predictors.clone();
    let h_pred = spawn_ticker(100, stop.clone(), move |_| {
        let mut ps = pred_ref.lock().unwrap();
        for p in ps.iter_mut() {
            p.tick(&vp_pred);
        }
        vp_pred.predictor_ticks.fetch_add(1, Ordering::Relaxed);
    });

    // High-level predictor, 20 Hz
    let hl = Arc::new(Mutex::new(HighLevelPredictor::new()));
    let vp_hl = vp.clone();
    let hl_ref = hl.clone();
    let h_hl = spawn_ticker(20, stop.clone(), move |_| {
        let mut hlp = hl_ref.lock().unwrap();
        hlp.tick(&vp_hl);
    });

    // Viewport health recompute, 50 Hz — observer only renders this value.
    let vp_health = vp.clone();
    let h_health = spawn_ticker(50, stop.clone(), move |_| {
        vp_health.update_health_state(now_us());
    });

    // Unified ingress accumulation ticker, 10 Hz.
    // Closes quiescent consequence windows across all modalities and attempts
    // auto-pairing for newly closed windows. Single authoritative path for
    // quiescence-based window closure.
    let vp_ingress = vp.clone();
    let h_ingress = spawn_ticker(10, stop.clone(), move |_| {
        tick_all_ingress_accumulation(&vp_ingress, now_us(), PHOT_CONSEQUENCE_QUIESCENCE_US);
    });

    // Binding maintenance refresh ticker, 5 Hz by default.
    let vp_binding = vp.clone();
    let h_binding = spawn_ticker(BINDING_REFRESH_HZ, stop.clone(), move |_| {
        vp_binding.tick_binding_maintenance(now_us());
    });

    // Endogenous activity ticker, 10 Hz.
    // Err-driven attention rebalancing (self-regulation law §2.2) and
    // quiet-period history replay (endogenous activity law §2.1).
    // Both are substrate-level mechanisms — no world content or meaning.
    let vp_endo = vp.clone();
    let h_endogenous = spawn_ticker(10, stop.clone(), move |_| {
        let t = now_us();
        vp_endo.tick_err_driven_attention(t);
        vp_endo.tick_quiet_history_replay(t);
    });

    // Global orchestrator: attention → relations → snapshot (30 Hz).
    //
    // Replaces the three independent tickers that previously ran at 30/20/30 Hz.
    // Single serial cycle ensures relations read same-round attention shares and
    // the snapshot always captures post-refresh state.
    let vp_orch = vp.clone();
    let paths_orch = paths.clone();
    let h_orchestrator = spawn_ticker(30, stop.clone(), move |_| {
        run_global_refresh_cycle(&vp_orch, &paths_orch, now_us());
    });

    if duration_secs > 0 {
        let t0 = std::time::Instant::now();
        while t0.elapsed() < Duration::from_secs(duration_secs) {
            std::thread::sleep(Duration::from_millis(100));
        }
        stop.store(true, Ordering::Relaxed);
    } else {
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    }

    let _ = h_adapters.join();
    let _ = h_pred.join();
    let _ = h_hl.join();
    let _ = h_health.join();
    let _ = h_ingress.join();
    let _ = h_binding.join();
    let _ = h_endogenous.join();
    let _ = h_orchestrator.join();
    let _ = h_master.join();

    std::process::exit(0);
}
