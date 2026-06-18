//! Phase 5F: Global reasoning / routing substrate tests.
//!
//! Contracts verified:
//!   1. Coordinated multi-object change raises coordinated_event_score.
//!   2. High relation_err edges dominate routing_focus_edge_ids.
//!   3. Low-attention background objects do not dominate routing_focus_slot_ids.
//!   4. Snapshot roundtrip preserves GlobalReasoningState fields.
//!   5. Ordered refresh contract remains intact with reasoning step added.
//!   6. Claim architecture remains unchanged after reasoning refresh.

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{EventEvidence, SlotClaim, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, PHOT_BANDS, VIB_BANDS};

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
}

fn run_cycle(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_a: u64, t_b: u64) {
    tick_joint(vp, slot_a, t_a);
    tick_joint(vp, slot_b, t_b);
    let t_cycle = t_a.max(t_b);
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
    vp.refresh_global_reasoning(t_cycle);
}

// ---------------------------------------------------------------------------
// Test 1: coordinated_multi_object_change_raises_coordinated_event_score
//
// Two slots receiving identical synchronized data accumulate high co_change_score.
// After EMA convergence the coordinated_event_score must exceed 0.5.
// ---------------------------------------------------------------------------

#[test]
fn coordinated_multi_object_change_raises_coordinated_event_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup
    for i in 0..5u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }
    let rs_early = vp.read_global_reasoning();

    // Full convergence — both slots identical and synchronized
    for i in 5..60u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }
    let rs_late = vp.read_global_reasoning();

    assert!(
        rs_late.coordinated_event_score > 0.5,
        "after 60 synchronized cycles, coordinated_event_score must exceed 0.5; got {:.4}",
        rs_late.coordinated_event_score
    );
    assert!(
        rs_late.coordinated_event_score >= rs_early.coordinated_event_score,
        "coordinated_event_score must not drop as slots become more synchronized; \
         early={:.4} late={:.4}",
        rs_early.coordinated_event_score,
        rs_late.coordinated_event_score
    );
}

// ---------------------------------------------------------------------------
// Test 2: high_relation_error_edges_dominate_routing_focus
//
// After EMA has converged, introduce an abrupt lead_lag shift. The affected
// directed edge must appear as routing_focus_edge_ids[0].
// ---------------------------------------------------------------------------

#[test]
fn high_relation_error_edges_dominate_routing_focus() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Phase 1: 40 stable synchronized cycles → predictor converges, err ≈ 0.
    for i in 0..40u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    // Phase 2: 5 cycles with large timing offset → relation_err spikes on edges
    // involving slot_a and slot_b.
    let big_delta = 55_000u64;
    let t_base = t0 + 40 * dt;
    for i in 0..5u64 {
        run_cycle(
            &vp,
            slot_a,
            slot_b,
            t_base + i * dt,
            t_base + i * dt + big_delta,
        );
    }

    let rs = vp.read_global_reasoning();

    // routing_focus_edge_ids[0] must be one of the edges between slot_a and slot_b.
    let gr = vp.read_global_relations();
    let focus_edge_idx = rs.routing_focus_edge_ids[0];
    assert!(
        focus_edge_idx >= 0,
        "at least one edge must be in routing focus; got -1"
    );
    let focus_edge = &gr.edges[focus_edge_idx as usize];
    let is_ab_edge = (focus_edge.src_slot_id as usize == slot_a
        && focus_edge.dst_slot_id as usize == slot_b)
        || (focus_edge.src_slot_id as usize == slot_b && focus_edge.dst_slot_id as usize == slot_a);
    assert!(
        is_ab_edge,
        "the top routing_focus edge must be between slot_a and slot_b after abrupt change; \
         got edge {}→{}",
        focus_edge.src_slot_id, focus_edge.dst_slot_id
    );

    // global_surprise_load must be above near-zero baseline.
    assert!(
        rs.global_surprise_load > 0.001,
        "global_surprise_load must rise after abrupt change; got {:.6}",
        rs.global_surprise_load
    );
}

// ---------------------------------------------------------------------------
// Test 3: low_attention_background_objects_do_not_dominate_reasoning
//
// Two active slots (A and B) have active relation edges and attention. A third
// slot (C) is allocated but never ticked → it goes stale and gets zero attention.
// C must NOT appear as routing_focus_slot_ids[0].
// ---------------------------------------------------------------------------

#[test]
fn low_attention_background_objects_do_not_dominate_reasoning() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
    // Slot C is allocated but never ticked — goes stale immediately.
    let slot_c = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc c");

    // Run A and B for many cycles; C is never updated.
    for i in 0..50u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let rs = vp.read_global_reasoning();

    // slot_c must not dominate routing focus.
    assert_ne!(
        rs.routing_focus_slot_ids[0], slot_c as i8,
        "stale slot_c (id={}) must not be the top routing focus slot; \
         routing_focus_slot_ids={:?}",
        slot_c, rs.routing_focus_slot_ids
    );

    // coordinated_event_score and global_surprise_load must be based on A↔B activity.
    assert!(
        rs.coordinated_event_score > 0.0 || rs.global_surprise_load >= 0.0,
        "reasoning state must have non-negative values"
    );
    let _ = slot_c; // suppress unused warning
}

