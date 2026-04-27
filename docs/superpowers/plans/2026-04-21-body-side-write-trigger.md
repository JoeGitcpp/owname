# Body-Side Write Trigger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the first live body-side write trigger so that `action_output` substrate emits real
Begin/Adjust/Decay/Stop events driven by existing viewport signals (`last_avg_mse` from the
high-level predictor), and `action_consequence` windows are formed and auto-paired from those
same signals — making both substrates live rather than permanently empty in every body run.

**Architecture:** Two free functions in a new `src/probe.rs` library module:
`tick_probe_output` (reads `vp.read_hl_state().1 = last_avg_mse`, emits write events) and
`tick_consequence_ingress` (accumulates ingress changes, closes quiescent windows, auto-pairs).
Both are called from the existing serial global orchestrator in `body.rs` after
`refresh_global_reasoning_history` and before `write_snapshot`. The signal threshold and
magnitude scale are labeled engineering approximations — not fixed theoretical bounds.

**Tech Stack:** Rust (`std`, `RwLock`, `AtomicU64`), existing `Viewport` /
`action_output` / `action_consequence` / `highlevel` APIs, Cargo integration tests.

---

## Audit Summary — Current State

### What Already Exists (形式完备)

Both substrates are formally complete at the static / passive level:

| Layer | Status |
|---|---|
| `action_output` substrate (`src/action_output.rs`) | All 9 minimal contracts satisfied |
| `action_consequence` substrate (`src/action_consequence.rs`) | Window lifecycle, quiescence, temporal pairing all in |
| Viewport integration (`pub action_output`, `pub action_consequence`) | Mounted as `RwLock<T>` first-class fields |
| Snapshot serialization | SNAPSHOT_VERSION=28, both subtrees serialized |
| Observer rendering (`out:` / `acq:`) | No human semantics; numeric/time only |
| Tests | 7 action_output tests + 33 action_consequence tests |

### What Is Missing (真实缺口)

**Gap 1 — body.rs issues no Begin events.**

`run_global_refresh_cycle` calls: `refresh_global_attention` → `refresh_global_relations` →
`refresh_global_relation_predictor` → `refresh_global_reasoning` →
`refresh_global_reasoning_history` → `write_snapshot`.

None of these paths call `apply_action_output_event`. In every live run:

```
action_output.n_channels == 0
action_output.write_epoch_history is empty
```

**Gap 2 — consequence ingress accumulation is never triggered.**

`accumulate_ingress_change` and `tick_ingress_accumulation` exist as Viewport APIs but are
never called from any adapter or ticker. In every live run:

```
action_consequence.n_windows == 0
action_consequence.n_pairs == 0
```

**Gap 3 — temporal auto-pairing has no caller.**

`try_auto_pair_latest_epoch_to_window` is fully implemented but is never called from
the body loop.

**Gap 4 — trigger condition logic is absent.**

No code path reads err/precision/attention_bias values and conditionally decides
"now it is appropriate to begin a probe." The probe laws spec (§2.2) requires this
decision to be based on structural conditions — not an external command.

### Minimum Legal Write Trigger Location

Based on the probe laws spec (§2.2, §2.3) and action-output-substrate spec (§4.1):

> The trigger must (a) live inside the body's continuously running loop,
> (b) derive from existing substrate signals without pre-writing world content,
> (c) use only body law / learning-loop-prerequisite / safety-boundary constraints,
> and (d) label all numeric approximations clearly as engineering knobs, not
> theoretical bounds.

**Chosen signal**: `last_avg_mse` from `vp.read_hl_state()`.

Rationale: it is a purely numeric substrate value that measures residual prediction
error across the high-level layer — a valid engineering approximation of
"unfilled predictive structure" without encoding any world-content target. It is
already updated by the 20 Hz HighLevelPredictor tick before the 30 Hz orchestrator
runs, so it is always current when the orchestrator fires.

**Chosen location**: the end of `run_global_refresh_cycle` in `body.rs`, after
`refresh_global_reasoning_history` and before `write_snapshot`. This keeps the serial
orchestrator pattern intact and ensures the snapshot captures the post-trigger state.

---

## Scope Boundary (This Plan)

