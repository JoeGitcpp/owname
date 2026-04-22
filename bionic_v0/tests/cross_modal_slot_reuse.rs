//! Phase 3C: the viewport — not any adapter — decides when two modalities
//! share an ObjectSlot. These tests drive the claim API through
//! `EventEvidence` directly (and through the synthetic adapters) and
//! verify synchronous/desynchronous/stale behaviour.

use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, SyntheticPhotAdapter, SyntheticVibAdapter};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::synthetic_vib::SyntheticVibSource;
use bionic_v0::viewport::{EventEvidence, SlotClaim, SpatialEvidence, Viewport};
use bionic_v0::{CROSS_MODAL_WINDOW_US, MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, SLOT_IDLE_US};

fn count_active_slots(vp: &Viewport) -> usize {
    (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count()
}

fn joint_slot_indices(vp: &Viewport) -> Vec<usize> {
    (0..N_OBJECT_SLOTS)
        .filter(|&i| {
            let s = vp.read_slot(i).unwrap();
            s.active && (s.source_modality & MOD_VIB) != 0 && (s.source_modality & MOD_PHOT) != 0
        })
        .collect()
}

#[test]
fn claim_api_allocates_then_reuses_same_modality() {
    let vp = Viewport::new();
    let t0 = 1_000_000u64;
    let ev = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0,
        event_energy: 0.5,
        surprise: 0.1,
        spatial: None,
        nomination_seq_hint: 0,
    };

    let first = vp.claim_slot_for_event(&ev);
    assert!(matches!(first, SlotClaim::Allocated(_)));

    // Activate the slot so reuse rule can see it — the claim API only
    // allocates the slot with `source_modality = ev.modality_bit`. Subsequent
    // evidence within SLOT_IDLE_US should reuse it.
    let ev2 = EventEvidence {
        event_t_us: t0 + 1_000,
        ..ev
    };
    let second = vp.claim_slot_for_event(&ev2);
    match (first, second) {
        (SlotClaim::Allocated(a), SlotClaim::Reused(b)) => assert_eq!(a, b),
        other => panic!("expected Allocated then Reused, got {:?}", other),
    }
}

#[test]
fn same_time_events_reuse_same_slot() {
    let vp = Arc::new(Viewport::new());
    // Drive both synthetic adapters at times that fall inside the cross-
    // modal window. Use aligned timestamps — the adapters run on the same
    // clock in real life, so this test assumes the adapter scheduler fires
    // them at sub-window granularity.
    let mut phot = SyntheticPhotAdapter::new(1_000_000);
    let mut vib = SyntheticVibAdapter::new(SyntheticVibSource::new(1_000_000, 100_000, 30_000));

    // Warm up both smoothed-energy integrators past their respective
    // thresholds. Step 16_666 us ≈ 60 Hz, matching the synthetic PHOT tick.
    let mut last_t = 0;
    for step in 0..30 {
        let t = 1_000_000 + (step as u64) * 16_666;
        // Interleave the two adapters at the *same* tick so both adapter
        // evidence timestamps match and the claim API sees them within the
        // same cross-modal window.
        vib.tick(&vp, t);
        phot.tick(&vp, t);
        last_t = t;
    }

    let joint = joint_slot_indices(&vp);
    assert!(
        !joint.is_empty(),
        "synchronous synthetic VIB + PHOT should co-inhabit at least one slot, active_slots={} last_t={}",
        count_active_slots(&vp),
        last_t
    );
    // At most 4 slots, but we expect 1 joint.
    assert!(joint.len() >= 1);

    // Sanity: the joint slot carries both family states.
    let s = vp.read_slot(joint[0]).unwrap();
    assert!(s.vib.is_some(), "joint slot must carry vib family");
    assert!(s.phot.is_some(), "joint slot must carry phot family");
    assert!(
        s.spatial.is_some(),
        "joint slot must carry spatial (provided by PHOT)"
    );
    assert_eq!(s.source_modality & MOD_VIB, MOD_VIB);
    assert_eq!(s.source_modality & MOD_PHOT, MOD_PHOT);
}

#[test]
fn desynced_events_allocate_different_slots() {
    // VIB event arrives, its slot becomes stale to cross-modal (outside
    // CROSS_MODAL_WINDOW_US), then PHOT event arrives. They must NOT share.
    let vp = Viewport::new();

    let t_vib = 1_000_000u64;
    let ev_vib = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t_vib,
        event_energy: 0.5,
        surprise: 0.0,
        spatial: None,
        nomination_seq_hint: 0,
    };
    let vib_claim = vp.claim_slot_for_event(&ev_vib);
    let vib_slot = vib_claim.slot_id().expect("vib allocate");

    // PHOT event arrives AFTER the cross-modal window has elapsed but BEFORE
    // the slot would age out. Must allocate a new slot.
    let t_phot = t_vib + CROSS_MODAL_WINDOW_US + 10_000;
    assert!(
        t_phot < t_vib + SLOT_IDLE_US,
        "test precondition: PHOT still inside SLOT_IDLE_US"
    );
    let ev_phot = EventEvidence {
        modality_bit: MOD_PHOT,
        event_t_us: t_phot,
        event_energy: 0.4,
        surprise: 0.0,
        spatial: Some(SpatialEvidence {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: 0.1,
            span_y: 0.1,
            precision: 0.5,
        }),
        nomination_seq_hint: 1,
    };
    let phot_claim = vp.claim_slot_for_event(&ev_phot);
    let phot_slot = phot_claim.slot_id().expect("phot allocate");

    assert_ne!(
        vib_slot, phot_slot,
        "desynced events must land on different slots (vib={}, phot={})",
        vib_slot, phot_slot
    );
    assert!(matches!(phot_claim, SlotClaim::Allocated(_)));

    // Neither slot should have become joint.
    let s_vib = vp.read_slot(vib_slot).unwrap();
    let s_phot = vp.read_slot(phot_slot).unwrap();
    assert_eq!(s_vib.source_modality, MOD_VIB);
    assert_eq!(s_phot.source_modality, MOD_PHOT);
}

