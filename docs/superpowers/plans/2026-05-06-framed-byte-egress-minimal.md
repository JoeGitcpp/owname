# Framed Byte Egress Minimal Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the smallest admissible body-side framed-byte egress loop: framed bytes become an `action_output` write epoch, then ordinary return/correction adjusts numeric substrate traces.

**Architecture:** Add a `framed_byte_egress` body substrate that records only numeric/time/byte-shape traces. It writes through `action_output` using a dedicated channel kind, receives returns through `action_consequence` windows, and exposes observer-visible snapshot state without tool/controller, meaning, discrete-language, identity, or result ontology.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `action_output`, `action_consequence`, `snapshot`, and observer text renderer.

---

## File Structure

- Create: `bionic_v0/src/framed_byte_egress.rs`
  - Owns `FramedByteEgressState`, frame traces, return/correction traces, and numeric confidence update rules.
- Modify: `bionic_v0/src/lib.rs`
  - Export the new module, bump snapshot version, and document the version addition.
- Modify: `bionic_v0/src/action_output.rs`
  - Add `OutputChannelKind::FramedByteEgress` after `GenericWrite`.
- Modify: `bionic_v0/src/action_consequence.rs`
  - Add `ConsequenceModalityKind::FramedByteReturn` after `GenericIngress`.
- Modify: `bionic_v0/src/viewport.rs`
  - Add `framed_byte_egress: RwLock<FramedByteEgressState>` and body-side APIs.
- Modify: `bionic_v0/src/snapshot.rs`
  - Serialize/deserialize snapshot-visible framed-byte egress traces.
- Modify: `bionic_v0/src/bin/observer.rs`
  - Render numeric framed-byte egress traces for research observation only.
- Modify/Rename: the misleading 2026-05-06 computer-I/O draft
  - Replace the misleading controller direction with a corrected digital-disturbance egress boundary.
- Test: `bionic_v0/tests/framed_byte_egress.rs`
  - TDD coverage for write epoch alignment, return pairing, correction update, snapshot visibility, and boundary hygiene.

## Chunk 1: Plan And Boundary Spec

### Task 1: Save This Plan

**Files:**
- Create: `docs/superpowers/plans/2026-05-06-framed-byte-egress-minimal.md`

- [ ] **Step 1: Write this implementation plan**

- [ ] **Step 2: Confirm the plan names only framed-byte/digital-disturbance egress**

Review the header, goal, architecture, file structure, and tasks. Expected: the plan describes a body-side disturbance channel, not a controller path.

## Chunk 2: RED Tests

### Task 2: Add Failing Framed-Byte Egress Tests

**Files:**
- Create: `bionic_v0/tests/framed_byte_egress.rs`

- [ ] **Step 1: Write the failing tests**

Tests to add:

```rust
#[test]
fn framed_byte_begin_writes_frame_and_action_epoch() {
    let vp = Viewport::new();
    let frame = vp.begin_framed_byte_egress(1_000_000, 7, &[0, 255, 10], 0.4);
    let st = vp.read_framed_byte_egress();
    let out = vp.read_action_output();

    assert_eq!(st.n_frames, 1);
    assert_eq!(frame.byte_len, 3);
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.write_epoch_history[0].channel_kind, OutputChannelKind::FramedByteEgress);
    assert_eq!(out.write_epoch_history[0].write_seq, frame.write_seq);
}

#[test]
fn framed_byte_return_forms_numeric_consequence_pair() {
    let vp = Viewport::new();
    let frame = vp.begin_framed_byte_egress(1_000_000, 7, &[1, 2, 3], 0.4);
    let window_id = vp.observe_framed_byte_return(1_020_000, 7, &[1, 2, 4], 0.3, 0.2);
    let ac = vp.read_action_consequence();

    assert_eq!(ac.n_windows, 1);
    assert_eq!(ac.windows[0].window_id, window_id);
    assert_eq!(ac.windows[0].modality_kind, ConsequenceModalityKind::FramedByteReturn);
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].write_seq, frame.write_seq);
    assert_eq!(ac.pairs[0].window_id, window_id);
}
```

