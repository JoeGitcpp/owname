//! Phase 5D: Cross-Object Temporal Relation Substrate tests.
//!
//! Architecture invariants checked here:
//!   * Relation state lives in the viewport (GlobalRelationState), not in slots.
//!   * Only slots with attention_share > 0 enter relation computation.
//!   * co_change_score rises for synchronously-behaving slot pairs.
//!   * lead_lag_score has consistent direction for slots with stable timing offset.
//!   * Low-attention / stale slots produce low relation_confidence and drop out.
//!   * Snapshot roundtrip preserves all relation edge fields.
//!   * claim/scoring contract is completely unaffected.

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{EventEvidence, SlotClaim, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, N_REL_EDGES, PHOT_BANDS, VIB_BANDS};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tick_joint(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);

    let phot_obs = vec![0.2f64; PHOT_BANDS];
    let phot_pred = vec![0.18f64; PHOT_BANDS];
    let phot_err = vec![0.02f64; PHOT_BANDS];
    let phot_prec = vec![0.4f64; PHOT_BANDS];
    vp.write_slot_phot(slot_id, t_us, &phot_obs, &phot_pred, &phot_err, &phot_prec);
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);

    vp.refresh_slot_high_layer(slot_id, t_us);
}

#[allow(dead_code)]
fn tick_vib_only(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
    vp.refresh_slot_high_layer(slot_id, t_us);
}

/// Find the relation edge (src→dst) by linear search.
fn find_edge<'a>(
    edges: &'a [bionic_v0::viewport::RelationEdgeState; N_REL_EDGES],
    src: usize,
    dst: usize,
) -> &'a bionic_v0::viewport::RelationEdgeState {
    edges
        .iter()
        .find(|e| e.src_slot_id as usize == src && e.dst_slot_id as usize == dst)
        .expect("edge not found")
}

/// Run N full refresh cycles (attention then relations) at timestamps spaced 10ms apart.
#[allow(dead_code)]
fn run_cycles(vp: &Arc<Viewport>, n: u64, tick_fn: impl Fn(u64)) {
    for i in 0..n {
        let t = i * 10_000;
        tick_fn(t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }
}

// ---------------------------------------------------------------------------
// Test 1: synchronous_slots_raise_co_change_score
// ---------------------------------------------------------------------------

/// Two slots ticked identically (same attention_bias trajectory) must produce
/// a high co_change_score after many refreshes.
#[test]
fn synchronous_slots_raise_co_change_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Tick both identically for 40 cycles (their attention_bias values converge
    // to the same level → obs_co = 1.0 each tick → co_change_score → 1.0).
    for i in 0..40u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        tick_joint(&vp, slot_b, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    let gr = vp.read_global_relations();
    let ab = find_edge(&gr.edges, slot_a, slot_b);
    let ba = find_edge(&gr.edges, slot_b, slot_a);

    assert!(ab.active, "A→B edge must be active");
    assert!(ba.active, "B→A edge must be active");
    assert!(
        ab.co_change_score > 0.7,
        "co_change_score(A→B) must be high for sync slots; got {:.4}",
        ab.co_change_score
    );
    assert!(
        ba.co_change_score > 0.7,
        "co_change_score(B→A) must be high for sync slots; got {:.4}",
        ba.co_change_score
    );
}

// ---------------------------------------------------------------------------
// Test 2: lead_lag_detects_consistent_precedence
// ---------------------------------------------------------------------------

/// Slot A is always fused at t_base, slot B at t_base + delta.
/// After many cycles, lead_lag_score(A→B) must be positive.
/// The B→A edge must have negative lead_lag_score (B lags A).
#[test]
fn lead_lag_detects_consistent_precedence() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;
    let delta = 5_000u64; // A consistently 5ms before B

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..40u64 {
        let t_a = t0 + i * 20_000;
        let t_b = t_a + delta; // B always comes after A
        tick_joint(&vp, slot_a, t_a);
        tick_joint(&vp, slot_b, t_b);
        vp.refresh_global_attention(t_b);
        vp.refresh_global_relations(t_b);
    }

    let gr = vp.read_global_relations();
    let ab = find_edge(&gr.edges, slot_a, slot_b);
    let ba = find_edge(&gr.edges, slot_b, slot_a);

    assert!(
        ab.lead_lag_score > 0.0,
        "A→B lead_lag must be positive (A precedes B); got {:.4}",
        ab.lead_lag_score
    );
    assert!(
        ba.lead_lag_score < 0.0,
        "B→A lead_lag must be negative (B lags A); got {:.4}",
        ba.lead_lag_score
    );
    // They must be exact negatives of each other (symmetric by construction).
    assert!(
        (ab.lead_lag_score + ba.lead_lag_score).abs() < 1e-9,
        "lead_lag scores must sum to zero across directed pair; got {:.6} + {:.6}",
        ab.lead_lag_score,
        ba.lead_lag_score
    );
}

// ---------------------------------------------------------------------------
// Test 3: stale_or_low_attention_slots_drop_out_of_relation_set
// ---------------------------------------------------------------------------

/// A slot that never receives high_layer + attention data must not participate
/// in the relation computation: edges to/from it must remain inactive and
/// relation_confidence must stay near zero.
#[test]
fn stale_or_low_attention_slots_drop_out_of_relation_set() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;

    // Only slot_a is given data. slot_b is allocated but never ticked.
    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..20u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        // slot_b intentionally not ticked → no high_layer, no fusion
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    let gr = vp.read_global_relations();
    let ab = find_edge(&gr.edges, slot_a, slot_b);
    let ba = find_edge(&gr.edges, slot_b, slot_a);

    assert!(!ab.active, "A→B edge must be inactive (B has no data)");
    assert!(!ba.active, "B→A edge must be inactive (B has no data)");
    assert!(
        ab.relation_confidence < 0.05,
        "relation_confidence(A→B) must be near zero; got {:.4}",
        ab.relation_confidence
    );
}

