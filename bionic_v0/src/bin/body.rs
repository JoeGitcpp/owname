use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use signal_hook::flag as signal_flag;
use std::time::Duration;

use bionic_v0::action_consequence::tick_all_ingress_accumulation;
use bionic_v0::adapter::{
    spawn_registry_driver, AdapterRegistry, ModalityAdapter, RealPhotAdapter, VibAdapter,
};
use bionic_v0::clock::{now_us, spawn_master_clock_hz, spawn_ticker};
use bionic_v0::highlevel::HighLevelPredictor;
use bionic_v0::learned_bias::{
    read_learned_bias_checkpoint, write_learned_bias_checkpoint, LearnedBiasCheckpointPaths,
};
use bionic_v0::live_byte_carrier::spawn_live_byte_ingress;
use bionic_v0::live_byte_closure::LiveByteClosureDriver;
use bionic_v0::phot_camera::{RealCameraFrameProvider, StubRealCameraFrameProvider};
use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_adapter::spawn_vocal_output;
use bionic_v0::VIB_BANDS;
use bionic_v0::{BINDING_REFRESH_HZ, PHOT_CONSEQUENCE_QUIESCENCE_US};

#[derive(Clone, Copy)]
struct BodyRuntimeProfile {
    master_clock_hz: u32,
    live_byte_closure_hz: u32,
    predictor_hz: u32,
    high_level_hz: u32,
    health_hz: u32,
    ingress_hz: u32,
    binding_hz: u32,
    endogenous_hz: u32,
    vocal_hz: u32,
    checkpoint_hz: u32,
    orchestrator_hz: u32,
}

impl BodyRuntimeProfile {
    fn normal() -> Self {
        Self {
            master_clock_hz: 1_000,
            live_byte_closure_hz: 200,
            predictor_hz: 100,
            high_level_hz: 20,
            health_hz: 50,
            ingress_hz: 10,
            binding_hz: BINDING_REFRESH_HZ,
            endogenous_hz: 10,
            vocal_hz: 50,
            checkpoint_hz: 1,
            orchestrator_hz: 30,
        }
    }

    fn solitary_low_power() -> Self {
        Self {
            master_clock_hz: 100,
            live_byte_closure_hz: 2,
            predictor_hz: 20,
            high_level_hz: 5,
            health_hz: 5,
            ingress_hz: 2,
            binding_hz: 1,
            endogenous_hz: 2,
            vocal_hz: 5,
            checkpoint_hz: 1,
            orchestrator_hz: 2,
        }
    }
}

struct BodyArgs {
    runtime_dir: PathBuf,
    duration_secs: u64,
    profile: BodyRuntimeProfile,
    nursery_bias: bool,
    /// Headless (server) mode: path to the Unix socket the gateway connects to.
    /// When set, hardware adapters (cpal mic/camera/speakers) are replaced by
    /// network adapters that receive Senses frames from the gateway.
    headless_socket: Option<PathBuf>,
}

fn parse_body_args(args: &[String]) -> BodyArgs {
    let mut runtime_dir: Option<PathBuf> = None;
    let mut duration_secs: Option<u64> = None;
    let mut low_power = false;
    let mut nursery_bias = false;
    let mut headless_socket: Option<PathBuf> = None;

    let mut iter = args.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        if arg == "--solitary-low-power" {
            low_power = true;
        } else if arg == "--nursery-bias" {
            nursery_bias = true;
        } else if arg == "--headless" {
            // --headless <socket_path>
            if let Some(p) = iter.next() {
                headless_socket = Some(PathBuf::from(p));
            }
        } else if runtime_dir.is_none() {
            runtime_dir = Some(PathBuf::from(arg));
        } else if duration_secs.is_none() {
            duration_secs = arg.parse::<u64>().ok();
        }
    }

    BodyArgs {
        runtime_dir: runtime_dir.unwrap_or_else(|| PathBuf::from("runtime")),
        duration_secs: duration_secs.unwrap_or(0),
        profile: if low_power {
            BodyRuntimeProfile::solitary_low_power()
        } else {
            BodyRuntimeProfile::normal()
        },
        nursery_bias,
        headless_socket,
    }
}

