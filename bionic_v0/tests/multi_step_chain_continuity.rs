use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::OutputChannelKind;
use bionic_v0::prediction_pressure_field::PredictionWaveInput;
use bionic_v0::prediction_pressure_field::{
    PREDICTION_WAVE_EVENT, PREDICTION_WAVE_GAP, PREDICTION_WAVE_RESIDUE,
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
        resonance: 0.74,
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

fn run_chain_step(vp: &Viewport, t_us: u64, slot_id: i8, edge_id: i16, matched: bool) {
    run_chain_step_with_source(vp, t_us, PREDICTION_WAVE_GAP, slot_id, edge_id, matched);
}

fn run_chain_step_with_source(
    vp: &Viewport,
    t_us: u64,
    source_kind: u8,
    slot_id: i8,
    edge_id: i16,
    matched: bool,
) {
    vp.refresh_prediction_pressure_field(t_us, &[wave(t_us, source_kind, slot_id, edge_id, 0.98)]);
    vp.refresh_active_surface(t_us + 1_000);
    vp.refresh_probe_law(t_us + 2_000);
    vp.tick_probe_output_from_law(t_us + 2_000);
    let write_magnitude = latest_generic_write_magnitude(vp);
    let return_magnitude = if matched {
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

fn quiet_surface(vp: &Viewport, start_t_us: u64) {
    for step in 0..40 {
        let t_us = start_t_us + step * 20_000;
        vp.refresh_prediction_pressure_field(t_us, &[]);
        vp.refresh_active_surface(t_us + 1_000);
    }
}

#[test]
fn multi_step_chain_continuity_strengthens_same_coordinate_chain() {
    let vp = Viewport::new();
    run_chain_step(&vp, 1_000_000, 1, 2, true);
    let after_first = vp.read_active_surface();
    let first_score = after_first.chain_memory_dominant_continuity_score;

    run_chain_step(&vp, 1_300_000, 1, 2, true);
    run_chain_step(&vp, 1_600_000, 1, 2, true);
    let after_third = vp.read_active_surface();

    assert_eq!(after_third.chain_memory_traces_alive, 1);
    assert!(after_third.chain_memory_continuity_steps >= 2);
    assert!(after_third.chain_memory_dominant_continuity_score > first_score);
    assert_eq!(after_third.chain_memory_traces[0].recurrence_count, 3);
}

#[test]
fn multi_step_chain_continuity_retains_parallel_branch_pressure() {
    let vp = Viewport::new();
    run_chain_step(&vp, 2_000_000, 1, 2, true);
    run_chain_step(&vp, 2_300_000, 1, 2, true);
    vp.apply_active_surface_closure(1.0);
    quiet_surface(&vp, 2_440_000);
    run_chain_step_with_source(&vp, 3_300_000, PREDICTION_WAVE_EVENT, 2, 4, false);
    let surface = vp.read_active_surface();

    assert!(
        surface.chain_memory_traces_alive >= 2,
        "traces={:?}",
        surface.chain_memory_traces
    );
    assert!(surface.chain_memory_parallel_branch_count >= 1);
    assert!(surface.chain_memory_branch_pressure > 0.0);
    assert!(surface.chain_memory_dominant_chain_index >= 0);
}

#[test]
fn multi_step_chain_continuity_keeps_single_aperture_while_branch_remains() {
    let vp = Viewport::new();
    run_chain_step(&vp, 3_000_000, 1, 2, true);
    run_chain_step(&vp, 3_300_000, 1, 2, true);
    vp.apply_active_surface_closure(1.0);
    quiet_surface(&vp, 3_440_000);
    run_chain_step_with_source(&vp, 4_300_000, PREDICTION_WAVE_EVENT, 2, 4, false);

    vp.tick_active_surface_chain_memory(4_480_000);
    vp.refresh_active_surface(4_481_000);
    let surface = vp.read_active_surface();

    assert_eq!(surface.active_aperture_count(), 1);
    assert!(surface
        .traces
        .iter()
        .any(|trace| trace.source_kind == PREDICTION_WAVE_RESIDUE && trace.pressure > 0.0));
    assert!(surface.chain_memory_parallel_branch_count >= 1);
}
