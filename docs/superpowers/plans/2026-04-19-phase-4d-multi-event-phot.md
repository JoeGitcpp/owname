# Phase 4D Multi-Event PHOT Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow one camera frame to yield multiple low-level PHOT events that are independently claimed into object slots, without adding semantics or changing viewport claim/scoring.

**Architecture:** Keep `CameraFrameProvider` unchanged: one provider tick still yields at most one frame. Move multi-event capability into PHOT extraction and adapter orchestration. `MotionExtractor` should segment one motion map into up to `N` numeric regions, sort them deterministically, and let `RealPhotAdapter` submit one `EventEvidence` per region to the existing viewport `claim_slot_for_event()` path. Slot claim, reuse, and scoring remain centralized in the viewport.

**Tech Stack:** Rust 2021, existing `bionic_v0` viewport/adapter/snapshot architecture, `nokhwa`/synthetic camera providers, `cargo test`, `cargo build --all-targets`

---

## File Map

**Modify:**
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/phot_camera.rs`
- `bionic_v0/src/adapter.rs`
- `bionic_v0/src/bin/observer.rs`
- `bionic_v0/tests/real_phot_adapter.rs`

**Optional only if needed for observability:**
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/snapshot.rs`

**Do not modify in this phase unless blocked:**
- `bionic_v0/src/predictor.rs`
- `bionic_v0/src/highlevel.rs`
- `bionic_v0/src/vib.rs`

## Scope Guardrails

- This phase does **not** change `CameraFrameProvider` to emit multiple frames or pre-decoded regions.
- This phase does **not** add semantic labels, classes, or tracking IDs.
- This phase does **not** change `claim_slot_for_event()` scoring or hard gates.
- This phase does **not** add cross-modal predictor fusion.
- This phase does **not** add multi-camera or depth.
- This phase does **not** add instinct or action output.

## Chunk 1: Multi-Region Extraction Contract

### Task 1: Replace single-event extraction with bounded multi-region extraction

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/phot_camera.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Write failing tests for multi-region extraction**

Add tests for:
- one frame containing two spatially separated moving blobs
- extractor returns two regions, not one merged centroid
- regions are returned in deterministic order
- number of returned regions is capped

Recommended assertions:

```rust
assert_eq!(regions.len(), 2);
assert!(regions[0].event_energy >= regions[1].event_energy);
assert!((regions[0].pos_x - regions[1].pos_x).abs() > 0.2);
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
cargo test --test real_phot_adapter multi_event_extraction
```

Expected: FAIL because `MotionExtractor` still returns one event.

- [ ] **Step 3: Add bounded multi-region extraction**

In `bionic_v0/src/lib.rs` add numeric-only constants such as:
- `PHOT_MAX_EVENTS_PER_FRAME`
- `PHOT_REGION_THRESHOLD`
- `PHOT_REGION_MIN_PIXELS`

In `bionic_v0/src/phot_camera.rs`:
- change the extractor API from single `Option<MotionExtract>` to `Vec<MotionExtract>`
- implement a minimal numeric segmentation strategy over the motion map
- sort returned regions deterministically, recommended:
  1. higher `event_energy` first
  2. tie-break on lower `pos_x`
  3. tie-break on lower `pos_y`
- cap to `PHOT_MAX_EVENTS_PER_FRAME`

Rules:
- still no semantics
- still one frame in, numeric regions out
- keep region outputs aligned with existing `phot_bands / pos / span / precision / energy / surprise`

- [ ] **Step 4: Re-run the targeted extraction tests**

Run:

```bash
cargo test --test real_phot_adapter multi_event_extraction
```

Expected: PASS

## Chunk 2: Adapter Multi-Claim Loop

### Task 2: Let one PHOT frame generate multiple slot claims

