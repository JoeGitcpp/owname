# Endogenous Activity Laws (Group D P3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement three endogenous-activity laws on existing substrate: err → attention rebalancing, precision → update gating (inside quiet-time history replay), and quiet → history replay ticker.

**Architecture:** Three new methods in `viewport.rs` (`tick_err_driven_attention`, private `precision_gate_from_mse`, `tick_quiet_history_replay`); four new constants in `lib.rs`; one 2 Hz ticker in `body.rs`. No new state types, no new fields on existing structs.

**Tech Stack:** Rust, existing `Viewport` / `PredictorState` / `GlobalAttentionState` / `GlobalReasoningHistoryState` APIs.

**Spec:** `docs/superpowers/specs/2026-04-20-endogenous-activity-self-regulation-laws-design.md`

---

## File Structure

**Create:**
- `bionic_v0/tests/endogenous_activity.rs`

**Modify:**
- `bionic_v0/src/lib.rs` — 4 new constants
- `bionic_v0/src/viewport.rs` — 2 new `pub` methods + 1 private helper; extend `use crate::{...}` block
- `bionic_v0/src/bin/body.rs` — 2 Hz quiet-replay ticker + join handle

---

## Background for the implementer

**Key types (all in `bionic_v0::viewport`):**

| Type | Relevant field |
|---|---|
| `ObjectSlot` | `active: bool`, `high_layer: Option<ObjectHighLayerState>` |
| `ObjectHighLayerState` | `object_err: f64` — RMS prediction error; high when slot is volatile |
| `GlobalAttentionState` | `attention_shares: [f64; N_OBJECT_SLOTS]` — normalised weights |
| `PredictorState` | `mean_sq_err: f64` — EMA of squared prediction error |
| `GlobalReasoningHistoryState` | `get_recent(i) -> Option<&ReasoningHistoryEntry>`, `count: usize` |
| `ReasoningHistoryEntry` | `surprise_load: f64`, `t_us: u64` |

**Key `Viewport` methods already available:**
- `read_global_attention() -> GlobalAttentionState`
- `push_err(axis, t_us, err)` — appends to err_stream ring for one axis
- `health_state: AtomicU8` (pub) — starts `HEALTH_QUIET`

**Constants already in `lib.rs`:** `HEALTH_QUIET: u8 = 1`, `HEALTH_OK: u8 = 2`, `AXIS_COUNT: usize = 4`.

**`AXIS_COUNT` is already imported in viewport.rs.** The four new constants are not.

---

## Task 1: New Constants

**Files:**
- Create: `bionic_v0/tests/endogenous_activity.rs` (stub + constants test)
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Create test file with failing constants test**

```rust
// bionic_v0/tests/endogenous_activity.rs
use bionic_v0::{
    ATTN_ERR_REBALANCE_ALPHA, HISTORY_REPLAY_SIGNAL_SCALE, PRECISION_GATE_MIN,
    PRECISION_GATE_SCALE,
};

#[test]
fn constants_in_range() {
    assert!(ATTN_ERR_REBALANCE_ALPHA > 0.0 && ATTN_ERR_REBALANCE_ALPHA < 1.0);
    assert!(PRECISION_GATE_MIN > 0.0 && PRECISION_GATE_MIN < 1.0);
    assert!(PRECISION_GATE_SCALE > 0.0);
    assert!(HISTORY_REPLAY_SIGNAL_SCALE > 0.0 && HISTORY_REPLAY_SIGNAL_SCALE < 1.0);
}
```

- [ ] **Step 2: Verify FAIL**

```
cargo test -p bionic_v0 --test endogenous_activity constants_in_range
```
Expected: compile error — constants not defined.

- [ ] **Step 3: Add constants to `bionic_v0/src/lib.rs`**

Add after the existing `PHOT_PRED_PRECISION_ALPHA` constant (or any logical grouping):

```rust
// --- Endogenous activity law constants (spec 2026-04-20 §2.1-§2.3) ---

/// EMA alpha for err-driven attention rebalancing.
/// Engineering approximation of the precision-resource redistribution law (§2.3).
pub const ATTN_ERR_REBALANCE_ALPHA: f64 = 0.1;

/// Scale factor mapping mean_sq_err → precision gate ∈ [PRECISION_GATE_MIN, 1.0].
/// gate = mse*K/(1 + mse*K); higher K → gate saturates sooner as error grows.
pub const PRECISION_GATE_SCALE: f64 = 10.0;

/// Floor for the precision gate: replay always injects at least this fraction.
pub const PRECISION_GATE_MIN: f64 = 0.01;

/// Fraction of history entry surprise_load used as replay signal per axis.
pub const HISTORY_REPLAY_SIGNAL_SCALE: f64 = 0.05;
```

