//! Developmental precision schedule (2026-06-06).
//!
//! These tests pin the two mechanisms that break the cold-start bootstrap
//! deadlock identified in the learning-curve analysis:
//!
//!   1. A developmental clock m∈[0,1] that advances with earned competence,
//!      saturating and bounded.
//!   2. Vocal self-play: a babbling newborn that hears its OWN voice (echo)
//!      now accrues vocal_efficacy + familiarity + maturity with NO caregiver —
//!      previously the echo guard cancelled self-voice and the loop never
//!      closed (0×x = 0 forever).
//!
//! Plus the maturity-scaled binding precision (loose newborn → strict mature).

use std::sync::Arc;

use bionic_v0::learned_bias::{
    read_learned_bias_checkpoint, write_learned_bias_checkpoint, LearnedBiasCheckpointPaths,
    LearnedBiasState,
};
use bionic_v0::predictor::ContextGatedPredictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::{
    ar2_lr_scale, binding_align_min, binding_deviate_max, vpc_prior_gain_scale,
    AR2_LR_SCALE_NAIVE, MOD_VIB, VIB_BANDS, VPC_PRIOR_GAIN_SCALE_NAIVE,
};

/// Run ONE self-play closure on a fresh viewport with the developmental clock
/// pinned to `m`, the heard-self bands set to `self_bands`, and the target
/// contour set to `target_bands`. Returns the vocal_efficacy after the single
/// closure. Since a newborn starts at efficacy 0 (and the time-forgetting law
/// only relaxes the already-zero value), the returned efficacy is exactly
/// `closure_support · 0.01` — a clean readout of the support the self-play loop
/// chose to credit. We only ever compare these readings against each other, so
/// the 0.01 scale is irrelevant; what we assert is the DIRECTION of the change.
fn self_play_efficacy_for(
    m: f64,
    self_bands: [f64; VIB_BANDS],
    target_bands: [f64; VIB_BANDS],
) -> f64 {
    let vp = Arc::new(Viewport::new());
    vp.restore_maturity(m);
    // Pin the target contour (the echo is dropped from the target path, so this
    // stands as the comparison reference for the acoustic-fit weighting).
    vp.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, target_bands);
    // Pin what the body "hears itself" produce.
    *vp.self_vocal_bands.write().unwrap() = (self_bands, 1);

    let t = 1_000_000u64;
    vp.begin_voicing(t, 0.6, 0.4, 0.7); // organ Active → self_match high
    let heard = t + 10_000; // within the 150 ms self-match guard
    // Re-pin self bands right before the closure (begin_voicing does not touch
    // them, but keep it explicit and robust to any internal resets).
    *vp.self_vocal_bands.write().unwrap() = (self_bands, heard);
    vp.write_ambient_for(MOD_VIB, 0.6, 0.1, Some(heard), true);
    vp.tick_vocal_target_from_vib(heard); // echo → single self-play closure
    vp.read_learned_bias().vocal_efficacy
}

#[test]
fn closure_efficacy_is_honest_about_acoustic_fit() {
    // Two band shapes: one concentrated in the low bands, one in the high bands.
    let mut low = [0.0f64; VIB_BANDS];
    let mut high = [0.0f64; VIB_BANDS];
    for i in 0..4 {
        low[i] = 1.0;
        high[VIB_BANDS - 1 - i] = 1.0;
    }

    // A single closure nudges efficacy (EMA from 0) by ≈ alpha·competence, where
    // competence = support·acoustic_fit. Heard-self matching the target → high
    // fit → larger efficacy bump; heard-self unlike the target → the body
    // honestly observes "I sound nothing like it" → a much smaller bump.
    let matched = self_play_efficacy_for(1.0, low, low);
    let mismatched = self_play_efficacy_for(1.0, high, low);
    assert!(
        mismatched < matched * 0.5,
        "an unlike babble must move efficacy markedly less than a matching one \
         (matched={matched}, mismatched={mismatched})"
    );
    assert!(matched > 0.0, "a real acoustic match must move efficacy: {matched}");

    // The acoustic-fit honesty is UNIFORM across maturity: at EVERY age an
    // unlike babble must earn markedly less than a matching one — maturity
    // never buys credit for a bad fit. What DOES anneal is the EMA step size
    // (A3 family, same shape as ar2_lr_scale): a newborn's matching closure
    // moves efficacy VOCAL_EFFICACY_ALPHA_SCALE_NAIVE× faster than a mature
    // body's, so the first genuine fits can lower the reflex gate at all
    // (0×x=0 bootstrap escape); the multiplier anneals to exactly 1.0 at m=1.
    let matched_newborn = self_play_efficacy_for(0.0, low, low);
    let mismatched_newborn = self_play_efficacy_for(0.0, high, low);
    assert!(
        mismatched_newborn < matched_newborn * 0.5,
        "acoustic-fit honesty must hold for newborns too \
         (matched={matched_newborn}, mismatched={mismatched_newborn})"
    );
    let scale = matched_newborn / matched;
    let naive = bionic_v0::VOCAL_EFFICACY_ALPHA_SCALE_NAIVE;
    assert!(
        (scale - naive).abs() < naive * 0.05,
        "newborn EMA step must be the annealed multiple of the mature step \
         (newborn={matched_newborn}, mature={matched}, scale={scale}, want≈{naive})"
    );
}

