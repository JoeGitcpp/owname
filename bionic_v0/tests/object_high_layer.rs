//! Phase 5B: Object-level high-layer predictor substrate tests.
//!
//! These tests verify the single-object high-layer prediction closed loop:
//!   fused state → high-layer prediction → object_err → attention_bias
//!
//! Architecture invariants checked here:
//!   * High-layer state lives in the slot (not adapter, not global).
//!   * Input comes exclusively from the fusion subtree.
//!   * claim/scoring is never touched.
//!   * No semantic labels anywhere.

use std::sync::Arc;

use bionic_v0::viewport::{EventEvidence, SlotClaim, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, PHOT_BANDS, SLOT_IDLE_US, VIB_BANDS};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn allocate_joint_slot(vp: &Arc<Viewport>, t_us: u64) -> usize {
    vp.allocate_slot(MOD_VIB, t_us, 0)
        .expect("must allocate slot")
}

/// Write both VIB and PHOT at the same timestamp, refresh fusion, refresh
/// high-layer. Models a perfectly synchronised joint event.
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

/// Write only VIB, refresh fusion, refresh high-layer. Models stale PHOT.
fn tick_vib_only(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
    vp.refresh_slot_high_layer(slot_id, t_us);
}

// ---------------------------------------------------------------------------
// Test 1: joint slot creates high-layer state
// ---------------------------------------------------------------------------

/// A slot that has had both VIB and PHOT fusion-refreshed, plus a high-layer
/// refresh, must carry a Some(ObjectHighLayerState).
#[test]
fn joint_slot_creates_high_layer_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let slot_id = allocate_joint_slot(&vp, t0);

    // Before any refresh: high_layer must be None.
    {
        let snap = vp.read_slot(slot_id).unwrap();
        assert!(
            snap.high_layer.is_none(),
            "high_layer must be None before any refresh"
        );
    }

    tick_joint(&vp, slot_id, t0);

    // After one joint tick: high_layer must exist.
    {
        let snap = vp.read_slot(slot_id).unwrap();
        assert!(
            snap.high_layer.is_some(),
            "high_layer must be Some after joint tick"
        );
        let hl = snap.high_layer.unwrap();
        assert_eq!(hl.last_update_t_us, t0);
    }
}

// ---------------------------------------------------------------------------
// Test 2: single-modality slot does not fake high-layer confidence
// ---------------------------------------------------------------------------

/// A VIB-only slot must never produce predicted_confidence > 0.5, because the
/// fusion subtree hard-caps single-modality confidence at 0.5.
#[test]
fn single_modality_slot_does_not_fake_high_layer_confidence() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;
    let slot_id = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc");

    // Run 25 VIB-only ticks (well past warmup, predictor seeded on fusion confidence ≤ 0.5).
    for i in 0..25u64 {
        tick_vib_only(&vp, slot_id, t0 + i * 1_000);
    }

    let snap = vp.read_slot(slot_id).unwrap();
    let hl = snap
        .high_layer
        .expect("high_layer must exist after vib-only ticks");
    assert!(
        hl.predicted_confidence <= 0.5 + 1e-9,
        "VIB-only: predicted_confidence must be ≤ 0.5, got {}",
        hl.predicted_confidence
    );
    // attention_bias should also be moderate, not faking full stability.
    // (It rises toward 1.0 on stable single-modal input, which is correct —
    // the object IS stable, just single-modal. The key guard is confidence.)
    assert!(
        snap.fusion.map(|f| !f.is_cross_modal).unwrap_or(true),
        "single-modality fusion must not be cross-modal"
    );
}

// ---------------------------------------------------------------------------
// Test 3: object_err converges on stable fused state
// ---------------------------------------------------------------------------

/// After warm-up, a slot receiving identical fused input on every tick must
/// converge to near-zero object_err.
#[test]
fn high_layer_err_drops_on_stable_fused_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;
    let slot_id = allocate_joint_slot(&vp, t0);

    // Warm-up: two ticks that seed the smoother with the stable fusion values.
    tick_joint(&vp, slot_id, t0);
    tick_joint(&vp, slot_id, t0 + 1_000);

    // Stable phase: 25 more ticks with identical fusion inputs.
    // After warmup, predicted ≈ observed → err → 0.
    for i in 0..25u64 {
        tick_joint(&vp, slot_id, t0 + 2_000 + i * 1_000);
    }

    let snap = vp.read_slot(slot_id).unwrap();
    let hl = snap.high_layer.expect("high_layer must exist");
    assert!(
        hl.object_err < 1e-9,
        "stable fused state must yield near-zero object_err; got {}",
        hl.object_err
    );
    assert_eq!(hl.last_update_t_us, t0 + 2_000 + 24 * 1_000);
}

// ---------------------------------------------------------------------------
// Test 4: object_err spikes when fused state changes abruptly
// ---------------------------------------------------------------------------

