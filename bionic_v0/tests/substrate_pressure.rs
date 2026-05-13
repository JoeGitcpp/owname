use bionic_v0::substrate_pressure::{compute_pressure, PressureSample, PressureWeights};

#[test]
fn generic_pressure_rises_with_error_ambiguity_and_deviation() {
    let weights = PressureWeights::default();
    let quiet = compute_pressure(
        PressureSample {
            err: 0.05,
            ambiguity: 0.05,
            deviation: 0.05,
            ..Default::default()
        },
        &weights,
    );
    let active = compute_pressure(
        PressureSample {
            err: 0.8,
            ambiguity: 0.6,
            deviation: 0.7,
            ..Default::default()
        },
        &weights,
    );
    assert!(active > quiet);
}

#[test]
fn generic_pressure_is_bounded_for_extreme_inputs() {
    let pressure = compute_pressure(
        PressureSample {
            err: 99.0,
            recurrence: 99.0,
            ambiguity: 99.0,
            alignment: 99.0,
            support: 99.0,
            deviation: 99.0,
            temporal_gap: 99.0,
            cross_channel: 99.0,
        },
        &PressureWeights::default(),
    );
    assert!((0.0..=2.0).contains(&pressure));
}