- [ ] **Step 4: Verify PASS**

```
cargo test -p bionic_v0 --test endogenous_activity constants_in_range
```

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/tests/endogenous_activity.rs bionic_v0/src/lib.rs
git commit -m "feat: endogenous activity law constants (Group D P3)"
```

---

## Task 2: Err-Driven Attention Rebalancing

**Files:**
- Modify: `bionic_v0/tests/endogenous_activity.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Write failing test**

Append to `bionic_v0/tests/endogenous_activity.rs`:

```rust
use std::sync::Arc;
use bionic_v0::viewport::{ObjectHighLayerState, Viewport};

#[test]
fn high_err_slot_gains_attention_share() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;

    // Directly inject high_layer state via per-slot lock — avoids full pipeline.
    {
        let mut s = vp.slots[0].write().unwrap();
        s.active = true;
        s.last_update_t_us = t0;
        s.high_layer = Some(ObjectHighLayerState {
            object_err: 0.01,  // low err → small attention target
            ..Default::default()
        });
    }
    {
        let mut s = vp.slots[1].write().unwrap();
        s.active = true;
        s.last_update_t_us = t0;
        s.high_layer = Some(ObjectHighLayerState {
            object_err: 0.5,   // high err → large attention target
            ..Default::default()
        });
    }

    vp.tick_err_driven_attention(t0);

    let ga = vp.read_global_attention();
    assert!(
        ga.attention_shares[1] > ga.attention_shares[0],
        "high-err slot[1] must gain more share: [1]={:.4} [0]={:.4}",
        ga.attention_shares[1],
        ga.attention_shares[0],
    );
}

#[test]
fn tick_err_driven_attention_noop_when_no_active_slots() {
    let vp = Arc::new(Viewport::new());
    // No slots active — method must not panic and shares stay at 0.
    vp.tick_err_driven_attention(1_000_000);
    let ga = vp.read_global_attention();
    for share in ga.attention_shares.iter() {
        assert_eq!(*share, 0.0, "all shares must remain 0 when no active slots");
    }
}

#[test]
fn tick_err_driven_attention_noop_when_all_err_zero() {
    let vp = Arc::new(Viewport::new());
    {
        let mut s = vp.slots[0].write().unwrap();
        s.active = true;
        s.high_layer = Some(ObjectHighLayerState {
            object_err: 0.0,
            ..Default::default()
        });
    }
    // All errs are zero — total_err < 1e-9, must return without modifying shares.
    vp.tick_err_driven_attention(1_000_000);
    let ga = vp.read_global_attention();
    assert_eq!(ga.attention_shares[0], 0.0, "zero-err must not modify shares");
}
```

- [ ] **Step 2: Verify FAIL**

```
cargo test -p bionic_v0 --test endogenous_activity high_err_slot_gains_attention_share
```
Expected: compile error — `tick_err_driven_attention` not found.

- [ ] **Step 3: Extend `use crate::{...}` in `viewport.rs`**

Add `ATTN_ERR_REBALANCE_ALPHA` to the existing import block near line 11.

- [ ] **Step 4: Implement `tick_err_driven_attention` in `viewport.rs`**

Add after `read_global_attention()` (around line 1838):

```rust
/// Law: err → attention rebalancing (§2.3, spec 2026-04-20).
///
/// EMA-shifts `global_attention.attention_shares` toward err-proportional weights.
/// High `object_err` on a slot → that slot's target share is larger.
/// No-op when no active slots have a `high_layer`, or when total err < 1e-9.
pub fn tick_err_driven_attention(&self, _now_us: u64) {
    struct SlotErr {
        id: usize,
        err: f64,
    }
    // Lock each slot individually (slots is Vec<RwLock<ObjectSlot>>).
    let slot_errs: Vec<SlotErr> = self
        .slots
        .iter()
        .enumerate()
        .filter_map(|(i, lock)| {
            let s = lock.read().unwrap();
            if s.active {
                s.high_layer
                    .as_ref()
                    .map(|hl| SlotErr { id: i, err: hl.object_err })
            } else {
                None
            }
        })
        .collect();

    if slot_errs.is_empty() {
        return;
    }
    let total_err: f64 = slot_errs.iter().map(|se| se.err).sum();
    if total_err < 1e-9 {
        return;
    }

    let mut ga = self.global_attention.write().unwrap();
    for se in &slot_errs {
        let target = se.err / total_err;
        ga.attention_shares[se.id] =
            (1.0 - ATTN_ERR_REBALANCE_ALPHA) * ga.attention_shares[se.id]
                + ATTN_ERR_REBALANCE_ALPHA * target;
    }
}
```

