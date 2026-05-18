use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::OutputChannelKind;
use bionic_v0::prediction_pressure_field::{PredictionWaveInput, PREDICTION_WAVE_GAP};
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

fn seed_surface_and_probe(vp: &Viewport, t_us: u64) -> f64 {
    vp.refresh_prediction_pressure_field(t_us, &[wave(t_us, 0.94)]);
    vp.refresh_active_surface(t_us + 1_000);
    let surface = vp.read_active_surface();
    assert_eq!(surface.active_aperture_count(), 1);
    assert!(surface.aperture_pressure > 0.0);

    vp.refresh_probe_law(t_us + 2_000);
    vp.tick_probe_output_from_law(t_us + 2_000);
    assert!(vp
        .read_action_output()
        .write_epoch_history
        .iter()
        .any(|record| record.channel_kind == OutputChannelKind::GenericWrite));
    surface.aperture_pressure
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

fn latest_generic_write_magnitude(vp: &Viewport) -> f64 {
    vp.read_action_output()
        .write_epoch_history
        .iter()
        .rev()
        .find(|record| record.channel_kind == OutputChannelKind::GenericWrite)
        .map(|record| record.start_magnitude)
        .unwrap_or(0.0)
}

#[test]
fn local_probe_chain_high_support_return_reduces_aperture_pressure() {
    let vp = Viewport::new();
    let before_pressure = seed_surface_and_probe(&vp, 1_000_000);
    let write_magnitude = latest_generic_write_magnitude(&vp);
    assert!(pair_latest_generic_write(&vp, 1_120_000, write_magnitude));

    vp.tick_active_surface_probe_chain(1_121_000);
    let after = vp.read_active_surface();

    assert_eq!(after.local_probe_chain_steps, 1);
    assert_eq!(after.local_probe_chain_closures, 1);
    assert_eq!(after.local_probe_chain_mismatches, 0);
    assert!(after.aperture_pressure < before_pressure);
    assert!(after.local_probe_chain_traces[0].closure_pressure > 0.0);
    assert!(after.local_probe_chain_traces[0].mismatch_pressure <= 0.01);
}

#[test]
fn local_probe_chain_low_support_return_raises_next_probe_pressure_without_deleting_trace() {
    let vp = Viewport::new();
    seed_surface_and_probe(&vp, 2_000_000);
    let write_magnitude = latest_generic_write_magnitude(&vp);
    let return_magnitude = if write_magnitude >= 0.5 { 0.0 } else { 1.0 };
    assert!(pair_latest_generic_write(&vp, 2_150_000, return_magnitude));

    vp.tick_active_surface_probe_chain(2_151_000);
    let after = vp.read_active_surface();

    assert_eq!(after.local_probe_chain_steps, 1);
    assert_eq!(after.local_probe_chain_closures, 0);
    assert_eq!(after.local_probe_chain_mismatches, 1);
    assert_eq!(after.active_aperture_count(), 1);
    assert!(after.traces_alive >= 1);
    assert!(after.local_probe_chain_next_probe_count >= 1);
    assert!(after.local_probe_chain_traces[0].next_probe_pressure > 0.0);
}

#[test]
fn local_probe_chain_consumes_each_pair_once() {
    let vp = Viewport::new();
    seed_surface_and_probe(&vp, 3_000_000);
    let write_magnitude = latest_generic_write_magnitude(&vp);
    assert!(pair_latest_generic_write(&vp, 3_120_000, write_magnitude));

    vp.tick_active_surface_probe_chain(3_121_000);
    vp.tick_active_surface_probe_chain(3_122_000);
    let after = vp.read_active_surface();

    assert_eq!(after.local_probe_chain_steps, 1);
    assert_eq!(after.local_probe_chain_traces.len(), 1);
}

#[test]
fn local_probe_chain_body_order_revises_surface_before_probe_law() {
    let body_source = include_str!("../src/bin/body.rs");
    let surface_pos = body_source.find("refresh_active_surface").unwrap();
    let chain_pos = body_source.find("tick_active_surface_probe_chain").unwrap();
    let probe_pos = body_source.find("refresh_probe_law").unwrap();

    assert!(surface_pos < chain_pos);
    assert!(chain_pos < probe_pos);
}