**Files:**
- Modify: `bionic_v0/src/adapter.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Add failing tests for one-frame-two-claims behavior**

Add tests that cover:
- two strong regions in one frame -> two slot claims
- two regions far apart -> two different slots
- one strong and one weak region -> weak region filtered out if below threshold

- [ ] **Step 2: Run the targeted tests**

Run:

```bash
cargo test --test real_phot_adapter multi_event_claim
```

Expected: FAIL because adapter still assumes one event per frame.

- [ ] **Step 3: Update `RealPhotAdapter` to iterate extracted regions**

In `bionic_v0/src/adapter.rs`:
- keep provider logic, health logic, and staleness logic unchanged
- compute frame-level ambient once from the full motion map / aggregate extraction output
- for each extracted region:
  - construct `EventEvidence`
  - call `claim_slot_for_event()`
  - if not dropped, call `update_slot_phot_with_predictor()`
  - write `slot.spatial`

Rules:
- one frame may emit `0..PHOT_MAX_EVENTS_PER_FRAME` claims
- claims are submitted in deterministic region order
- adapter still does not decide slot identity

- [ ] **Step 4: Preserve existing single-event behavior as a special case**

Verify that a one-region frame still behaves exactly like Phase 4C:
- same `phot_health`
- same claim path
- same slot-local predictor behavior

- [ ] **Step 5: Re-run the targeted adapter tests**

Run:

```bash
cargo test --test real_phot_adapter multi_event_claim
```

Expected: PASS

## Chunk 3: Synthetic Multi-Blob Coverage

### Task 3: Extend synthetic camera generation for multi-event frames

**Files:**
- Modify: `bionic_v0/src/phot_camera.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Add failing tests for multi-blob synthetic frames**

Add synthetic frame cases that encode:
- two blobs in one frame
- three blobs with one below threshold
- two blobs whose order would be unstable without tie-breaks

- [ ] **Step 2: Run the targeted tests**

Run:

```bash
cargo test --test real_phot_adapter synthetic_multi_blob
```

Expected: FAIL because the synthetic provider cannot yet create rich multi-region frames.

- [ ] **Step 3: Add multi-blob synthetic frame support**

In `bionic_v0/src/phot_camera.rs`:
- extend `SynthCamStep` with a multi-blob variant or equivalent
- preserve old single-blob helpers for backward compatibility
- keep deterministic frame generation for tests

- [ ] **Step 4: Re-run the targeted tests**

Run:

```bash
cargo test --test real_phot_adapter synthetic_multi_blob
```

Expected: PASS

## Chunk 4: Observer and Verification

### Task 4: Make multi-event PHOT observable without changing semantics

**Files:**
- Modify: `bionic_v0/src/bin/observer.rs`
- Optional modify: `bionic_v0/src/viewport.rs`
- Optional modify: `bionic_v0/src/snapshot.rs`
- Test: `bionic_v0/tests/real_phot_adapter.rs`

- [ ] **Step 1: Decide the minimum observability surface**

Preferred minimum:
- observer shows multiple active PHOT slots in the existing slot list
- no semantic grouping
- no observer-side inference

Optional if needed:
- add a tiny recent-claim debug ring in viewport/snapshot for same-tick visibility

- [ ] **Step 2: Add failing observer/snapshot tests if observability changes**

If adding recent-claim history, write failing tests for snapshot round-trip first.

- [ ] **Step 3: Update observer rendering only as needed**

Rules:
- observer must remain a pure renderer
- it may show multiple PHOT slots from a single frame, but never explain “same object” or “different object”

- [ ] **Step 4: Run the full suite**

Run:

```bash
cargo test
```

Expected: PASS, including old single-event and claim/regression suites.

- [ ] **Step 5: Run full build**

Run:

```bash
cargo build --all-targets
```

Expected: PASS

- [ ] **Step 6: Manual smoke**

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
- one moving object -> one PHOT slot
- two clearly separated moving objects -> two PHOT slots
- slot claim/debug architecture still works

- [ ] **Step 7: Summarize the phase**

Report:
- which files changed
- how many regions per frame are supported
- extraction ordering rule
- what stayed untouched in claim/scoring
- whether any observability structure was added
- what remains for later cross-modal or calibration work