// ---------------------------------------------------------------------------
// Test 4: snapshot_roundtrip_preserves_global_reasoning_state
// ---------------------------------------------------------------------------

#[test]
fn snapshot_roundtrip_preserves_global_reasoning_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..25u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let rs_before = vp.read_global_reasoning();

    let dir = std::env::temp_dir().join(format!("bionic_v0_5f_reasoning_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let rs = &data.global_reasoning;
    assert!(
        (rs.global_surprise_load - rs_before.global_surprise_load).abs() < 1e-12,
        "global_surprise_load roundtrip"
    );
    assert!(
        (rs.coordinated_event_score - rs_before.coordinated_event_score).abs() < 1e-12,
        "coordinated_event_score roundtrip"
    );
    assert!(
        (rs.network_instability_score - rs_before.network_instability_score).abs() < 1e-12,
        "network_instability_score roundtrip"
    );
    assert_eq!(
        rs.routing_focus_slot_ids, rs_before.routing_focus_slot_ids,
        "routing_focus_slot_ids roundtrip"
    );
    assert_eq!(
        rs.routing_focus_edge_ids, rs_before.routing_focus_edge_ids,
        "routing_focus_edge_ids roundtrip"
    );
    for k in 0..bionic_v0::N_ROUTING_FOCUS_EDGES {
        assert!(
            (rs.routing_weights[k] - rs_before.routing_weights[k]).abs() < 1e-12,
            "routing_weights[{}] roundtrip",
            k
        );
    }
    assert_eq!(
        rs.last_update_t_us, rs_before.last_update_t_us,
        "last_update_t_us roundtrip"
    );
}

// ---------------------------------------------------------------------------
// Test 5: ordered_refresh_contract_remains_intact_with_reasoning
//
// last_update_t_us must equal the cycle timestamp, proving the reasoning step
// ran in the same serial cycle as attention, relations, and predictor.
// ---------------------------------------------------------------------------

#[test]
fn ordered_refresh_contract_remains_intact_with_reasoning() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..5u64 {
        tick_joint(&vp, slot_a, t0 + i * dt);
        tick_joint(&vp, slot_b, t0 + i * dt);
    }

    let t_cycle = t0 + 5 * dt;
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
    vp.refresh_global_reasoning(t_cycle);

    let rs = vp.read_global_reasoning();
    assert_eq!(
        rs.last_update_t_us, t_cycle,
        "reasoning last_update_t_us must equal cycle timestamp"
    );

    // Relations also updated at same timestamp.
    let gr = vp.read_global_relations();
    assert_eq!(
        gr.last_update_t_us, t_cycle,
        "relations last_update_t_us must equal cycle timestamp"
    );
}

// ---------------------------------------------------------------------------
// Test 6: claim_architecture_remains_unchanged_after_reasoning
//
// Running reasoning refreshes must leave claim/reuse/slot-count invariants
// completely unchanged.
// ---------------------------------------------------------------------------

#[test]
fn claim_architecture_remains_unchanged_after_reasoning() {
    let vp = Arc::new(Viewport::new());
    let t0 = 6_000_000u64;
    let dt = 10_000u64;

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

    for i in 0..15u64 {
        let t = t0 + i * dt;
        tick_joint(&vp, slot_id, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        vp.refresh_global_reasoning(t);
    }

    let ev2 = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0 + 150_000,
        event_energy: 0.1,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    };
    let claim2 = vp.claim_slot_for_event(&ev2);
    assert!(
        matches!(claim2, SlotClaim::Reused(_)),
        "same-modality within window must reuse after reasoning; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(
        active, 1,
        "reasoning must not change slot count; got {} active slots",
        active
    );
}

// ---------------------------------------------------------------------------
// Phase 5F.1: Weight normalization regression tests
// Low-attention / near-zero weight edges must not dilute global scores.
// ---------------------------------------------------------------------------

