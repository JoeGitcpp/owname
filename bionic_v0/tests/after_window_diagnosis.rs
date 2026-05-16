use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::evolution_priors::BiasAxis;
use bionic_v0::learned_bias::BiasLocator;
use bionic_v0::viewport::Viewport;

#[derive(Debug)]
struct AfterWindowSample {
    after_steps: u64,
    epoch_last_update_t_us: u64,
    peak_magnitude: f64,
    pair_curve_similarity: f64,
    learned_delta_value: f64,
    learned_event_count: u32,
}

#[derive(Debug)]
struct ContinuedAfterSample {
    after_600_delta_value: f64,
    after_900_delta_value: f64,
    after_600_similarity: f64,
    after_900_similarity: f64,
    learned_event_count: u32,
}

#[derive(Debug)]
struct RuntimeAfterSample {
    after_steps: u64,
    epoch_start_magnitude: f64,
    epoch_last_update_t_us: u64,
    live_peak_magnitude: f64,
    pair_curve_similarity: f64,
    learned_delta_value: f64,
    learned_event_count: u32,
}

fn adjust_same_epoch(vp: &Viewport, start_t_us: u64, from_step: u64, to_step: u64) {
    let step_us = 1_000;
    let base_magnitude = 0.40;
    for step in from_step..=to_step {
        let fraction = step as f64 / 900.0;
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Adjust,
            magnitude: (base_magnitude + 0.60 * fraction).clamp(0.0, 1.0),
            event_t_us: start_t_us + step * step_us,
        });
    }
}

fn pair_current_epoch(vp: &Viewport, write_seq: u64, window_start_t_us: u64) -> f64 {
    let start_t_us = 1_000_000;
    let consequence_magnitude = 0.40;
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: window_start_t_us,
        end_t_us: window_start_t_us + 20_000,
        magnitude: consequence_magnitude,
    });
    let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, window_start_t_us - start_t_us,));
    vp.read_action_consequence()
        .pairs
        .iter()
        .find(|pair| pair.window_id == window_id)
        .map(|pair| pair.curve_similarity)
        .unwrap()
}

fn action_consequence_delta(vp: &Viewport) -> f64 {
    vp.read_learned_bias()
        .curves
        .iter()
        .find(|curve| curve.axis == BiasAxis::AlignmentGain && curve.locator == BiasLocator::Global)
        .map(|curve| curve.delta_value)
        .unwrap_or(0.0)
}

fn sample_after_window(after_steps: u64) -> AfterWindowSample {
    let vp = Viewport::new();
    let start_t_us = 1_000_000;
    let step_us = 1_000;
    let consequence_magnitude = 0.40;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: consequence_magnitude,
        event_t_us: start_t_us,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    adjust_same_epoch(&vp, start_t_us, 1, after_steps);

    let window_start_t_us = start_t_us + after_steps * step_us + 10_000;
    let pair_curve_similarity = pair_current_epoch(&vp, write_seq, window_start_t_us);
    vp.tick_life_closure_producers(window_start_t_us + 1_000);

    let out = vp.read_action_output();
    let learned = vp.read_learned_bias();

    AfterWindowSample {
        after_steps,
        epoch_last_update_t_us: out.write_epoch_history[0].last_update_t_us,
        peak_magnitude: out.channels[0].peak_magnitude,
        pair_curve_similarity,
        learned_delta_value: action_consequence_delta(&vp),
        learned_event_count: learned.n_recent_events,
    }
}

fn sample_continued_after_dilution() -> ContinuedAfterSample {
    let vp = Viewport::new();
    let start_t_us = 1_000_000;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.40,
        event_t_us: start_t_us,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    adjust_same_epoch(&vp, start_t_us, 1, 600);
    let after_600_window_start_t_us = start_t_us + 600_000 + 10_000;
    let after_600_similarity = pair_current_epoch(&vp, write_seq, after_600_window_start_t_us);
    vp.tick_life_closure_producers(after_600_window_start_t_us + 1_000);
    let after_600_delta_value = action_consequence_delta(&vp);

    adjust_same_epoch(&vp, start_t_us, 601, 900);
    let after_900_window_start_t_us = start_t_us + 900_000 + 10_000;
    let after_900_similarity = pair_current_epoch(&vp, write_seq, after_900_window_start_t_us);
    vp.tick_life_closure_producers(after_900_window_start_t_us + 1_000);
    let after_900_delta_value = action_consequence_delta(&vp);

    ContinuedAfterSample {
        after_600_delta_value,
        after_900_delta_value,
        after_600_similarity,
        after_900_similarity,
        learned_event_count: vp.read_learned_bias().n_recent_events,
    }
}

