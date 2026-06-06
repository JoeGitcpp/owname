//! Phase 5G: Global reasoning temporal memory / history substrate tests.
//!
//! Contracts verified:
//!   1. Brief spike has high novelty but low persistence.
//!   2. Sustained pattern raises persistence_score.
//!   3. Stable routing raises routing_stability_score.
//!   4. Repeated pattern raises recurrence_score.
//!   5. Snapshot roundtrip preserves memory summary.
//!   6. Ordered refresh contract remains intact with history step.
//!   7. Claim architecture remains unchanged after history.

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{EventEvidence, SlotClaim, Viewport};
use bionic_v0::{MOD_VIB, N_OBJECT_SLOTS, VIB_BANDS};

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
    vp.refresh_slot_high_layer(slot_id, t_us);
}

fn run_cycle(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_a: u64, t_b: u64) {
    tick_joint(vp, slot_a, t_a);
    tick_joint(vp, slot_b, t_b);
    let t_cycle = t_a.max(t_b);
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
    vp.refresh_global_reasoning(t_cycle);
    vp.refresh_global_reasoning_history(t_cycle);
}

fn run_cycle_with_offset(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_a: u64, t_b: u64) {
    tick_joint(vp, slot_a, t_a);
    tick_joint(vp, slot_b, t_b);
    let t_cycle = t_a.max(t_b);
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
    vp.refresh_global_reasoning(t_cycle);
    vp.refresh_global_reasoning_history(t_cycle);
}

// ---------------------------------------------------------------------------
// Test 1: brief_spike_has_high_novelty_but_low_persistence
//
// A single cycle with high surprise should produce high novelty but
// persistence should remain low (needs multiple cycles to accumulate).
// ---------------------------------------------------------------------------

#[test]
fn brief_spike_has_high_novelty_but_low_persistence() {
    let vp = Arc::new(Viewport::new());
    let t0 = 10_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup: establish baseline
    for i in 0..20u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    // Get baseline memory state
    let mem_before = vp.read_global_reasoning_memory_summary();

    // Single spike cycle with large timing offset
    let t_spike = t0 + 20 * dt;
    let big_delta = 80_000u64;
    tick_joint(&vp, slot_a, t_spike);
    tick_joint(&vp, slot_b, t_spike + big_delta);
    let t_cycle = t_spike + big_delta;
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
    vp.refresh_global_reasoning(t_cycle);
    vp.refresh_global_reasoning_history(t_cycle);

    let mem_after = vp.read_global_reasoning_memory_summary();

    // Novelty should spike due to deviation from baseline
    assert!(
        mem_after.novelty_score > mem_before.novelty_score || mem_after.novelty_score > 0.1,
        "brief spike should raise novelty; before={:.4}, after={:.4}",
        mem_before.novelty_score,
        mem_after.novelty_score
    );

    // Persistence should remain low (single spike, not sustained)
    assert!(
        mem_after.persistence_score < 0.5,
        "brief spike should have low persistence; got {:.4}",
        mem_after.persistence_score
    );
}

// ---------------------------------------------------------------------------
// Test 2: sustained_pattern_raises_persistence_score
//
// Multiple consecutive cycles with high surprise/instability should
// raise the persistence_score as the pattern persists.
// ---------------------------------------------------------------------------

#[test]
fn sustained_pattern_raises_persistence_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 11_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup
    for i in 0..15u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let mem_early = vp.read_global_reasoning_memory_summary();

    // Sustained high-instability pattern: many cycles with timing offset
    let t_base = t0 + 15 * dt;
    for i in 0..15u64 {
        let big_delta = 60_000u64;
        run_cycle_with_offset(
            &vp,
            slot_a,
            slot_b,
            t_base + i * dt,
            t_base + i * dt + big_delta,
        );
    }

    let mem_late = vp.read_global_reasoning_memory_summary();

    // Persistence should rise with sustained pattern
    assert!(
        mem_late.persistence_score > mem_early.persistence_score,
        "sustained pattern should raise persistence; early={:.4}, late={:.4}",
        mem_early.persistence_score,
        mem_late.persistence_score
    );

    // After 15 cycles of high intensity, persistence should be significant
    assert!(
        mem_late.persistence_score > 0.3,
        "after 15 cycles of high intensity, persistence should be > 0.3; got {:.4}",
        mem_late.persistence_score
    );
}

