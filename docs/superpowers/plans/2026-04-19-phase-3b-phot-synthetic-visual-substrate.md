# Phase 3B PHOT Synthetic Visual Substrate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze health semantics into the viewport, then replace the PHOT no-op stub with a synthetic visual substrate that exercises `ambient.phot`, `slot.phot`, and `slot.spatial` without introducing a real camera or visual semantics.

**Architecture:** Keep the AI body as a continuous viewport-driven system. First move hearing health from observer-derived semantics into first-class viewport state. Then add a synthetic PHOT adapter that publishes low-level band/spatial curves through the existing registry, object slots, and snapshot path, so Phase 4 can swap the synthetic source for a real camera without changing the body loop or object substrate.

**Tech Stack:** Rust 2021, std only, existing `bionic_v0` viewport/adapter/snapshot architecture, `cargo test`, `cargo build --all-targets`

---

## File Map

**Create:**
- `bionic_v0/src/phot.rs`
- `bionic_v0/tests/health_state.rs`
- `bionic_v0/tests/phot_snapshot_roundtrip.rs`
- `bionic_v0/tests/synthetic_phot_adapter.rs`

**Modify:**
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/snapshot.rs`
- `bionic_v0/src/adapter.rs`
- `bionic_v0/src/bin/body.rs`
- `bionic_v0/src/bin/observer.rs`

**Do not modify in this phase unless blocked:**
- `bionic_v0/src/vib.rs`
- `bionic_v0/src/highlevel.rs`
- `bionic_v0/src/predictor.rs`

## Scope Guardrails

- This phase does **not** connect a real camera.
- This phase does **not** add visual semantics, labels, object detection, or tracking IDs beyond slot index and nomination sequence.
- This phase does **not** add cross-modal binding logic.
- This phase does **not** add instinct, action output, or language.
- This phase does **not** change body architecture away from `adapter -> viewport -> snapshot -> observer`.

## Chunk 1: Phase 3A.1 Semantic Freeze

### Task 1: Make health a first-class viewport state

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/health_state.rs`

- [ ] **Step 1: Add the failing test for health state transitions**

Create `bionic_v0/tests/health_state.rs` with coverage for:
- startup default
- hearing fault
- stale observation window
- fresh low-energy observation
- fresh active observation

Expected API shape:

```rust
use bionic_v0::viewport::{HealthState, Viewport};

#[test]
fn health_defaults_to_quiet() {
    let vp = Viewport::new();
    assert_eq!(vp.read_health_state(), HealthState::Quiet);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test health_state
```

Expected: compile or assertion failure because `HealthState` and `read_health_state()` do not exist yet.

- [ ] **Step 3: Add a numeric health enum to the viewport**

In `bionic_v0/src/lib.rs` add numeric constants or a small repr enum for:
- `HEALTH_FAULT`
- `HEALTH_QUIET`
- `HEALTH_OK`

In `bionic_v0/src/viewport.rs` add:
- a first-class health field
- `read_health_state()`
- `update_health_state(...)`

Rules:
- `FAULT` wins if hearing fault is set
- `QUIET` if observations are stale or energy is below quiet threshold
- `OK` if fresh observations exist and activity crosses the threshold

- [ ] **Step 4: Serialize health state through snapshot**

In `bionic_v0/src/snapshot.rs`:
- bump `SNAPSHOT_VERSION`
- write health state
- read health state into `SnapshotData`

- [ ] **Step 5: Make observer render viewport truth only**

In `bionic_v0/src/bin/observer.rs`:
- delete `health_label()`
- read `snap.health_state`
- render the numeric state as `FAULT/QUIET/OK` in one small display helper only
- do not recompute thresholds in observer

- [ ] **Step 6: Run the test to verify it passes**

Run:

```bash
cargo test --test health_state
```

Expected: PASS

### Task 2: Unify nomination semantics with quiet/ok semantics

**Files:**
- Modify: `bionic_v0/src/adapter.rs`
- Test: `bionic_v0/tests/health_state.rs`

- [ ] **Step 1: Add a failing test that nomination does not fire while health is quiet**