#[test]
fn high_rate_unlike_closures_do_not_saturate_efficacy() {
    // The exact failure observed live: the action-consequence pair path fires at
    // the orchestrator rate, so under a ratchet efficacy pinned to 1.0 within
    // seconds regardless of acoustic fit (illusory convergence). With the EMA +
    // acoustic-fit gating, a long stream of closures whose heard-self does NOT
    // match the target must keep efficacy LOW (bounded near the low competence),
    // never saturating.
    let mut low = [0.0f64; VIB_BANDS];
    let mut high = [0.0f64; VIB_BANDS];
    for i in 0..4 {
        low[i] = 1.0;
        high[VIB_BANDS - 1 - i] = 1.0;
    }

    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, low); // target = low shape
    *vp.self_vocal_bands.write().unwrap() = (high, 1); // heard-self = unlike (high)

    // 5000 full-support closures (far more than saturate a ratchet in seconds).
    // begin_voicing makes the heard-self FRESH (D-fix gate); the unlike shape then
    // earns low fit (high shape_dist), so efficacy stays low — not zeroed by the
    // gate (the body did voice), just bounded by the poor acoustic match.
    for k in 0..5000u64 {
        vp.begin_voicing(1 + k, 0.8, 0.5, 0.5);
        *vp.self_vocal_bands.write().unwrap() = (high, 1 + k);
        vp.observe_vocal_reflex_closure(1 + k, 1.0, true);
    }
    let eff_unlike = vp.read_learned_bias().vocal_efficacy;
    assert!(
        eff_unlike < 0.5,
        "high-rate unlike closures must NOT saturate efficacy (got {eff_unlike})"
    );

    // The leak that pinned the LIVE body to 1.0: the orchestrator pair path
    // fires mostly during silent moments where self_vocal_bands is empty. Such
    // evidence-less closures must carry NO competence — they must not move
    // efficacy at all (no "no-evidence → fit 1.0" fallback).
    let vp_silent = Arc::new(Viewport::new());
    vp_silent.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, low);
    // self_vocal_bands left EMPTY throughout.
    for k in 0..5000u64 {
        vp_silent.observe_vocal_reflex_closure(1 + k, 1.0, true);
    }
    let eff_silent = vp_silent.read_learned_bias().vocal_efficacy;
    assert!(
        eff_silent < 1e-6,
        "evidence-less (empty self_bands) closures must not move efficacy at all \
         (got {eff_silent})"
    );

    // Now matching closures: efficacy must rise well above the unlike case toward
    // the genuine competence level — efficacy reports present ability.
    let vp2 = Arc::new(Viewport::new());
    vp2.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, low);
    for k in 0..5000u64 {
        vp2.begin_voicing(1 + k, 0.8, 0.5, 0.5); // fresh heard-self (D-fix gate)
        *vp2.self_vocal_bands.write().unwrap() = (low, 1 + k); // heard == target
        vp2.observe_vocal_reflex_closure(1 + k, 1.0, true);
    }
    let eff_match = vp2.read_learned_bias().vocal_efficacy;
    assert!(
        eff_match > eff_unlike * 2.0,
        "matching closures must raise efficacy well above unlike ones \
         (match={eff_match}, unlike={eff_unlike})"
    );
}

#[test]
fn maturity_clock_is_monotone_saturating_and_bounded() {
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.developmental_maturity(), 0.0, "newborn starts naive");

    // Zero / negative support is a no-op.
    vp.advance_maturity(0.0);
    vp.advance_maturity(-1.0);
    assert_eq!(vp.developmental_maturity(), 0.0);

    // Each earned closure advances the clock, but never past 1.0.
    let mut prev = 0.0;
    for _ in 0..1000 {
        vp.advance_maturity(1.0);
        let m = vp.developmental_maturity();
        assert!(m >= prev, "clock must be monotone non-decreasing");
        assert!(m <= 1.0, "clock must be bounded by 1.0");
        prev = m;
    }
    assert!(prev > 0.99, "saturates toward 1.0 with enough competence: {prev}");
}