**Included:**
- `tick_probe_output` — emit Begin/Adjust/Decay/Stop based on `last_avg_mse`
- `tick_consequence_ingress` — accumulate + close quiescent + auto-pair
- Three engineering-knob constants in `lib.rs`
- Wire both into the body orchestrator
- TDD contract tests

**Excluded (belongs to downstream specs):**
- Introducing named "probe module" / "initiative engine" / "curiosity core"
- Pre-writing target objects, result boundaries, or exploration ranges
- Imitation, self-other comparison, or fast-path formation
- Language binding or semantic annotation of probe events
- Any consequence meaning classification or result evaluation
- Specific actuator / hardware output (voice, motor, screen)

---

## File Structure

**Create:**
- `bionic_v0/src/probe.rs`
- `bionic_v0/tests/probe_tick.rs`

**Modify:**
- `bionic_v0/src/lib.rs` — add `pub mod probe;` + three engineering constants
- `bionic_v0/src/bin/body.rs` — two call sites in `run_global_refresh_cycle`

**Do Not Modify In This Plan:**
- Any adapter implementation
- `snapshot.rs` (no new snapshot fields required — both substrates already serialized)
- `observer.rs` (already renders `out:` and `acq:`)
- Language binding, imitation, or action-consequence learning logic

**Responsibilities:**
- `src/probe.rs`: two pure-viewport functions; no thread state; directly testable.
- `src/lib.rs`: three labeled engineering-knob constants.
- `src/bin/body.rs`: two call sites; `use bionic_v0::probe::{…}` import.
- `tests/probe_tick.rs`: TDD contract tests seeding `vp.hl` directly.

---

## Task 1: Add engineering constants and empty module stub

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Create (stub): `bionic_v0/src/probe.rs`

- [ ] **Step 1: Add three constants to `lib.rs`**

Append after the `PHOT_CONSEQUENCE_QUIESCENCE_US` constant block
(currently around line 152):

```rust
/// Engineering approximation: high-level last_avg_mse above this is used as
/// the body-side signal that unfilled predictive structure is present and a
/// write probe is warranted. NOT a fixed theoretical bound — this is a
/// substrate-level engineering knob that should be revised as exploration
/// history forms.
pub const PROBE_MSE_THRESHOLD: f64 = 0.01;

/// Engineering approximation: write magnitude = last_avg_mse × PROBE_MAG_SCALE,
/// clamped to [0, 1]. NOT a theoretical magnitude law — an initial substrate
/// shadow only.
pub const PROBE_MAG_SCALE: f64 = 0.5;

/// Engineering approximation: quiescence gap for the probe-path consequence
/// window. Open window whose last_update_t_us is this many µs behind now_t_us
/// is auto-closed. Same concept as PHOT_CONSEQUENCE_QUIESCENCE_US; separate
/// constant so probe and PHOT paths can be tuned independently.
/// NOT a fixed theoretical consequence boundary.
pub const PROBE_CONSEQUENCE_QUIESCENCE_US: u64 = 500_000; // 500 ms
```

- [ ] **Step 2: Add `pub mod probe;` to `lib.rs`**

Add after `pub mod action_consequence;`:

```rust
pub mod probe;
```

- [ ] **Step 3: Create empty `src/probe.rs` stub**

```rust
// probe.rs — Body-side write trigger substrate.
//
// tick_probe_output and tick_consequence_ingress are the minimum callers that
// make action_output and action_consequence live in a running body.
// Both functions use only existing Viewport substrate signals and carry no
// world content, target selection, or result semantics.

use std::sync::Arc;
use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase,
                            OutputChannelKind};
use crate::viewport::Viewport;
use crate::{PROBE_CONSEQUENCE_QUIESCENCE_US, PROBE_MAG_SCALE, PROBE_MSE_THRESHOLD};

/// Emit Begin/Adjust/Decay/Stop on the GenericWrite channel based on
/// `last_avg_mse`. All thresholds and scale factors are engineering
/// approximations, not theoretical bounds.
pub fn tick_probe_output(_vp: &Arc<Viewport>, _t: u64) {
    // stub — implementation in Task 3
}

/// Accumulate `last_avg_mse` into consequence windows, close quiescent
/// windows, and attempt temporal auto-pairing. All knobs are engineering
/// approximations, not theoretical bounds.
pub fn tick_consequence_ingress(_vp: &Arc<Viewport>, _t: u64) {
    // stub — implementation in Task 3
}
```

