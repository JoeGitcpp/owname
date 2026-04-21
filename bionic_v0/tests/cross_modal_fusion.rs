//! Phase 5A: same-slot cross-modal fusion substrate tests.
//!
//! These tests verify that CrossModalFusionState is populated correctly inside
//! an ObjectSlot without touching claim/scoring or introducing semantic labels.

use std::sync::Arc;

use bionic_v0::viewport::{
    EventEvidence, SlotClaim, SpatialEvidence, Viewport,
};
use bionic_v0::{
    CROSS_MODAL_WINDOW_US, MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, SLOT_IDLE_US,
};

// ---- helpers ---------------------------------------------------------------

fn make_vib_evidence(t_us: u64, energy: f64) -> EventEvidence {
    EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t_us,
        event_energy: energy,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 1,
    }
}

fn make_phot_evidence(t_us: u64, energy: f64) -> EventEvidence {
    EventEvidence {
        modality_bit: MOD_PHOT,
        event_t_us: t_us,
        event_energy: energy,
        surprise: 0.1,
        spatial: Some(SpatialEvidence {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: 0.1,
            span_y: 0.1,
            precision: 1.0,
        }),
        nomination_seq_hint: 1,
    }
}

/// Write a minimal VIB band family into a slot and call refresh_slot_fusion.
fn write_vib_and_fuse(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let bands = [0.1f64; bionic_v0::VIB_BANDS];
    let pred = [0.09f64; bionic_v0::VIB_BANDS];
    let err = [0.01f64; bionic_v0::VIB_BANDS];
    let prec = [0.5f64; bionic_v0::VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &bands, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
}

/// Write a minimal PHOT band family into a slot and call refresh_slot_fusion.
fn write_phot_and_fuse(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let obs = vec![0.2f64; bionic_v0::PHOT_BANDS];
    let pred = vec![0.18f64; bionic_v0::PHOT_BANDS];
    let err = vec![0.02f64; bionic_v0::PHOT_BANDS];
    let prec = vec![0.4f64; bionic_v0::PHOT_BANDS];
    vp.write_slot_phot(slot_id, t_us, &obs, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
}

// ---- tests -----------------------------------------------------------------

/// A slot that has both VIB and PHOT must have a fusion subtree with the
/// correct modality_mask.
#[test]
fn joint_slot_produces_fused_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;

    // Allocate a slot via VIB.
    let ev_vib = make_vib_evidence(t0, 0.1);
    let slot_id = match vp.claim_slot_for_event(&ev_vib) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("slot should allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);

    // VIB only so far: fusion present but not cross-modal.
    {
        let snap = vp.read_slot(slot_id).unwrap();
        let f = snap.fusion.expect("fusion must exist after VIB refresh");
        assert!(!f.is_cross_modal, "VIB-only must not be cross-modal");
        assert_eq!(f.modality_mask & MOD_VIB, MOD_VIB);
    }

    // Add PHOT to the same slot (within CROSS_MODAL_WINDOW_US).
    let t1 = t0 + 10_000; // 10 ms after VIB
    let ev_phot = make_phot_evidence(t1, 0.1);
    let phot_slot = match vp.claim_slot_for_event(&ev_phot) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("phot slot should allocate or reuse"),
    };
    write_phot_and_fuse(&vp, phot_slot, t1);

    // If both landed on the same slot, verify cross-modal fusion.
    if phot_slot == slot_id {
        let snap = vp.read_slot(slot_id).unwrap();
        let f = snap.fusion.expect("fusion must exist after PHOT refresh");
        assert_eq!(f.modality_mask & (MOD_VIB | MOD_PHOT), MOD_VIB | MOD_PHOT,
            "modality_mask must carry both bits");
        assert!(f.is_cross_modal, "joint slot must be cross-modal");
    }
}

/// A slot that only ever got VIB must not present as fully fused.
/// cross_modal_confidence must be ≤ 0.5 and is_cross_modal = false.
#[test]
fn single_modality_slot_does_not_fake_cross_modal_confidence() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;

    let ev = make_vib_evidence(t0, 0.2);
    let slot_id = match vp.claim_slot_for_event(&ev) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("slot must allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);

    let snap = vp.read_slot(slot_id).unwrap();
    let f = snap.fusion.expect("fusion must exist");
    assert!(!f.is_cross_modal, "VIB-only must not be cross-modal");
    assert!(
        f.cross_modal_confidence <= 0.5 + 1e-9,
        "single-modality confidence must be ≤ 0.5, got {}",
        f.cross_modal_confidence
    );
    assert_eq!(f.phot_weight, 0.0, "phot_weight must be 0 for VIB-only slot");
}

/// When VIB and PHOT write at the same timestamp, temporal_alignment_score
/// should be near 1.0.
#[test]
fn temporal_alignment_improves_for_synchronous_updates() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;

    // Allocate via VIB.
    let ev_vib = make_vib_evidence(t0, 0.15);
    let slot_id = match vp.claim_slot_for_event(&ev_vib) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("must allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);

    // PHOT writes at the exact same time.
    write_phot_and_fuse(&vp, slot_id, t0);

    let snap = vp.read_slot(slot_id).unwrap();
    let f = snap.fusion.expect("fusion must exist");
    assert!(
        f.temporal_alignment_score > 0.99,
        "synchronous writes must have temporal_alignment near 1.0, got {}",
        f.temporal_alignment_score
    );
    assert!(f.is_cross_modal, "synchronous joint slot must be cross-modal");
    assert!(
        f.cross_modal_confidence > 0.5,
        "synchronous joint confidence must exceed 0.5, got {}",
        f.cross_modal_confidence
    );
}

/// When VIB and PHOT write at CROSS_MODAL_WINDOW_US apart, temporal alignment
/// should be near 0.0.
#[test]
fn temporal_alignment_decays_when_modalities_drift_apart() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;

    // Allocate via VIB.
    let ev_vib = make_vib_evidence(t0, 0.15);
    let slot_id = match vp.claim_slot_for_event(&ev_vib) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("must allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);

    // PHOT writes exactly CROSS_MODAL_WINDOW_US later.
    let t_phot = t0 + CROSS_MODAL_WINDOW_US;
    write_phot_and_fuse(&vp, slot_id, t_phot);

    let snap = vp.read_slot(slot_id).unwrap();
    let f = snap.fusion.expect("fusion must exist");
    assert!(
        f.temporal_alignment_score < 0.05,
        "max-drift writes must have temporal_alignment near 0.0, got {}",
        f.temporal_alignment_score
    );
}

/// After VIB and PHOT both write, if only VIB writes again later, the PHOT
/// weight should be lower than the VIB weight.
#[test]
fn stale_modality_reduces_fusion_weight() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;

    // Allocate via VIB.
    let ev_vib = make_vib_evidence(t0, 0.1);
    let slot_id = match vp.claim_slot_for_event(&ev_vib) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("must allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);

    // PHOT also writes at t0.
    write_phot_and_fuse(&vp, slot_id, t0);

    // Now VIB writes again at t0 + half of SLOT_IDLE_US; PHOT stays at t0.
    let t1 = t0 + SLOT_IDLE_US / 2;
    write_vib_and_fuse(&vp, slot_id, t1);

    let snap = vp.read_slot(slot_id).unwrap();
    let f = snap.fusion.expect("fusion must exist");
    assert!(
        f.vib_weight > f.phot_weight,
        "fresh VIB weight ({}) must exceed stale PHOT weight ({})",
        f.vib_weight, f.phot_weight
    );
}

/// Snapshot write/read roundtrip must preserve all fusion fields.
#[test]
fn snapshot_roundtrip_preserves_fused_slot_state() {
    use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};

    let vp = Arc::new(Viewport::new());
    let t0 = 6_000_000u64;

    // Build a joint slot.
    let ev = make_vib_evidence(t0, 0.1);
    let slot_id = match vp.claim_slot_for_event(&ev) {
        SlotClaim::Allocated(i) | SlotClaim::Reused(i) => i,
        SlotClaim::Dropped => panic!("must allocate"),
    };
    write_vib_and_fuse(&vp, slot_id, t0);
    write_phot_and_fuse(&vp, slot_id, t0);

    let snap_before = vp.read_slot(slot_id).unwrap();
    let f_before = snap_before.fusion.expect("fusion must exist");

    // Write and read snapshot.
    let dir = std::env::temp_dir().join("bionic_v0_phase5a_fusion_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let slot_snap = &data.slots[slot_id];
    let f = slot_snap.fusion.expect("fusion must survive roundtrip");
    assert_eq!(f.modality_mask, f_before.modality_mask);
    assert_eq!(f.last_vib_t_us, f_before.last_vib_t_us);
    assert_eq!(f.last_phot_t_us, f_before.last_phot_t_us);
    assert!((f.temporal_alignment_score - f_before.temporal_alignment_score).abs() < 1e-12);
    assert!((f.cross_modal_confidence - f_before.cross_modal_confidence).abs() < 1e-12);
    assert_eq!(f.is_cross_modal, f_before.is_cross_modal);
}

/// Regression: claim_slot_for_event remains the sole slot-decision authority.
/// After adding fusion, claim outcomes must be unchanged.
#[test]
fn claim_architecture_remains_unchanged() {
    let vp = Arc::new(Viewport::new());
    let t0 = 7_000_000u64;

    // Allocate first slot.
    let ev1 = make_vib_evidence(t0, 0.1);
    let claim1 = vp.claim_slot_for_event(&ev1);
    assert!(matches!(claim1, SlotClaim::Allocated(_)), "first VIB must allocate");

    let slot_id = claim1.slot_id().unwrap();
    write_vib_and_fuse(&vp, slot_id, t0);

    // Same-modality reuse within SLOT_IDLE_US must still reuse.
    let t1 = t0 + 1_000;
    let ev2 = make_vib_evidence(t1, 0.1);
    let claim2 = vp.claim_slot_for_event(&ev2);
    assert!(
        matches!(claim2, SlotClaim::Reused(_)),
        "same-modality within window must reuse, got {:?}", claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    // Verify fusion did not change the slot count.
    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(active, 1, "still only 1 active slot");
}
