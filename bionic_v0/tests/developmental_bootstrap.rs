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

use bionic_v0::viewport::Viewport;
use bionic_v0::{binding_align_min, binding_deviate_max, MOD_VIB};

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