Also add tests for numeric correction, snapshot visibility, and boundary hygiene.

- [ ] **Step 2: Run tests to verify RED**

Run: `cargo test --test framed_byte_egress`
Expected: fail because `framed_byte_egress` APIs and enum variants do not exist.

## Chunk 3: GREEN Minimal Substrate

### Task 3: Add Minimal Framed-Byte Egress State And APIs

**Files:**
- Create: `bionic_v0/src/framed_byte_egress.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/action_output.rs`
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Add enum variants**

Add:

```rust
OutputChannelKind::FramedByteEgress
ConsequenceModalityKind::FramedByteReturn
```

Both variants are numeric channel/modality codes only.

- [ ] **Step 2: Implement minimal state**

Add `FramedByteEgressState` with phase, active stream id, current write seq, emitted frame traces, return traces, output pressure, projection confidence, correction deviation, and timestamps.

- [ ] **Step 3: Implement viewport APIs**

Add:

```rust
read_framed_byte_egress()
begin_framed_byte_egress(now_t_us, stream_id, bytes, pressure)
adjust_framed_byte_egress(now_t_us, bytes, pressure)
stop_framed_byte_egress(now_t_us)
observe_framed_byte_return(now_t_us, stream_id, bytes, magnitude, deviation)
```

Each nonzero begin/adjust/stop must call `apply_action_output_event`.

- [ ] **Step 4: Run RED tests to verify GREEN**

Run: `cargo test --test framed_byte_egress`
Expected: pass.

## Chunk 4: Snapshot And Observer Trace

### Task 4: Expose Observer-Visible Numeric Trace

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/framed_byte_egress.rs`

- [ ] **Step 1: Write snapshot test first**

Test that `write_snapshot` / `read_snapshot` preserves framed-byte egress counts, phase, current write seq, last frame metadata, projection confidence, and correction deviation.

- [ ] **Step 2: Run snapshot test to verify RED**

Run: `cargo test --test framed_byte_egress snapshot_roundtrip_preserves_framed_byte_egress_trace`
Expected: fail until snapshot serialization exists.

- [ ] **Step 3: Implement snapshot serialization**

Bump `SNAPSHOT_VERSION` and add write/read helpers. Store numeric metadata and byte counts/checksums; do not store result, source, or meaning fields.

- [ ] **Step 4: Render observer trace**

Add a compact observer column such as:

```text
fbe:ph0|ws1|nf1|nr1|pc0.530|dev0.200
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test framed_byte_egress`
Expected: pass.

## Chunk 5: Correct The Misleading Spec

### Task 5: Replace Code-Oriented Draft With Digital-Disturbance Boundary

**Files:**
- Rename/Modify: the misleading 2026-05-06 computer-I/O draft

- [ ] **Step 1: Write/rename corrected spec**

Rename to:

```text
docs/superpowers/specs/2026-05-06-digital-disturbance-egress-substrate-design.md
```

The corrected spec must only admit sandbox framed-byte disturbance egress and ordinary return/correction. It must explicitly reject controller carriers, request/response shape, and meaning-level fields without listing future carrier technologies as a roadmap.

- [ ] **Step 2: Run hygiene checks**

Run: `cargo test --test framed_byte_egress boundary_hygiene_rejects_controller_and_discrete_language_drift`
Expected: pass.

## Chunk 6: Full Verification

### Task 6: Verify The Whole Crate

**Files:**
- All touched files.

- [ ] **Step 1: Run focused test**

Run: `cargo test --test framed_byte_egress`
Expected: all tests pass.

- [ ] **Step 2: Run full test suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 3: Inspect git diff**

Run: `git diff --stat && git diff --check`
Expected: intended files only, no whitespace errors.