fn sample_runtime_after_window(after_steps: u64) -> RuntimeAfterSample {
    let vp = Viewport::new();
    let start_t_us = 1_000_000;
    let step_us = 1_000;

    for step in 0..=after_steps {
        let t_us = start_t_us + step * step_us;
        let fraction = step as f64 / 900.0;
        let target_energy = (0.80 + 1.20 * fraction).clamp(0.0, 2.0);
        vp.observe_vocal_target(t_us, target_energy, 0.35, 0.60, 0.20);
        vp.tick_vocal_imitation_loop(t_us);
    }

    let out_before_pair = vp.read_action_output();
    let epoch = out_before_pair.write_epoch_history[0].clone();
    let window_start_t_us = start_t_us + after_steps * step_us + 10_000;
    let window_id = vp.accumulate_ingress_change(window_start_t_us, epoch.start_magnitude);
    assert!(vp.try_auto_pair_latest_epoch_to_window(window_start_t_us));
    vp.tick_life_closure_producers(window_start_t_us + 1_000);

    let pair = vp
        .read_action_consequence()
        .pairs
        .iter()
        .find(|pair| pair.window_id == window_id)
        .cloned()
        .unwrap();
    let learned = vp.read_learned_bias();

    RuntimeAfterSample {
        after_steps,
        epoch_start_magnitude: epoch.start_magnitude,
        epoch_last_update_t_us: epoch.last_update_t_us,
        live_peak_magnitude: out_before_pair.channels[0].peak_magnitude,
        pair_curve_similarity: pair.curve_similarity,
        learned_delta_value: action_consequence_delta(&vp),
        learned_event_count: learned.n_recent_events,
    }
}

#[test]
fn after_900_continuing_writes_do_not_weaken_closed_pair_curve_evidence_vs_after_600() {
    let after_600 = sample_after_window(600);
    let after_900 = sample_after_window(900);

    println!("after_600={:?}", after_600);
    println!("after_900={:?}", after_900);

    assert_eq!(after_600.learned_event_count, 1);
    assert_eq!(after_900.learned_event_count, 1);
    assert_eq!(after_600.after_steps, 600);
    assert_eq!(after_900.after_steps, 900);

    assert!(
        after_900.epoch_last_update_t_us > after_600.epoch_last_update_t_us,
        "longer after window should keep writing into the same epoch: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.peak_magnitude > after_600.peak_magnitude,
        "longer after window should raise the current epoch peak magnitude: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.pair_curve_similarity >= after_600.pair_curve_similarity,
        "closed write evidence should not weaken after continued writes: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.learned_delta_value >= after_600.learned_delta_value,
        "continued writes should not lower action-consequence delta: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
}

#[test]
fn short_runtime_after_600_900_replay_keeps_closed_curve_evidence_intact() {
    let after_600 = sample_runtime_after_window(600);
    let after_900 = sample_runtime_after_window(900);

    println!("runtime_after_600={:?}", after_600);
    println!("runtime_after_900={:?}", after_900);

    assert_eq!(after_600.learned_event_count, 1);
    assert_eq!(after_900.learned_event_count, 1);
    assert_eq!(after_600.after_steps, 600);
    assert_eq!(after_900.after_steps, 900);
    assert!(
        after_900.epoch_last_update_t_us > after_600.epoch_last_update_t_us,
        "runtime after=900 must actually keep writing longer: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.live_peak_magnitude > after_600.live_peak_magnitude,
        "runtime after=900 must still raise the live channel peak: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        (after_900.epoch_start_magnitude - after_600.epoch_start_magnitude).abs() < 1e-12,
        "closed epoch boundary evidence should remain the same across after windows: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.pair_curve_similarity >= after_600.pair_curve_similarity,
        "runtime after=900 should not weaken closed pair evidence: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
    assert!(
        after_900.learned_delta_value >= after_600.learned_delta_value,
        "runtime after=900 should not lower learned action-consequence delta: after_600={:?} after_900={:?}",
        after_600,
        after_900
    );
}

#[test]
fn after_900_late_event_does_not_dilute_delta_formed_at_after_600() {
    let sample = sample_continued_after_dilution();

    println!("continued_after={:?}", sample);

    assert_eq!(sample.learned_event_count, 2);
    assert!(
        sample.after_900_similarity >= sample.after_600_similarity,
        "continued writes should not weaken closed pair evidence: {:?}",
        sample
    );
    assert!(
        sample.after_900_delta_value >= sample.after_600_delta_value,
        "late after event should not pull down the previously formed action-consequence delta: {:?}",
        sample
    );
}