#[test]
fn binding_precision_is_loose_for_newborn_and_strict_for_mature() {
    // Newborn: no dead zone — align/deviate meet at the neutral midpoint.
    assert!((binding_align_min(0.0) - binding_deviate_max(0.0)).abs() < 1e-9);
    // Mature: restores the strict, separated thresholds (a real dead zone).
    assert!(binding_align_min(1.0) > binding_deviate_max(1.0) + 0.3);
    // Monotone tightening with maturity.
    assert!(binding_align_min(1.0) > binding_align_min(0.5));
    assert!(binding_deviate_max(1.0) < binding_deviate_max(0.5));
}

#[test]
fn vocal_self_play_builds_familiarity_but_not_target_competence() {
    // Corrected developmental semantics: babbling and hearing one's own echo is
    // a predictable motor→sound consequence — it builds FAMILIARITY (the body
    // learns its voice is its own and predictable), which breaks the cold-start
    // deadlock (there is now something learnable). But vocal *competence*
    // (efficacy) and the developmental clock (maturity) are reserved for
    // matching a target contour: a body babbling into a room with NO caregiver
    // model has nothing to be competent *at*, so neither advances. (This is what
    // stops "I babbled" from masquerading as "I learned to speak" — the illusory
    // convergence that pinned efficacy/maturity to 1.0.)
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.read_learned_bias().vocal_efficacy, 0.0, "newborn efficacy 0");
    assert_eq!(vp.developmental_maturity(), 0.0, "newborn naive");

    // Pure self-play, NO external/caregiver sound. The echo is kept out of the
    // imitation target (anti self-excitation); self_vocal_bands is never
    // populated here, mirroring the no-evidence-vs-a-target case.
    let mut t = 1_000_000u64;
    for _ in 0..20 {
        vp.begin_voicing(t, 0.6, 0.4, 0.7); // babble: organ goes Active
        let heard = t + 10_000; // hear self within the 150 ms self-match guard
        vp.write_ambient_for(MOD_VIB, 0.6, 0.1, Some(heard), true);
        vp.tick_vocal_target_from_vib(heard); // echo → self-play closure
        t += 300_000; // > VOCAL_SELF_PLAY_MIN_INTERVAL_US so each one counts
    }

    assert!(
        vp.read_learned_bias().prediction_familiarity > 0.0,
        "self-play must build familiarity (a predictable motor→sound consequence)"
    );
    assert_eq!(
        vp.read_learned_bias().vocal_efficacy, 0.0,
        "without a target to match, babble must NOT build vocal competence: {}",
        vp.read_learned_bias().vocal_efficacy
    );
    assert_eq!(
        vp.developmental_maturity(), 0.0,
        "without a target to match, babble must NOT advance the developmental clock: {}",
        vp.developmental_maturity()
    );
}

#[test]
fn external_sound_with_idle_organ_is_not_counted_as_self_play() {
    // Guard: if the organ is NOT voicing, an incoming VIB is external — it must
    // NOT be mistaken for self-play (self_match = 0), so efficacy stays 0.
    let vp = Arc::new(Viewport::new());
    let mut t = 1_000_000u64;
    for _ in 0..20 {
        // No begin_voicing → organ idle → self_match 0.
        vp.write_ambient_for(MOD_VIB, 0.6, 0.1, Some(t), true);
        vp.tick_vocal_target_from_vib(t);
        t += 300_000;
    }
    assert_eq!(
        vp.read_learned_bias().vocal_efficacy,
        0.0,
        "external sound (idle organ) must not be credited as vocal self-play"
    );
}

// --- A3: maturity persistence (跨重启继承) + precision annealing schedule ---

