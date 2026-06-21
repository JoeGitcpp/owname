//! 改造1 闭环: voicing effectiveness accumulates and survives death.
//!
//! When voicing brings a real closure, vocal_efficacy grows (爱叫). And the
//! vocal biases — prediction_familiarity, action_suppression, vocal_efficacy —
//! now actually persist across a checkpoint roundtrip (a pre-existing refactor
//! gap: before v4 they were dropped on save and reset to 0 on load).
//!
//! Contracts:
//!   1. Voicing + closure raises vocal_efficacy from 0.
//!   2. checkpoint write→read preserves all three vocal biases (死后保留).

use std::sync::Arc;

use bionic_v0::learned_bias::{
    read_learned_bias_checkpoint, write_learned_bias_checkpoint, LearnedBiasCheckpointPaths,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;

/// Set up a viewport so that a vocalized closure carries real acoustic
/// competence evidence: a target contour AND a heard-self that matches it
/// (fit≈1), mirroring the real nursery where the body hears itself voice and
/// compares to the fed audio. Without this, vocal_efficacy correctly stays 0
/// (no target to be competent at).
fn pin_matching_self_and_target(vp: &Arc<Viewport>) {
    let mut shape = [0.0f64; VIB_BANDS];
    shape[0] = 1.0;
    shape[1] = 0.6;
    shape[2] = 0.3;
    vp.observe_vocal_target_bands(1, 0.6, 0.4, 0.7, 0.6, shape);
    *vp.self_vocal_bands.write().unwrap() = (shape, 1); // heard-self == target → fit≈1
}

#[test]
fn voicing_closure_raises_efficacy() {
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.read_learned_bias().vocal_efficacy, 0.0, "newborn efficacy 0");
    pin_matching_self_and_target(&vp);

    // Repeated effective voicing whose heard-self matches the target → real
    // acoustic competence → efficacy EMA rises from 0.
    // begin_voicing makes the organ Active so the D-fix freshness gate
    // (vib_self_match_weight>0, voiced within 150ms) admits the heard-self as
    // evidence — exactly the real nursery loop the docstring references. The
    // pinned self_vocal_bands (written AFTER voicing) controls the matched shape.
    let mut t = 1_000_000u64;
    for k in 0..200u64 {
        vp.begin_voicing(t, 0.8, 0.5, 0.5);
        *vp.self_vocal_bands.write().unwrap() = ({
            let mut s = [0.0f64; VIB_BANDS];
            s[0] = 1.0;
            s[1] = 0.6;
            s[2] = 0.3;
            s
        }, t);
        vp.observe_vocal_reflex_closure(t, 0.9, true);
        t += 100_000;
        let _ = k;
    }

    let eff = vp.read_learned_bias().vocal_efficacy;
    assert!(
        eff > 0.0,
        "voicing that hears itself match the target must raise vocal_efficacy (爱叫): {eff}"
    );
}

#[test]
fn vocal_biases_survive_checkpoint_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_efficacy_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = LearnedBiasCheckpointPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    pin_matching_self_and_target(&vp);
    let mut t = 1_000_000u64;
    for _ in 0..200 {
        vp.begin_voicing(t, 0.8, 0.5, 0.5); // organ Active → D-fix freshness gate admits evidence
        *vp.self_vocal_bands.write().unwrap() = ({
            let mut s = [0.0f64; VIB_BANDS];
            s[0] = 1.0;
            s[1] = 0.6;
            s[2] = 0.3;
            s
        }, t);
        vp.observe_vocal_reflex_closure(t, 0.9, true); // matches target → raises fam + efficacy
        vp.observe_vocal_reflex_closure(t + 50_000, 0.9, false); // raises suppression
        t += 200_000;
    }
    let before = vp.read_learned_bias();
    assert!(before.vocal_efficacy > 0.0 && before.prediction_familiarity > 0.0);

    write_learned_bias_checkpoint(&before, &paths).expect("write");
    let restored = read_learned_bias_checkpoint(&paths.final_path).expect("read");

    assert!(
        (restored.vocal_efficacy - before.vocal_efficacy).abs() < 1e-9,
        "vocal_efficacy must persist: {} vs {}",
        restored.vocal_efficacy,
        before.vocal_efficacy
    );
    assert!(
        (restored.prediction_familiarity - before.prediction_familiarity).abs() < 1e-9,
        "prediction_familiarity must persist (was dropped before v4)"
    );
    assert!(
        (restored.action_suppression - before.action_suppression).abs() < 1e-9,
        "action_suppression must persist (was dropped before v4)"
    );

    std::fs::remove_dir_all(&dir).ok();
}
