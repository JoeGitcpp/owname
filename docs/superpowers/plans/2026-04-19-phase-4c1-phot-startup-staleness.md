# Phase 4C.1 PHOT Startup Staleness Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make PHOT ingress distinguish "camera opened but never delivered the first frame" from normal quiet frames, without changing claim/scoring or introducing any visual semantics.

**Architecture:** Keep real camera ingress numeric-only and viewport-centered. Extend the PHOT substrate so startup time, last-frame time, and provider health become first-class time facts; then drive `phot_health` from those facts so `FAULT`, `STALE`, `QUIET`, and `OK` are true substrate states instead of ad hoc adapter behavior.

**Tech Stack:** Rust 2021, existing `bionic_v0` viewport/adapter/snapshot architecture, `nokhwa` AVFoundation backend, `cargo test`, `cargo build --all-targets`

---

## File Map

**Modify:**
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/snapshot.rs`
- `bionic_v0/src/adapter.rs`
- `bionic_v0/src/bin/observer.rs`
- `bionic_v0/src/phot_camera.rs` (only if provider API needs explicit startup-ready semantics)
- `bionic_v0/tests/real_phot_adapter.rs`

**Do not modify in this phase unless blocked:**
- `bionic_v0/src/predictor.rs`
- `bionic_v0/src/highlevel.rs`
- `bionic_v0/src/vib.rs`
- `bionic_v0/src/bin/body.rs`

## Scope Guardrails

- This phase does **not** add a PHOT predictor.
- This phase does **not** add multi-event extraction.
- This phase does **not** change `claim_slot_for_event()` scoring or hard gates.
- This phase does **not** introduce any semantic labels, tracking IDs, or object classes.
- This phase does **not** add new hardware beyond the existing single macOS camera path.

## Chunk 1: Freeze PHOT Startup-Time Semantics

### Task 1: Make startup/no-frame timing a first-class PHOT substrate fact

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Write failing tests for startup staleness truth**

Add tests that cover:
- provider healthy, zero frames ever received, time past stale window => `STALE`
- provider healthy, zero frames ever received, time before stale window => not `STALE`
- provider received at least one frame, then no frames past stale window => `STALE`

Recommended assertions:

```rust
assert_eq!(vp.read_phot_health(), HealthState::Stale);
assert_eq!(snap.phot_health, HEALTH_STALE);
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
cargo test --test real_phot_adapter startup
```

Expected: FAIL because successful-open/no-first-frame is not represented yet.

- [ ] **Step 3: Add PHOT ingress timing fields to viewport**

In `bionic_v0/src/viewport.rs`, add first-class numeric state for:
- `phot_started_t_us`
- `phot_last_frame_t_us`

Add small read/write helpers:
- `set_phot_started_t_us(...)`
- `read_phot_started_t_us()`
- `set_phot_last_frame_t_us(...)`
- `read_phot_last_frame_t_us()`

Rules:
- `phot_started_t_us` is set once when the provider is constructed successfully.
- `phot_last_frame_t_us` updates on each received frame.
- These are substrate facts, not observer-only debug values.

- [ ] **Step 4: Carry the timing fields through snapshot**

In `bionic_v0/src/snapshot.rs`:
- bump `SNAPSHOT_VERSION`
- serialize/deserialize `phot_started_t_us`
- serialize/deserialize `phot_last_frame_t_us`

- [ ] **Step 5: Expose the timing fields in observer without interpretation**

In `bionic_v0/src/bin/observer.rs`:
- print `phot_started_t_us` and `phot_last_frame_t_us` in compact form or as derived age columns
- do not recompute `phot_health`
- keep observer as renderer of snapshot truth only

- [ ] **Step 6: Re-run the targeted tests**

Run:

```bash
cargo test --test real_phot_adapter startup
```

Expected: still failing until adapter state machine is updated.

## Chunk 2: Derive STALE From Provider-Healthy + No-Frame Time

### Task 2: Update RealPhotAdapter state machine for initial no-frame staleness

**Files:**
- Modify: `bionic_v0/src/adapter.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Add a failing test for successful-open/no-first-frame -> STALE**

Use a fake healthy provider that returns `Ok(None)` repeatedly forever.

Required checks:
- before `PHOT_STALE_WINDOW_US`: not `STALE`
- after `PHOT_STALE_WINDOW_US`: `STALE`
- no slot claim occurs

- [ ] **Step 2: Run the targeted test**

Run:

```bash
cargo test --test real_phot_adapter no_first_frame
```

Expected: FAIL because current logic requires `last_frame_t_us > 0`.

- [ ] **Step 3: Add explicit startup-staleness logic**

In `bionic_v0/src/adapter.rs`:
- store or read `phot_started_t_us`
- when provider is healthy and returns `Ok(None)`:
  - if a frame has been seen, compare `now_us - phot_last_frame_t_us`
  - if no frame has been seen yet, compare `now_us - phot_started_t_us`
- if the appropriate duration exceeds `PHOT_STALE_WINDOW_US`, set `phot_health = STALE`

Rules:
- `FAULT` still wins on provider errors
- `QUIET` still means frames are arriving but motion is low
- `OK` still means frames are arriving and motion exceeds threshold
- `STALE` means provider path is healthy but frame delivery has stalled or never begun

- [ ] **Step 4: Keep claim/scoring untouched**

Verify that no code in:
- `claim_slot_for_event`
- scoring weights
- hard gates
- `EventEvidence`

has changed in this phase.

- [ ] **Step 5: Re-run the targeted tests**

Run:

```bash
cargo test --test real_phot_adapter no_first_frame
```

Expected: PASS

### Task 3: Add regression coverage for state transitions

**Files:**
- Modify: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Add state transition tests**

Cover:
- healthy open -> no frame -> `STALE`
- first idle frame arrives -> `QUIET`
- motion frame arrives -> `OK`
- subsequent no-frame timeout -> back to `STALE`
- runtime error after frames -> `FAULT`

- [ ] **Step 2: Run the real PHOT test file**

Run:

```bash
cargo test --test real_phot_adapter
```

Expected: PASS

## Chunk 3: Verification and Handoff

### Task 4: Verify the phase without changing architecture

**Files:**
- Modify: `bionic_v0/tests/real_phot_adapter.rs` (if an extra regression is needed)

- [ ] **Step 1: Run the full test suite**

Run:

```bash
cargo test
```

Expected: PASS, including existing claim/reuse and scoring suites.

- [ ] **Step 2: Run the full build**

Run:

```bash
cargo build --all-targets
```

Expected: PASS

- [ ] **Step 3: Manual smoke on macOS camera path**

Run in one terminal:

```bash
mkdir -p runtime
cargo run --bin body -- runtime 10
```

Run in a second terminal:

```bash
cargo run --bin observer -- runtime
```

Expected:
- camera denied/unavailable => `phot_h=FAULT`
- camera healthy but frozen/no frames => `phot_h=STALE`
- frames arriving but static scene => `phot_h=QUIET`
- frames arriving with motion => `phot_h=OK`

- [ ] **Step 4: Summarize the phase outcome**

Report:
- which files changed
- exact `STALE` truth rules
- which viewport/snapshot fields were added
- confirmation that claim/scoring stayed untouched
- what remains for `Phase 4C` (PHOT predictor)