#[test]
fn stale_slot_not_reused_cross_modally() {
    // A VIB slot falls out of SLOT_IDLE_US entirely. A PHOT event arriving
    // later must not inherit it, regardless of the cross-modal window.
    let vp = Viewport::new();

    let t_vib = 1_000_000u64;
    let ev_vib = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t_vib,
        event_energy: 0.5,
        surprise: 0.0,
        spatial: None,
        nomination_seq_hint: 0,
    };
    let vib_slot = vp.claim_slot_for_event(&ev_vib).slot_id().expect("alloc");

    // Advance time past SLOT_IDLE_US then sweep — the stale VIB slot should
    // be reclaimed.
    let t_late = t_vib + SLOT_IDLE_US + 50_000;
    vp.sweep_slots(t_late);
    assert!(
        !vp.read_slot(vib_slot).unwrap().active,
        "VIB slot should have been swept"
    );

    // PHOT event arrives now. Must allocate fresh — not recycle the old id
    // with mixed source_modality.
    let ev_phot = EventEvidence {
        modality_bit: MOD_PHOT,
        event_t_us: t_late + 1_000,
        event_energy: 0.4,
        surprise: 0.0,
        spatial: Some(SpatialEvidence {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: 0.1,
            span_y: 0.1,
            precision: 0.5,
        }),
        nomination_seq_hint: 1,
    };
    let claim = vp.claim_slot_for_event(&ev_phot);
    let id = claim.slot_id().expect("phot allocate");

    let s = vp.read_slot(id).unwrap();
    assert_eq!(
        s.source_modality, MOD_PHOT,
        "fresh allocation must carry only MOD_PHOT, no residual VIB bit"
    );
    assert!(matches!(claim, SlotClaim::Allocated(_)));
}

#[test]
fn cross_modal_reuse_rejected_when_spatial_far_apart() {
    // Both sides provide spatial estimates, but the PHOT claimant is far
    // from the existing slot's spatial centroid. Cross-modal reuse must be
    // skipped even within the time window.
    //
    // Scenario: seed a slot as MOD_PHOT (easy way to park a spatial
    // estimate), then send a MOD_VIB event with its own disagreeing spatial
    // estimate. (Yes, VIB normally has no geometry; this test treats
    // `spatial` on EventEvidence as a guardrail on the claim rule, not a
    // statement about any adapter's real output.)
    let vp = Viewport::new();
    let t0 = 1_000_000u64;

    let ev_phot = EventEvidence {
        modality_bit: MOD_PHOT,
        event_t_us: t0,
        event_energy: 0.4,
        surprise: 0.0,
        spatial: Some(SpatialEvidence {
            pos_x: 0.1,
            pos_y: 0.1,
            span_x: 0.05,
            span_y: 0.05,
            precision: 0.5,
        }),
        nomination_seq_hint: 0,
    };
    let phot_slot = vp.claim_slot_for_event(&ev_phot).slot_id().expect("alloc");
    // Seed the slot's spatial field so the claim rule has a centroid to
    // compare against.
    vp.write_slot_spatial(phot_slot, MOD_PHOT, t0, 0.1, 0.1, 0.05, 0.05, 0.5);

    let ev_vib_far = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t0 + 1_000,
        event_energy: 0.5,
        surprise: 0.0,
        spatial: Some(SpatialEvidence {
            pos_x: 0.9,
            pos_y: 0.9,
            span_x: 0.05,
            span_y: 0.05,
            precision: 0.5,
        }),
        nomination_seq_hint: 1,
    };
    let claim = vp.claim_slot_for_event(&ev_vib_far);
    assert!(
        matches!(claim, SlotClaim::Allocated(_)),
        "far-apart spatial evidence must not reuse cross-modally, got {:?}",
        claim
    );
    let new_id = claim.slot_id().unwrap();
    assert_ne!(new_id, phot_slot);
}

#[test]
fn observer_sees_joint_slot_through_snapshot() {
    let vp = Arc::new(Viewport::new());
    let mut phot = SyntheticPhotAdapter::new(1_000_000);
    let mut vib = SyntheticVibAdapter::new(SyntheticVibSource::new(1_000_000, 100_000, 30_000));

    for step in 0..30 {
        let t = 1_000_000 + (step as u64) * 16_666;
        vib.tick(&vp, t);
        phot.tick(&vp, t);
    }

    let dir = std::env::temp_dir().join("bionic_v0_phase3c_snap");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let re = read_snapshot(&paths.final_path).unwrap();

    let mut saw_joint = false;
    for s in re.slots.iter() {
        if !s.active {
            continue;
        }
        let has_vib = s.vib.is_some();
        let has_phot = s.phot.is_some();
        let both_bits = (s.source_modality & MOD_VIB) != 0 && (s.source_modality & MOD_PHOT) != 0;
        if has_vib && has_phot && both_bits {
            saw_joint = true;
        }
    }
    assert!(
        saw_joint,
        "snapshot must carry a joint slot (vib + phot + both modality bits)"
    );
}
