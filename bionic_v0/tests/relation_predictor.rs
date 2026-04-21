//! Phase 5E: Relation-level predictor tests.
//!
//! Contracts verified:
//!   1. Stable relation drives relation_err down as the AR-1 EMA converges.
//!   2. Abrupt lead_lag change spikes relation_err from its near-zero baseline.
//!   3. Warmup prevents fake non-zero relation_err / relation_precision.
//!   4. Snapshot roundtrip preserves all RelationEdgePredictionState fields.
//!   5. Ordered refresh contract holds: predictor runs after relations, same t.
//!   6. Claim/scoring architecture is completely unaffected.

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

/// Run one full ordered refresh cycle.
fn run_cycle(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_a: u64, t_b: u64) {
    tick_joint(vp, slot_a, t_a);
    tick_joint(vp, slot_b, t_b);
    // Use the later of the two as the cycle time so both slots appear fresh.
    let t_cycle = t_a.max(t_b);
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
}

/// Find the first active directed edge (src → dst) in an edge array.
fn find_active_edge_idx(
    edges: &[bionic_v0::viewport::RelationEdgeState; N_REL_EDGES],
    src: usize,
    dst: usize,
) -> Option<&bionic_v0::viewport::RelationEdgeState> {
    edges
        .iter()
        .find(|e| e.src_slot_id as usize == src && e.dst_slot_id as usize == dst)
}

// ---------------------------------------------------------------------------
// Test 1: stable_relation_drives_err_down
//
// When both slots receive identical synchronized data for many cycles, the
// AR-1 EMA converges to the stable signal values. The prediction error
// should be near-zero at cycle 60 and strictly below its value at cycle 5.
// ---------------------------------------------------------------------------

#[test]
fn stable_relation_drives_err_down() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Early cycles — EMA still settling.
    for i in 0..5u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }
    let err_early: f64 = {
        let gr = vp.read_global_relations();
        gr.edges.iter()
            .filter(|e| e.active)
            .map(|e| e.pred.relation_err)
            .fold(f64::MAX, f64::min)
    };

    // Many more cycles — EMA fully converged.
    for i in 5..60u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }
    let err_late: f64 = {
        let gr = vp.read_global_relations();
        gr.edges.iter()
            .filter(|e| e.active)
            .map(|e| e.pred.relation_err)
            .fold(f64::MAX, f64::min)
    };

    assert!(
        err_late < err_early,
        "relation_err must decrease with stable input; early={:.6}, late={:.6}",
        err_early, err_late
    );
    assert!(
        err_late < 0.01,
        "after 60 stable cycles, relation_err must be near-zero; got {:.6}",
        err_late
    );
}

// ---------------------------------------------------------------------------
// Test 2: relation_change_spikes_err
//
// After the EMA has converged on synchronized slots (small err), introduce a
// large timing offset between the two slots. The lead_lag_score shifts
// abruptly while the predictor EMA lags, causing relation_err to spike.
// ---------------------------------------------------------------------------

#[test]
fn relation_change_spikes_err() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Phase 1: 40 synchronized cycles → err converges near 0.
    for i in 0..40u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }
    let err_stable: f64 = {
        let gr = vp.read_global_relations();
        gr.edges.iter()
            .filter(|e| e.active)
            .map(|e| e.pred.relation_err)
            .fold(f64::MAX, f64::min)
    };

    // Phase 2: 10 cycles with slot_b significantly delayed (55ms offset).
    // This introduces a large lead_lag change that the predictor's converged
    // EMA cannot immediately track, spiking relation_err.
    let big_delta = 55_000u64;
    let t_base = t0 + 40 * dt;
    for i in 0..10u64 {
        let t_a = t_base + i * dt;
        let t_b = t_a + big_delta;
        run_cycle(&vp, slot_a, slot_b, t_a, t_b);
    }
    let err_spiked: f64 = {
        let gr = vp.read_global_relations();
        // Use the A→B edge which will show the lead_lag change most clearly.
        find_active_edge_idx(&gr.edges, slot_a, slot_b)
            .map(|e| e.pred.relation_err)
            .unwrap_or(0.0)
    };

    assert!(
        err_spiked > err_stable + 0.03,
        "relation_err must spike after abrupt lead_lag change; stable={:.6}, spiked={:.6}",
        err_stable, err_spiked
    );
}

// ---------------------------------------------------------------------------
// Test 3: warmup_does_not_fake_relation_confidence
//
// After exactly one full refresh cycle, active edges must report
// relation_err = 0.0 and relation_precision = 0.0 (warmup gate).
// ---------------------------------------------------------------------------

