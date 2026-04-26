//! End-to-end test: disruption-pattern err in VibAdapter should produce
//! amplified surprise compared to baseline (low-err) input under the same
//! priors and same processing path.
//!
//! Strategy: directly inject controlled err into the viewport's err streams,
//! then call process_frame. The adapter reads latest_err from the viewport,
//! so we push err samples that it will pick up.

use bionic_v0::adapter::VibAdapter;
use bionic_v0::evolution_priors::EvolutionPriors;
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use std::sync::Arc;

/// Drive the adapter through process_frame_for_test with pre-injected err.
/// Returns the resulting smoothed_surprise from the VIB ambient channel.
fn run_frame_with_err(
    err_to_inject: [f64; VIB_BANDS],
    bands: [f64; VIB_BANDS],
    priors: Arc<EvolutionPriors>,
) -> f64 {
    let vp = Arc::new(Viewport::new());
    let mut adapter = VibAdapter::with_priors(&vp, priors);

    // Push err into the viewport so latest_err() returns them
    for axis in 0..VIB_BANDS {
        vp.push_err(axis, 10_000, err_to_inject[axis]);
    }

    adapter.process_frame_for_test(&vp, 20_000, &bands);

    let ambient = vp.read_ambient();
    ambient.vib.surprise
}

#[test]
fn test_disruption_pattern_amplifies_surprise_vs_baseline() {
    let priors = Arc::new(EvolutionPriors::default());

    // Baseline: low err across all bands
    let baseline_err = [0.05, 0.05, 0.05, 0.05];
    let baseline = run_frame_with_err(
        baseline_err,
        [0.1, 0.1, 0.1, 0.1],
        priors.clone(),
    );

    // Disruption: distress band (2) dominant err
    let disruption_err = [0.05, 0.1, 0.8, 0.1];
    let disruption = run_frame_with_err(
        disruption_err,
        [0.1, 0.15, 0.8, 0.1],
        priors.clone(),
    );

    assert!(
        disruption > baseline * 1.5,
        "disruption pattern should amplify surprise >1.5x: baseline={}, disruption={}",
        baseline, disruption
    );
}

#[test]
fn test_low_disruption_multiplier_means_smaller_amplification() {
    let mut priors_low = EvolutionPriors::default();
    priors_low.disruption_surprise_multiplier = 1.0; // No amplification

    let mut priors_high = EvolutionPriors::default();
    priors_high.disruption_surprise_multiplier = 5.0; // Strong amplification

    let err = [0.05, 0.1, 0.8, 0.1];
    let bands = [0.1, 0.15, 0.8, 0.1];

    let surprise_low = run_frame_with_err(err, bands, Arc::new(priors_low));
    let surprise_high = run_frame_with_err(err, bands, Arc::new(priors_high));

    assert!(
        surprise_high > surprise_low,
        "higher disruption_surprise_multiplier should produce higher surprise: low={}, high={}",
        surprise_low, surprise_high
    );
}
