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
use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::{
    ar2_lr_scale, binding_align_min, binding_deviate_max, vpc_prior_gain_scale,
    AR2_LR_SCALE_NAIVE, MOD_VIB, VPC_PRIOR_GAIN_SCALE_NAIVE,
};

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
fn vocal_self_play_breaks_bootstrap_without_caregiver() {
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.read_learned_bias().vocal_efficacy, 0.0, "newborn efficacy 0");
    assert_eq!(vp.developmental_maturity(), 0.0, "newborn naive");

    // Pure self-play: the body babbles (begin_voicing) and 10 ms later hears its
    // own voice come back as a VIB candidate (the echo). No external/caregiver
    // sound is ever injected. The echo is still kept out of the imitation target
    // (anti self-excitation), but it now closes a slow self-play loop.
    let mut t = 1_000_000u64;
    for _ in 0..20 {
        vp.begin_voicing(t, 0.6, 0.4, 0.7); // babble: organ goes Active
        let heard = t + 10_000; // hear self within the 150 ms self-match guard
        vp.write_ambient_for(MOD_VIB, 0.6, 0.1, Some(heard), true);
        vp.tick_vocal_target_from_vib(heard); // echo → self-play closure
        t += 300_000; // > VOCAL_SELF_PLAY_MIN_INTERVAL_US so each one counts
    }

    assert!(
        vp.read_learned_bias().vocal_efficacy > 0.0,
        "vocal self-play (babble + hear-self) must raise efficacy with no caregiver: {}",
        vp.read_learned_bias().vocal_efficacy
    );
    assert!(
        vp.read_learned_bias().prediction_familiarity > 0.0,
        "self-play must build familiarity (a predictable motor→sound consequence)"
    );
    assert!(
        vp.developmental_maturity() > 0.0,
        "self-play closures must advance the developmental clock: {}",
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
        vp.set_surprise(0, 1.0); // identical nonzero surprise for both bodies
        let mut p = Ar2Predictor::new(0);

        vp.push_observation(0, 10_000, 0.10);
        p.tick(&vp); // insufficient context
        vp.push_observation(0, 20_000, 0.20);
        p.tick(&vp); // open publication (horizon 10ms → target 30_000)
        vp.push_observation(0, 30_000, 0.50);
        p.tick(&vp); // resolves → one Widrow-Hoff step scaled by ar2_lr_scale(m)

        let (a, b, _, _, _, _) = vp.read_predictor(0);
        a.abs() + b.abs()
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
