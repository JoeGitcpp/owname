use bionic_v0::learned_bias::{LearnedBiasState, LearnedBiasSummary};
use bionic_v0::vocal_reflex::{VocalReflexDrive, VocalReflexState};

// ---------------------------------------------------------------------------
// Test 1: default LearnedBiasSummary vocal fields are zero
// ---------------------------------------------------------------------------
#[test]
fn learned_bias_vocal_fields_default_to_zero() {
    let summary = LearnedBiasSummary::default();

    assert_eq!(summary.prediction_familiarity, 0.0);
    assert_eq!(summary.action_suppression, 0.0);
}

// ---------------------------------------------------------------------------
// Test 2: LearnedBiasState → summary() preserves prediction_familiarity
//         and action_suppression values
// ---------------------------------------------------------------------------
#[test]
fn learned_bias_vocal_fields_summary_roundtrip() {
    let state = LearnedBiasState {
        prediction_familiarity: 0.42,
        action_suppression: 0.73,
        ..LearnedBiasState::default()
    };
    let summary = state.summary();

    assert!(
        (summary.prediction_familiarity - 0.42).abs() < f64::EPSILON,
        "prediction_familiarity should survive state→summary",
    );
    assert!(
        (summary.action_suppression - 0.73).abs() < f64::EPSILON,
        "action_suppression should survive state→summary",
    );
}

// ---------------------------------------------------------------------------
// Test 3: tick_drive uses prediction_familiarity and action_suppression to
//         modulate the regulating-pressure threshold, gating the burst output.
// ---------------------------------------------------------------------------
#[test]
fn vocal_reflex_regulating_pressure_uses_learned_bias() {
    // --- scenario A: moderate drive, moderate learned bias → burst fires ---
    {
        let mut state = VocalReflexState::default();
        let drive = VocalReflexDrive {
            drive_pressure: 0.8,
            energy: 0.5,
        };

        let burst = state.tick_drive(1_000_000, drive, 0.5, 0.5, 0.0, 0.0);

        assert!(
            burst.is_some(),
            "reflex_pressure should exceed 0.40 threshold with moderate bias"
        );
    }

    // --- scenario B: low drive, high learned bias → burst suppressed ---
    {
        let mut state = VocalReflexState::default();
        let drive = VocalReflexDrive {
            drive_pressure: 0.4,
            energy: 0.5,
        };

        let burst = state.tick_drive(2_000_000, drive, 0.8, 0.8, 0.0, 0.0);

        assert!(
            burst.is_none(),
            "regulating-pressure from high learned bias should suppress burst below 0.40"
        );
    }
}