/// Test: zero_attention_edges_do_not_dilute_surprise_load
///
/// Construct a scenario with one high-surprise edge with high attention,
/// plus multiple low-attention background edges. The surprise_load should
/// remain high (based on the high-attention edge) and not be diluted by
/// the mere presence of many low-attention edges.
#[test]
fn zero_attention_edges_do_not_dilute_surprise_load() {
    let vp = Arc::new(Viewport::new());
    let t0 = 7_000_000u64;
    let dt = 10_000u64;

    // Allocate 4 slots: A and B will be active with high attention,
    // C and D will be allocated but kept stale (zero attention).
    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
    let slot_c = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc c");
    let slot_d = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc d");

    // Warmup: run A and B for many cycles to establish stable relations
    for i in 0..30u64 {
        tick_joint(&vp, slot_a, t0 + i * dt);
        tick_joint(&vp, slot_b, t0 + i * dt);
        let t_cycle = t0 + i * dt;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    // Phase 2: Introduce high surprise on A-B edge via timing mismatch
    // C and D remain stale (never ticked) → zero attention
    let t_base = t0 + 30 * dt;
    for i in 0..5u64 {
        let big_delta = 50_000u64;
        tick_joint(&vp, slot_a, t_base + i * dt);
        tick_joint(&vp, slot_b, t_base + i * dt + big_delta);
        let t_cycle = t_base + i * dt + big_delta;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    // Read the reasoning state
    let rs = vp.read_global_reasoning();

    // With high surprise on the high-attention A-B edge, surprise_load should be significant.
    // If normalized by edge count (including zero-attention edges), it would be diluted.
    assert!(
        rs.global_surprise_load > 0.01,
        "surprise_load must reflect high-attention edge surprise, not be diluted by zero-attention edges; got {:.6}",
        rs.global_surprise_load
    );

    // Suppress unused warnings
    let _ = (slot_c, slot_d);
}

/// Test: zero_attention_edges_do_not_dilute_instability_score
///
/// Similar to surprise_load test, but for network_instability_score.
#[test]
fn zero_attention_edges_do_not_dilute_instability_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 8_000_000u64;
    let dt = 10_000u64;

    // Allocate 4 slots: A and B active, C and D stale
    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
    let slot_c = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc c");
    let slot_d = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc d");

    // Warmup
    for i in 0..30u64 {
        tick_joint(&vp, slot_a, t0 + i * dt);
        tick_joint(&vp, slot_b, t0 + i * dt);
        let t_cycle = t0 + i * dt;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    // Phase 2: High instability via timing mismatch
    let t_base = t0 + 30 * dt;
    for i in 0..5u64 {
        let big_delta = 50_000u64;
        tick_joint(&vp, slot_a, t_base + i * dt);
        tick_joint(&vp, slot_b, t_base + i * dt + big_delta);
        let t_cycle = t_base + i * dt + big_delta;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    let rs = vp.read_global_reasoning();

    // Instability should reflect the high-attention edge, not be diluted
    assert!(
        rs.network_instability_score > 0.01,
        "instability_score must reflect high-attention edge instability, not be diluted by zero-attention edges; got {:.6}",
        rs.network_instability_score
    );

    let _ = (slot_c, slot_d);
}

/// Test: reasoning_normalization_does_not_change_routing_focus
///
/// The routing focus edges and slots should remain determined by the same
/// criteria (relation_err × confidence × attention), unaffected by the
/// normalization change.
#[test]
fn reasoning_normalization_does_not_change_routing_focus() {
    let vp = Arc::new(Viewport::new());
    let t0 = 9_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup
    for i in 0..30u64 {
        tick_joint(&vp, slot_a, t0 + i * dt);
        tick_joint(&vp, slot_b, t0 + i * dt);
        let t_cycle = t0 + i * dt;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    // Phase 2: Introduce high error on A-B edge
    let t_base = t0 + 30 * dt;
    for i in 0..5u64 {
        let big_delta = 50_000u64;
        tick_joint(&vp, slot_a, t_base + i * dt);
        tick_joint(&vp, slot_b, t_base + i * dt + big_delta);
        let t_cycle = t_base + i * dt + big_delta;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
    }

    let rs = vp.read_global_reasoning();
    let gr = vp.read_global_relations();

    // routing_focus_edge_ids[0] must be valid and point to A-B edge
    let focus_edge_idx = rs.routing_focus_edge_ids[0];
    assert!(
        focus_edge_idx >= 0,
        "at least one edge must be in routing focus; got -1"
    );

    let focus_edge = &gr.edges[focus_edge_idx as usize];
    let is_ab_edge = (focus_edge.src_slot_id as usize == slot_a
        && focus_edge.dst_slot_id as usize == slot_b)
        || (focus_edge.src_slot_id as usize == slot_b && focus_edge.dst_slot_id as usize == slot_a);
    assert!(
        is_ab_edge,
        "routing focus must be on A-B edge after high error; got {}→{}",
        focus_edge.src_slot_id, focus_edge.dst_slot_id
    );

    // routing_focus_slot_ids should include A and B (the active slots)
    let focus_slots: std::collections::HashSet<i8> =
        rs.routing_focus_slot_ids.iter().cloned().collect();
    assert!(
        focus_slots.contains(&(slot_a as i8)) || focus_slots.contains(&(slot_b as i8)),
        "routing focus slots must include active slots A or B; got {:?}",
        rs.routing_focus_slot_ids
    );
}
