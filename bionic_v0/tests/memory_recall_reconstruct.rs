//! R1 (within-lifetime): recall re-descend can RECONSTRUCT a memory's detail
//! from the learned cross-layer RELATION RULE, not only fetch a live slot's raw
//! value. "想起梗概 → 仔细想想 → 用关系把细节推断出来."
//!
//! When the target slot carries a learned cross-layer predictor (xlayer), the
//! recall can re-derive the target's object_err by running that rule forward
//! over the relation-neighbours' current state — and marks the result
//! `reconstructed = true`. With no rule, it falls back to the raw live detail
//! (reconstructed = false), or None if there is nothing to descend to.
//!
//! Scope: within a single lifetime (the object/slot still exists). Surviving
//! restart + re-identifying the same object depends on persistent object
//! identity, which is deferred to C5 (caregiver imprinting), same root cause.
//!
//! Contracts:
//!   1. A target with a learned relation rule re-descends with reconstructed=true
//!      and a finite reconstructed_err.
//!   2. A target with NO relation rule re-descends with reconstructed=false
//!      (raw live detail path), still detail_present.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_VIB, VIB_BANDS};

fn tick_slot(vp: &Arc<Viewport>, slot: usize, t: u64, level: f64) {
    let obs = [level; VIB_BANDS];
    let pred = [level * 0.9; VIB_BANDS];
    let err = [level * 0.1; VIB_BANDS];
    let prec = [0.5; VIB_BANDS];
    vp.write_slot_vib(slot, t, &obs, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot, MOD_VIB, t);
    vp.refresh_slot_high_layer(slot, t);
}

/// Drive two coupled slots and the target's cross-layer predictor, plus the
/// reasoning history so a gist with a back-link to the target gets pushed.
fn drive_with_rule(vp: &Arc<Viewport>, a: usize, b: usize, t0: u64, dt: u64, n: u64) {
    for i in 0..n {
        let t = t0 + i * dt;
        tick_slot(vp, a, t, 0.6);
        tick_slot(vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        vp.refresh_global_reasoning(t);
        vp.refresh_global_reasoning_history(t);
        vp.tick_slot_cross_layer(a, t);
    }
}

#[test]
fn recall_reconstructs_from_relation_rule() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    drive_with_rule(&vp, a, b, t0, dt, 40);

    // Precondition: a learned a relation rule.
    let xl = vp.read_slot_cross_layer(a).expect("a should have an xlayer");
    assert!(!xl.source_slots.is_empty(), "a must have learned a relation source");

    let detail = vp.recall_redescend(0).expect("most recent gist re-descends");
    assert!(
        detail.reconstructed,
        "with a learned relation rule, recall must reconstruct (not just raw fetch)"
    );
    assert!(
        detail.reconstructed_err.is_finite(),
        "reconstructed_err must be finite"
    );
}

#[test]
fn recall_without_rule_falls_back_to_raw() {
    let vp = Arc::new(Viewport::new());
    let t0 = 30_000_000u64;
    let dt = 10_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Drive slots + reasoning history, but NEVER tick the cross-layer predictor,
    // so the focused target carries no learned relation rule.
    for i in 0..40u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        vp.refresh_global_reasoning(t);
        vp.refresh_global_reasoning_history(t);
    }

    let detail = vp.recall_redescend(0).expect("most recent gist re-descends");
    assert!(
        !detail.reconstructed,
        "with no relation rule, recall falls back to raw live detail"
    );
    assert!(
        detail.detail_present,
        "raw fallback still has live detail present"
    );
}