#[test]
fn warmup_does_not_fake_relation_confidence() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Single cycle.
    tick_joint(&vp, slot_a, t0);
    tick_joint(&vp, slot_b, t0);
    vp.refresh_global_attention(t0);
    vp.refresh_global_relations(t0);
    vp.refresh_global_relation_predictor(t0);

    let gr = vp.read_global_relations();
    let active_edges: Vec<_> = gr.edges.iter().filter(|e| e.active).collect();
    assert!(!active_edges.is_empty(), "at least one edge must be active after first cycle");

    for edge in &active_edges {
        assert_eq!(
            edge.pred.relation_err, 0.0,
            "warmup edge must report relation_err=0; got {} for edge {}→{}",
            edge.pred.relation_err, edge.src_slot_id, edge.dst_slot_id
        );
        assert_eq!(
            edge.pred.relation_precision, 0.0,
            "warmup edge must report relation_precision=0 (no fake confidence); got {} for edge {}→{}",
            edge.pred.relation_precision, edge.src_slot_id, edge.dst_slot_id
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: snapshot_roundtrip_preserves_relation_prediction_state
//
// All RelationEdgePredictionState fields must survive write_snapshot →
// read_snapshot unchanged (to floating-point round-trip precision).
// ---------------------------------------------------------------------------

#[test]
fn snapshot_roundtrip_preserves_relation_prediction_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Run enough cycles for the predictor to produce non-trivial values.
    for i in 0..20u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let gr_before = vp.read_global_relations();

    let dir = std::env::temp_dir().join("bionic_v0_5e_pred_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let gr = &data.global_relations;
    assert_eq!(gr.n_active_edges, gr_before.n_active_edges, "n_active_edges");

    for i in 0..N_REL_EDGES {
        let a = &gr_before.edges[i];
        let b = &gr.edges[i];
        assert_eq!(b.active, a.active, "edge[{}].active", i);
        assert!(
            (b.pred.predicted_co_change - a.pred.predicted_co_change).abs() < 1e-12,
            "edge[{}].pred.predicted_co_change roundtrip",
            i
        );
        assert!(
            (b.pred.predicted_lead_lag - a.pred.predicted_lead_lag).abs() < 1e-12,
            "edge[{}].pred.predicted_lead_lag roundtrip",
            i
        );
        assert!(
            (b.pred.predicted_predictive - a.pred.predicted_predictive).abs() < 1e-12,
            "edge[{}].pred.predicted_predictive roundtrip",
            i
        );
        assert!(
            (b.pred.predicted_confidence - a.pred.predicted_confidence).abs() < 1e-12,
            "edge[{}].pred.predicted_confidence roundtrip",
            i
        );
        assert!(
            (b.pred.relation_err - a.pred.relation_err).abs() < 1e-12,
            "edge[{}].pred.relation_err roundtrip",
            i
        );
        assert!(
            (b.pred.relation_precision - a.pred.relation_precision).abs() < 1e-12,
            "edge[{}].pred.relation_precision roundtrip",
            i
        );
        assert_eq!(
            b.pred.last_predict_t_us, a.pred.last_predict_t_us,
            "edge[{}].pred.last_predict_t_us roundtrip",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5: ordered_refresh_contract_remains_intact
//
// refresh_global_relation_predictor must run after refresh_global_relations
// in the same cycle (same timestamp). last_predict_t_us == t_cycle proves
// the predictor was called with the same t as the relation refresh.
// ---------------------------------------------------------------------------

#[test]
fn ordered_refresh_contract_remains_intact() {
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

    let gr = vp.read_global_relations();
    // Relations substrate was stamped with t_cycle.
    assert_eq!(gr.last_update_t_us, t_cycle, "relations last_update must equal cycle t");

    // Every active edge's predictor output was also stamped with t_cycle.
    for edge in gr.edges.iter().filter(|e| e.active) {
        assert_eq!(
            edge.pred.last_predict_t_us, t_cycle,
            "active edge last_predict_t_us must equal cycle t (proves ordering)"
        );
    }

    // At least one edge must have been active.
    assert!(
        gr.edges.iter().any(|e| e.active),
        "at least one edge must be active after 5 cycles"
    );
}

// ---------------------------------------------------------------------------
// Test 6: claim_architecture_remains_unchanged_after_relation_prediction
//
// Running relation predictor refreshes must leave the claim/reuse/slot-count
// invariants completely unchanged.
// ---------------------------------------------------------------------------

#[test]
fn claim_architecture_remains_unchanged_after_relation_prediction() {
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
    assert!(matches!(claim1, SlotClaim::Allocated(_)), "first VIB must allocate");
    let slot_id = claim1.slot_id().unwrap();

    // Run several ordered refresh cycles (including relation predictor).
    for i in 0..15u64 {
        let t = t0 + i * dt;
        tick_joint(&vp, slot_id, t);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
    }

    // Same-modality reuse must still work.
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
        "same-modality within window must reuse after relation predictor; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    // Slot count must be exactly 1.
    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(
        active, 1,
        "relation predictor must not change slot count; got {} active slots",
        active
    );
}