- [ ] **Step 4: Verify project compiles with stubs**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo build 2>&1 | tail -5
```

Expected: `Compiling bionic_v0` → `Finished` with no errors.

- [ ] **Step 5: Commit stubs**

```bash
git add bionic_v0/src/probe.rs bionic_v0/src/lib.rs
git commit -m "feat: add probe module stub and engineering constants"
```

---

## Task 2: Write failing tests

**Files:**
- Create: `bionic_v0/tests/probe_tick.rs`

- [ ] **Step 1: Create `tests/probe_tick.rs`**

```rust
// TDD tests for the body-side write trigger.
//
// Plan: docs/superpowers/plans/2026-04-21-body-side-write-trigger.md
//
// Run order:
//   1. Tests written first → all fail (RED, because stubs are no-ops)
//   2. Implementation lands → all pass (GREEN)

use std::sync::Arc;
use bionic_v0::action_output::ActionOutputPhase;
use bionic_v0::action_consequence::ConsequenceWindowPhase;
use bionic_v0::probe::{tick_consequence_ingress, tick_probe_output};
use bionic_v0::viewport::Viewport;
use bionic_v0::PROBE_MSE_THRESHOLD;

// ─── 1. High MSE triggers a Begin event ───────────────────────────────────────

#[test]
fn tick_probe_begins_when_mse_above_threshold() {
    let vp = Arc::new(Viewport::new());
    // Seed last_avg_mse above threshold directly.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;

    tick_probe_output(&vp, 1_000);

    let st = vp.read_action_output();
    assert_eq!(st.n_channels, 1, "Begin must create one channel");
    assert_eq!(st.channels[0].phase, ActionOutputPhase::Active,
        "Channel must be Active after Begin");
    assert!(st.channels[0].current_magnitude > 0.0,
        "Magnitude must be positive after Begin");
    assert_eq!(st.n_write_epochs, 1, "Begin must record one epoch in the ledger");
}

// ─── 2. Low MSE emits no Begin ────────────────────────────────────────────────

#[test]
fn tick_probe_no_begin_when_mse_below_threshold() {
    let vp = Arc::new(Viewport::new());
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 0.1;

    tick_probe_output(&vp, 1_000);

    let st = vp.read_action_output();
    assert_eq!(st.n_channels, 0, "No Begin must occur when MSE is below threshold");
}

// ─── 3. Active channel with falling MSE enters Decay ─────────────────────────

#[test]
fn tick_probe_decays_active_channel_when_mse_drops() {
    let vp = Arc::new(Viewport::new());

    // Begin with high MSE.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;
    tick_probe_output(&vp, 1_000);
    assert_eq!(vp.read_action_output().channels[0].phase, ActionOutputPhase::Active);

    // Drop MSE well below threshold.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 0.1;
    tick_probe_output(&vp, 2_000);

    let st = vp.read_action_output();
    let phase = st.channels[0].phase;
    assert!(
        phase == ActionOutputPhase::Decaying || phase == ActionOutputPhase::Idle,
        "Phase must be Decaying or Idle after MSE drop; got {:?}", phase
    );
}

// ─── 4. Consequence ingress creates a window when MSE is high ─────────────────

#[test]
fn tick_consequence_ingress_opens_window_when_mse_above_threshold() {
    let vp = Arc::new(Viewport::new());
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;

    tick_consequence_ingress(&vp, 2_000);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 1, "First ingress must open a consequence window");
    assert_eq!(st.windows[0].phase, ConsequenceWindowPhase::Open,
        "Window must be Open immediately after ingress");
}

// ─── 5. No consequence window when MSE is low ────────────────────────────────

#[test]
fn tick_consequence_ingress_no_window_when_mse_below_threshold() {
    let vp = Arc::new(Viewport::new());
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 0.1;

    tick_consequence_ingress(&vp, 2_000);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 0, "No window must be opened when MSE is below threshold");
}