// ---------------------------------------------------------------------------
// Test 4: snapshot_roundtrip_preserves_relation_state
// ---------------------------------------------------------------------------

/// write_snapshot → read_snapshot must preserve all GlobalRelationState fields
/// and edge scores to within floating-point round-trip precision.
#[test]
fn snapshot_roundtrip_preserves_relation_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..20u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        tick_joint(&vp, slot_b, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    let gr_before = vp.read_global_relations();

    let dir = std::env::temp_dir().join("bionic_v0_phase5d_rel_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let gr = &data.global_relations;
    assert_eq!(
        gr.n_active_edges, gr_before.n_active_edges,
        "n_active_edges"
    );
    assert_eq!(
        gr.last_update_t_us, gr_before.last_update_t_us,
        "last_update_t_us"
    );

    for i in 0..N_REL_EDGES {
        let a = &gr_before.edges[i];
        let b = &gr.edges[i];
        assert_eq!(b.src_slot_id, a.src_slot_id, "edge[{}].src_slot_id", i);
        assert_eq!(b.dst_slot_id, a.dst_slot_id, "edge[{}].dst_slot_id", i);
        assert_eq!(b.active, a.active, "edge[{}].active", i);
        assert!(
            (b.co_change_score - a.co_change_score).abs() < 1e-12,
            "edge[{}].co_change_score roundtrip",
            i
        );
        assert!(
            (b.lead_lag_score - a.lead_lag_score).abs() < 1e-12,
            "edge[{}].lead_lag_score roundtrip",
            i
        );
        assert!(
            (b.predictive_score - a.predictive_score).abs() < 1e-12,
            "edge[{}].predictive_score roundtrip",
            i
        );
        assert!(
            (b.relation_confidence - a.relation_confidence).abs() < 1e-12,
            "edge[{}].relation_confidence roundtrip",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5: observer_sees_relation_state (via snapshot)
// ---------------------------------------------------------------------------

/// After running relation refreshes, the snapshot's global_relations field must
/// contain active edges with non-trivial co_change_score for sync slots.
/// This is the test-level proxy for "observer can render relation state".
#[test]
fn observer_sees_relation_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..30u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        tick_joint(&vp, slot_b, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    let dir = std::env::temp_dir().join("bionic_v0_phase5d_obs_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let gr = &data.global_relations;
    let ab = find_edge(&gr.edges, slot_a, slot_b);

    assert!(ab.active, "A→B edge must be active in snapshot");
    assert!(
        ab.co_change_score > 0.5,
        "snapshot co_change_score(A→B) must be readable and > 0.5; got {:.4}",
        ab.co_change_score
    );
    assert!(
        gr.n_active_edges >= 2,
        "at least 2 directed edges must be active"
    );
}

// ---------------------------------------------------------------------------
// Test 6: claim_architecture_remains_unchanged_after_relations
// ---------------------------------------------------------------------------

/// refresh_global_relations must not alter claim/reuse/slot-count invariants.
#[test]
fn claim_architecture_remains_unchanged_after_relations() {
    let vp = Arc::new(Viewport::new());
    let t0 = 6_000_000u64;

    let ev1 = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0,
        event_energy: 0.1,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    };
    let claim1 = vp.claim_slot_for_event(&ev1);
    assert!(
        matches!(claim1, SlotClaim::Allocated(_)),
        "first VIB must allocate"
    );
    let slot_id = claim1.slot_id().unwrap();

    // Run relation refreshes.
    for i in 0..10u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_id, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    // Same-modality reuse must still work.
    let ev2 = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0 + 100_000,
        event_energy: 0.1,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    };
    let claim2 = vp.claim_slot_for_event(&ev2);
    assert!(
        matches!(claim2, SlotClaim::Reused(_)),
        "same-modality within window must reuse after relation refresh; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    // Active slot count unchanged.
    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(active, 1, "relation refresh must not change slot count");
}
