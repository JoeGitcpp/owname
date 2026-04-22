// TDD tests for endogenous activity laws (spec 2026-04-20).
//
// Plan: docs/superpowers/plans/2026-04-22-endogenous-activity-laws.md
//
// Run order:
//   1. Tests written first → all fail (RED, because stubs are no-ops)
//   2. Implementation lands → all pass (GREEN)

use std::sync::atomic::Ordering;
use std::sync::Arc;

use bionic_v0::object_high_layer::ObjectHighLayerState;
use bionic_v0::viewport::{ReasoningHistoryEntry, Viewport};
use bionic_v0::{HEALTH_OK, HEALTH_QUIET};

fn make_hl_state(object_err: f64) -> ObjectHighLayerState {
    ObjectHighLayerState {
        predicted_confidence: 0.0,
        predicted_alignment: 0.0,
        predicted_precision: 0.0,
        object_err,
        attention_bias: 0.5,
        last_update_t_us: 0,
    }
}

// ─── 1. Err-driven attention rebalancing ─────────────────────────────────────

#[test]
fn high_err_slot_gains_attention_share() {
    let vp = Arc::new(Viewport::new());

    // Activate two slots with different err levels.
    {
        let mut s0 = vp.slots[0].write().unwrap();
        s0.active = true;
        s0.high_layer = Some(make_hl_state(0.01));

        let mut s1 = vp.slots[1].write().unwrap();
        s1.active = true;
        s1.high_layer = Some(make_hl_state(0.5));
    }

    // Seed attention shares equally.
    {
        let mut attn = vp.global_attention.write().unwrap();
        attn.attention_shares[0] = 0.5;
        attn.attention_shares[1] = 0.5;
        attn.n_active = 2;
        attn.top_slot_ids[0] = 0;
        attn.top_slot_ids[1] = 1;
    }

    let before = vp.read_global_attention();
    vp.tick_err_driven_attention(1_000);
    let after = vp.read_global_attention();

    // Slot 1 (high err) should have gained attention share.
    assert!(
        after.attention_shares[1] > before.attention_shares[1],
        "High-err slot must gain attention share: before={}, after={}",
        before.attention_shares[1],
        after.attention_shares[1]
    );
    // Slot 0 (low err) should have lost attention share.
    assert!(
        after.attention_shares[0] < before.attention_shares[0],
        "Low-err slot must lose attention share: before={}, after={}",
        before.attention_shares[0],
        after.attention_shares[0]
    );
}

#[test]
fn no_active_slots_no_rebalance() {
    let vp = Arc::new(Viewport::new());

    // No active slots.
    {
        let mut attn = vp.global_attention.write().unwrap();
        attn.attention_shares[0] = 0.5;
        attn.n_active = 0;
    }

    vp.tick_err_driven_attention(1_000);

    let after = vp.read_global_attention();
    assert_eq!(
        after.attention_shares[0], 0.5,
        "Attention must not change when no slots are active"
    );
}

#[test]
fn zero_err_no_rebalance() {
    let vp = Arc::new(Viewport::new());

    // Activate a slot with zero err.
    {
        let mut s0 = vp.slots[0].write().unwrap();
        s0.active = true;
        s0.high_layer = Some(make_hl_state(0.0));
    }

    vp.tick_err_driven_attention(1_000);

    let after = vp.read_global_attention();
    assert_eq!(
        after.attention_shares[0], 0.0,
        "Zero total err must not change attention"
    );
}

// ─── 2. Quiet-period history replay ──────────────────────────────────────────

#[test]
fn quiet_period_replay_produces_nonzero_intensity() {
    let vp = Arc::new(Viewport::new());

    // Force QUIET state.
    vp.health_state.store(HEALTH_QUIET, Ordering::Relaxed);

    // Seed reasoning history with entries.
    {
        let mut hist = vp.global_reasoning_history.write().unwrap();
        for i in 0..5 {
            hist.push(ReasoningHistoryEntry {
                surprise_load: 0.3 + i as f64 * 0.1,
                coordinated_event: 0.2,
                network_instability: 0.1,
                top_slot_id: 0,
                top_edge_id: 0,
                t_us: 1_000 + i as u64 * 100_000,
            });
        }
    }

    let before = vp.read_global_attention();
    assert_eq!(before.replay_intensity, 0.0, "Replay must start at zero");

    vp.tick_quiet_history_replay(1_000_000);

    let after = vp.read_global_attention();
    assert!(
        after.replay_intensity > 0.0,
        "Quiet replay must produce non-zero replay_intensity, got {}",
        after.replay_intensity
    );
    assert!(
        after.last_replay_t_us > 0,
        "last_replay_t_us must be updated"
    );
}

#[test]
fn non_quiet_health_skips_replay() {
    let vp = Arc::new(Viewport::new());

    // Set health to OK (not QUIET).
    vp.health_state.store(HEALTH_OK, Ordering::Relaxed);

    // Seed reasoning history.
    {
        let mut hist = vp.global_reasoning_history.write().unwrap();
        hist.push(ReasoningHistoryEntry {
            surprise_load: 0.5,
            coordinated_event: 0.2,
            network_instability: 0.1,
            top_slot_id: 0,
            top_edge_id: 0,
            t_us: 1_000,
        });
    }

    vp.tick_quiet_history_replay(1_000_000);

    let after = vp.read_global_attention();
    assert_eq!(
        after.replay_intensity, 0.0,
        "Replay must not run when health is not QUIET"
    );
    assert_eq!(
        after.last_replay_t_us, 0,
        "last_replay_t_us must remain zero"
    );
}

#[test]
fn empty_history_skips_replay() {
    let vp = Arc::new(Viewport::new());

    // Force QUIET state.
    vp.health_state.store(HEALTH_QUIET, Ordering::Relaxed);

    // No history entries.
    vp.tick_quiet_history_replay(1_000_000);

    let after = vp.read_global_attention();
    assert_eq!(
        after.replay_intensity, 0.0,
        "Replay must not run with empty history"
    );
}

#[test]
fn replay_intensity_accumulates_over_multiple_ticks() {
    let vp = Arc::new(Viewport::new());

    vp.health_state.store(HEALTH_QUIET, Ordering::Relaxed);

    {
        let mut hist = vp.global_reasoning_history.write().unwrap();
        for i in 0..10 {
            hist.push(ReasoningHistoryEntry {
                surprise_load: 0.5,
                coordinated_event: 0.2,
                network_instability: 0.1,
                top_slot_id: 0,
                top_edge_id: 0,
                t_us: 1_000 + i as u64 * 100_000,
            });
        }
    }

    let mut intensities = Vec::new();
    for i in 0..5 {
        vp.tick_quiet_history_replay(1_000_000 + i as u64 * 100_000);
        intensities.push(vp.read_global_attention().replay_intensity);
    }

    // Replay intensity should accumulate (EMA).
    assert!(
        intensities[4] > intensities[0],
        "Replay intensity should accumulate over ticks: {:?}",
        intensities
    );
}
