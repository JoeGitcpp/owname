//! E-1: Memory recall re-descend.
//!
//! Cognitive-index axiom (公理5): the high layer stores only an INDEX
//! (a reasoning-history entry: gist + a back-link to the source slot + t_us).
//! The body of the memory lives in the low layer. "Recalling" a gist and then
//! "re-descending" to recover precise detail = following that back-link down to
//! the low-layer slot that is still alive in the viewport.
//!
//! Contracts verified:
//!   1. A history entry whose top_slot_id points at a live slot can re-descend
//!      and recover that slot's current low-layer detail (precise, not faded).
//!   2. An empty history re-descends to None.
//!   3. Re-descend is read-only: it does not mutate viewport state.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_VIB, VIB_BANDS};

fn tick_joint(vp: &Arc<Viewport>, slot_id: usize, t_us: u64, level: f64) {
    let vib_obs = [level; VIB_BANDS];
    let vib_pred = [level * 0.9; VIB_BANDS];
    let vib_err = [level * 0.1; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
}

/// One full reasoning + history cycle over two synchronized slots, so a
/// routing focus forms and a gist with a slot back-link gets pushed.
fn cycle(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_us: u64, level: f64) {
    tick_joint(vp, slot_a, t_us, level);
    tick_joint(vp, slot_b, t_us, level * 0.8);
    vp.refresh_global_attention(t_us);
    vp.refresh_global_relations(t_us);
    vp.refresh_global_relation_predictor(t_us);
    vp.refresh_global_reasoning(t_us);
    vp.refresh_global_reasoning_history(t_us);
}

#[test]
fn redescend_recovers_live_low_layer_detail() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Enough synchronized cycles for a routing focus (top_slot) to form.
    for i in 0..16u64 {
        cycle(&vp, slot_a, slot_b, t0 + i * dt, 0.6);
    }

    // Recall the most recent gist (index 0) and re-descend to detail.
    let detail = vp.recall_redescend(0);
    assert!(
        detail.is_some(),
        "most recent gist should re-descend to live low-layer detail"
    );
    let d = detail.unwrap();
    assert!(
        d.slot_id >= 0,
        "recovered detail must carry the source slot id"
    );
    assert!(
        d.detail_present,
        "re-descend must recover present low-layer detail, not a faded blank"
    );
}

#[test]
fn redescend_out_of_range_is_none() {
    let vp = Arc::new(Viewport::new());
    assert!(
        vp.recall_redescend(0).is_none(),
        "empty history must re-descend to None"
    );
}

#[test]
fn redescend_is_read_only() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
    for i in 0..16u64 {
        cycle(&vp, slot_a, slot_b, t0 + i * dt, 0.5);
    }

    let before = vp.read_global_reasoning_memory_summary();
    let _ = vp.recall_redescend(0);
    let after = vp.read_global_reasoning_memory_summary();
    assert_eq!(
        before, after,
        "re-descend must be read-only; summary must be unchanged"
    );
}
