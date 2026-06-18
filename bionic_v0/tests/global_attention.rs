//! Phase 5C: Global Attention Allocator tests.
//!
//! Architecture invariants checked here:
//!   * attention_share sums to 1 across active slots (or 0 when no slots).
//!   * Joint (cross-modal) slots score higher than VIB-only slots.
//!   * Recency decay reduces stale-slot shares relative to fresh slots.
//!   * precision_bias = (2 * attention_share).clamp(0, 2) — suggestion only.
//!   * Snapshot roundtrip preserves all GlobalAttentionState fields.
//!   * claim/scoring contract is unchanged by Phase 5C.

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{EventEvidence, SlotClaim, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, PHOT_BANDS, SLOT_IDLE_US, VIB_BANDS};

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

fn tick_vib_only(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
}

// ---------------------------------------------------------------------------
// Test 1: single active slot gets full attention
// ---------------------------------------------------------------------------

/// When exactly one slot is active and warmed-up, it must receive the entire
/// attention budget: share = 1.0, precision_bias = 2.0.
#[test]
fn single_active_slot_gets_full_attention() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let slot_id = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc");

    for i in 0..5u64 {
        tick_joint(&vp, slot_id, t0 + i * 1_000);
    }
    let t_now = t0 + 5_000;
    vp.refresh_global_attention(t_now);

    let ga = vp.read_global_attention();
    assert_eq!(
        ga.focused_slot_id, slot_id as i8,
        "focused_slot_id must point to the only active slot"
    );
    assert!(
        (ga.attention_shares[slot_id] - 1.0).abs() < 1e-9,
        "single active slot must receive share=1.0; got {}",
        ga.attention_shares[slot_id]
    );
    assert_eq!(ga.n_active, 1, "n_active must be 1");

    let snap = vp.read_slot(slot_id).unwrap();
    assert!(
        (snap.attention_share - 1.0).abs() < 1e-9,
        "slot attention_share must be 1.0"
    );
    assert!(
        (snap.precision_bias - 2.0).abs() < 1e-9,
        "single slot precision_bias must be 2.0 (= 2 * 1.0); got {}",
        snap.precision_bias
    );
}

// ---------------------------------------------------------------------------
// Test 2: multi-slot attention sorted by score
// ---------------------------------------------------------------------------

/// A joint (cross-modal) slot must receive a higher attention score and be
/// listed first in top_slot_ids relative to a VIB-only slot.
#[test]
fn multi_slot_attention_sorted_by_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 2_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warm up slot_a as joint (high confidence), slot_b as VIB-only (low confidence).
    for i in 0..25u64 {
        tick_joint(&vp, slot_a, t0 + i * 1_000);
    }
    for i in 0..5u64 {
        tick_vib_only(&vp, slot_b, t0 + i * 1_000);
    }

    let t_now = t0 + 25_000;
    vp.refresh_global_attention(t_now);

    let ga = vp.read_global_attention();
    assert_eq!(ga.n_active, 2, "both slots must be active");
    assert_eq!(
        ga.top_slot_ids[0], slot_a as i8,
        "joint slot must rank first; top_slot_ids[0]={} want {}",
        ga.top_slot_ids[0], slot_a
    );
    assert!(
        ga.attention_shares[slot_a] > ga.attention_shares[slot_b],
        "joint slot share ({}) must exceed VIB-only share ({})",
        ga.attention_shares[slot_a],
        ga.attention_shares[slot_b]
    );
}

// ---------------------------------------------------------------------------
// Test 3: low-confidence slot gets lower attention
// ---------------------------------------------------------------------------

/// After warm-up, a joint slot's share must be substantially above 0.5 while
/// the VIB-only counterpart takes the rest.
#[test]
fn low_confidence_slot_gets_lower_attention() {
    let vp = Arc::new(Viewport::new());
    let t0 = 3_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..25u64 {
        tick_joint(&vp, slot_a, t0 + i * 1_000);
    }
    for i in 0..5u64 {
        tick_vib_only(&vp, slot_b, t0 + i * 1_000);
    }

    vp.refresh_global_attention(t0 + 25_000);

    let ga = vp.read_global_attention();
    assert!(
        ga.attention_shares[slot_a] > 0.6,
        "joint slot must capture >60% of attention; got {:.3}",
        ga.attention_shares[slot_a]
    );
    // Shares must sum to ~1.
    let total: f64 = ga.attention_shares.iter().sum();
    assert!(
        (total - 1.0).abs() < 1e-9,
        "attention shares must sum to 1.0; got {:.6}",
        total
    );
}