/// After stabilising on a joint fusion state, abruptly removing PHOT (so the
/// fusion flips to single-modality with low confidence and zero alignment) must
/// cause an immediate spike in object_err.
#[test]
fn high_layer_err_spikes_on_fused_state_change() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;
    let slot_id = allocate_joint_slot(&vp, t0);

    // Stabilise: 25 joint ticks → object_err converges near 0.
    for i in 0..25u64 {
        tick_joint(&vp, slot_id, t0 + i * 1_000);
    }
    let stable_snap = vp.read_slot(slot_id).unwrap();
    let stable_err = stable_snap.high_layer.unwrap().object_err;
    assert!(
        stable_err < 1e-9,
        "must be stable before spike test; stable_err={}",
        stable_err
    );

    // Abrupt state change: VIB only at t0 + SLOT_IDLE_US so PHOT weight → 0.
    // This flips cross_modal_confidence from ~1.0 to 0.5, temporal_alignment
    // from 1.0 to 0.0 — a large jump the predictor did not expect.
    let t_spike = t0 + SLOT_IDLE_US;
    tick_vib_only(&vp, slot_id, t_spike);

    let spike_snap = vp.read_slot(slot_id).unwrap();
    let spike_err = spike_snap.high_layer.unwrap().object_err;
    assert!(
        spike_err > 0.1,
        "object_err must spike on abrupt fusion change; got {}",
        spike_err
    );
    assert!(
        spike_err > stable_err * 100.0,
        "spike_err ({}) must greatly exceed stable_err ({})",
        spike_err,
        stable_err
    );
}

// ---------------------------------------------------------------------------
// Test 5: attention_bias tracks object stability
// ---------------------------------------------------------------------------

/// On a stable joint object attention_bias must rise toward 1.0.
/// After a sudden state change it must drop noticeably.
#[test]
fn high_layer_attention_bias_tracks_object_stability() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;
    let slot_id = allocate_joint_slot(&vp, t0);

    // Stable phase: 35 ticks → err_sq_ema ≈ 0 → attention_bias → 1.0.
    for i in 0..35u64 {
        tick_joint(&vp, slot_id, t0 + i * 1_000);
    }
    let stable_snap = vp.read_slot(slot_id).unwrap();
    let stable_bias = stable_snap.high_layer.unwrap().attention_bias;
    assert!(
        stable_bias > 0.95,
        "stable joint object must have attention_bias > 0.95; got {}",
        stable_bias
    );

    // Spike: PHOT goes stale → fusion state jumps.
    let t_spike = t0 + SLOT_IDLE_US;
    tick_vib_only(&vp, slot_id, t_spike);

    let spike_snap = vp.read_slot(slot_id).unwrap();
    let spike_bias = spike_snap.high_layer.unwrap().attention_bias;
    assert!(
        spike_bias < stable_bias,
        "attention_bias must drop after fusion change; stable={} spike={}",
        stable_bias,
        spike_bias
    );
    assert!(
        spike_bias < 0.95,
        "attention_bias must drop below stable threshold; got {}",
        spike_bias
    );
}

// ---------------------------------------------------------------------------
// Test 6: snapshot roundtrip preserves high-layer state
// ---------------------------------------------------------------------------

/// Snapshot write → read must preserve all ObjectHighLayerState fields.
#[test]
fn snapshot_roundtrip_preserves_high_layer_state() {
    use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};

    let vp = Arc::new(Viewport::new());
    let t0 = 6_000_000u64;
    let slot_id = allocate_joint_slot(&vp, t0);

    // Build stable high-layer state: run enough ticks to be past warmup.
    for i in 0..5u64 {
        tick_joint(&vp, slot_id, t0 + i * 1_000);
    }

    let snap_before = vp.read_slot(slot_id).unwrap();
    let hl_before = snap_before
        .high_layer
        .expect("high_layer must exist before snapshot");

    let dir = std::env::temp_dir().join("bionic_v0_phase5b_hl_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let slot_snap = &data.slots[slot_id];
    let hl = slot_snap
        .high_layer
        .expect("high_layer must survive roundtrip");

    assert!(
        (hl.predicted_confidence - hl_before.predicted_confidence).abs() < 1e-12,
        "predicted_confidence roundtrip failed"
    );
    assert!(
        (hl.predicted_alignment - hl_before.predicted_alignment).abs() < 1e-12,
        "predicted_alignment roundtrip failed"
    );
    assert!(
        (hl.predicted_surprise - hl_before.predicted_surprise).abs() < 1e-12,
        "predicted_surprise roundtrip failed"
    );
    assert!(
        (hl.object_err - hl_before.object_err).abs() < 1e-12,
        "object_err roundtrip failed"
    );
    assert!(
        (hl.attention_bias - hl_before.attention_bias).abs() < 1e-12,
        "attention_bias roundtrip failed"
    );
    assert_eq!(
        hl.last_update_t_us, hl_before.last_update_t_us,
        "last_update_t_us roundtrip failed"
    );
}

// ---------------------------------------------------------------------------
// Test 7: claim_slot_for_event architecture remains unchanged (regression)
// ---------------------------------------------------------------------------

/// Adding Phase 5B must not alter the claim/scoring contract in any way.
/// Same-modality reuse, allocation, and slot-count invariants must hold.
#[test]
fn claim_architecture_remains_unchanged_after_high_layer() {
    let vp = Arc::new(Viewport::new());
    let t0 = 7_000_000u64;

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

    // Run some high-layer refreshes on this slot.
    tick_joint(&vp, slot_id, t0);
    tick_joint(&vp, slot_id, t0 + 1_000);

    // Same-modality reuse within SLOT_IDLE_US must still reuse.
    let ev2 = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0 + 2_000,
        event_energy: 0.1,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    };
    let claim2 = vp.claim_slot_for_event(&ev2);
    assert!(
        matches!(claim2, SlotClaim::Reused(_)),
        "same-modality within window must reuse; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    // Total active slots: still just 1 (the high-layer did not create extras).
    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(active, 1, "high-layer refresh must not change slot count");
}