- [ ] **Step 5: Verify PASS**

```
cargo test -p bionic_v0 --test endogenous_activity
```
Expected: all three attention tests pass (plus `constants_in_range`).

- [ ] **Step 6: Commit**

```bash
git add bionic_v0/tests/endogenous_activity.rs bionic_v0/src/viewport.rs
git commit -m "feat: tick_err_driven_attention — err→attention rebalancing law (Group D P3)"
```

---

## Task 3: Quiet History Replay with Precision Gate

**Files:**
- Modify: `bionic_v0/tests/endogenous_activity.rs`
- Modify: `bionic_v0/src/viewport.rs`

The precision gate is a private helper inside `viewport.rs`; it is tested indirectly through `tick_quiet_history_replay`.

**Design of precision gate:**
```
gate(mse) = clamp(mse * K / (1 + mse * K),  MIN_GATE,  1.0)

mse = 0   → gate ≈ MIN_GATE  (converged predictor, replay barely perturbs)
mse = 0.1 → gate ≈ 0.5       (medium error, moderate replay)
mse >> 1  → gate → 1.0       (bad predictor, full replay perturbation)
```

This embodies "high precision → small update": low MSE = high prediction quality = small gate.

- [ ] **Step 1: Write failing tests**

Append to `bionic_v0/tests/endogenous_activity.rs`:

```rust
use std::sync::atomic::Ordering;
use bionic_v0::viewport::ReasoningHistoryEntry;
use bionic_v0::{AXIS_COUNT, HEALTH_OK};

#[test]
fn quiet_replay_returns_zero_when_not_quiet() {
    let vp = Arc::new(Viewport::new());
    vp.health_state.store(HEALTH_OK, Ordering::Relaxed);
    let activity = vp.tick_quiet_history_replay(1_000_000);
    assert_eq!(activity, 0.0, "must return 0 when health ≠ QUIET");
}

#[test]
fn quiet_replay_returns_zero_when_history_empty() {
    let vp = Arc::new(Viewport::new());
    // Viewport starts HEALTH_QUIET; history ring starts empty.
    let activity = vp.tick_quiet_history_replay(1_000_000);
    assert_eq!(activity, 0.0, "must return 0 when history ring is empty");
}

#[test]
fn quiet_replay_returns_nonzero_with_history() {
    let vp = Arc::new(Viewport::new());
    // Seed one entry with nonzero surprise.
    {
        let mut hist = vp.global_reasoning_history.write().unwrap();
        hist.push(ReasoningHistoryEntry {
            surprise_load: 0.8,
            t_us: 500_000,
            ..Default::default()
        });
    }
    let activity = vp.tick_quiet_history_replay(1_000_000);
    assert!(
        activity > 0.0,
        "must return > 0 when QUIET and history is non-empty"
    );
}

#[test]
fn quiet_replay_larger_with_high_predictor_mse() {
    let vp = Arc::new(Viewport::new());
    {
        let mut hist = vp.global_reasoning_history.write().unwrap();
        hist.push(ReasoningHistoryEntry {
            surprise_load: 0.5,
            t_us: 500_000,
            ..Default::default()
        });
    }

    // Case A: all predictors at default mse = 0.0 (high precision → small gate).
    let activity_low_mse = vp.tick_quiet_history_replay(1_000_000);

    // Case B: set high mse (low precision → large gate).
    for axis in 0..AXIS_COUNT {
        vp.predictors[axis].write().unwrap().mean_sq_err = 1.0;
    }
    let activity_high_mse = vp.tick_quiet_history_replay(1_000_001);

    assert!(
        activity_high_mse > activity_low_mse,
        "high-mse replay must produce more activity: high={:.6} low={:.6}",
        activity_high_mse,
        activity_low_mse,
    );
}
```

- [ ] **Step 2: Verify FAIL**

```
cargo test -p bionic_v0 --test endogenous_activity quiet_replay
```
Expected: compile error — `tick_quiet_history_replay` not found.

- [ ] **Step 3: Extend `use crate::{...}` in `viewport.rs`**

Add to the existing import block:
```
HISTORY_REPLAY_SIGNAL_SCALE, PRECISION_GATE_MIN, PRECISION_GATE_SCALE,
```

- [ ] **Step 4: Implement the private helper and public method in `viewport.rs`**

