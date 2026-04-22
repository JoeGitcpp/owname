//! Phase 3D: viewport upgrades from first-match to best-match claim. These
//! tests exercise the numeric scoring rule directly — no semantic labels, no
//! adapter-internal state, only `EventEvidence` + explicit slot seeding via
//! the public viewport API.

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{EventEvidence, SlotClaim, SpatialEvidence, Viewport};
use bionic_v0::{
    CLAIM_KIND_ALLOCATED, CLAIM_KIND_REUSED, CLAIM_REUSE_MIN_SCORE, CROSS_MODAL_WINDOW_US,
    MOD_PHOT, MOD_VIB, SLOT_IDLE_US,
};
use std::sync::Arc;

fn ev(
    modality: u8,
    t: u64,
    spatial: Option<SpatialEvidence>,
    energy: f64,
    surprise: f64,
) -> EventEvidence {
    EventEvidence {
        modality_bit: modality,
        event_t_us: t,
        event_energy: energy,
        surprise,
        spatial,
        nomination_seq_hint: 0,
    }
}

#[test]
fn best_match_prefers_closer_time_candidate() {
    // Two same-modality slots exist at different ages. A new event arrives;
    // the temporally closer slot must win even though iteration order would
    // have picked slot 0 first under the old first-match rule.
    let vp = Viewport::new();

    // Seed slot 0 as an older VIB slot; seed slot 1 as a fresher VIB slot.
    let t_old = 1_000_000u64;
    let t_new = 1_150_000u64; // 150ms later, still within SLOT_IDLE_US
    let s_old = vp.allocate_slot(MOD_VIB, t_old, 1).expect("alloc 0");
    let s_new = vp.allocate_slot(MOD_VIB, t_new, 2).expect("alloc 1");
    assert_ne!(s_old, s_new);

    // New VIB event at t_new + small epsilon — both candidates pass hard
    // gates. The scorer must prefer the slot closer in time.
    let claim = vp.claim_slot_for_event(&ev(MOD_VIB, t_new + 500, None, 1e-3, 0.0));
    match claim {
        SlotClaim::Reused(id) => assert_eq!(id, s_new, "should reuse the fresher slot"),
        other => panic!("expected Reused(fresher slot), got {:?}", other),
    }
    let dbg = vp.read_last_claim_debug();
    assert_eq!(dbg.chosen_slot as usize, s_new);
    assert_eq!(dbg.n_candidates_considered, 2);
}

#[test]
fn best_match_prefers_spatially_closer_candidate() {
    // Two same-modality slots, same-age, different spatial centroids. A new
    // PHOT event with spatial must pick the geometrically closer slot.
    let vp = Viewport::new();
    let t0 = 1_000_000u64;
    let s_far = vp.allocate_slot(MOD_PHOT, t0, 1).unwrap();
    let s_near = vp.allocate_slot(MOD_PHOT, t0, 2).unwrap();
    assert_ne!(s_far, s_near);
    // Far slot centroid at (0.1, 0.1); near slot centroid at (0.5, 0.5).
    vp.write_slot_spatial(s_far, MOD_PHOT, t0, 0.1, 0.1, 0.02, 0.02, 0.5);
    vp.write_slot_spatial(s_near, MOD_PHOT, t0, 0.5, 0.5, 0.02, 0.02, 0.5);

    let ev_spatial = SpatialEvidence {
        pos_x: 0.52,
        pos_y: 0.52,
        span_x: 0.02,
        span_y: 0.02,
        precision: 0.5,
    };
    let claim = vp.claim_slot_for_event(&ev(MOD_PHOT, t0 + 500, Some(ev_spatial), 1e-3, 0.0));
    match claim {
        SlotClaim::Reused(id) => assert_eq!(id, s_near, "should reuse spatially closer slot"),
        other => panic!("expected Reused(near), got {:?}", other),
    }
}

#[test]
fn same_modality_continuation_beats_cross_modal_when_scores_close() {
    // Two candidates in-window:
    //   A) VIB slot at t0 (same modality as incoming)
    //   B) PHOT slot at t0 with a spatial centroid matching the ev spatial
    //      (cross-modal but geometrically tight)
    // Incoming VIB event at t0+1ms with the same spatial. Modality bonus
    // should tip the balance toward A.
    let vp = Viewport::new();
    let t0 = 1_000_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 1).unwrap();
    let b = vp.allocate_slot(MOD_PHOT, t0, 2).unwrap();
    // Both candidates carry a matching spatial estimate, so the spatial
    // component ties (~1.0 each). Modality should be the deciding factor.
    vp.write_slot_spatial(a, MOD_VIB, t0, 0.5, 0.5, 0.05, 0.05, 0.5);
    vp.write_slot_spatial(b, MOD_PHOT, t0, 0.5, 0.5, 0.05, 0.05, 0.5);

    let ev_spatial = SpatialEvidence {
        pos_x: 0.5,
        pos_y: 0.5,
        span_x: 0.05,
        span_y: 0.05,
        precision: 0.5,
    };
    // Note: VIB event carries spatial purely as a scoring input; the adapter
    // contract doesn't require it, but the viewport must still compare on it.
    let claim = vp.claim_slot_for_event(&ev(MOD_VIB, t0 + 1_000, Some(ev_spatial), 1e-3, 0.0));
    match claim {
        SlotClaim::Reused(id) => assert_eq!(id, a, "same-modality should win on tie"),
        other => panic!("expected Reused(same-mod VIB slot), got {:?}", other),
    }
    let dbg = vp.read_last_claim_debug();
    // Modality score on the winner should match the same-modality bonus.
    assert!(
        dbg.best_modality_score > 0.9,
        "same-mod modality score should be ~1.0"
    );
}