// ---------------------------------------------------------------------------
// Test 3: stable_routing_raises_routing_stability_score
//
// When routing focus remains on the same slot for multiple cycles,
// routing_stability_score should rise.
// ---------------------------------------------------------------------------

#[test]
fn stable_routing_raises_routing_stability_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup with stable synchronized pattern (low instability)
    for i in 0..10u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    // Now create a stable focus on slot A by giving it higher activity
    let t_base = t0 + 10 * dt;
    for i in 0..12u64 {
        // Slot A gets regular updates, slot B gets stale
        tick_joint(&vp, slot_a, t_base + i * dt);
        // Don't tick slot B - it goes stale
        let t_cycle = t_base + i * dt;
        vp.refresh_global_attention(t_cycle);
        vp.refresh_global_relations(t_cycle);
        vp.refresh_global_relation_predictor(t_cycle);
        vp.refresh_global_reasoning(t_cycle);
        vp.refresh_global_reasoning_history(t_cycle);
    }

    let mem = vp.read_global_reasoning_memory_summary();

    // With stable focus on slot A, routing stability should be high
    assert!(
        mem.routing_stability_score > 0.5,
        "stable routing focus should yield high stability; got {:.4}",
        mem.routing_stability_score
    );

    // Dominant focus persistence should also be elevated
    assert!(
        mem.dominant_focus_persistence > 0.3,
        "stable focus should yield elevated persistence; got {:.4}",
        mem.dominant_focus_persistence
    );
}

// ---------------------------------------------------------------------------
// Test 4: repeated_pattern_raises_recurrence_score
//
// When a similar pattern appears multiple times in history,
// pattern_recurrence_score should rise.
// ---------------------------------------------------------------------------

#[test]
fn repeated_pattern_raises_recurrence_score() {
    let vp = Arc::new(Viewport::new());
    let t0 = 13_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Warmup
    for i in 0..10u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    // Create a specific pattern: slot A active, moderate surprise
    let t_base = t0 + 10 * dt;
    for i in 0..5u64 {
        run_cycle_with_offset(
            &vp,
            slot_a,
            slot_b,
            t_base + i * dt,
            t_base + i * dt + 30_000u64,
        );
    }

    let _mem_mid = vp.read_global_reasoning_memory_summary();

    // Repeat the same pattern
    let t_repeat = t_base + 5 * dt + 50_000u64;
    for i in 0..5u64 {
        run_cycle_with_offset(
            &vp,
            slot_a,
            slot_b,
            t_repeat + i * dt,
            t_repeat + i * dt + 30_000u64,
        );
    }

    let mem_late = vp.read_global_reasoning_memory_summary();

    // Recurrence should be detected when pattern repeats
    assert!(
        mem_late.pattern_recurrence_score > 0.1,
        "repeated pattern should raise recurrence; got {:.4}",
        mem_late.pattern_recurrence_score
    );
}

// ---------------------------------------------------------------------------
// Test 5: snapshot_roundtrip_preserves_memory_summary
// ---------------------------------------------------------------------------