// ─── 6. Quiescence closes an Open window ──────────────────────────────────────

#[test]
fn tick_consequence_ingress_closes_quiescent_window() {
    use bionic_v0::PROBE_CONSEQUENCE_QUIESCENCE_US;
    let vp = Arc::new(Viewport::new());
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;

    // Open a window.
    tick_consequence_ingress(&vp, 1_000);
    assert_eq!(vp.read_action_consequence().windows[0].phase, ConsequenceWindowPhase::Open);

    // Drop MSE so no further accumulation happens; advance time past quiescence.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 0.1;
    let far_future = 1_000 + PROBE_CONSEQUENCE_QUIESCENCE_US + 1;
    tick_consequence_ingress(&vp, far_future);

    let st = vp.read_action_consequence();
    assert_eq!(st.windows[0].phase, ConsequenceWindowPhase::Closed,
        "Window must be Closed after quiescence period");
}

// ─── 7. Auto-pair links a probe epoch to a consequence window ─────────────────

#[test]
fn tick_consequence_ingress_auto_pairs_probe_and_window() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Arc::new(Viewport::new());

    // Fire a probe at t=1_000_000.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;
    tick_probe_output(&vp, 1_000_000);
    let n_epochs = vp.read_action_output().n_write_epochs;
    assert_eq!(n_epochs, 1, "Probe must have recorded one epoch");

    // Consequence window arrives well within the engineering time window.
    let win_t: u64 = 1_500_000;
    assert!(win_t - 1_000_000 <= TEMPORAL_PAIR_MAX_DELAY_US,
        "Test premise: window is within TEMPORAL_PAIR_MAX_DELAY_US");
    tick_consequence_ingress(&vp, win_t);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "Auto-pair must form one pair");
    assert_eq!(st.pairs[0].write_seq, 1, "Pair must reference the probe's write_seq");
}

// ─── 8. No phantom pair when no probe epoch was emitted ──────────────────────

#[test]
fn tick_consequence_ingress_no_phantom_pair_without_probe() {
    let vp = Arc::new(Viewport::new());

    // Consequence window — but no prior probe epoch.
    vp.hl.write().unwrap().last_avg_mse = PROBE_MSE_THRESHOLD * 5.0;
    tick_consequence_ingress(&vp, 2_000_000);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "No pair must form when no probe epoch exists");
}
```

- [ ] **Step 2: Run tests — expect RED (all assertions fail because stubs are no-ops)**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test probe_tick 2>&1 | tail -20
```

Expected: multiple `FAILED` — confirm all 8 tests fail before proceeding.

- [ ] **Step 3: Commit failing tests**

```bash
git add bionic_v0/tests/probe_tick.rs
git commit -m "test: add failing TDD tests for body-side probe tick"
```

---

## Task 3: Implement `probe.rs`

**Files:**
- Modify: `bionic_v0/src/probe.rs`

Replace the stubs with the real implementation:

- [ ] **Step 1: Write `tick_probe_output`**

```rust
pub fn tick_probe_output(vp: &Arc<Viewport>, t: u64) {
    // last_avg_mse is the engineering proxy for "unfilled predictive structure".
    // Threshold and scale are substrate-level approximations, not theoretical bounds.
    let (_, mse, _, _, _) = vp.read_hl_state();
    let st = vp.read_action_output();
    let phase = st.channels.first().map(|c| c.phase).unwrap_or(ActionOutputPhase::Idle);
    let mag = (mse * PROBE_MAG_SCALE).clamp(0.0, 1.0);

    match phase {
        ActionOutputPhase::Idle => {
            if mse > PROBE_MSE_THRESHOLD {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: mag,
                    event_t_us: t,
                });
            }
        }
        ActionOutputPhase::Active => {
            if mse > PROBE_MSE_THRESHOLD * 0.5 {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: mag,
                    event_t_us: t,
                });
            } else {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Decay,
                    magnitude: 0.1,
                    event_t_us: t,
                });
            }
        }
        ActionOutputPhase::Decaying => {
            if mse > PROBE_MSE_THRESHOLD {
                // Residue resurged — resume active.
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: mag,
                    event_t_us: t,
                });
            } else {
                let cur_mag = st.channels.first().map(|c| c.current_magnitude).unwrap_or(0.0);
                if cur_mag < 0.01 {
                    vp.apply_action_output_event(ActionOutputEvent {
                        channel_kind: OutputChannelKind::GenericWrite,
                        feedback: ActionOutputFeedback::Stop,
                        magnitude: 0.0,
                        event_t_us: t,
                    });
                } else {
                    vp.apply_action_output_event(ActionOutputEvent {
                        channel_kind: OutputChannelKind::GenericWrite,
                        feedback: ActionOutputFeedback::Decay,
                        magnitude: 0.1,
                        event_t_us: t,
                    });
                }
            }
        }
    }
}
```