fn temp_checkpoint_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bionic_a3_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn learned_bias_v5_roundtrip_preserves_developmental_maturity() {
    let dir = temp_checkpoint_dir("roundtrip");
    let paths = LearnedBiasCheckpointPaths::new(&dir);

    let state = LearnedBiasState {
        developmental_maturity: 0.37,
        ..LearnedBiasState::default()
    };
    write_learned_bias_checkpoint(&state, &paths).unwrap();
    let read_back = read_learned_bias_checkpoint(&paths.final_path).unwrap();
    assert_eq!(
        read_back.developmental_maturity, 0.37,
        "maturity must survive a checkpoint roundtrip losslessly"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn learned_bias_v4_bytes_are_rejected_as_invalid_data() {
    let dir = temp_checkpoint_dir("v4_reject");
    let paths = LearnedBiasCheckpointPaths::new(&dir);

    let state = LearnedBiasState {
        developmental_maturity: 0.9,
        ..LearnedBiasState::default()
    };
    write_learned_bias_checkpoint(&state, &paths).unwrap();

    // Forge a v4 file: same magic, version u64 (bytes 8..16) patched to 4.
    let mut bytes = std::fs::read(&paths.final_path).unwrap();
    bytes[8..16].copy_from_slice(&4u64.to_le_bytes());
    std::fs::write(&paths.final_path, &bytes).unwrap();

    let err = read_learned_bias_checkpoint(&paths.final_path)
        .expect_err("a v4 checkpoint must be rejected (newborn fallback), not misread");
    assert_eq!(
        err.kind(),
        std::io::ErrorKind::InvalidData,
        "old-version bytes must surface as InvalidData"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn restore_maturity_clamps_and_newborn_default_holds() {
    // No checkpoint involved at all: a fresh Viewport is a naive newborn.
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.developmental_maturity(), 0.0, "newborn default must stay 0.0");

    vp.restore_maturity(0.5);
    assert_eq!(vp.developmental_maturity(), 0.5);

    vp.restore_maturity(-1.0);
    assert_eq!(vp.developmental_maturity(), 0.0, "restore clamps below to 0");

    vp.restore_maturity(2.0);
    assert_eq!(vp.developmental_maturity(), 1.0, "restore clamps above to 1");
}

#[test]
fn precision_annealing_schedules_are_loose_naive_exact_mature_and_monotone() {
    // m=0: tolerant (coarse) values, strictly looser than mature.
    assert_eq!(ar2_lr_scale(0.0), AR2_LR_SCALE_NAIVE);
    assert!(AR2_LR_SCALE_NAIVE > 1.0, "naive AR-2 lr must be amplified");
    assert_eq!(vpc_prior_gain_scale(0.0), VPC_PRIOR_GAIN_SCALE_NAIVE);
    assert!(VPC_PRIOR_GAIN_SCALE_NAIVE > 1.0, "naive prior gain must be amplified");

    // m=1: exactly back to the existing hard-coded behaviour (scale ≈ 1.0).
    assert!((ar2_lr_scale(1.0) - 1.0).abs() < 1e-12);
    assert!((vpc_prior_gain_scale(1.0) - 1.0).abs() < 1e-12);

    // Monotone tightening with maturity (and clamped outside [0,1]).
    let mut prev_lr = ar2_lr_scale(0.0);
    let mut prev_gain = vpc_prior_gain_scale(0.0);
    for i in 1..=10 {
        let m = i as f64 / 10.0;
        let lr = ar2_lr_scale(m);
        let gain = vpc_prior_gain_scale(m);
        assert!(lr <= prev_lr, "ar2_lr_scale must anneal monotonically");
        assert!(gain <= prev_gain, "vpc_prior_gain_scale must anneal monotonically");
        prev_lr = lr;
        prev_gain = gain;
    }
    assert_eq!(ar2_lr_scale(-3.0), ar2_lr_scale(0.0));
    assert_eq!(ar2_lr_scale(7.0), ar2_lr_scale(1.0));
}

/// Honest observation of the annealing dynamic on one axis (no behaviour
/// target): the same sample sequence is fed to a naive (m=0) and a mature
/// (m=1) body; after one resolved AR-2 update the naive body's coefficients
/// must have moved further (larger effective learning rate). Directional
/// numeric relation only.
#[test]
fn naive_body_takes_coarser_ar2_steps_than_mature_body() {
    let run = |maturity: f64| -> f64 {
        let vp = Arc::new(Viewport::new());
        vp.restore_maturity(maturity);
        vp.set_surprise(0, 1.0);
        let mut p = ContextGatedPredictor::new(0);

        vp.push_observation(0, 10_000, 0.10);
        p.tick(&vp);
        vp.push_observation(0, 20_000, 0.20);
        p.tick(&vp);
        vp.push_observation(0, 30_000, 0.50);
        p.tick(&vp);

        p.coeffs.iter().flat_map(|c| c.iter()).map(|v| v.abs()).sum()
    };

    let naive_step = run(0.0);
    let mature_step = run(1.0);
    assert!(
        naive_step > 0.0 && mature_step > 0.0,
        "both bodies must learn from the same surprise: naive {naive_step}, mature {mature_step}"
    );
    assert!(
        naive_step > mature_step,
        "naive effective learning rate must exceed mature: naive {naive_step} vs mature {mature_step}"
    );
}