#[test]
fn snapshot_roundtrip_preserves_memory_summary() {
    let vp = Arc::new(Viewport::new());
    let t0 = 14_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Run cycles to populate history
    for i in 0..20u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let mem_before = vp.read_global_reasoning_memory_summary();

    let dir = std::env::temp_dir().join(format!("bionic_v0_5g_history_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let data = read_snapshot(&paths.final_path).expect("read ok");

    let mem = &data.global_reasoning_memory;
    assert!(
        (mem.novelty_score - mem_before.novelty_score).abs() < 1e-12,
        "novelty_score roundtrip"
    );
    assert!(
        (mem.persistence_score - mem_before.persistence_score).abs() < 1e-12,
        "persistence_score roundtrip"
    );
    assert!(
        (mem.routing_stability_score - mem_before.routing_stability_score).abs() < 1e-12,
        "routing_stability_score roundtrip"
    );
    assert!(
        (mem.pattern_recurrence_score - mem_before.pattern_recurrence_score).abs() < 1e-12,
        "pattern_recurrence_score roundtrip"
    );
    assert!(
        (mem.history_fill_ratio - mem_before.history_fill_ratio).abs() < 1e-12,
        "history_fill_ratio roundtrip"
    );
    assert!(
        (mem.dominant_focus_persistence - mem_before.dominant_focus_persistence).abs() < 1e-12,
        "dominant_focus_persistence roundtrip"
    );
    assert_eq!(
        mem.last_update_t_us, mem_before.last_update_t_us,
        "last_update_t_us roundtrip"
    );
}

// ---------------------------------------------------------------------------
// Test 6: ordered_refresh_contract_remains_intact_with_history
//
// last_update_t_us must equal the cycle timestamp, proving the history step
// ran in the same serial cycle as reasoning.
// ---------------------------------------------------------------------------

#[test]
fn ordered_refresh_contract_remains_intact_with_history() {
    let vp = Arc::new(Viewport::new());
    let t0 = 15_000_000u64;
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
    vp.refresh_global_reasoning_history(t_cycle);

    let mem = vp.read_global_reasoning_memory_summary();
    assert_eq!(
        mem.last_update_t_us, t_cycle,
        "memory summary last_update_t_us must equal cycle timestamp"
    );

    // Reasoning also updated at same timestamp
    let rs = vp.read_global_reasoning();
    assert_eq!(
        rs.last_update_t_us, t_cycle,
        "reasoning last_update_t_us must equal cycle timestamp"
    );
}

// ---------------------------------------------------------------------------
// Test 7: claim_architecture_remains_unchanged_after_history
//
// Running history refreshes must leave claim/reuse/slot-count invariants
// completely unchanged.
// ---------------------------------------------------------------------------

#[test]
fn claim_architecture_remains_unchanged_after_history() {
    let vp = Arc::new(Viewport::new());
    let t0 = 16_000_000u64;
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
        vp.refresh_global_reasoning_history(t);
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
        "same-modality within window must reuse after history; got {:?}",
        claim2
    );
    assert_eq!(claim2.slot_id(), claim1.slot_id(), "must reuse same slot");

    let active = (0..N_OBJECT_SLOTS)
        .filter(|&i| vp.read_slot(i).map(|s| s.active).unwrap_or(false))
        .count();
    assert_eq!(
        active, 1,
        "history must not change slot count; got {} active slots",
        active
    );
}

// ---------------------------------------------------------------------------
// Test 8: history_fill_ratio_tracks_buffer_population
//
// As more cycles are run, history_fill_ratio should increase until it
// reaches 1.0 (full buffer).
// ---------------------------------------------------------------------------

#[test]
fn history_fill_ratio_tracks_buffer_population() {
    let vp = Arc::new(Viewport::new());
    let t0 = 17_000_000u64;
    let dt = 10_000u64;

    let slot_a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let slot_b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Initial state: empty buffer
    let mem_initial = vp.read_global_reasoning_memory_summary();
    assert!(
        mem_initial.history_fill_ratio < 0.1,
        "initial fill ratio should be near zero; got {:.4}",
        mem_initial.history_fill_ratio
    );

    // Run some cycles
    for i in 0..10u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let mem_partial = vp.read_global_reasoning_memory_summary();
    assert!(
        mem_partial.history_fill_ratio > mem_initial.history_fill_ratio,
        "fill ratio should increase with cycles"
    );

    // Run many more cycles to fill buffer (32 is REASONING_HISTORY_RING_CAP)
    for i in 10..50u64 {
        run_cycle(&vp, slot_a, slot_b, t0 + i * dt, t0 + i * dt);
    }

    let mem_full = vp.read_global_reasoning_memory_summary();
    assert!(
        mem_full.history_fill_ratio >= 0.99,
        "after 50 cycles, fill ratio should be ~1.0; got {:.4}",
        mem_full.history_fill_ratio
    );
}
