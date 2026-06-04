// Verify that probe drive follows *learning progress* (error declining rate),
// not raw surprise height.
//
// Noisy-TV contract: a region with high but unlearnable surprise must produce
// near-zero INTERO_LEARNING_PROGRESS and therefore near-zero probe drive.
// A region with moderate but *declining* surprise must produce positive
// learning progress and positive probe drive.
//
// Three test groups:
//   A. prediction_improvement computation — noise vs learning sequences.
//   B. probe intensity — learning_progress drives probe; raw surprise does not.
//   C. end-to-end — learnable axis preferred over noisy axis after self-dev ticks.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;

/// Deterministic pseudo-noise in (0.1, 0.9) with no temporal structure.
fn pseudo_noise(seed: u64) -> f64 {
    let h = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    0.1 + 0.8 * (((h >> 33) % 1_000) as f64 / 1_000.0)
}

// ── A: prediction_improvement is near-0 for noise, positive for learning ──

#[test]
fn prediction_improvement_stays_low_for_stationary_noise() {
    // Warmup with stable gap, then 120 steps of noisy gap — slow and fast
    // EMAs track the same mean → signed trend ≈ 0 → improvement ≈ 0.
    let vp = Arc::new(Viewport::new());
    let mut t = 1_000u64;
    const STEP: u64 = 50_000;

    // Warmup: stable gap 0.5 so slow EMA converges before noise starts.
    for _ in 0..60u64 {
        vp.global_reasoning.write().unwrap().global_surprise_load = 0.5;
        vp.tick_self_development_from_substrate(t);
        t += STEP;
    }

    // Noise: stationary fluctuation around 0.5.
    for step in 0..120u64 {
        let gap = 0.35 + 0.3 * pseudo_noise(step);
        vp.global_reasoning.write().unwrap().global_surprise_load = gap;
        vp.tick_self_development_from_substrate(t);
        t += STEP;
    }

    let lp = vp.read_self_development().prediction_improvement;
    assert!(
        lp < 0.05,
        "stationary noise must produce near-zero learning progress, got {lp:.4}"
    );
}

#[test]
fn prediction_improvement_is_positive_during_gap_decline() {
    // Warmup, then steadily declining gap — slow EMA lags above current → trend > 0.
    let vp = Arc::new(Viewport::new());
    let mut t = 1_000u64;
    const STEP: u64 = 50_000;

    // Warmup: stable gap 0.7 so slow EMA converges near 0.7.
    for _ in 0..60u64 {
        vp.global_reasoning.write().unwrap().global_surprise_load = 0.7;
        vp.tick_self_development_from_substrate(t);
        t += STEP;
    }

    // Decline: 0.7 → 0.15 over 120 steps.
    for step in 0..120u64 {
        let gap = 0.70 - 0.55 * (step as f64 / 120.0);
        vp.global_reasoning.write().unwrap().global_surprise_load = gap;
        vp.tick_self_development_from_substrate(t);
        t += STEP;
    }

    let lp = vp.read_self_development().prediction_improvement;
    assert!(
        lp > 0.05,
        "declining gap must produce positive learning progress, got {lp:.4}"
    );
}

// ── B: probe drive rewiring ────────────────────────────────────────────────

#[test]
fn probe_drive_learning_progress_starts_write_epoch() {
    let vp = Viewport::new();
    vp.self_development.write().unwrap().prediction_improvement = 0.75;

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    assert_eq!(
        vp.read_action_output().n_write_epochs,
        1,
        "learning progress must start a probe write epoch"
    );
}

#[test]
fn probe_drive_raw_surprise_alone_fires_nothing() {
    // High global_surprise_load but zero learning progress → no probe.
    let vp = Viewport::new();
    vp.global_reasoning.write().unwrap().global_surprise_load = 1.0;
    // prediction_improvement defaults to 0.

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    assert_eq!(
        vp.read_action_output().n_write_epochs,
        0,
        "raw surprise alone must not drive probe (noisy-TV fix)"
    );
}

#[test]
fn probe_intensity_higher_with_learning_progress_than_with_raw_surprise() {
    // Learnable axis: moderate learning_progress, zero raw surprise.
    let vp_learnable = Viewport::new();
    vp_learnable
        .self_development
        .write()
        .unwrap()
        .prediction_improvement = 0.65;
    vp_learnable.refresh_probe_law(1_000_000);
    let intensity_learnable = vp_learnable.read_probe_law().intensity;

    // Noisy axis: high raw surprise, zero learning_progress.
    let vp_noisy = Viewport::new();
    vp_noisy
        .global_reasoning
        .write()
        .unwrap()
        .global_surprise_load = 1.0;
    vp_noisy.refresh_probe_law(1_000_000);
    let intensity_noisy = vp_noisy.read_probe_law().intensity;

    assert!(
        intensity_learnable > intensity_noisy,
        "learning_progress(0.65) must drive probe more than raw surprise(1.0): \
         learnable={intensity_learnable:.4}, noisy={intensity_noisy:.4}"
    );
    assert!(intensity_learnable > 0.0, "learning progress must produce nonzero probe intensity");
    assert_eq!(intensity_noisy, 0.0, "raw surprise must produce zero probe intensity");
}

// ── C: end-to-end — self-dev ticks drive probe preference ─────────────────

#[test]
fn probe_prefers_learnable_over_noisy_axis_after_self_dev_ticks() {
    // Learnable axis: warmup at 0.6, then decline to 0.15.
    // Noisy axis: warmup at 0.5, then stationary noise around 0.5.
    // After the same number of self-dev ticks the learnable axis should
    // have higher probe intensity than the noisy one.
    const STEP: u64 = 50_000;

    let vp_learnable = Arc::new(Viewport::new());
    let mut t = 1_000u64;
    for _ in 0..60u64 {
        vp_learnable
            .global_reasoning
            .write()
            .unwrap()
            .global_surprise_load = 0.6;
        vp_learnable.tick_self_development_from_substrate(t);
        t += STEP;
    }
    for step in 0..120u64 {
        let gap = 0.60 - 0.45 * (step as f64 / 120.0);
        vp_learnable
            .global_reasoning
            .write()
            .unwrap()
            .global_surprise_load = gap;
        vp_learnable.tick_self_development_from_substrate(t);
        t += STEP;
    }
    vp_learnable.refresh_probe_law(t);
    let intensity_learnable = vp_learnable.read_probe_law().intensity;

    let vp_noisy = Arc::new(Viewport::new());
    let mut t = 1_000u64;
    for _ in 0..60u64 {
        vp_noisy
            .global_reasoning
            .write()
            .unwrap()
            .global_surprise_load = 0.5;
        vp_noisy.tick_self_development_from_substrate(t);
        t += STEP;
    }
    for step in 0..120u64 {
        let gap = 0.35 + 0.30 * pseudo_noise(step + 7);
        vp_noisy
            .global_reasoning
            .write()
            .unwrap()
            .global_surprise_load = gap;
        vp_noisy.tick_self_development_from_substrate(t);
        t += STEP;
    }
    vp_noisy.refresh_probe_law(t);
    let intensity_noisy = vp_noisy.read_probe_law().intensity;

    assert!(
        intensity_learnable > intensity_noisy,
        "learnable axis (declining gap) must drive probe more than noisy axis: \
         learnable={intensity_learnable:.4}, noisy={intensity_noisy:.4}"
    );
}
