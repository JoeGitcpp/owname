//! Phase 5D.1: Ordered Global Refresh tests.
//!
//! Contracts verified:
//!   1. `refresh_global_relations` reads same-round attention, not stale zeros.
//!   2. Snapshot captures post-refresh state — attention and relations consistent.
//!   3. Serial refresh cycle does not alter claim/scoring architecture.
//!   4. Observer renders snapshot truth only — values match live viewport exactly.

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

    vp.refresh_slot_high_layer(slot_id, t_us);
}

// ---------------------------------------------------------------------------
// Test 1: ordered_refresh_updates_attention_before_relations
//
// Without a prior attention refresh all attention_shares are 0.0 (default).
// refresh_global_relations requires attention_share > 1e-6 to activate any
// edge, so calling it alone produces no active edges.  After the ordered
// attention → relations sequence the edges become active, proving relations
// consumed the same-round attention output.
// ---------------------------------------------------------------------------

#[test]
fn ordered_refresh_updates_attention_before_relations() {
    let vp = Arc::new(Viewport::new());
    let t0 = 10_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..25u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        tick_joint(&vp, slot_b, t);
    }

    // Relations-only call with no prior attention refresh.
    // attention_share defaults are 0.0 → every edge must be inactive.
    let t_check = t0 + 250_000;
    vp.refresh_global_relations(t_check);
    let gr_before = vp.read_global_relations();
    assert!(
        !gr_before.edges.iter().any(|e| e.active),
        "without prior attention refresh no edges should be active (attention_share=0)"
    );

    // Ordered cycle: attention first, then relations.
    vp.refresh_global_attention(t_check);
    vp.refresh_global_relations(t_check);
    let gr_after = vp.read_global_relations();
    assert!(
        gr_after.edges.iter().any(|e| e.active),
        "after ordered attention→relations cycle at least one edge must be active"
    );
}

// ---------------------------------------------------------------------------
// Test 2: snapshot_captures_post_refresh_state
//
// The snapshot written immediately after an ordered cycle must contain
// attention and relation data from the same round (same t), proving the
// snapshot never captures an intermediate state.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_captures_post_refresh_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 20_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..30u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_a, t);
        tick_joint(&vp, slot_b, t);
    }

    let t_cycle = t0 + 300_000;
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);

    let dir = std::env::temp_dir().join("bionic_v0_5d1_post_refresh_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    // Snapshot must reflect post-refresh attention.
    assert!(
        data.global_attention.n_active > 0,
        "snapshot must show active slots after refresh; got n_active={}",
        data.global_attention.n_active
    );

    // Snapshot must reflect post-refresh relations.
    assert!(
        data.global_relations.n_active_edges > 0,
        "snapshot must show active relation edges after refresh; got n_active_edges={}",
        data.global_relations.n_active_edges
    );

    // Relation state was stamped with t_cycle — proves same-round provenance.
    assert_eq!(
        data.global_relations.last_update_t_us, t_cycle,
        "snapshot relation timestamp must match the cycle now_us"
    );

    // Live viewport state must match the snapshot (no divergence).
    let ga_live = vp.read_global_attention();
    assert_eq!(
        data.global_attention.n_active, ga_live.n_active,
        "snapshot attention n_active must match live viewport"
    );
    let gr_live = vp.read_global_relations();
    assert_eq!(
        data.global_relations.n_active_edges, gr_live.n_active_edges,
        "snapshot relation n_active_edges must match live viewport"
    );
}

// ---------------------------------------------------------------------------
// Test 3: global_orchestrator_does_not_change_claim_architecture
//
// Running a full ordered refresh cycle must leave the claim/reuse/slot-count
// invariants unchanged — this is the regression gate.
// ---------------------------------------------------------------------------

#[test]
fn global_orchestrator_does_not_change_claim_architecture() {
    let vp = Arc::new(Viewport::new());
    let t0 = 30_000_000u64;

    let ev1 = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0,
        event_energy: 0.1,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    };
    let claim1 = vp.claim_slot_for_event(&ev1);
    assert!(matches!(claim1, SlotClaim::Allocated(_)), "first VIB must allocate");
    let slot_id = claim1.slot_id().unwrap();

    for i in 0..15u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_id, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
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
        "same-modality within window must reuse after ordered refresh; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(
        active, 1,
        "ordered refresh must not change slot count; got {} active slots",
        active
    );
}

// ---------------------------------------------------------------------------
// Test 3b: ordered_refresh_cycle_stops_before_semantic_layers
//
// Cleanup spec 2026-04-20: the formal body cycle terminates at
// reasoning history + snapshot. semantic-readiness and symbolizer must NOT
// be reachable as part of the ordered refresh contract.
// ---------------------------------------------------------------------------

#[test]
fn ordered_refresh_cycle_stops_before_semantic_layers() {
    let body_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/body.rs"
    ))
    .expect("read body.rs");
    assert!(
        !body_src.contains("refresh_semantic_readiness"),
        "body.rs must not call refresh_semantic_readiness in main cycle"
    );
    assert!(
        !body_src.contains("refresh_symbolizer"),
        "body.rs must not call refresh_symbolizer in main cycle"
    );
}

// ---------------------------------------------------------------------------
// Test 4: observer_still_renders_snapshot_truth_only
//
// After a full refresh cycle, the snapshot must exactly mirror the live
// viewport state. Observers reading the snapshot see correct values without
// performing any additional computation.
// ---------------------------------------------------------------------------

#[test]
fn observer_still_renders_snapshot_truth_only() {
    let vp = Arc::new(Viewport::new());
    let t0 = 40_000_000u64;
    let slot_id = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc");

    for i in 0..20u64 {
        let t = t0 + i * 10_000;
        tick_joint(&vp, slot_id, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
    }

    let dir = std::env::temp_dir().join("bionic_v0_5d1_observer_truth_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let ga_live = vp.read_global_attention();
    assert_eq!(
        data.global_attention.focused_slot_id, ga_live.focused_slot_id,
        "snapshot focused_slot_id must match live viewport"
    );
    assert_eq!(
        data.global_attention.n_active, ga_live.n_active,
        "snapshot n_active must match live viewport"
    );
    for i in 0..N_OBJECT_SLOTS {
        assert!(
            (data.global_attention.attention_shares[i] - ga_live.attention_shares[i]).abs()
                < 1e-12,
            "snapshot attention_shares[{}] must match live viewport",
            i
        );
    }

    let slot_live = vp.read_slot(slot_id).unwrap();
    let slot_file = &data.slots[slot_id];
    assert!(
        (slot_file.attention_share - slot_live.attention_share).abs() < 1e-12,
        "snapshot per-slot attention_share must match live viewport"
    );
    assert!(
        (slot_file.precision_bias - slot_live.precision_bias).abs() < 1e-12,
        "snapshot per-slot precision_bias must match live viewport"
    );
}