#[test]
fn low_score_candidate_does_not_reuse_slot() {
    // Single same-modality candidate whose age sits near SLOT_IDLE_US. Its
    // time_score is tiny; without spatial/energy boost the total stays below
    // CLAIM_REUSE_MIN_SCORE, so viewport must allocate a fresh slot.
    let vp = Viewport::new();
    let t_old = 1_000_000u64;
    let stale = vp.allocate_slot(MOD_VIB, t_old, 1).unwrap();

    // dt = 190ms of 200ms => time_score ~= 0.05. No spatial, ambient energy
    // zero => energy_score = 0.5, surprise = 0.5. Total ~= 0.42 < 0.50.
    let t_new = t_old + 190_000;
    let claim = vp.claim_slot_for_event(&ev(MOD_VIB, t_new, None, 1e-3, 0.0));
    match claim {
        SlotClaim::Allocated(id) => assert_ne!(id, stale, "low score must force fresh allocation"),
        other => panic!("expected Allocated(fresh), got {:?}", other),
    }
    let dbg = vp.read_last_claim_debug();
    assert!(
        dbg.best_score < CLAIM_REUSE_MIN_SCORE,
        "best_score should be below reuse threshold, got {}",
        dbg.best_score
    );
    assert_eq!(dbg.kind, CLAIM_KIND_ALLOCATED);
}

#[test]
fn observer_sees_best_match_metadata_via_snapshot() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let seed = vp.allocate_slot(MOD_VIB, t0, 1).unwrap();
    let _ = seed;

    let claim = vp.claim_slot_for_event(&ev(MOD_VIB, t0 + 1_000, None, 1e-3, 0.0));
    assert!(matches!(claim, SlotClaim::Reused(_)));

    let dir = std::env::temp_dir().join("bionic_v0_phase3d_snap");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.last_claim.kind, CLAIM_KIND_REUSED);
    assert!(snap.last_claim.chosen_slot >= 0);
    assert!(snap.last_claim.best_score >= CLAIM_REUSE_MIN_SCORE);
    assert!(snap.last_claim.n_candidates_considered >= 1);
    assert!((snap.last_claim.min_reuse_score - CLAIM_REUSE_MIN_SCORE).abs() < 1e-9);
    // Soft components must be numeric-finite and in [0, 1].
    for s in [
        snap.last_claim.best_time_score,
        snap.last_claim.best_spatial_score,
        snap.last_claim.best_modality_score,
        snap.last_claim.best_energy_score,
        snap.last_claim.best_surprise_score,
    ] {
        assert!(s.is_finite());
        assert!((0.0..=1.0).contains(&s));
    }
}

#[test]
fn claim_scoring_still_respects_previous_phase_guards() {
    // (a) Stale slot past SLOT_IDLE_US must not be reused even if it is the
    //     only candidate.
    let vp = Viewport::new();
    let t_old = 1_000_000u64;
    let stale = vp.allocate_slot(MOD_VIB, t_old, 1).unwrap();
    let t_late = t_old + SLOT_IDLE_US + 10_000;
    let claim = vp.claim_slot_for_event(&ev(MOD_VIB, t_late, None, 1e-3, 0.0));
    match claim {
        SlotClaim::Allocated(id) => assert_ne!(id, stale, "stale slot must not be reused"),
        other => panic!(
            "expected Allocated (stale should hard-gate), got {:?}",
            other
        ),
    }

    // (b) Cross-modal: outside CROSS_MODAL_WINDOW_US must not reuse.
    let vp = Viewport::new();
    let t_vib = 2_000_000u64;
    let vib_slot = vp.allocate_slot(MOD_VIB, t_vib, 1).unwrap();
    let t_phot = t_vib + CROSS_MODAL_WINDOW_US + 10_000;
    let claim = vp.claim_slot_for_event(&ev(
        MOD_PHOT,
        t_phot,
        Some(SpatialEvidence {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: 0.1,
            span_y: 0.1,
            precision: 0.5,
        }),
        1e-3,
        0.0,
    ));
    match claim {
        SlotClaim::Allocated(id) => assert_ne!(id, vib_slot),
        other => panic!(
            "expected Allocated (cross-modal window hard-gate), got {:?}",
            other
        ),
    }

    // (c) Cross-modal: spatial far-apart must not reuse even inside window.
    let vp = Viewport::new();
    let t0 = 3_000_000u64;
    let phot_slot = vp.allocate_slot(MOD_PHOT, t0, 1).unwrap();
    vp.write_slot_spatial(phot_slot, MOD_PHOT, t0, 0.1, 0.1, 0.05, 0.05, 0.5);
    let claim = vp.claim_slot_for_event(&ev(
        MOD_VIB,
        t0 + 1_000,
        Some(SpatialEvidence {
            pos_x: 0.9,
            pos_y: 0.9,
            span_x: 0.05,
            span_y: 0.05,
            precision: 0.5,
        }),
        1e-3,
        0.0,
    ));
    match claim {
        SlotClaim::Allocated(id) => assert_ne!(id, phot_slot),
        other => panic!(
            "expected Allocated (far-spatial hard-gate), got {:?}",
            other
        ),
    }
}
