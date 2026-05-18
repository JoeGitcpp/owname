use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::OutputChannelKind;
use bionic_v0::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RESIDUE,
};
use bionic_v0::viewport::Viewport;

fn wave(t_us: u64, amplitude: f64) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind: PREDICTION_WAVE_GAP,
        slot_id: 1,
        edge_id: 2,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance: 0.72,
        closure_affordance: 0.82,
        write_risk: 0.05,
    }
}

fn seed_surface_and_probe(vp: &Viewport, t_us: u64) {
    vp.refresh_prediction_pressure_field(t_us, &[wave(t_us, 0.94)]);
    vp.refresh_active_surface(t_us + 1_000);
    vp.refresh_probe_law(t_us + 2_000);
    vp.tick_probe_output_from_law(t_us + 2_000);
    assert!(vp
        .read_action_output()
        .write_epoch_history
        .iter()
        .any(|record| record.channel_kind == OutputChannelKind::GenericWrite));
}

fn latest_generic_write_magnitude(vp: &Viewport) -> f64 {
    vp.read_action_output()
        .write_epoch_history
        .iter()
        .rev()
        .find(|record| record.channel_kind == OutputChannelKind::GenericWrite)
        .map(|record| record.start_magnitude)
        .unwrap_or(0.0)
}

fn pair_latest_generic_write(vp: &Viewport, return_t_us: u64, return_magnitude: f64) -> bool {
    let latest = vp
        .read_action_output()
        .write_epoch_history
        .iter()
        .rev()
        .find(|record| record.channel_kind == OutputChannelKind::GenericWrite)
        .cloned();
    let Some(record) = latest else {
        return false;
    };
    let window_idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: return_t_us,
        end_t_us: return_t_us + 1,
        magnitude: return_magnitude.clamp(0.0, 1.0),
    });
    let window_id = vp
        .read_action_consequence()
        .windows
        .get(window_idx as usize)
        .map(|window| window.window_id)
        .unwrap_or_default();
    vp.close_action_consequence_window(window_id, return_t_us + 2);
    vp.pair_action_consequence(record.write_seq, window_id, return_t_us - record.start_t_us)
}

fn run_closed_chain(vp: &Viewport, t_us: u64) {
    seed_surface_and_probe(vp, t_us);
    let write_magnitude = latest_generic_write_magnitude(vp);
    assert!(pair_latest_generic_write(
        vp,
        t_us + 120_000,
        write_magnitude
    ));
    vp.tick_active_surface_probe_chain(t_us + 121_000);
}

fn run_mismatch_chain(vp: &Viewport, t_us: u64) {
    seed_surface_and_probe(vp, t_us);
    let write_magnitude = latest_generic_write_magnitude(vp);
    let return_magnitude = if write_magnitude >= 0.5 { 0.0 } else { 1.0 };
    assert!(pair_latest_generic_write(
        vp,
        t_us + 150_000,
        return_magnitude
    ));
    vp.tick_active_surface_probe_chain(t_us + 151_000);
}

#[test]
fn chain_memory_sediments_local_probe_chain_once() {
    let vp = Viewport::new();
    run_closed_chain(&vp, 1_000_000);

    vp.tick_active_surface_chain_memory(1_130_000);
    vp.tick_active_surface_chain_memory(1_140_000);
    let surface = vp.read_active_surface();

    assert_eq!(surface.chain_memory_traces_alive, 1);
    assert_eq!(surface.chain_memory_sediment_updates, 1);
    assert!(surface.chain_memory_traces[0].sediment_strength > 0.0);
    assert!(surface.chain_memory_traces[0].closure_support > 0.0);
    assert_eq!(surface.chain_memory_last_pair_seq, 1);
}

#[test]
fn chain_memory_reentry_rehydrates_residue_pressure_after_quiet() {
    let vp = Viewport::new();
    run_mismatch_chain(&vp, 2_000_000);
    vp.tick_active_surface_chain_memory(2_170_000);

    for step in 0..6 {
        let t_us = 2_220_000 + step * 20_000;
        vp.refresh_prediction_pressure_field(t_us, &[]);
        vp.refresh_active_surface(t_us + 1_000);
    }

    vp.tick_active_surface_chain_memory(2_380_000);
    let field = vp.read_prediction_pressure_field();
    assert!(field
        .waves
        .iter()
        .any(|wave| wave.source_kind == PREDICTION_WAVE_RESIDUE && wave.amplitude > 0.0));

    vp.refresh_active_surface(2_381_000);
    let surface = vp.read_active_surface();
    assert!(surface.chain_memory_last_reentry_pressure > 0.0);
    assert!(surface
        .traces
        .iter()
        .any(|trace| trace.source_kind == PREDICTION_WAVE_RESIDUE && trace.pressure > 0.0));
}

#[test]
fn chain_memory_persistent_reentry_exceeds_cold_baseline() {
    let persistent = Viewport::new();
    run_closed_chain(&persistent, 3_000_000);
    persistent.tick_active_surface_chain_memory(3_130_000);
    persistent.tick_active_surface_chain_memory(3_260_000);
    let persistent_pressure = persistent
        .read_active_surface()
        .chain_memory_last_reentry_pressure;

    let cold = Viewport::new();
    cold.tick_active_surface_chain_memory(3_260_000);
    let cold_pressure = cold
        .read_active_surface()
        .chain_memory_last_reentry_pressure;

    assert!(persistent_pressure > cold_pressure);
    assert!(persistent_pressure - cold_pressure > 0.05);
}

#[test]
fn chain_memory_body_order_runs_after_probe_chain_before_probe_law() {
    let body_source = include_str!("../src/bin/body.rs");
    let chain_pos = body_source.find("tick_active_surface_probe_chain").unwrap();
    let memory_pos = body_source
        .find("tick_active_surface_chain_memory")
        .unwrap();
    let probe_pos = body_source.find("refresh_probe_law").unwrap();

    assert!(chain_pos < memory_pos);
    assert!(memory_pos < probe_pos);
}
