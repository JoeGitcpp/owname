use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::OutputChannelKind;
use bionic_v0::prediction_pressure_field::{
    PredictionWaveInput, PREDICTION_WAVE_EVENT, PREDICTION_WAVE_GAP,
};
use bionic_v0::viewport::Viewport;

fn wave(
    t_us: u64,
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    amplitude: f64,
) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind,
        slot_id,
        edge_id,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance: 0.76,
        closure_affordance: 0.84,
        write_risk: 0.05,
    }
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

fn quiet_surface(vp: &Viewport, start_t_us: u64) {
    for step in 0..40 {
        let t_us = start_t_us + step * 20_000;
        vp.refresh_prediction_pressure_field(t_us, &[]);
        vp.refresh_active_surface(t_us + 1_000);
    }
}

fn run_chain_step(
    vp: &Viewport,
    t_us: u64,
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    matched_return: bool,
) {
    vp.refresh_prediction_pressure_field(t_us, &[wave(t_us, source_kind, slot_id, edge_id, 0.98)]);
    vp.refresh_active_surface(t_us + 1_000);
    vp.refresh_probe_law(t_us + 2_000);
    vp.tick_probe_output_from_law(t_us + 2_000);
    let write_magnitude = latest_generic_write_magnitude(vp);
    let return_magnitude = if matched_return {
        write_magnitude
    } else if write_magnitude >= 0.5 {
        0.0
    } else {
        1.0
    };
    assert!(pair_latest_generic_write(
        vp,
        t_us + 120_000,
        return_magnitude
    ));
    vp.tick_active_surface_probe_chain(t_us + 121_000);
    vp.tick_active_surface_chain_memory(t_us + 122_000);
}

fn build_long_a(vp: &Viewport) {
    for idx in 0..12 {
        run_chain_step(
            vp,
            1_000_000 + idx * 150_000,
            PREDICTION_WAVE_GAP,
            1,
            2,
            true,
        );
    }
}

fn introduce_b(vp: &Viewport, t_us: u64, matched_return: bool) {
    vp.apply_active_surface_closure(1.0);
    quiet_surface(vp, t_us - 860_000);
    run_chain_step(vp, t_us, PREDICTION_WAVE_EVENT, 2, 4, matched_return);
}

#[test]
fn long_horizon_chain_accumulates_depth_span_and_score() {
    let vp = Viewport::new();
    build_long_a(&vp);
    let surface = vp.read_active_surface();
    let trace = surface
        .chain_memory_traces
        .iter()
        .find(|trace| trace.slot_id == 1 && trace.edge_id == 2)
        .copied()
        .expect("long chain trace");

    assert!(trace.recurrence_count >= 12);
    assert!(trace.span_us > 1_000_000);
    assert!(trace.depth_score > 0.80);
    assert!(trace.long_chain_score > 0.50);
    assert!(surface.chain_memory_long_chain_steps >= 12);
    assert!(surface.chain_memory_long_chain_span_us > 1_000_000);
    assert!(surface.chain_memory_long_chain_score > 0.50);
    assert_eq!(surface.active_aperture_count(), 1);
}

#[test]
fn long_horizon_chain_retains_branch_pressure_under_interruption() {
    let vp = Viewport::new();
    build_long_a(&vp);
    introduce_b(&vp, 3_900_000, true);
    introduce_b(&vp, 4_900_000, true);
    introduce_b(&vp, 5_900_000, true);
    let surface = vp.read_active_surface();
    let long_trace = surface
        .chain_memory_traces
        .iter()
        .find(|trace| trace.slot_id == 1 && trace.edge_id == 2)
        .copied()
        .expect("interrupted long trace");

    assert_eq!(surface.chain_memory_dominant_slot_id, 2);
    assert!(long_trace.interruption_count >= 1);
    assert!(surface.chain_memory_interruption_count >= 1);
    assert!(surface.chain_memory_interruption_branch_pressure > 0.0);
    assert!(surface.chain_memory_traces_alive >= 2);
}

#[test]
fn long_horizon_chain_recovers_after_delayed_reentry() {
    let vp = Viewport::new();
    build_long_a(&vp);
    introduce_b(&vp, 3_900_000, true);
    introduce_b(&vp, 4_900_000, true);
    introduce_b(&vp, 5_900_000, true);
    let interrupted = vp.read_active_surface();
    assert_eq!(interrupted.chain_memory_dominant_slot_id, 2);

    vp.apply_active_surface_closure(1.0);
    quiet_surface(&vp, 6_100_000);
    run_chain_step(&vp, 7_100_000, PREDICTION_WAVE_GAP, 1, 2, true);
    run_chain_step(&vp, 7_400_000, PREDICTION_WAVE_GAP, 1, 2, true);
    let recovered = vp.read_active_surface();
    let long_trace = recovered
        .chain_memory_traces
        .iter()
        .find(|trace| trace.slot_id == 1 && trace.edge_id == 2)
        .copied()
        .expect("recovered long trace");

    assert_eq!(recovered.chain_memory_dominant_slot_id, 1);
    assert!(long_trace.recovery_count >= 1);
    assert!(recovered.chain_memory_recovery_count >= 1);
    assert!(recovered.chain_memory_delayed_reentry_pressure > 0.0);
    assert!(recovered.chain_memory_long_chain_score > 0.50);
    assert_eq!(recovered.active_aperture_count(), 1);
}
