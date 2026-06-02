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

#[test]
fn voicing_closure_raises_efficacy() {
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.read_learned_bias().vocal_efficacy, 0.0, "newborn efficacy 0");

    // Repeated effective voicing (vocalized=true with closure support).
    let mut t = 1_000_000u64;
    for _ in 0..20 {
        vp.observe_vocal_reflex_closure(t, 0.9, true);
        t += 100_000;
    }

    let eff = vp.read_learned_bias().vocal_efficacy;
    assert!(
        eff > 0.0,
        "effective voicing must raise vocal_efficacy (爱叫): {eff}"
    );
}

#[test]
fn vocal_biases_survive_checkpoint_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_efficacy_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = LearnedBiasCheckpointPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    let mut t = 1_000_000u64;
    for _ in 0..30 {
        vp.observe_vocal_reflex_closure(t, 0.9, true); // raises fam + efficacy
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
