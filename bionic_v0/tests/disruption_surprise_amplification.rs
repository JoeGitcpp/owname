//! End-to-end test: broadband err in VibAdapter should produce
//! amplified surprise compared to baseline (low-err) input under the same
//! priors and same processing path.

use bionic_v0::adapter::VibAdapter;
use bionic_v0::evolution_priors::BiasPriors;
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use std::sync::Arc;

/// Tile a 4-band pattern across the 16 VIB bands (M1.5 widened VIB_BANDS 4→16).
fn vib16(p: [f64; 4]) -> [f64; VIB_BANDS] {
    std::array::from_fn(|i| p[i % 4])
}

fn run_frame_with_err(
    err_to_inject: [f64; VIB_BANDS],
    bands: [f64; VIB_BANDS],
    priors: Arc<BiasPriors>,
) -> f64 {
    let vp = Arc::new(Viewport::new());
    let mut adapter = VibAdapter::with_priors(&vp, priors);

    for axis in 0..VIB_BANDS {
        vp.push_err(axis, 10_000, err_to_inject[axis]);
    }

    adapter.process_frame_for_test(&vp, 20_000, &bands);

    let ambient = vp.read_ambient();
    ambient.vib.surprise
}

#[test]
fn test_broadband_err_amplifies_surprise_vs_baseline() {
    let priors = Arc::new(BiasPriors::default());

    let baseline_err = vib16([0.05, 0.05, 0.05, 0.05]);
    let baseline = run_frame_with_err(baseline_err, vib16([0.1, 0.1, 0.1, 0.1]), priors.clone());

    let broadband_err = vib16([0.05, 0.1, 0.8, 0.1]);
    let broadband = run_frame_with_err(broadband_err, vib16([0.1, 0.15, 0.8, 0.1]), priors.clone());

    assert!(
        broadband > baseline * 1.5,
        "broadband err should amplify surprise >1.5x: baseline={}, broadband={}",
        baseline,
        broadband
    );
}

#[test]
fn test_low_multiplier_means_smaller_amplification() {
    let mut priors_low = BiasPriors::default();
    priors_low.pressure_weights.multiplier = 0.5;

    let mut priors_high = BiasPriors::default();
    priors_high.pressure_weights.multiplier = 2.0;

    let err = vib16([0.05, 0.1, 0.8, 0.1]);
    let bands = vib16([0.1, 0.15, 0.8, 0.1]);

    let surprise_low = run_frame_with_err(err, bands, Arc::new(priors_low));
    let surprise_high = run_frame_with_err(err, bands, Arc::new(priors_high));

    assert!(
        surprise_high > surprise_low,
        "higher generic pressure multiplier should produce higher surprise: low={}, high={}",
        surprise_low,
        surprise_high
    );
}