// ---------------------------------------------------------------------------
// Test 4: stale slot attention decays over time
// ---------------------------------------------------------------------------

/// Slot B that has not received a fresh fusion refresh is penalised by the
/// recency factor. Slot A that was refreshed at t_late must dominate.
#[test]
fn stale_slot_attention_decays_over_time() {
    let vp = Arc::new(Viewport::new());
    let t0 = 4_000_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Both slots get 5 joint ticks at t0.
    for i in 0..5u64 {
        tick_joint(&vp, slot_a, t0 + i * 1_000);
        tick_joint(&vp, slot_b, t0 + i * 1_000);
    }

    // At t_late, only slot_a receives a fresh refresh.
    // t_late is well within SLOT_IDLE_US so sweep_slots would not clear B,
    // but B's recency factor decays because its last_fused_t_us is t0+4_000.
    let t_late = t0 + SLOT_IDLE_US * 2 / 3;
    tick_joint(&vp, slot_a, t_late);

    vp.refresh_global_attention(t_late);

    let ga = vp.read_global_attention();
    assert!(
        ga.attention_shares[slot_a] > ga.attention_shares[slot_b],
        "fresh slot_a share ({:.3}) must exceed stale slot_b share ({:.3})",
        ga.attention_shares[slot_a],
        ga.attention_shares[slot_b]
    );
    assert_eq!(
        ga.focused_slot_id, slot_a as i8,
        "focused slot must be the fresh one"
    );
}

// ---------------------------------------------------------------------------
// Test 5: snapshot roundtrip preserves global attention state
// ---------------------------------------------------------------------------

/// All GlobalAttentionState fields and per-slot attention_share/precision_bias
/// must survive a write_snapshot → read_snapshot roundtrip unchanged.
#[test]
fn snapshot_roundtrip_preserves_global_attention_state() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;

    let slot_id = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc");
    for i in 0..5u64 {
        tick_joint(&vp, slot_id, t0 + i * 1_000);
    }
    vp.refresh_global_attention(t0 + 5_000);

    let ga_before = vp.read_global_attention();
    let snap_before = vp.read_slot(slot_id).unwrap();

    let dir = std::env::temp_dir().join(format!("bionic_v0_phase5c_ga_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let ga = data.global_attention;
    assert_eq!(
        ga.focused_slot_id, ga_before.focused_slot_id,
        "focused_slot_id"
    );
    assert_eq!(ga.n_active, ga_before.n_active, "n_active");
    assert!(
        (ga.focused_score - ga_before.focused_score).abs() < 1e-12,
        "focused_score roundtrip"
    );
    assert!(
        (ga.attention_entropy - ga_before.attention_entropy).abs() < 1e-12,
        "attention_entropy roundtrip"
    );
    for i in 0..N_OBJECT_SLOTS {
        assert!(
            (ga.attention_shares[i] - ga_before.attention_shares[i]).abs() < 1e-12,
            "attention_shares[{}] roundtrip",
            i
        );
        assert_eq!(
            ga.top_slot_ids[i], ga_before.top_slot_ids[i],
            "top_slot_ids[{}]",
            i
        );
    }

    let slot_snap = &data.slots[slot_id];
    assert!(
        (slot_snap.attention_share - snap_before.attention_share).abs() < 1e-12,
        "per-slot attention_share roundtrip"
    );
    assert!(
        (slot_snap.precision_bias - snap_before.precision_bias).abs() < 1e-12,
        "per-slot precision_bias roundtrip"
    );
}

// ---------------------------------------------------------------------------
// Test 6: claim architecture unchanged after attention
// ---------------------------------------------------------------------------

/// refresh_global_attention must not alter the claim/scoring contract:
/// same-modality reuse, allocation, and slot-count invariants must hold.
#[test]
fn claim_architecture_unchanged_after_attention() {
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

    // Run attention refreshes on this slot.
    tick_joint(&vp, slot_id, t0);
    tick_joint(&vp, slot_id, t0 + 1_000);
    vp.refresh_global_attention(t0 + 2_000);

    // Same-modality reuse must still work after attention allocation.
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

    // Total active slots: attention refresh must not create extras.
    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(active, 1, "attention refresh must not change slot count");
}
