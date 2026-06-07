//! P2 TDD: active surface aperture_pressure → vocal drive raw_urge.
//!
//! Before the fix, tick_vocal_drive_field only reads prediction field waves and
//! the active reasoning gap for raw_urge. The active surface's aperture_pressure
//! is NOT consulted, even though it carries the richest integrated attention signal.
//!
//! The fix: include `effective_probe_pressure() * 0.60` as a MAX contributor to
//! raw_urge, so accumulated surface attention can drive vocalization independently.
//!
//! Test setup uses ZERO_BASELINE_T where the endogenous baseline rhythm is
//! exactly zero (sin(3π/2) = −1 → rhythm = 0), so no voicing fires unless
//! there is a real source.

use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_organ::VocalOrganPhase;

/// µs at which the endogenous baseline rhythm is exactly zero:
/// phase = (3_750_000 / 5_000_000) * 2π = 1.5π → sin(1.5π) = −1
/// → rhythm = −1 * 0.5 + 0.5 = 0.0 → baseline = 0
const ZERO_BASELINE_T: u64 = 3_750_000;

/// Set the active surface's aperture pressure directly (without going through
/// the prediction field) to simulate an accumulated focus state.
fn seed_aperture(vp: &Viewport, pressure: f64) {
    let mut surf = vp.active_surface.write().unwrap();
    surf.aperture_pressure = pressure;
    surf.echo_pressure = 0.0;
}

/// With high surface aperture_pressure (0.80) but an empty prediction field and
/// zero baseline, tick_vocal_drive_field must fire a vocal burst (P2 fix).
///
/// Arithmetic: effective_probe_pressure = 0.80 → raw_urge = 0.80 * 0.60 = 0.48
///   reflex_pressure = 0.48 * 0.75 + 0.48 * 0.35 ≈ 0.53 > 0.40 threshold → burst.
#[test]
fn surface_aperture_drives_vocal_burst_at_zero_baseline_time() {
    let vp = Viewport::new();
    seed_aperture(&vp, 0.80);
    vp.tick_vocal_drive_field(ZERO_BASELINE_T);

    let organ = vp.read_vocal_organ();
    assert_eq!(
        organ.phase,
        VocalOrganPhase::Active,
        "aperture_pressure=0.80 at zero-baseline time must drive a vocal burst"
    );
    assert!(organ.breath_drive > 0.0, "breath_drive must be positive after burst");
}

/// Without any surface pressure, at zero-baseline time with an empty prediction
/// field, the organ must remain Idle (no phantom voicing).
#[test]
fn no_surface_no_burst_at_zero_baseline_time() {
    let vp = Viewport::new(); // fresh: aperture_pressure = 0, no prediction field
    vp.tick_vocal_drive_field(ZERO_BASELINE_T);

    let organ = vp.read_vocal_organ();
    assert_eq!(
        organ.phase,
        VocalOrganPhase::Idle,
        "zero aperture + empty field + zero baseline must leave organ Idle"
    );
}

/// Monotone: higher aperture_pressure → higher (or equal) breath_drive output.
/// aperture=0.50 → raw_urge=0.30 < 0.36 → Idle (breath=0)
/// aperture=0.90 → raw_urge=0.54 > 0.36 → burst (breath>0)
#[test]
fn surface_boost_monotone_with_aperture_pressure() {
    let vp_low = Viewport::new();
    seed_aperture(&vp_low, 0.50);
    vp_low.tick_vocal_drive_field(ZERO_BASELINE_T);
    let breath_low = vp_low.read_vocal_organ().breath_drive;

    let vp_high = Viewport::new();
    seed_aperture(&vp_high, 0.90);
    vp_high.tick_vocal_drive_field(ZERO_BASELINE_T);
    let breath_high = vp_high.read_vocal_organ().breath_drive;

    assert!(
        breath_high > breath_low,
        "higher aperture_pressure must produce stronger breath_drive: \
         low={breath_low:.4} high={breath_high:.4}"
    );
}
