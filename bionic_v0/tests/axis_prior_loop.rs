//! A2 (VPC mechanism 5): the axis-level prior write-back leg is actually wired.
//!
//! These tests OBSERVE numeric dynamics as-is (推论4: 如实观察, no behavioral
//! targets):
//!   1. With a learnable target (its low-layer err correlates with a sparse
//!      source) plus a pure-noise source, after some ticks the prior channel
//!      holds a non-zero finite value bounded by MAX_PRIOR_MAGNITUDE, and the
//!      noise source's weight stays far below the real source's.
//!   2. Closing the loop through a REAL AR-2 predictor lowers the target's
//!      realized |err| EMA versus an identical run without the high layer.
//!   3. push_prior clamps: 1e9 → MAX_PRIOR_MAGNITUDE, NaN/inf → 0.
//!   4. The bank's learned state survives a snapshot roundtrip field-by-field;
//!      a pre-v59 snapshot reads back an empty bank without panicking.
//!   5. The hard-coded prior table is sparse: every target has at most a
//!      handful of explicit sources, never an all-axis fan-in.

use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc_cross_layer::{axis_predictor_bank, CrossLayerPredictor};
use bionic_v0::{AXIS_COUNT, AXIS_VIB, MAX_PRIOR_MAGNITUDE};

/// Drive a synthetic coupling: source axis B oscillates, target axis A's
/// low-layer err is a function of B; noise axis is uncorrelated.
fn drive(vp: &Arc<Viewport>, axis_a: usize, axis_b: usize, axis_noise: usize, t: u64, step: u64) {
    let b = 0.5 + 0.4 * (((step % 8) as f64) / 8.0 - 0.5);
    let a_err = 0.3 * (b - 0.5);
    let noise = 0.5 + 0.4 * (((step.wrapping_mul(2654435761) % 7) as f64) / 7.0 - 0.5);
    vp.push_observation(axis_b, t, b);
    vp.push_observation(axis_noise, t, noise);
    vp.push_err(axis_a, t, a_err);
}

#[test]
fn wired_loop_writes_bounded_nonzero_prior_and_rejects_noise() {
    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b, axis_noise) = (AXIS_VIB, AXIS_VIB + 1, AXIS_VIB + 2);
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b, axis_noise]);

    assert_eq!(vp.read_prior(axis_a), 0.0, "prior starts at 0 (no producer)");

    let mut t = 1_000u64;
    for step in 0..400u64 {
        drive(&vp, axis_a, axis_b, axis_noise, t, step);
        clp.tick(&vp);
        t += 10_000;
    }

    // (a) prior became non-zero, finite, and bounded by the precision ceiling.
    let prior = vp.read_prior(axis_a);
    assert!(prior.is_finite(), "prior must be finite, got {prior}");
    assert!(prior.abs() > 1e-9, "prior must become non-zero after learning");
    assert!(
        prior.abs() <= MAX_PRIOR_MAGNITUDE,
        "|prior| must respect MAX_PRIOR_MAGNITUDE: {prior}"
    );

    // (b) noise rejection: uncorrelated source keeps far less weight.
    let w_b = clp.weight_for(axis_b).abs();
    let w_noise = clp.weight_for(axis_noise).abs();
    assert!(
        w_noise < w_b,
        "noise source must earn less weight than the coupled source: w_noise={w_noise}, w_b={w_b}"
    );
}

/// Closed loop through a REAL AR-2: the target axis carries a constant offset
/// component the cold AR-2 (a,b from 0) is slow to capture; the high layer
/// learns the err structure and its written prior lowers the realized |err|
/// EMA compared to an identical run with no high layer.
#[test]
fn closing_the_loop_lowers_target_err_ema_vs_control() {
    fn run(with_high_layer: bool) -> f64 {
        let vp = Arc::new(Viewport::new());
        let (axis_a, axis_b) = (AXIS_VIB, AXIS_VIB + 1);
        let mut low = Ar2Predictor::new(axis_a);
        let mut high = CrossLayerPredictor::new(axis_a, &[axis_b]);

        let mut err_ema = 0.0f64;
        let mut t = 1_000u64;
        for step in 0..600u64 {
            // The world: a steady offset on the target axis; the source axis
            // sits at a correlated steady level (so the high layer can explain
            // the constant err component via bias + source weight).
            let a_val = 0.5;
            let b_val = 0.5;
            vp.push_observation(axis_a, t, a_val);
            vp.push_observation(axis_b, t, b_val);
            // Low layer first (produces err for this step, consuming whatever
            // prior currently stands), then the high layer (reads that err and
            // refreshes the prior for the next step) — body rhythm order.
            low.tick(&vp);
            if with_high_layer {
                high.tick(&vp);
            }
            if let Some(e) = vp.latest_err(axis_a) {
                err_ema = 0.98 * err_ema + 0.02 * e.abs();
            }
            t += 10_000;
        }
        err_ema
    }

    let ema_control = run(false);
    let ema_looped = run(true);
    assert!(
        ema_looped < ema_control,
        "prior write-back must lower target |err| EMA vs control: looped={ema_looped}, control={ema_control}"
    );
}

