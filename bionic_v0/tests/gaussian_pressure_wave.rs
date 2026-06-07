//! P1 TDD: Gaussian recon_mse → direct PredictionWaveInput.
//!
//! Before the fix, Gaussian visual surprise reaches the prediction pressure
//! field only through a long indirect chain:
//!   recon_mse → push_observation → slot error → global reasoning
//!     → evidence tension → tick_prediction_pressure_field_from_substrate
//! This chain attenuates the signal to near-zero on a stable scene.
//!
//! The fix: when recon_mse exceeds a threshold, directly inject a
//! PREDICTION_WAVE_EVENT on the same substrate tick, bypassing the chain.

use bionic_v0::prediction_pressure_field::PREDICTION_WAVE_EVENT;
use bionic_v0::viewport::Viewport;

const T: u64 = 1_000_000;

fn set_gaussian_mse(vp: &Viewport, mse: f64) {
    vp.gaussian_field.write().unwrap().recon_mse = mse;
}

/// High Gaussian MSE (well above threshold) must inject a PREDICTION_WAVE_EVENT
/// into the prediction pressure field on the very next substrate tick.
#[test]
fn gauss_high_mse_injects_event_wave_into_pressure_field() {
    let vp = Viewport::new();
    set_gaussian_mse(&vp, 0.30); // well above 0.04 threshold
    vp.tick_prediction_pressure_field_from_substrate(T);

    let field = vp.read_prediction_pressure_field();
    let has_event_wave = field
        .waves
        .iter()
        .any(|w| w.source_kind == PREDICTION_WAVE_EVENT && w.amplitude > 0.0);
    assert!(
        has_event_wave,
        "Gaussian recon_mse=0.30 must inject a PREDICTION_WAVE_EVENT; \
         got waves: {:?}",
        field
            .waves
            .iter()
            .map(|w| (w.source_kind, w.amplitude))
            .collect::<Vec<_>>()
    );
}

/// Gaussian MSE below the injection threshold must produce no waves at all
/// (since all other substrate sources are also zero on a fresh Viewport).
#[test]
fn gauss_below_threshold_produces_no_waves() {
    let vp = Viewport::new();
    set_gaussian_mse(&vp, 0.02); // below 0.04 threshold
    vp.tick_prediction_pressure_field_from_substrate(T);

    let field = vp.read_prediction_pressure_field();
    assert!(
        field.waves.is_empty(),
        "Gaussian recon_mse=0.02 must not inject any wave on a fresh Viewport; \
         got {} wave(s)",
        field.waves.len()
    );
}

/// The injected EVENT wave amplitude must increase monotonically with MSE.
/// Higher visual surprise → stronger prediction pressure.
#[test]
fn gauss_wave_amplitude_increases_with_mse() {
    let vp_low = Viewport::new();
    set_gaussian_mse(&vp_low, 0.10);
    vp_low.tick_prediction_pressure_field_from_substrate(T);

    let vp_high = Viewport::new();
    set_gaussian_mse(&vp_high, 0.50);
    vp_high.tick_prediction_pressure_field_from_substrate(T);

    let amp_low = vp_low
        .read_prediction_pressure_field()
        .waves
        .iter()
        .filter(|w| w.source_kind == PREDICTION_WAVE_EVENT)
        .map(|w| w.amplitude)
        .fold(0.0_f64, f64::max);

    let amp_high = vp_high
        .read_prediction_pressure_field()
        .waves
        .iter()
        .filter(|w| w.source_kind == PREDICTION_WAVE_EVENT)
        .map(|w| w.amplitude)
        .fold(0.0_f64, f64::max);

    assert!(
        amp_low > 0.0,
        "MSE=0.10 must produce a non-zero EVENT wave, got amp={amp_low}"
    );
    assert!(
        amp_high > amp_low,
        "higher Gaussian MSE must produce higher wave amplitude: \
         mse=0.10 → {amp_low:.4}, mse=0.50 → {amp_high:.4}"
    );
}
