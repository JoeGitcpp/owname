//! B1-3a: slot→slot cross-layer with RELATION-GATED source selection.
//!
//! 跨层粒度 = 对象间 (slot→slot). A cross-layer predictor for a TARGET slot
//! predicts that slot's object_err from a SPARSE set of SOURCE slots — and the
//! source set is chosen AUTOMATICALLY by the relation graph: only slots that
//! have an active relation edge into the target (with enough confidence) become
//! sources. A noise object grows no active relation edge → it never enters the
//! source set (this completes step3: relation-gated noise rejection).
//!
//! PRIME-DIRECTIVE RED LINE at this granularity:
//!   - SPARSE only: a target reads just its relation-neighbours, NEVER all 8
//!     slots (an all-slots matrix would be a disguised BP / global coupling).
//!   - Each predictor updates only its OWN weights from its OWN local error.
//!   - It never writes another slot's low-layer predictor coefficients.
//!
//! Contracts:
//!   1. gated_sources(target) returns slots that have an active relation edge
//!      into the target — a SPARSE subset, never the full slot set.
//!   2. A slot with no active relation edge (isolated/noise) is NOT selected.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::vpc_cross_layer::gated_sources;
use bionic_v0::{MOD_VIB, N_OBJECT_SLOTS, VIB_BANDS};

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
fn gated_sources_are_sparse_relation_neighbours_only() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;

    // Two coupled, co-active slots (a,b) build a relation between them.
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Drive a,b together for many cycles so an active relation edge forms.
    for i in 0..24u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
    }

    // Sources gated for target `a` must be a sparse subset (not all slots),
    // and must come from its relation neighbours (here: b).
    let srcs = gated_sources(&vp, a);
    assert!(
        srcs.len() < N_OBJECT_SLOTS,
        "gated sources must be SPARSE, never the full slot set (no全连接): got {}",
        srcs.len()
    );
    assert!(
        srcs.contains(&b),
        "relation neighbour b must be gated in as a source for a: {:?}",
        srcs
    );
    assert!(
        !srcs.contains(&a),
        "target must not be its own source: {:?}",
        srcs
    );
}

#[test]
fn isolated_noise_slot_is_not_gated_in() {
    let vp = Arc::new(Viewport::new());
    let t0 = 20_000_000u64;
    let dt = 10_000u64;

    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
    // c is allocated but driven on a DISJOINT schedule (never co-active with a)
    // so it builds no active relation edge into a → a noise/isolated object.
    let c = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc c");

    for i in 0..24u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        // c only ticked on far-apart, non-overlapping times (no co-activation).
        if i == 0 {
            tick_slot(&vp, c, t0 - 5_000_000, 0.6);
        }
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
    }

    let srcs = gated_sources(&vp, a);
    assert!(
        !srcs.contains(&c),
        "isolated/noise slot c (no active relation into a) must NOT be gated in: {:?}",
        srcs
    );
}