- [ ] **Step 2: Write `tick_consequence_ingress`**

```rust
pub fn tick_consequence_ingress(vp: &Arc<Viewport>, t: u64) {
    // Accumulate ingress: use last_avg_mse as a numeric proxy for "observable
    // change in the world". Threshold and quiescence_us are engineering knobs.
    let (_, mse, _, _, _) = vp.read_hl_state();
    if mse > PROBE_MSE_THRESHOLD {
        vp.accumulate_ingress_change(t, mse.min(1.0));
    }
    // Close windows that have been quiescent — engineering time approximation.
    vp.tick_ingress_accumulation(t, PROBE_CONSEQUENCE_QUIESCENCE_US);
    // Attempt temporal auto-pairing: substrate-time only, no meaning inference.
    vp.try_auto_pair_latest_epoch_to_window(t);
}
```

- [ ] **Step 3: Replace the full `src/probe.rs` content**

Final file (combine Step 1 and Step 2 with header):

```rust
// probe.rs — Body-side write trigger substrate.
//
// Implements the minimum caller chain that makes action_output and
// action_consequence live in a running body.
//
// Both functions use only existing Viewport substrate signals and carry no
// world content, target selection, result classification, or meaning semantics.
// All numeric thresholds and scale factors are engineering approximations
// of the abstract laws — they are substrate knobs, not theoretical definitions.

use std::sync::Arc;
use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase,
                            OutputChannelKind};
use crate::viewport::Viewport;
use crate::{PROBE_CONSEQUENCE_QUIESCENCE_US, PROBE_MAG_SCALE, PROBE_MSE_THRESHOLD};

/// Emit Begin/Adjust/Decay/Stop on the GenericWrite channel based on
/// `last_avg_mse` from the high-level predictor layer.
///
/// `PROBE_MSE_THRESHOLD` and `PROBE_MAG_SCALE` are engineering approximations
/// of "unfilled predictive structure warrants a probe" — not fixed theoretical
/// bounds. They should be revised as exploration history accumulates.
pub fn tick_probe_output(vp: &Arc<Viewport>, t: u64) {
    let (_, mse, _, _, _) = vp.read_hl_state();
    let st = vp.read_action_output();
    let phase = st.channels.first().map(|c| c.phase).unwrap_or(ActionOutputPhase::Idle);
    let mag = (mse * PROBE_MAG_SCALE).clamp(0.0, 1.0);

    match phase {
        ActionOutputPhase::Idle => {
            if mse > PROBE_MSE_THRESHOLD {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: mag,
                    event_t_us: t,
                });
            }
        }
        ActionOutputPhase::Active => {
            if mse > PROBE_MSE_THRESHOLD * 0.5 {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: mag,
                    event_t_us: t,
                });
            } else {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Decay,
                    magnitude: 0.1,
                    event_t_us: t,
                });
            }
        }
        ActionOutputPhase::Decaying => {
            if mse > PROBE_MSE_THRESHOLD {
                vp.apply_action_output_event(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: mag,
                    event_t_us: t,
                });
            } else {
                let cur_mag = st.channels.first().map(|c| c.current_magnitude).unwrap_or(0.0);
                if cur_mag < 0.01 {
                    vp.apply_action_output_event(ActionOutputEvent {
                        channel_kind: OutputChannelKind::GenericWrite,
                        feedback: ActionOutputFeedback::Stop,
                        magnitude: 0.0,
                        event_t_us: t,
                    });
                } else {
                    vp.apply_action_output_event(ActionOutputEvent {
                        channel_kind: OutputChannelKind::GenericWrite,
                        feedback: ActionOutputFeedback::Decay,
                        magnitude: 0.1,
                        event_t_us: t,
                    });
                }
            }
        }
    }
}

/// Accumulate `last_avg_mse` into consequence windows, close quiescent
/// windows, and attempt temporal auto-pairing.
///
/// `PROBE_MSE_THRESHOLD`, `PROBE_CONSEQUENCE_QUIESCENCE_US`, and
/// `TEMPORAL_PAIR_MAX_DELAY_US` are engineering approximations — not fixed
/// theoretical consequence boundaries. They should be revised as exploration
/// history accumulates.
pub fn tick_consequence_ingress(vp: &Arc<Viewport>, t: u64) {
    let (_, mse, _, _, _) = vp.read_hl_state();
    if mse > PROBE_MSE_THRESHOLD {
        vp.accumulate_ingress_change(t, mse.min(1.0));
    }
    vp.tick_ingress_accumulation(t, PROBE_CONSEQUENCE_QUIESCENCE_US);
    vp.try_auto_pair_latest_epoch_to_window(t);
}
```