// Serial global refresh cycle carries numeric pressure through probe output
// before snapshot observation.
// Per cleanup spec 2026-04-20, rejected naming layers are not part of the
// body main loop and must not be reintroduced here.
fn run_global_refresh_cycle(vp: &Arc<Viewport>, paths: &Arc<SnapshotPaths>, t: u64) {
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    // B1-3b: drive each active slot's cross-layer predictor AFTER the relation
    // graph is refreshed (its sources are relation-gated). object_err is kept
    // fresh by the sensory adapters' refresh_slot_high_layer.
    for slot_id in 0..bionic_v0::N_OBJECT_SLOTS {
        vp.tick_slot_cross_layer(slot_id, t);
    }
    // Stage 1 (recursive PC depth): grow + tick the residual tower above tier 1,
    // level by level, AFTER all tier-1 errors are fresh. READ-UP ONLY — these
    // tiers never write a prior downward (that is Stage 2). Depth emerges only
    // where the tier below is a permanent explainer, and self-limits on noise.
    vp.tick_cross_layer_tower_sweep(t);
    // C5: re-recognize active objects into the familiar-objects sediment;
    // object identity and the responsible-person bond emerge from familiarity.
    vp.tick_familiar_objects(t);
    // C1: drive the vital core (reserve depletes with exertion, restores with
    // input continuity; bottoms out → Standby). The self-survival drive source.
    vp.tick_vital_core(t);
    // A2: grow the self model around the interoceptive (vital) seed + agency.
    vp.tick_self_model(t);
    // B1-4: arbitrate self-preservation vs caregiver attachment (纠结/牺牲).
    vp.tick_dual_drive(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    vp.tick_evidence_tension_from_pairs(t);
    vp.tick_evidence_tension_from_code_ingress(t);
    vp.refresh_active_reasoning_gap(t);
    vp.tick_prediction_pressure_field_from_substrate(t);
    vp.tick_active_surface_relation_resonance(t);
    vp.refresh_active_surface(t);
    vp.tick_active_surface_probe_chain(t);
    vp.tick_active_surface_chain_memory(t);
    vp.tick_auditory_visual_from_pairs(t);
    vp.tick_vocal_reflex_from_pairs(t);
    vp.tick_auditory_visual_from_fusion(t);
    vp.tick_auditory_visual_expression(t);
    vp.tick_self_development_from_substrate(t);
    vp.refresh_probe_law(t);
    vp.tick_probe_output_from_law(t);
    if write_snapshot(vp, paths).is_err() {
        vp.mark_snapshot_fail();
    }
}

fn register_or_abort(reg: &mut AdapterRegistry, adapter: Box<dyn ModalityAdapter>) {
    if reg.register(adapter).is_err() {
        std::process::abort();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let body_args = parse_body_args(&args);
    let runtime_dir = body_args.runtime_dir;
    let duration_secs = body_args.duration_secs;
    let profile = body_args.profile;
    let headless_socket = body_args.headless_socket;

    std::fs::create_dir_all(&runtime_dir).ok();
    let paths = Arc::new(SnapshotPaths::new(&runtime_dir));
    let learned_bias_paths = Arc::new(LearnedBiasCheckpointPaths::new(&runtime_dir));

    let vp = if body_args.nursery_bias {
        Arc::new(Viewport::with_birth_sensitivity(Arc::new(
            bionic_v0::evolution_priors::BiasPriors::nursery_newborn(),
        )))
    } else {
        Arc::new(Viewport::new())
    };
    if let Ok(state) = read_learned_bias_checkpoint(&learned_bias_paths.final_path) {
        *vp.learned_bias.write().unwrap() = state;
    }
    // Nursery boot: cap action_suppression so the endogenous baseline can cross
    // the 0.40 vocal gate. Without this, accumulated suppression from prior runs
    // (observed at 0.155) keeps reflex_pressure at ~0.34 — just below the gate —
    // and the body never makes its first sound (0×x=0 bootstrapping deadlock).
    if body_args.nursery_bias {
        let mut bias = vp.learned_bias.write().unwrap();
        bias.action_suppression = bias.action_suppression.min(0.02);
    }
    // C5: restore familiar objects so the responsible-person bond is
    // re-recognized after restart (死后保留).
    let familiar_paths =
        bionic_v0::familiar_objects::FamiliarObjectsCheckpointPaths::new(&runtime_dir);
    if let Ok(state) =
        bionic_v0::familiar_objects::read_familiar_objects_checkpoint(&familiar_paths.final_path)
    {
        *vp.familiar_objects.write().unwrap() = state;
    }
    // C1: restore the survival reserve (a body that was running low comes back
    // still tired, not magically reset).
    let vital_paths = bionic_v0::vital_core::VitalCoreCheckpointPaths::new(&runtime_dir);
    if let Ok(state) =
        bionic_v0::vital_core::read_vital_core_checkpoint(&vital_paths.final_path)
    {
        *vp.vital_core.write().unwrap() = state;
    }
    // A2: restore the self model (a body that grew a self comes back as itself).
    let self_model_paths = bionic_v0::self_model::SelfModelCheckpointPaths::new(&runtime_dir);
    if let Ok(state) =
        bionic_v0::self_model::read_self_model_checkpoint(&self_model_paths.final_path)
    {
        *vp.self_model.write().unwrap() = state;
    }
    // Phoneme memory: restore vocal proto pool so learned sound→motor mappings
    // survive across restarts (babble sessions are cumulative, not per-run).
    let phoneme_memory_path = runtime_dir.join("phoneme_memory.bin");
    if let Ok(bytes) = std::fs::read(&phoneme_memory_path) {
        if let Some(pm) = bionic_v0::phoneme_memory::PhonemeMemory::deserialize(&bytes) {
            *vp.phoneme_memory.write().unwrap() = pm;
        }
    }
    let stop = Arc::new(AtomicBool::new(false));
    // Register SIGTERM so the OS-level kill causes a graceful flush rather than
    // instant termination.  signal_flag::register atomically sets `stop` on
    // SIGTERM delivery, which exits the ticker loops and falls through to the
    // final checkpoint-write block below.
    signal_flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop))
        .expect("register SIGTERM handler");

    let h_master = spawn_master_clock_hz(vp.clone(), stop.clone(), profile.master_clock_hz);
    let h_live_byte = spawn_live_byte_ingress(
        vp.clone(),
        stop.clone(),
        runtime_dir.join("live_bytes").join("in.frames"),
    );

    let h_live_byte_closure = {
        let vp_live_byte_closure = vp.clone();
        let live_byte_closure = Arc::new(Mutex::new(LiveByteClosureDriver::new(
            runtime_dir.join("live_bytes").join("out.frames"),
        )));
        let live_byte_closure_ref = live_byte_closure.clone();
        spawn_ticker(profile.live_byte_closure_hz, stop.clone(), move |_| {
            let _ = live_byte_closure_ref
                .lock()
                .unwrap()
                .tick(&vp_live_byte_closure, now_us());
        })
    };

    // All sensory I/O goes through the registry. body.rs is oblivious to
    // which modalities are present — only that the registry exists.
    //
    // Headless mode (--headless <socket>): NetSensesAdapter replaces the
    // hardware adapters. It reads compact binary Senses frames from the
    // Unix socket written by aborn-gateway and pushes them onto the shared
    // axes.  The same socket carries vocal PCM back to the gateway.
    //
    // Hardware mode (default): real mic + camera via cpal/nokhwa.
    use std::sync::{Arc as StdArc, Mutex as StdMutex};
    use std::os::unix::net::UnixStream as StdUnixStream;
    type VocalTx = StdArc<StdMutex<Option<StdUnixStream>>>;

    let net_vocal_tx: VocalTx;
    let h_adapters;
    if let Some(socket_path) = headless_socket {
        let (net_adapter, vt) = bionic_v0::net_adapter::NetSensesAdapter::new(socket_path);
        net_vocal_tx = vt;
        h_adapters = spawn_registry_driver(vp.clone(), stop.clone(), move || {
            let mut reg = AdapterRegistry::new();
            register_or_abort(&mut reg, Box::new(net_adapter));
            reg
        });
    } else {
        net_vocal_tx = StdArc::new(StdMutex::new(None));
        let vp_reg = vp.clone();
        h_adapters = spawn_registry_driver(vp.clone(), stop.clone(), move || {
            let mut reg = AdapterRegistry::new();
            // VIB: real mic. Owns its cpal stream inside this thread.
            register_or_abort(&mut reg, Box::new(VibAdapter::new(&vp_reg)));
            // PHOT: attempt to bind the default monocular camera. On success
            // the real hardware path is live; on failure phot_health surfaces
            // FAULT immediately via the stub — never a silent fallback to
            // synthetic data.
            let cam: Box<dyn bionic_v0::phot_camera::CameraFrameProvider> =
                match RealCameraFrameProvider::try_new() {
                    Ok(p) => Box::new(p),
                    Err(_) => Box::new(StubRealCameraFrameProvider::try_new()),
                };
            register_or_abort(
                &mut reg,
                Box::new(RealPhotAdapter::with_birth_sensitivity(
                    cam,
                    vp_reg.birth_sensitivity(),
                )),
            );
            reg
        });
    };

    // AR-2 predictors for ALL active axes (hearing + vision + interoception),
    // 100 Hz. Previously only VIB_BANDS (4) were covered; expanding to AXIS_COUNT
    // (34) means visual and interoceptive axes are now predicted — they produce
    // err signals, feed into the relation graph, and enter the abstraction tower.
    // This is Stage 0 of the interoceptive substrate unification: zero behavior
    // change to the boxes; the predictors simply observe whatever each axis holds.
    let predictors: Arc<Mutex<Vec<Ar2Predictor>>> =
        Arc::new(Mutex::new((0..bionic_v0::AXIS_COUNT).map(Ar2Predictor::new).collect()));
    let vp_pred = vp.clone();
    let pred_ref = predictors.clone();
    let h_pred = spawn_ticker(profile.predictor_hz, stop.clone(), move |_| {
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
    let h_hl = spawn_ticker(profile.high_level_hz, stop.clone(), move |_| {
        let mut hlp = hl_ref.lock().unwrap();
        hlp.tick(&vp_hl);
    });

    // Viewport health recompute, 50 Hz — observer only renders this value.
    let vp_health = vp.clone();
    let h_health = spawn_ticker(profile.health_hz, stop.clone(), move |_| {
        vp_health.update_health_state(now_us());
    });

    // Unified ingress accumulation ticker, 10 Hz.
    // Closes quiescent consequence windows across all modalities and attempts
    // auto-pairing for newly closed windows. Single authoritative path for
    // quiescence-based window closure.
    let vp_ingress = vp.clone();
    let h_ingress = spawn_ticker(profile.ingress_hz, stop.clone(), move |_| {
        tick_all_ingress_accumulation(&vp_ingress, now_us(), PHOT_CONSEQUENCE_QUIESCENCE_US);
    });

    // Binding maintenance refresh ticker, 5 Hz by default.
    let vp_binding = vp.clone();
    let h_binding = spawn_ticker(profile.binding_hz, stop.clone(), move |_| {
        let t = now_us();
        vp_binding.tick_binding_maintenance(t);
        vp_binding.tick_binding_resonance(t);
    });

    // Endogenous activity ticker, 10 Hz.
    // Err-driven attention rebalancing (self-regulation law §2.2) and
    // quiet-period history replay (endogenous activity law §2.1).
    // Both are substrate-level mechanisms — no world content or meaning.
    let vp_endo = vp.clone();
    let h_endogenous = spawn_ticker(profile.endogenous_hz, stop.clone(), move |_| {
        let t = now_us();
        vp_endo.tick_err_driven_attention(t);
        vp_endo.tick_quiet_history_replay(t);
        vp_endo.tick_life_closure_producers(t);
        vp_endo.tick_adaptive_stability(t);
    });

    // Vocal substrate ticker, 50 Hz. Body-side execution only: learned vocal
    // curves update continuously without text, TTS, or request/response logic.
    // In headless mode, also sample the vocal organ and send PCM to the gateway.
    let vp_vocal = vp.clone();
    let vt_for_ticker = net_vocal_tx.clone();
    let h_vocal = spawn_ticker(profile.vocal_hz, stop.clone(), move |_| {
        let t = now_us();
        // 改造6: external sound is observed into the target (no voicing here),
        // then ONE unified outlet combines all drive sources and voices once.
        vp_vocal.tick_vocal_target_from_vib(t);
        vp_vocal.tick_vocal_drive_field(t);
        vp_vocal.tick_vocal_memory(t);
        // Headless mode: send vocal PCM to gateway via the shared socket.
        // In hardware mode the tx slot is None and this is a no-op.
        {
            let guard = vt_for_ticker.lock().unwrap();
            if guard.is_some() {
                drop(guard); // release before the (slightly expensive) PCM synthesis
                let samples = bionic_v0::net_adapter::sample_vocal_pcm(&vp_vocal);
                bionic_v0::net_adapter::send_vocal_frame(&vt_for_ticker, &samples);
            }
        }
    });

    // Hardware vocal output (cpal speakers). In headless mode net_vocal_tx
    // carries the audio back to the gateway instead; this ticker is a no-op
    // (spawn_vocal_output checks for audio device and silently skips if absent).
    let h_vocal_output = spawn_vocal_output(vp.clone(), stop.clone());

    // AudioAdapter: raw waveform capture (tier-0 audio)
    let h_audio = {
        let vp_audio = vp.clone();
        let audio_buf = vp_audio.audio_raw.clone();
        let stop_audio = stop.clone();
        let h_audio = std::thread::spawn(move || {
            let _adapter = bionic_v0::audio_adapter::AudioAdapter::new(audio_buf);
            // Keep the cpal stream alive — it runs in its own callback thread
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if stop_audio.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
        h_audio
    };

    // Headless-only: render Gaussian visual field and push to gateway at 30 Hz.
    // send_gauss() reads the live Gaussian field (populated each senses frame by
    // NetSensesAdapter::tick → tick_gaussian_field_from_net) and renders a soft
    // organic splatted image of what the body currently perceives.
    // Falls back silently to no-op when the field has no Gaussians yet.
    let h_surface = {
        let vt_surf = net_vocal_tx.clone();
        let vp_surf = vp.clone();
        let mut sender = bionic_v0::net_adapter::SurfaceSender::new();
        spawn_ticker(30, stop.clone(), move |_| {
            sender.send_gauss(&vp_surf, &vt_surf);
        })
    };

    let h_learned_bias_checkpoint = {
        let vp_checkpoint = vp.clone();
        let paths_checkpoint = learned_bias_paths.clone();
        // Life-state checkpoints written periodically too, so the living
        // process can be observed over time (not only at exit).
        let fam_cp = bionic_v0::familiar_objects::FamiliarObjectsCheckpointPaths::new(&runtime_dir);
        let vital_cp = bionic_v0::vital_core::VitalCoreCheckpointPaths::new(&runtime_dir);
        let self_cp = bionic_v0::self_model::SelfModelCheckpointPaths::new(&runtime_dir);
        let phoneme_cp = runtime_dir.join("phoneme_memory.bin");
        spawn_ticker(profile.checkpoint_hz, stop.clone(), move |_| {
            let state = vp_checkpoint.read_learned_bias();
            let _ = write_learned_bias_checkpoint(&state, &paths_checkpoint);
            let _ = bionic_v0::familiar_objects::write_familiar_objects_checkpoint(
                &vp_checkpoint.read_familiar_objects(),
                &fam_cp,
            );
            let _ = bionic_v0::vital_core::write_vital_core_checkpoint(
                &vp_checkpoint.read_vital_core(),
                &vital_cp,
            );
            let _ = bionic_v0::self_model::write_self_model_checkpoint(
                &vp_checkpoint.read_self_model(),
                &self_cp,
            );
            let pm_bytes = vp_checkpoint.phoneme_memory.read().unwrap().serialize();
            let _ = std::fs::write(&phoneme_cp, &pm_bytes);
        })
    };

    // Global orchestrator: attention → relations → snapshot (30 Hz).
    //
    // Replaces the three independent tickers that previously ran at 30/20/30 Hz.
    // Single serial cycle ensures relations read same-round attention shares and
    // the snapshot always captures post-refresh state.
    let vp_orch = vp.clone();
    let paths_orch = paths.clone();
    let h_orchestrator = spawn_ticker(profile.orchestrator_hz, stop.clone(), move |_| {
        run_global_refresh_cycle(&vp_orch, &paths_orch, now_us());
    });

    if duration_secs > 0 {
        let t0 = std::time::Instant::now();
        while t0.elapsed() < Duration::from_secs(duration_secs) {
            std::thread::sleep(Duration::from_millis(100));
        }
        stop.store(true, Ordering::Relaxed);
    } else {
        // Poll stop flag at 100 ms so a SIGTERM (or any other flag-setter)
        // is noticed quickly and falls through to the checkpoint flush below.
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    let _ = h_surface.join();
    let _ = h_adapters.join();
    let _ = h_pred.join();
    let _ = h_hl.join();
    let _ = h_health.join();
    let _ = h_ingress.join();
    let _ = h_binding.join();
    let _ = h_endogenous.join();
    let _ = h_vocal.join();
    let _ = h_vocal_output.join();
    let _ = h_audio.join();
    let _ = h_learned_bias_checkpoint.join();
    let _ = h_orchestrator.join();
    let _ = h_live_byte.join();
    let _ = h_live_byte_closure.join();
    let _ = h_master.join();

    let state = vp.read_learned_bias();
    let _ = write_learned_bias_checkpoint(&state, &learned_bias_paths);
    // C5: persist familiar objects (死后保留).
    let _ = bionic_v0::familiar_objects::write_familiar_objects_checkpoint(
        &vp.read_familiar_objects(),
        &familiar_paths,
    );
    // C1: persist the survival reserve (死后保留).
    let _ = bionic_v0::vital_core::write_vital_core_checkpoint(
        &vp.read_vital_core(),
        &vital_paths,
    );
    // A2: persist the self model (死后保留).
    let _ = bionic_v0::self_model::write_self_model_checkpoint(
        &vp.read_self_model(),
        &self_model_paths,
    );
    // L1-4: persist phoneme proto pool (vocal repertoire survives restart).
    {
        let pm_bytes = vp.phoneme_memory.read().unwrap().serialize();
        let _ = std::fs::write(&phoneme_memory_path, &pm_bytes);
    }

    std::process::exit(0);
}
