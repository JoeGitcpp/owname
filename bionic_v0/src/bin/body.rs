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
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc_cross_layer::{axis_predictor_bank, CrossLayerPredictor};
use bionic_v0::vocal_adapter::spawn_vocal_output;
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
fn run_global_refresh_cycle(
    vp: &Arc<Viewport>,
    paths: &Arc<SnapshotPaths>,
    xlayer_axis: &Arc<Mutex<Vec<CrossLayerPredictor>>>,
    t: u64,
) {
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    // (Phase 3 cutover) per-slot cross-layer + recursive tower removed —
    // replaced by perturbation_graph relations. The axis-level cross-layer
    // bank below remains active (axis tier, not slot tier).
    // A2 (VPC mechanism 5, prior write-back leg): tick the axis-level
    // cross-layer bank on the serial global-refresh rhythm — strictly slower
    // than the 100 Hz AR-2 ticker it explains. Each predictor reads its
    // target's err + sparse source observations via the viewport only, updates
    // its own weights locally, and (after warmup) writes a clamped prior back
    // so the low layer's next prediction can use it. The learned state is
    // mirrored into the viewport so the snapshot persists it across restarts.
    {
        let mut bank = xlayer_axis.lock().unwrap_or_else(|e| e.into_inner());
        for clp in bank.iter_mut() {
            clp.tick(vp);
        }
        vp.store_axis_xlayer_states(bank.iter().map(|c| c.to_state()).collect());
    }
    // Phase 3 replacement for tick_familiar_objects: mirror perturbation_graph
    // most_bound + active onto INTERO_ATTACHMENT + INTERO_PRESENCE so
    // dual_drive + vpc_cross_layer keep getting their attachment/presence input.
    // Cognitive middle layers (SDD 2026-06-27): L0→L4 pipeline.
    // Runs AFTER refresh_global_attention (reads axis observations + surprise)
    // and BEFORE tick_perturbation_intero (so downstream consumers can
    // eventually read cognitive state).
    vp.tick_cognitive_pipeline(t);
    vp.tick_pg_eligibility_decay(t); // decay edge + relation elig traces every tick
    vp.tick_phot_sequence_boundary(t); // visual sequence elig decay (mirrors vocal side)
    vp.tick_perturbation_intero(t);
    // C1: drive the vital core (reserve depletes with exertion, restores with
    // input continuity; bottoms out → Standby). The self-survival drive source.
    vp.tick_vital_core(t);
    // A2: grow the self model around the interoceptive (vital) seed + agency.
    vp.tick_self_model(t);
    // B1-4: arbitrate self-preservation vs caregiver attachment (纠结/牺牲).
    vp.tick_dual_drive(t);
    // Phase C2: arbitrate intention from the perturbation_graph anchor.
    // Runs BEFORE refresh_global_reasoning so the reasoning fabric can
    // see the active intention mark.
    vp.tick_intention_arbitrator(t);
    // Phase D1: spontaneous vocal expression — when intention points at a
    // node with a VocalProfile and pressure is high, push it as vocal_target
    // so the servo drives toward expressing it. Runs after the arbitrator so
    // the current_target is fresh.
    vp.tick_spontaneous_vocal_expression(t);
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
    // 推论1: the body never emits human-readable text. The default panic hook
    // would print the panic payload to stderr before catch_unwind absorbs it;
    // replace it with a no-op so an absorbed tick panic stays purely on the
    // tier-0 thread-fault axis.
    std::panic::set_hook(Box::new(|_| {}));

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
        let maturity = state.developmental_maturity;
        *vp.learned_bias.write().unwrap() = state;
        // A3: the developmental clock lives on the Viewport itself; restore it
        // from the checkpoint so maturation survives restart (factory_dna).
        vp.restore_maturity(maturity);
    }
    // B2: the former nursery-only one-shot suppression clamp is gone. The
    // 0×x=0 bootstrapping deadlock is now dissolved structurally for every
    // body: action_suppression carries a pure time half-life
    // (learned_bias::decay_vocal_suppression, driven by the body's own
    // closure-observation cadence) and the endogenous baseline keeps a
    // multiplicative damping floor (vocal_reflex::SUPPRESSION_DAMP_FLOOR).
    // (Phase 3 cutover) familiar_objects checkpoint restore removed —
    // perturbation_graph.bin loaded above carries the analogous identity state.
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
    // Vocal memory: restore motor prototypes (breath/pitch/resonance parameters)
    // accumulated by the imitation loop.  Without this, mn resets to 0 on every
    // nursery restart and all converged motor knowledge is lost.
    let vocal_memory_path = runtime_dir.join("vocal_memory.bin");
    if let Ok(bytes) = std::fs::read(&vocal_memory_path) {
        if let Some(vm) = bionic_v0::vocal_memory::VocalMemoryState::deserialize(&bytes) {
            *vp.vocal_memory.write().unwrap() = vm;
        }
    }
    // Phase 2: target→correction recall persists across sessions so learned vowel
    // corrections accumulate instead of re-solving from zero every restart.
    let vocal_recall_path = runtime_dir.join("vocal_target_recall.bin");
    if let Ok(bytes) = std::fs::read(&vocal_recall_path) {
        if let Some(vr) = bionic_v0::vocal_target_recall::VocalTargetRecall::deserialize(&bytes) {
            *vp.vocal_target_recall.write().unwrap() = vr;
        }
    }
    // #8 sequence layer: load the transition edges sidecar so learned vocal ORDER
    // accumulates across sessions (separate file → no SNAPSHOT_VERSION bump).
    let vocal_sequence_path = runtime_dir.join("vocal_sequence.bin");
    if let Ok(bytes) = std::fs::read(&vocal_sequence_path) {
        if let Some(vs) = bionic_v0::vocal_sequence::VocalSequence::deserialize(&bytes) {
            *vp.vocal_sequence.write().unwrap() = vs;
        }
    }
    // Q2/3 visual impression layer: sister sidecar to vocal_sequence.bin.
    // Persists the scene-fingerprint transition table + recent impression
    // buffer so visual ORDER and the body's "刚刚看过的画面" buffer survive
    // restart. Self-versioned, so loading does not bump SNAPSHOT_VERSION.
    let phot_sequence_path = runtime_dir.join("phot_sequence.bin");
    if let Ok(bytes) = std::fs::read(&phot_sequence_path) {
        if let Some(ps) = bionic_v0::phot_sequence::PhotSequence::deserialize(&bytes) {
            *vp.phot_sequence.write().unwrap() = ps;
        }
    }
    // Phase 3 perturbation_graph: load the sparse tier-0 perturbation graph
    // sidecar so the live-neuron node/edge state accumulates across sessions.
    // Self-versioned (PGRA magic), so loading never bumps SNAPSHOT_VERSION.
    // Stage 0 wiring: persistence only — no tick driver, no consumers yet.
    let perturbation_graph_path = runtime_dir.join("perturbation_graph.bin");
    if let Ok(bytes) = std::fs::read(&perturbation_graph_path) {
        if let Some(pg) =
            bionic_v0::perturbation_graph::PerturbationGraph::deserialize(&bytes)
        {
            *vp.perturbation_graph.write().unwrap() = pg;
        }
    }
    // Cognitive pipeline pattern memory: restore learned L4 patterns so
    // accumulated experiential prototypes survive restart.
    let cognitive_patterns_path = runtime_dir.join("cognitive_patterns.bin");
    if let Ok(bytes) = std::fs::read(&cognitive_patterns_path) {
        let mut pipe = vp.cognitive_pipeline.write().unwrap();
        if !pipe.restore_patterns(&bytes) {
            // malformed cognitive_patterns.bin — continue with empty pattern memory
        }
    }
    let stop = Arc::new(AtomicBool::new(false));
    // Register SIGTERM so the OS-level kill causes a graceful flush rather than
    // instant termination.  signal_flag::register atomically sets `stop` on
    // SIGTERM delivery, which exits the ticker loops and falls through to the
    // final checkpoint-write block below.
    if signal_flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stop)).is_err() {
        std::process::exit(1);
    }

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
        spawn_ticker(vp.clone(), profile.live_byte_closure_hz, stop.clone(), move |_| {
            // Poison recovery: a caught panic elsewhere must not let this lock
            // kill the rhythm on every later tick (state is re-derived per tick).
            let _ = live_byte_closure_ref
                .lock()
                .unwrap_or_else(|e| e.into_inner())
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
        let (net_adapter, vt) =
            bionic_v0::net_adapter::NetSensesAdapter::new(socket_path, vp.clone());
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

    // AR-2 predictors for ALL active axes (hearing + vision + interoception +
    // code + gauss summaries), 100 Hz. Originally only VIB_BANDS were covered;
    // covering all AXIS_COUNT (162) axes means every tier-0 axis is predicted —
    // each produces err signals, feeds the relation graph, and enters the
    // abstraction tower. The predictors simply observe whatever each axis holds.
    let predictors: Arc<Mutex<Vec<Ar2Predictor>>> =
        Arc::new(Mutex::new((0..bionic_v0::AXIS_COUNT).map(Ar2Predictor::new).collect()));

    // A2 (VPC mechanism 5): the axis-level cross-layer bank — the high-layer
    // half of the keystone loop, built from a hard-coded SPARSE prior table
    // (axis_predictor_bank). Restored from the last snapshot when one exists
    // so learned source weights survive a restart (死后保留); a missing,
    // older-version, or structurally incompatible snapshot yields a fresh bank.
    let xlayer_axis: Arc<Mutex<Vec<CrossLayerPredictor>>> = {
        let snap = read_snapshot(&paths.final_path).ok();
        // (Phase 3 cutover) per-slot cross-layer + tower restore removed —
        // SlotCrossLayer deleted, snapshot v61 no longer carries slot_xlayers /
        // slot_towers. The axis-level bank below is the only cross-layer state
        // that survives across restarts now.
        let restored: Vec<CrossLayerPredictor> = snap
            .map(|snap| {
                snap.axis_xlayers
                    .iter()
                    .filter_map(CrossLayerPredictor::from_state)
                    .collect()
            })
            .unwrap_or_default();
        let bank = if restored.is_empty() {
            axis_predictor_bank()
        } else {
            restored
        };
        Arc::new(Mutex::new(bank))
    };
    let vp_pred = vp.clone();
    let pred_ref = predictors.clone();
    let h_pred = spawn_ticker(vp.clone(), profile.predictor_hz, stop.clone(), move |_| {
        // Poison recovery: predictor state updates incrementally each tick, so
        // resuming from a poisoned guard self-corrects within a few ticks.
        let mut ps = pred_ref.lock().unwrap_or_else(|e| e.into_inner());
        for p in ps.iter_mut() {
            p.tick(&vp_pred);
        }
        vp_pred.predictor_ticks.fetch_add(1, Ordering::Relaxed);
    });

    // High-level predictor, 20 Hz
    let hl = Arc::new(Mutex::new(HighLevelPredictor::new()));
    let vp_hl = vp.clone();
    let hl_ref = hl.clone();
    let h_hl = spawn_ticker(vp.clone(), profile.high_level_hz, stop.clone(), move |_| {
        let mut hlp = hl_ref.lock().unwrap_or_else(|e| e.into_inner());
        hlp.tick(&vp_hl);
    });

    // Viewport health recompute, 50 Hz — observer only renders this value.
    let vp_health = vp.clone();
    let h_health = spawn_ticker(vp.clone(), profile.health_hz, stop.clone(), move |_| {
        vp_health.update_health_state(now_us());
    });

    // Unified ingress accumulation ticker, 10 Hz.
    // Closes quiescent consequence windows across all modalities and attempts
    // auto-pairing for newly closed windows. Single authoritative path for
    // quiescence-based window closure.
    let vp_ingress = vp.clone();
    let h_ingress = spawn_ticker(vp.clone(), profile.ingress_hz, stop.clone(), move |_| {
        tick_all_ingress_accumulation(&vp_ingress, now_us(), PHOT_CONSEQUENCE_QUIESCENCE_US);
    });

    // Binding maintenance refresh ticker, 5 Hz by default.
    let vp_binding = vp.clone();
    let h_binding = spawn_ticker(vp.clone(), profile.binding_hz, stop.clone(), move |_| {
        let t = now_us();
        vp_binding.tick_binding_maintenance(t);
        vp_binding.tick_binding_resonance(t);
        // LIVE symbol grounding (#11): when a heard sound (active VIB slot)
        // co-occurs with a seen object (active PHOT slot) AND a real, recent
        // earned action-consequence Paired pair exists, form/reinforce a binding.
        // Without this the deployed body only ever DECAYED bindings and never
        // formed any — the formation entry points were called only from the
        // offline curriculum harness. All maturity-gated precision, liveness, and
        // anti-runaway (one-binding-per-pair) gates live inside the call.
        vp_binding.tick_form_binding_from_cooccurrence(t);
        // Cross-modal AV-action grounding (companion to the VIB side above):
        // when one slot carries co-located VIB and PHOT evidence with salient
        // motion AND the same recent Paired pair earned the co-occurrence,
        // form a (seen-action → object-slot) binding. Together with the VIB
        // binding the slot becomes a rendezvous, so next time the body hears
        // the sound it can recover the action hash via
        // lookup_associated_patterns(slot) — "compressed direct understanding"
        // of the AV pair on re-encounter. Per-pair idempotence runs through
        // last_av_action_pair_consumed, independent of the VIB side.
        vp_binding.tick_form_av_action_binding(t);
    });

    // Endogenous activity ticker, 10 Hz.
    // Err-driven attention rebalancing (self-regulation law §2.2) and
    // quiet-period history replay (endogenous activity law §2.1).
    // Both are substrate-level mechanisms — no world content or meaning.
    let vp_endo = vp.clone();
    let h_endogenous = spawn_ticker(vp.clone(), profile.endogenous_hz, stop.clone(), move |_| {
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
    let h_vocal = spawn_ticker(vp.clone(), profile.vocal_hz, stop.clone(), move |_| {
        let t = now_us();
        // 改造6: external sound is observed into the target (no voicing here),
        // then ONE unified outlet combines all drive sources and voices once.
        vp_vocal.tick_vocal_target_from_vib(t);
        // Social turn-taking contingency → developmental clock. Runs every vocal
        // tick (incl. silent ones) so the ambient base rate is estimated, not
        // only the post-vocal rate. Must follow tick_vocal_target_from_vib so the
        // VIB self/external classification for this tick is current.
        vp_vocal.tick_social_contingency(t);
        vp_vocal.tick_vocal_drive_field(t);
        vp_vocal.tick_vocal_memory(t);
        vp_vocal.tick_vocal_sequence_boundary(t); // #8 Stage 1: decay traces + close utterances
        // Q1c: during long enough vocal silence the persistent edge table
        // computes its planned next-K-utterance chain so old impressions stay
        // "actively reproduced" in the prediction system. Pure-read on the
        // sequence; the chain length is published via atomic counters for
        // observers / downstream consumers.
        vp_vocal.tick_vocal_sequence_quiet_replay(t);
        // P3: hear our own voice through the same analyzer the target uses —
        // in EVERY mode. This reafference (self_vocal_bands → vocal_acoustic_*
        // distance) is the efference copy of the body's own voicing; gating it
        // on the gateway socket left hardware-mode bodies structurally deaf to
        // themselves, so the heard-self evidence gate (07a8a9a) froze efficacy
        // and the developmental clock forever. Synthesis here uses a
        // thread-local synthesizer reading organ state — independent of the
        // cpal speaker path, no double-advance. The reafference stays tier-0
        // numeric state; human-readable readouts belong to the observer only.
        let samples = bionic_v0::net_adapter::sample_vocal_pcm(&vp_vocal);
        vp_vocal.observe_self_vocal_pcm(t, &samples);
        // Headless mode: additionally send the same PCM to the gateway via the
        // shared socket. In hardware mode the tx slot is None and this no-ops
        // (the cpal speaker thread carries the audible output instead).
        {
            let guard = vt_for_ticker.lock().unwrap_or_else(|e| e.into_inner());
            if guard.is_some() {
                drop(guard); // release before the socket write
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
        spawn_ticker(vp.clone(), 30, stop.clone(), move |_| {
            sender.send_gauss(&vp_surf, &vt_surf);
        })
    };

    // Visual sensorimotor self-play ("neck crib"), 30 Hz. Closes the vision
    // perception→action→observation loop that is otherwise permanently open:
    // the body commands its OWN lateral eye-translation (a genuine efference
    // copy), so per-Gaussian PARALLAX depth becomes observable. Without this,
    // every live path reads translation_term()=None and per-Gaussian depth is a
    // 0×x=0 dead signal (update_depth(None) skips forever) — the visual analog
    // of the vocal self-hear loop the body already runs.
    //
    // Gated on the ABSENCE of live external vision (phot_health Fault/Stale): a
    // camera-less or dead-camera body runs the crib and earns depth from its own
    // motion; the instant a real camera/browser delivers frames (Ok/Quiet) the
    // crib stands down, so it never fights the shared Gaussian field that the
    // RealPhotAdapter / NetSensesAdapter own. (A deployed browser body — which
    // can never supply real parallax — therefore relies on its real input and
    // does not run the crib; giving it a SEPARATE synthetic depth field is a
    // deliberate follow-up, not done here.)
    let h_neck = {
        let vp_neck = vp.clone();
        let mut neck = bionic_v0::virtual_neck::VirtualNeck::new_default();
        let mut prev_b = [0.0f64; bionic_v0::VIS_REGIONS];
        let mut prev_m = [0.0f64; bionic_v0::VIS_REGIONS];
        spawn_ticker(vp.clone(), 30, stop.clone(), move |_| {
            use bionic_v0::viewport::HealthState;
            if matches!(
                vp_neck.read_phot_health(),
                HealthState::Fault | HealthState::Stale
            ) {
                let t = now_us();
                // Gaze as a first-class drive: the body chooses where to look from
                // its own visual prediction error (curiosity), with an endogenous
                // maturity-annealed exploratory floor. Command it, then drive the
                // neck — replacing the old scripted sine wobble.
                let (bias, floor) = vp_neck.compute_gaze_drive(t);
                neck.command_gaze(bias, floor);
                let (nb, nm) = bionic_v0::virtual_neck::tick_virtual_neck_into_viewport(
                    &vp_neck, &mut neck, &prev_b, &prev_m, t,
                );
                prev_b = nb;
                prev_m = nm;
            }
        })
    };

    let h_learned_bias_checkpoint = {
        let vp_checkpoint = vp.clone();
        let paths_checkpoint = learned_bias_paths.clone();
        // Life-state checkpoints written periodically too, so the living
        // process can be observed over time (not only at exit).
        // (Phase 3 cutover) familiar_objects checkpoint path removed.
        let vital_cp = bionic_v0::vital_core::VitalCoreCheckpointPaths::new(&runtime_dir);
        let self_cp = bionic_v0::self_model::SelfModelCheckpointPaths::new(&runtime_dir);
        let phoneme_cp = runtime_dir.join("phoneme_memory.bin");
        let vocal_mem_cp = runtime_dir.join("vocal_memory.bin");
        let vocal_recall_cp = runtime_dir.join("vocal_target_recall.bin");
        let vocal_seq_cp = runtime_dir.join("vocal_sequence.bin");
        let phot_seq_cp = runtime_dir.join("phot_sequence.bin");
        let cognitive_patterns_cp = runtime_dir.join("cognitive_patterns.bin");
        let perturbation_graph_cp = runtime_dir.join("perturbation_graph.bin");
        let vocal_diag_cp = runtime_dir.join("vocal_diag.csv");
        spawn_ticker(vp.clone(), profile.checkpoint_hz, stop.clone(), move |_| {
            let state = vp_checkpoint.read_learned_bias();
            let _ = write_learned_bias_checkpoint(&state, &paths_checkpoint);
            // 体检 telemetry dump: numeric trace of the vocal loop so a stalled
            // life is diagnosable from outside (burst? heard-self? evidence?
            // efficacy/maturity moving?). Append-only CSV; the body never reads
            // it back — observer-side instrumentation only.
            {
                let d = *vp_checkpoint
                    .vocal_diag
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                let line = format!(
                    "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{:.6},{},{},{:.6}\n",
                    now_us(),
                    d.outlet_ticks,
                    d.bursts,
                    d.closures,
                    d.closures_vocalized,
                    d.closures_with_evidence,
                    d.last_drive_pressure,
                    d.last_self_bands_sum,
                    d.last_fit,
                    state.vocal_efficacy,
                    state.prediction_familiarity,
                    state.action_suppression,
                    state.developmental_maturity,
                    // Phase 0 diagnosis columns
                    d.region_b_ticks,
                    d.last_self_voiced as u8,
                    d.last_shape_dist,
                    d.last_learning_gap,
                    d.last_target_active_energy,
                    d.last_target_mel64_sum,
                    d.last_target_bands_sum,
                    d.last_servo_pitch_off,
                    d.last_servo_res_off,
                    d.last_self_match,
                    d.last_organ_phase,
                    d.last_breath_drive,
                    d.last_since_vocal_us,
                    d.recall_entries,
                    d.last_social_contingency,
                );
                let new_file = !vocal_diag_cp.exists();
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&vocal_diag_cp)
                {
                    use std::io::Write as _;
                    if new_file {
                        let _ = f.write_all(b"t_us,outlet_ticks,bursts,closures,closures_vocalized,closures_with_evidence,drive_pressure,self_bands_sum,fit,vocal_efficacy,prediction_familiarity,action_suppression,developmental_maturity,region_b_ticks,self_voiced,shape_dist,learning_gap,target_energy,target_mel64_sum,target_bands_sum,servo_pitch_off,servo_res_off,self_match,organ_phase,breath_drive,since_vocal_us,recall_entries,social_contingency\n");
                    }
                    let _ = f.write_all(line.as_bytes());
                }
            }
            // (Phase 3 cutover) familiar_objects checkpoint write removed.
            let _ = bionic_v0::vital_core::write_vital_core_checkpoint(
                &vp_checkpoint.read_vital_core(),
                &vital_cp,
            );
            let _ = bionic_v0::self_model::write_self_model_checkpoint(
                &vp_checkpoint.read_self_model(),
                &self_cp,
            );
            let pm_bytes = vp_checkpoint
                .phoneme_memory
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize();
            bionic_v0::atomic_write_or_warn("checkpoint phoneme_memory", &phoneme_cp, &pm_bytes);
            // Persist motor prototypes so imitation learning accumulates across
            // nursery restarts (without this mn resets to 0 every restart).
            let vm_bytes = vp_checkpoint
                .vocal_memory
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize();
            bionic_v0::atomic_write_or_warn("checkpoint vocal_memory", &vocal_mem_cp, &vm_bytes);
            let vr_bytes = vp_checkpoint
                .vocal_target_recall
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize();
            bionic_v0::atomic_write_or_warn("checkpoint vocal_target_recall", &vocal_recall_cp, &vr_bytes);
            let vs_bytes = vp_checkpoint
                .vocal_sequence
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize();
            bionic_v0::atomic_write_or_warn("checkpoint vocal_sequence", &vocal_seq_cp, &vs_bytes);
            let ps_bytes = vp_checkpoint
                .phot_sequence
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize();
            bionic_v0::atomic_write_or_warn("checkpoint phot_sequence", &phot_seq_cp, &ps_bytes);
            // Phase 3 perturbation_graph Stage 0: persist the (currently empty
            // until Stage 1 wires the visual frontend) sparse tier-0 graph so
            // the lifetime node/edge accumulation survives restart.
            let pg_bytes = {
                let g = vp_checkpoint
                    .perturbation_graph
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                // Phase B5: also dump a tiny JSON stats sidecar so the gateway
                // can use relation/group/fast_path counts as growth evidence
                // without having to walk the binary's variable-stride blocks.
                // Written every checkpoint tick (~1 Hz) — cheap.
                let top_group_bind = g.most_bound_group().map(|gr| gr.bind_strength).unwrap_or(0.0);
                let top_node_bind = g.most_bound().map(|n| n.bind_strength).unwrap_or(0.0);
                // B2.3: held-scene readout — is a scene being held right now, and
                // what does it hold? Flat numeric keys for the observer sidecar.
                let scene = g.held_scene();
                let scene_lead = scene.lead.map(|id| id as i64).unwrap_or(-1);
                let scene_top0 = scene.nodes.first().copied().unwrap_or(0);
                let scene_top1 = scene.nodes.get(1).copied().unwrap_or(0);
                let scene_top2 = scene.nodes.get(2).copied().unwrap_or(0);
                // B2.4: which held node is being PLAYED right now (cursor), -1 if none.
                let scene_play = g.held_scene_play(now_us()).map(|(n, _)| n as i64).unwrap_or(-1);
                let stats = format!(
                    "{{\"n_nodes\":{},\"n_edges\":{},\"n_groups\":{},\"n_relations\":{},\"n_fast_paths\":{},\"n_kinds\":{},\"n_vocal_profiles\":{},\"top_node_bind\":{:.6},\"top_group_bind\":{:.6},\"fast_path_hit_rate\":{:.6},\"graph_density\":{:.6},\"post_hoc_disagree\":{:.6},\"scene_lead\":{},\"scene_n\":{},\"scene_sal\":{:.6},\"scene_top0\":{},\"scene_top1\":{},\"scene_top2\":{},\"scene_play\":{}}}\n",
                    g.nodes.len(),
                    g.edges.len(),
                    g.groups.len(),
                    g.relations.len(),
                    g.fast_paths.by_endpoints.len(),
                    g.kinds.len(),
                    g.vocal_profiles.len(),
                    top_node_bind,
                    top_group_bind,
                    g.fast_path_hit_rate(),
                    g.graph_density(),
                    g.post_hoc_disagree_ema,
                    scene_lead,
                    scene.nodes.len(),
                    scene.mean_salience,
                    scene_top0,
                    scene_top1,
                    scene_top2,
                    scene_play,
                );
                let pg_stats_cp = runtime_dir.join("pg_stats.json");
                bionic_v0::atomic_write_or_warn("checkpoint pg_stats", &pg_stats_cp, stats.as_bytes());
                g.serialize()
            };
            bionic_v0::atomic_write_or_warn(
                "checkpoint perturbation_graph",
                &perturbation_graph_cp,
                &pg_bytes,
            );
            // Cognitive pipeline L4 pattern memory: persist the Hopfield-style
            // experiential prototypes so they survive restart.
            let cp_bytes = vp_checkpoint
                .cognitive_pipeline
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .serialize_patterns();
            bionic_v0::atomic_write_or_warn(
                "checkpoint cognitive_patterns",
                &cognitive_patterns_cp,
                &cp_bytes,
            );
        })
    };

    // Global orchestrator: attention → relations → snapshot (30 Hz).
    //
    // Replaces the three independent tickers that previously ran at 30/20/30 Hz.
    // Single serial cycle ensures relations read same-round attention shares and
    // the snapshot always captures post-refresh state.
    let vp_orch = vp.clone();
    let paths_orch = paths.clone();
    let xlayer_orch = xlayer_axis.clone();
    let h_orchestrator = spawn_ticker(vp.clone(), profile.orchestrator_hz, stop.clone(), move |_| {
        run_global_refresh_cycle(&vp_orch, &paths_orch, &xlayer_orch, now_us());
    });

    // Watchdog (推论3): the main loop already wakes every 100 ms; piggyback a
    // liveness sweep over all rhythm threads. A handle that finished while
    // stop is still false means a rhythm died — record it once on the tier-0
    // thread-fault axis (no text, no restart in this stage) so the observer
    // sees a half-dead body instead of silence.
    {
        let watch_handles: [&std::thread::JoinHandle<()>; 17] = [
            &h_surface,
            &h_neck,
            &h_adapters,
            &h_pred,
            &h_hl,
            &h_health,
            &h_ingress,
            &h_binding,
            &h_endogenous,
            &h_vocal,
            &h_vocal_output,
            &h_audio,
            &h_learned_bias_checkpoint,
            &h_orchestrator,
            &h_live_byte,
            &h_live_byte_closure,
            &h_master,
        ];
        let mut watch_reported = [false; 17];
        let watchdog_sweep = |reported: &mut [bool; 17]| {
            for (i, h) in watch_handles.iter().enumerate() {
                if !reported[i] && h.is_finished() {
                    reported[i] = true;
                    vp.mark_thread_fault();
                }
            }
        };

        if duration_secs > 0 {
            let t0 = std::time::Instant::now();
            while t0.elapsed() < Duration::from_secs(duration_secs) {
                std::thread::sleep(Duration::from_millis(100));
                if !stop.load(Ordering::Relaxed) {
                    watchdog_sweep(&mut watch_reported);
                }
            }
            stop.store(true, Ordering::Relaxed);
        } else {
            // Poll stop flag at 100 ms so a SIGTERM (or any other flag-setter)
            // is noticed quickly and falls through to the checkpoint flush below.
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                if !stop.load(Ordering::Relaxed) {
                    watchdog_sweep(&mut watch_reported);
                }
            }
        }
    }

    let _ = h_surface.join();
    let _ = h_neck.join();
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
    // (Phase 3 cutover) familiar_objects shutdown checkpoint removed —
    // perturbation_graph.bin sidecar (written below) carries identity now.
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
        bionic_v0::atomic_write_or_warn("shutdown phoneme_memory", &phoneme_memory_path, &pm_bytes);
    }
    // Motor prototypes: persist at shutdown so accumulated imitation learning
    // is not lost when the nursery restarts (cumulative across sessions).
    {
        let vm_bytes = vp.vocal_memory.read().unwrap().serialize();
        bionic_v0::atomic_write_or_warn("shutdown vocal_memory", &vocal_memory_path, &vm_bytes);
    }
    // Target→correction recall: persist at shutdown so learned vowel corrections
    // accumulate across sessions (Phase 2).
    {
        let vr_bytes = vp.vocal_target_recall.read().unwrap().serialize();
        bionic_v0::atomic_write_or_warn("shutdown vocal_target_recall", &vocal_recall_path, &vr_bytes);
    }
    {
        let vs_bytes = vp.vocal_sequence.read().unwrap().serialize();
        bionic_v0::atomic_write_or_warn("shutdown vocal_sequence", &vocal_sequence_path, &vs_bytes);
    }
    {
        let ps_bytes = vp.phot_sequence.read().unwrap().serialize();
        bionic_v0::atomic_write_or_warn("shutdown phot_sequence", &phot_sequence_path, &ps_bytes);
    }
    // Phase 3 perturbation_graph Stage 0 shutdown checkpoint. Stage 0 just
    // proves the pipe is alive (the graph is still empty until Stage 1 plugs
    // in the new visual frontend); this write means the next boot will
    // round-trip through deserialize(), surfacing any layout bugs early.
    {
        let pg_bytes = vp.perturbation_graph.read().unwrap().serialize();
        bionic_v0::atomic_write_or_warn(
            "shutdown perturbation_graph",
            &perturbation_graph_path,
            &pg_bytes,
        );
    }
    // Cognitive pipeline L4 pattern memory: persist at shutdown so accumulated
    // experiential prototypes carry over to the next session.
    {
        let cp_bytes = vp.cognitive_pipeline.read().unwrap().serialize_patterns();
        bionic_v0::atomic_write_or_warn(
            "shutdown cognitive_patterns",
            &cognitive_patterns_path,
            &cp_bytes,
        );
    }

    std::process::exit(0);
}