- [ ] **Step 4: Run tests — expect GREEN**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test probe_tick 2>&1 | tail -20
```

Expected: `8 passed; 0 failed`.

- [ ] **Step 5: Run full test suite — no regressions**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -10
```

Expected: all pre-existing tests still pass.

- [ ] **Step 6: Commit implementation**

```bash
git add bionic_v0/src/probe.rs bionic_v0/src/lib.rs
git commit -m "feat: implement tick_probe_output and tick_consequence_ingress"
```

---

## Task 4: Wire into `body.rs`

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: Add import at top of `body.rs`**

Add after existing `use bionic_v0::` imports:

```rust
use bionic_v0::probe::{tick_consequence_ingress, tick_probe_output};
```

- [ ] **Step 2: Extend `run_global_refresh_cycle`**

Current:
```rust
fn run_global_refresh_cycle(vp: &Arc<Viewport>, paths: &Arc<SnapshotPaths>, t: u64) {
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    if write_snapshot(vp, paths).is_err() {
        vp.mark_snapshot_fail();
    }
}
```

Replace with:
```rust
fn run_global_refresh_cycle(vp: &Arc<Viewport>, paths: &Arc<SnapshotPaths>, t: u64) {
    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);
    // Body-side write trigger: emit probe event based on hl substrate state.
    // Threshold is an engineering approximation — not a fixed theoretical bound.
    tick_probe_output(vp, t);
    // Consequence ingress: accumulate hl residue, close quiescent windows,
    // attempt temporal auto-pairing. All knobs are engineering approximations.
    tick_consequence_ingress(vp, t);
    if write_snapshot(vp, paths).is_err() {
        vp.mark_snapshot_fail();
    }
}
```

- [ ] **Step 3: Build body binary**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo build --bin body 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Run full test suite — no regressions**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 5: Commit wiring**

```bash
git add bionic_v0/src/bin/body.rs
git commit -m "feat: wire probe tick and consequence ingress into body orchestrator"
```

---

## Verification Checklist

After all tasks complete, confirm:

- [ ] `cargo test` passes with zero failures
- [ ] `cargo build --bin body` succeeds
- [ ] `cargo build --bin observer` succeeds
- [ ] In a short body run (`cargo run --bin body -- runtime 5`), snapshot at
      `runtime/snapshot.bin` has `action_output.n_channels >= 1` after the
      HighLevelPredictor warms up (≥ 2 ticks / 100 ms)
- [ ] Observer output shows `out:n1|ch0[...]` with non-empty channel state
- [ ] No `eprintln!` / `println!` introduced in body-side code (cleanup spec §4.5)
- [ ] No new fields named `target`, `word`, `concept`, `semantic`, `motivation`,
      `curiosity`, `initiative` in any modified file

---

## Execution Handoff

Plan saved to `docs/superpowers/plans/2026-04-21-body-side-write-trigger.md`.

**Two execution options:**

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
Use **superpowers:subagent-driven-development**.

**2. Inline Execution** — execute tasks in this session using executing-plans.
Use **superpowers:executing-plans**.