#[test]
fn push_prior_clamps_magnitude_and_zeroes_non_finite() {
    let vp = Arc::new(Viewport::new());
    let axis = AXIS_VIB;

    vp.push_prior(axis, 1e9);
    assert_eq!(vp.read_prior(axis), MAX_PRIOR_MAGNITUDE, "huge prior clamps to +max");

    vp.push_prior(axis, -1e9);
    assert_eq!(vp.read_prior(axis), -MAX_PRIOR_MAGNITUDE, "huge negative clamps to -max");

    vp.push_prior(axis, f64::NAN);
    assert_eq!(vp.read_prior(axis), 0.0, "NaN prior collapses to 0");

    vp.push_prior(axis, f64::INFINITY);
    assert_eq!(vp.read_prior(axis), 0.0, "inf prior collapses to 0");

    vp.push_prior(axis, 3.25);
    assert_eq!(vp.read_prior(axis), 3.25, "in-range prior passes through");
}

#[test]
fn bank_state_survives_snapshot_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_axis_xlayer_rt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    let (axis_a, axis_b, axis_noise) = (AXIS_VIB, AXIS_VIB + 1, AXIS_VIB + 2);
    let mut clp = CrossLayerPredictor::new(axis_a, &[axis_b, axis_noise]);
    let mut t = 1_000u64;
    for step in 0..200u64 {
        drive(&vp, axis_a, axis_b, axis_noise, t, step);
        clp.tick(&vp);
        t += 10_000;
    }
    let live = clp.to_state();
    assert!(
        live.source_weights.iter().any(|w| w.abs() > 1e-9) || live.bias.abs() > 1e-9,
        "precondition: learned state must be non-trivial"
    );

    vp.store_axis_xlayer_states(vec![live.clone()]);
    write_snapshot(&vp, &paths).expect("write snapshot");
    let restored = read_snapshot(&paths.final_path).expect("read snapshot");

    assert_eq!(restored.axis_xlayers.len(), 1, "one bank entry must roundtrip");
    let p = &restored.axis_xlayers[0];
    assert_eq!(p.target_axis, live.target_axis);
    assert_eq!(p.source_axes, live.source_axes);
    assert_eq!(p.source_weights, live.source_weights);
    assert_eq!(p.bias, live.bias);
    assert_eq!(p.err_pred_mse, live.err_pred_mse);
    assert_eq!(p.prev_mse, live.prev_mse);
    assert_eq!(p.learning_progress, live.learning_progress);
    assert_eq!(p.target_err_var, live.target_err_var);
    assert_eq!(p.n_updates, live.n_updates);

    // The body's boot path rebuilds live predictors via from_state.
    let revived = CrossLayerPredictor::from_state(p).expect("from_state must accept its own state");
    assert_eq!(revived.to_state(), *p, "to_state(from_state(s)) == s");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn pre_v59_snapshot_reads_back_empty_bank_without_panic() {
    let dir = std::env::temp_dir().join(format!("bionic_axis_xlayer_old_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    vp.store_axis_xlayer_states(vec![CrossLayerPredictor::new(AXIS_VIB, &[AXIS_VIB + 1]).to_state()]);
    write_snapshot(&vp, &paths).expect("write snapshot");

    // Patch the version field (bytes [8..16), after the u64 magic) down to 58.
    // read_snapshot enforces STRICT version equality (snapshot.rs rejects any
    // version != SNAPSHOT_VERSION with InvalidData) — the long-standing reader
    // contract B1/A1 also live under. So "old snapshot" must degrade to a
    // graceful Err (never a panic), and the body's boot path then falls back
    // to the fresh hard-coded bank.
    let mut bytes = std::fs::read(&paths.final_path).expect("read raw snapshot");
    bytes[8..16].copy_from_slice(&58u64.to_le_bytes());
    std::fs::write(&paths.final_path, &bytes).expect("write patched snapshot");

    let restored = read_snapshot(&paths.final_path);
    assert!(
        restored.is_err(),
        "pre-v59 snapshot must be rejected gracefully (Err, not panic)"
    );

    // Mirror of the body boot fallback: Err → empty restore → fresh bank.
    let bank: Vec<CrossLayerPredictor> = restored
        .map(|snap| {
            snap.axis_xlayers
                .iter()
                .filter_map(CrossLayerPredictor::from_state)
                .collect()
        })
        .unwrap_or_default();
    let bank = if bank.is_empty() { axis_predictor_bank() } else { bank };
    assert!(
        !bank.is_empty(),
        "boot fallback must yield the fresh hard-coded bank"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn hardcoded_prior_table_is_sparse_and_in_range() {
    let bank = axis_predictor_bank();
    assert!(
        (10..=80).contains(&bank.len()),
        "first wiring stays restrained: tens of targets, not all axes (got {})",
        bank.len()
    );
    for clp in &bank {
        let st = clp.to_state();
        assert!(st.target_axis < AXIS_COUNT, "target in range");
        assert!(
            !st.source_axes.is_empty() && st.source_axes.len() <= 4,
            "each target reads a sparse explicit source set (1..=4), got {} for target {}",
            st.source_axes.len(),
            st.target_axis
        );
        for &s in &st.source_axes {
            assert!(s < AXIS_COUNT, "source in range");
            assert_ne!(s, st.target_axis, "never self-source");
        }
    }
}
