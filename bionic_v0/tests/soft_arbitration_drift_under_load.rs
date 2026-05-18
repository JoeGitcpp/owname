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

fn build_strong_a(vp: &Viewport) {
    run_chain_step(vp, 1_000_000, PREDICTION_WAVE_GAP, 1, 2, true);
    run_chain_step(vp, 1_300_000, PREDICTION_WAVE_GAP, 1, 2, true);
    run_chain_step(vp, 1_600_000, PREDICTION_WAVE_GAP, 1, 2, true);
}

fn introduce_b(vp: &Viewport, t_us: u64, matched_return: bool) {
    vp.apply_active_surface_closure(1.0);
    quiet_surface(vp, t_us - 860_000);
    run_chain_step(vp, t_us, PREDICTION_WAVE_EVENT, 2, 4, matched_return);
}

#[test]
fn soft_arbitration_holds_current_chain_under_small_challenger_margin() {
    let vp = Viewport::new();
    build_strong_a(&vp);
    let before = vp.read_active_surface();

    introduce_b(&vp, 2_600_000, false);
    let after = vp.read_active_surface();

    assert!(after.chain_memory_soft_arbitration_ticks > before.chain_memory_soft_arbitration_ticks);
    assert!(after.chain_memory_soft_hold_count > before.chain_memory_soft_hold_count);
    assert_eq!(after.chain_memory_dominant_slot_id, 1);
    assert_eq!(after.chain_memory_dominant_edge_id, 2);
    assert!(after.chain_memory_challenger_pressure > 0.0);
    assert!(after.chain_memory_parallel_branch_count >= 1);
}

#[test]
fn soft_arbitration_switches_after_sustained_challenger_pressure() {
    let vp = Viewport::new();
    build_strong_a(&vp);
    introduce_b(&vp, 2_600_000, true);
    introduce_b(&vp, 3_600_000, true);
    introduce_b(&vp, 4_600_000, true);
    let after = vp.read_active_surface();

    assert!(after.chain_memory_soft_switch_count >= 1);
    assert_eq!(after.chain_memory_dominant_slot_id, 2);
    assert_eq!(after.chain_memory_dominant_edge_id, 4);
    assert!(after.chain_memory_branch_pressure > 0.0);
    assert!(after.chain_memory_traces_alive >= 2);
}

#[test]
fn soft_arbitration_allows_old_chain_reentry_without_losing_branch() {
    let vp = Viewport::new();
    build_strong_a(&vp);
    introduce_b(&vp, 2_600_000, true);
    introduce_b(&vp, 3_600_000, true);
    introduce_b(&vp, 4_600_000, true);
    let switched = vp.read_active_surface();
    assert_eq!(switched.chain_memory_dominant_slot_id, 2);

    vp.apply_active_surface_closure(1.0);
    quiet_surface(&vp, 4_760_000);
    run_chain_step(&vp, 5_700_000, PREDICTION_WAVE_GAP, 1, 2, true);
    run_chain_step(&vp, 6_000_000, PREDICTION_WAVE_GAP, 1, 2, true);
    let reentered = vp.read_active_surface();

    assert!(reentered.chain_memory_soft_reentry_count > switched.chain_memory_soft_reentry_count);
    assert_eq!(reentered.chain_memory_dominant_slot_id, 1);
    assert_eq!(reentered.chain_memory_dominant_edge_id, 2);
    assert!(reentered.chain_memory_parallel_branch_count >= 1);
    assert_eq!(reentered.active_aperture_count(), 1);
}
