//! B1-3b-i: per-slot CrossLayer mounted on the viewport.
//!
//! The viewport owns a per-slot cross-layer predictor (ephemeral, like
//! phot_pred / hl_pred). `tick_slot_cross_layer(target, now)` is a viewport-
//! level method (it must read OTHER slots' object_err via the shared viewport —
//! 公理8), gates its sources through `gated_sources` (relation graph), and
//! learns to predict the target slot's object_err from its relation neighbours.
//!
//! PRIME DIRECTIVE: sparse (relation-gated) sources only, never all slots;
//! each predictor updates only its own weights; never writes another slot's
//! low-layer coefficients.
//!
//! Contracts:
//!   1. After driving two coupled slots, ticking the target's cross-layer
//!      predictor populates a readable cross-layer state (it ran).
//!   2. The mounted predictor's sources are the relation-gated neighbours
//!      (sparse), matching gated_sources — and it learns (mse finite, updates>0).

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

#[test]
fn mounted_cross_layer_runs_and_reads_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;

    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..40u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        // Drive the per-slot cross-layer predictor for target a.
        vp.tick_slot_cross_layer(a, t);
    }

    let state = vp
        .read_slot_cross_layer(a)
        .expect("target a must have a mounted cross-layer state after ticking");
    assert!(
        state.n_updates > 0,
        "mounted cross-layer must have run (n_updates>0)"
    );
    assert!(
        state.err_pred_mse.is_finite(),
        "cross-layer mse must stay finite"
    );
    // Sources must be the sparse relation-gated neighbours (b), not all slots.
    assert!(
        state.source_slots.contains(&b),
        "mounted predictor must gate in relation neighbour b: {:?}",
        state.source_slots
    );
    assert!(
        !state.source_slots.contains(&a),
        "target must not be its own source: {:?}",
        state.source_slots
    );
}