Add after `tick_err_driven_attention` (keeping the private helper immediately before the public method):

```rust
/// Precision gate: maps mean_sq_err → gate ∈ [PRECISION_GATE_MIN, 1.0].
///
/// High mse (low precision / bad predictor) → large gate → replay helps more.
/// Low mse (high precision / converged predictor) → small gate → replay is quieter.
///
/// Formula: mse * K / (1 + mse * K), then clamped to [MIN, 1].
fn precision_gate_from_mse(mse: f64) -> f64 {
    let s = mse * PRECISION_GATE_SCALE;
    (s / (1.0 + s)).clamp(PRECISION_GATE_MIN, 1.0)
}

/// Law: quiet → history replay; precision → update gating (§2.1, §2.3, spec 2026-04-20).
///
/// Called during `HEALTH_QUIET` periods to maintain endogenous internal activity.
/// Reads the most-recent reasoning history entry and injects a precision-gated
/// replay signal into each axis's `err_stream`. Axes whose predictors are already
/// converged (low `mean_sq_err`) receive a smaller perturbation — that is the
/// precision gate.
///
/// Returns total injected activity magnitude (sum of |replay_err| across all axes).
/// Returns `0.0` when health is not `HEALTH_QUIET` or the history ring is empty.
pub fn tick_quiet_history_replay(&self, now_us: u64) -> f64 {
    if self.health_state.load(Ordering::Relaxed) != HEALTH_QUIET {
        return 0.0;
    }
    let entry = {
        let hist = self.global_reasoning_history.read().unwrap();
        hist.get_recent(0).copied()
    };
    let Some(entry) = entry else {
        return 0.0;
    };

    let mut total_activity = 0.0f64;
    for axis in 0..AXIS_COUNT {
        let mse = self.predictors[axis].read().unwrap().mean_sq_err;
        let gate = Self::precision_gate_from_mse(mse);
        let replay_err = entry.surprise_load * HISTORY_REPLAY_SIGNAL_SCALE * gate;
        self.push_err(axis, now_us, replay_err);
        total_activity += replay_err.abs();
    }
    total_activity
}
```

- [ ] **Step 5: Verify all 4 quiet-replay tests PASS**

```
cargo test -p bionic_v0 --test endogenous_activity
```
Expected: all 7 tests pass (3 attention + 4 quiet-replay).

- [ ] **Step 6: Full suite green**

```
cargo test -p bionic_v0
```
Expected: all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/tests/endogenous_activity.rs bionic_v0/src/viewport.rs
git commit -m "feat: tick_quiet_history_replay + precision gate — quiet→replay + precision→gating laws (Group D P3)"
```

---

## Task 4: 2 Hz Quiet-Replay Ticker in body.rs

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`

No unit test for the ticker thread itself — verified by build + code review.

- [ ] **Step 1: Add ticker after the binding maintenance ticker (around line 124)**

```rust
// Quiet-period history replay, 2 Hz — endogenous activity law §2.1.
let vp_replay = vp.clone();
let h_replay = spawn_ticker(2, stop.clone(), move |_| {
    vp_replay.tick_quiet_history_replay(now_us());
});
```

- [ ] **Step 2: Add join handle to the teardown block**

In the join block near line 151, add:
```rust
let _ = h_replay.join();
```

- [ ] **Step 3: Verify build**

```
cargo build -p bionic_v0 --bin body
```
Expected: clean build, zero warnings.

- [ ] **Step 4: Full test suite**

```
cargo test -p bionic_v0
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/src/bin/body.rs
git commit -m "feat: 2 Hz quiet-replay ticker in body main loop (Group D P3)"
```

---

## Verification Checklist

- [ ] `cargo test -p bionic_v0` — all tests green (no regressions)
- [ ] `cargo build -p bionic_v0 --bin body` — clean build, no warnings
- [ ] `constants_in_range` passes
- [ ] `high_err_slot_gains_attention_share` passes (high-err slot gets more share)
- [ ] `tick_err_driven_attention_noop_when_no_active_slots` passes
- [ ] `tick_err_driven_attention_noop_when_all_err_zero` passes
- [ ] `quiet_replay_returns_zero_when_not_quiet` passes
- [ ] `quiet_replay_returns_zero_when_history_empty` passes
- [ ] `quiet_replay_returns_nonzero_with_history` passes
- [ ] `quiet_replay_larger_with_high_predictor_mse` passes (precision gate works)
- [ ] No new state types or new struct fields introduced
- [ ] No `println!` / `eprintln!` in body.rs code