Add a test that feeds low-level noise-like PHOT/VIB energy and verifies:
- health stays `QUIET`
- `candidate_flag` stays false
- no slot allocation occurs

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test health_state nomination
```

Expected: FAIL because adapter thresholds are independent today.

- [ ] **Step 3: Move event thresholds behind shared substrate constants**

In `bionic_v0/src/lib.rs` add shared constants for:
- quiet window
- quiet energy threshold
- event nomination threshold

In `bionic_v0/src/adapter.rs`, make adapter event nomination derive from the same substrate rule family, not a disconnected magic number.

- [ ] **Step 4: Re-run the test**

Run:

```bash
cargo test --test health_state nomination
```

Expected: PASS

## Chunk 2: Synthetic PHOT Source

### Task 3: Add a synthetic visual substrate generator

**Files:**
- Create: `bionic_v0/src/phot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/synthetic_phot_adapter.rs`

- [ ] **Step 1: Write a failing test for a deterministic synthetic PHOT frame**

Create a test that expects a generator to produce, for a given timestamp:
- fixed-width PHOT band vector
- normalized spatial tuple
- deterministic phase progression

Recommended API:

```rust
use bionic_v0::phot::SyntheticPhotSource;

#[test]
fn synthetic_phot_is_deterministic() {
    let src = SyntheticPhotSource::new(1_000_000);
    let f1 = src.sample(1_100_000);
    let f2 = src.sample(1_100_000);
    assert_eq!(f1.phot_bands, f2.phot_bands);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test synthetic_phot_adapter synthetic_phot_is_deterministic
```

Expected: FAIL because `phot.rs` does not exist yet.

- [ ] **Step 3: Implement the synthetic PHOT source**

Create `bionic_v0/src/phot.rs` with:
- `PHOT_BANDS` constant, start with `4`
- `SyntheticPhotFrame`
- `SyntheticPhotSource`

Use simple deterministic patterns only:
- moving bright blob left-to-right
- vertical pulse bar
- periodic full-field brightness swell

Expose only low-level outputs:
- `phot_bands: [f64; PHOT_BANDS]`
- `pos_x`, `pos_y`
- `span_x`, `span_y`
- `spatial_precision`
- `event_energy`

Do not expose labels or object names.

- [ ] **Step 4: Export the module**

In `bionic_v0/src/lib.rs`:
- `pub mod phot;`
- `pub const PHOT_BANDS: usize = 4;`

- [ ] **Step 5: Re-run the deterministic source test**

Run:

```bash
cargo test --test synthetic_phot_adapter synthetic_phot_is_deterministic
```

Expected: PASS

## Chunk 3: Replace PHOT Stub With Synthetic PHOT Adapter

### Task 4: Upgrade the adapter path from no-op to synthetic PHOT writes

**Files:**
- Modify: `bionic_v0/src/adapter.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/synthetic_phot_adapter.rs`

- [ ] **Step 1: Add a failing integration test for PHOT ambient and slot writes**

Test that after a few synthetic ticks:
- `ambient.phot` changes from zero
- at least one slot gets a `phot` family
- that slot also gets `spatial`

Recommended validation target:

```rust
let vp = Arc::new(Viewport::new());
let mut adapter = SyntheticPhotAdapter::new(0);
for step in 0..10 {
    adapter.tick(&vp, 1_000_000 + step * 33_333);
}
let amb = vp.read_ambient();
assert!(amb.phot.event_energy > 0.0);
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test synthetic_phot_adapter phot_writes_viewport
```

Expected: FAIL because `PhotStubAdapter` does no writes.

- [ ] **Step 3: Implement `SyntheticPhotAdapter`**

In `bionic_v0/src/adapter.rs`:
- replace or supersede `PhotStubAdapter`
- own a `SyntheticPhotSource`
- on each tick:
  - generate one frame
  - write `ambient.phot`
  - nominate or reuse a slot via existing slot APIs
  - write `slot.phot`
  - write `slot.spatial`

Rules:
- never write VIB state
- never read/write semantic labels
- no direct coupling to VIB adapter

- [ ] **Step 4: Register the synthetic adapter in body**

In `bionic_v0/src/bin/body.rs`:
- register `SyntheticPhotAdapter`
- keep `body.rs` modality-agnostic
- do not add camera code

- [ ] **Step 5: Re-run the viewport write test**

Run:

```bash
cargo test --test synthetic_phot_adapter phot_writes_viewport
```

Expected: PASS

### Task 5: Make slot lifetime and spatial invalidation observable

**Files:**
- Modify: `bionic_v0/src/adapter.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/synthetic_phot_adapter.rs`

- [ ] **Step 1: Add a failing test for slot aging**

Test that when the synthetic event leaves or idles:
- `slot.phot` stops updating
- slot ages out under `SLOT_IDLE_US`
- `spatial.valid` can be toggled false before release if no estimate is emitted

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --test synthetic_phot_adapter phot_slot_ages_out
```

Expected: FAIL

- [ ] **Step 3: Implement minimal lifetime behavior**

Use existing `sweep_slots()` and `invalidate_slot_spatial()`:
- invalidate spatial on frames without a live geometry estimate
- allow slot release through normal idle timeout

- [ ] **Step 4: Re-run the aging test**

Run:

```bash
cargo test --test synthetic_phot_adapter phot_slot_ages_out
```

Expected: PASS

## Chunk 4: Snapshot and Observer Visibility

### Task 6: Expose PHOT and spatial state in snapshot round-trip

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Test: `bionic_v0/tests/phot_snapshot_roundtrip.rs`

- [ ] **Step 1: Add a failing round-trip test**

Write a synthetic slot with:
- `phot` family present
- `spatial` present
- `ambient.phot` non-zero

Serialize and read back, then assert structural equality.

- [ ] **Step 2: Run the test to verify it fails if the new fields are incomplete**

Run:

```bash
cargo test --test phot_snapshot_roundtrip
```

Expected: FAIL if any PHOT/spatial field is missing or misordered.

- [ ] **Step 3: Finish snapshot support**

Ensure `SnapshotData` includes everything Phase 3B needs:
- health state
- `ambient.phot`
- `slot.phot`
- `slot.spatial`

- [ ] **Step 4: Re-run the round-trip test**

Run:

```bash
cargo test --test phot_snapshot_roundtrip
```

Expected: PASS

### Task 7: Make observer show PHOT substrate, not just VIB

**Files:**
- Modify: `bionic_v0/src/bin/observer.rs`

- [ ] **Step 1: Update the observer header**

Add columns for:
- viewport health state
- `amb_phot`
- per-slot `phot` presence and band values
- per-slot spatial summary

- [ ] **Step 2: Update row rendering**

For each active slot, print:
- slot id
- modality bitmap
- age
- `phot[n_bands]` summary
- `spatial(valid,pos_x,pos_y,span_x,span_y,precision)`

Keep this as rendering only. No thresholding or semantic inference.

- [ ] **Step 3: Run a manual smoke check**

Run:

```bash
cargo build --all-targets
./target/debug/body runtime 5 &
./target/debug/observer runtime 5 40
```

Expected:
- `amb_phot` is no longer zero forever
- at least one slot shows `phot`
- at least one slot shows spatial values

## Chunk 5: Final Verification and Handoff

### Task 8: Run the full verification set

**Files:**
- No new files

- [ ] **Step 1: Run targeted tests**

Run:

```bash
cargo test --test health_state
cargo test --test synthetic_phot_adapter
cargo test --test phot_snapshot_roundtrip
```

Expected: all PASS

- [ ] **Step 2: Run full suite**

Run:

```bash
cargo test
cargo build --all-targets
```

Expected: PASS

- [ ] **Step 3: Run manual runtime verification**

Run:

```bash
./target/debug/body runtime 8 &
./target/debug/observer runtime 5 60
```

Verify:
- health comes from snapshot state, not observer recomputation
- `ambient.phot` changes over time
- slot creation and release occur
- `slot.phot` and `slot.spatial` both appear
- no camera dependency exists

- [ ] **Step 4: Record what is still intentionally missing**

Document in handoff notes:
- no real camera
- no PHOT predictors yet unless explicitly added later
- no cross-modal slot reuse
- no semantic visual labels

## Exit Criteria

Phase 3B is complete when all are true:
- Health state is first-class substrate state and observer only renders it.
- `SyntheticPhotAdapter` replaces the no-op PHOT path.
- `ambient.phot` is active and snapshot-visible.
- At least one slot can carry `phot` and `spatial` together.
- Slot aging and spatial invalidation work without special observer logic.
- No camera, OpenCV, AVFoundation, or visual semantics are introduced.

## Recommended Next Phase

After this plan, move to:

`Phase 3C: Cross-modal slot reuse with synthetic VIB + synthetic PHOT correlation`

Do **not** jump to a real camera before Phase 3B passes, because that would mix:
- substrate bugs
- adapter bugs
- observer visibility gaps
- hardware and permission failures

Plan complete and saved to `docs/superpowers/plans/2026-04-19-phase-3b-phot-synthetic-visual-substrate.md`. Ready to execute?
