# v0 Life-Closure Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining v0 substrate gaps without introducing request/response control, semantic gates, concept IDs, source identity truth, owner fields, or body-side human-readable output.

**Architecture:** Add small numeric/observer-side contracts around the existing `viewport` substrate. Body-side additions remain time/curve/counter traces; observer-side additions may summarize evidence but never feed back into body state.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `snapshot`, `observer`, and Cargo integration tests.

---

## Boundary Rules

High-risk words in this plan are external review terms or engineering indices only. No task may introduce body-side ontology fields for semantic meaning, concept identity, token identity, source ownership, owner, personality, or ethics judgment.

All behavior changes must follow TDD:

1. Write a failing integration or unit test.
2. Run the targeted command and verify the expected failure.
3. Implement the smallest body/observer code needed.
4. Run targeted tests and then the full crate test command.

Full verification command:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

## File Map

- Modify: `bionic_v0/src/action_consequence.rs`
  Add monotonic pair sequence support and producer hooks for action-consequence route/bias samples.
- Modify: `bionic_v0/src/evidence_tension.rs`
  Keep evidence closure consumption ordered by monotonic pair sequence.
- Modify: `bionic_v0/src/viewport.rs`
  Store `action_consequence_pair_seq`, expose live evidence and producer methods.
- Modify: `bionic_v0/src/snapshot.rs`
  Roundtrip any new numeric fields that must be observer-visible.
- Create: `bionic_v0/src/live_io_evidence.rs`
  Observer-side summary over `SnapshotData`; no body writeback.
- Create: `bionic_v0/src/framed_byte_carrier.rs`
  Sandbox-only framed byte carrier harness for closed-loop tests.
- Modify: `bionic_v0/src/framed_byte_egress.rs`
  Route return/correction evidence into code-native learned bias and adaptive route samples.
- Modify: `bionic_v0/src/language_binding.rs`
  Preserve binding-event learning unit while exposing admitted producer pressure.
- Modify: `bionic_v0/src/bin/body.rs`
  Tick only producer maintenance paths that are body-side numeric and already admitted.
- Modify: `bionic_v0/src/lib.rs`
  Export new modules and snapshot version constants if needed.
- Test: `bionic_v0/tests/life_closure_hardening.rs`
  Cross-substrate TDD tests for pair sequence, live I/O evidence, carrier loop, and producers.

## Task 1: Monotonic Pair Sequence

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/evidence_tension.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Test: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing tests**

Add tests proving each paired action consequence receives a monotonic `pair_seq`, that evidence tension consumes by `pair_seq`, and that old history eviction cannot cause duplicate closure consumption.

```rust
#[test]
fn action_consequence_pairs_have_monotonic_pair_sequence() {
    let vp = Viewport::new();
    let first = make_paired_window(&vp, 1_000, 1_400, 0.8);
    let second = make_paired_window(&vp, 2_000, 2_500, 0.7);
    let st = vp.read_action_consequence();
    let a = st.pairs.iter().find(|p| p.window_id == first).unwrap();
    let b = st.pairs.iter().find(|p| p.window_id == second).unwrap();
    assert!(a.pair_seq > 0);
    assert_eq!(b.pair_seq, a.pair_seq + 1);
}

#[test]
fn evidence_tension_consumes_pair_closure_by_pair_sequence_once() {
    let vp = Viewport::new();
    vp.tick_evidence_tension(tense_features(), 900);
    make_paired_window(&vp, 1_000, 1_400, 0.9);
    vp.tick_evidence_tension_from_pairs(1_500);
    let after_first = vp.read_evidence_tension();
    vp.tick_evidence_tension_from_pairs(1_600);
    let after_second = vp.read_evidence_tension();
    assert_eq!(after_second.last_pair_cursor, after_first.last_pair_cursor);
    assert_eq!(after_second.unresolved_pressure, after_first.unresolved_pressure);
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test life_closure_hardening pair_sequence -- --nocapture
```

Expected: fail because `ActionConsequencePair` has no `pair_seq`.

- [ ] **Step 3: Implement minimal code**

Add `pair_seq: u64` to `ActionConsequencePair`, add `action_consequence_pair_seq: AtomicU64` to `Viewport`, assign it during every successful pair creation, snapshot it, and make `tick_evidence_tension_from_pairs` use max consumed `pair_seq`.

- [ ] **Step 4: Verify targeted tests pass**

Run the same targeted command. Expected: all `pair_sequence` tests pass.

## Task 2: Observer-Side Live I/O Evidence Summary

**Files:**
- Create: `bionic_v0/src/live_io_evidence.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing tests**

Add tests proving the evidence summary is computed from snapshot facts and cannot write to the body.

```rust
#[test]
fn live_io_evidence_summary_reaches_pairing_level_from_snapshot_only() {
    let vp = Viewport::new();
    vp.apply_vocal_write_event(vocal_begin(1_000), 0.45, 0.55);
    let window_id = make_paired_window(&vp, 1_100, 1_300, 0.85);
    let snap = snapshot_from_viewport(&vp);
    let summary = summarize_live_io_evidence(&snap);
    assert!(summary.body_write_out_exists);
    assert!(summary.write_epoch_exists);
    assert!(summary.consequence_return_exists);
    assert!(summary.temporal_pairing_exists);
    assert_eq!(summary.latest_window_id, window_id);
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test life_closure_hardening live_io_evidence -- --nocapture
```

Expected: fail because `live_io_evidence` module is missing.

- [ ] **Step 3: Implement minimal code**

Create a snapshot-only `LiveIoEvidenceSummary` with numeric/boolean observer fields and `summarize_live_io_evidence(&SnapshotData)`.

- [ ] **Step 4: Verify targeted tests pass**

Run the same targeted command. Expected: all `live_io_evidence` tests pass.

## Task 3: Sandbox Framed Byte Carrier Loop

**Files:**
- Create: `bionic_v0/src/framed_byte_carrier.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/framed_byte_egress.rs`
- Test: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing tests**

Add tests proving a sandbox carrier can accept framed-byte egress, produce return ingress through ordinary consequence windows, and update projection confidence numerically.

```rust
#[test]
fn sandbox_framed_byte_carrier_closes_write_return_loop_without_shell_meaning() {
    let vp = Viewport::new();
    let mut carrier = SandboxFramedByteCarrier::default();
    carrier.queue_return(7, b"ok", 0.72, 0.10);
    tick_sandbox_framed_byte_loop(&vp, &mut carrier, 1_000, 7, b"hi", 0.6);
    let egress = vp.read_framed_byte_egress();
    let ac = vp.read_action_consequence();
    assert_eq!(egress.n_frames, 1);
    assert_eq!(egress.n_returns, 1);
    assert!(egress.projection_confidence > 0.5);
    assert_eq!(ac.pairs.len(), 1);
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test life_closure_hardening sandbox_framed_byte -- --nocapture
```

Expected: fail because carrier module is missing.

- [ ] **Step 3: Implement minimal code**

Create a sandbox-only carrier with queued returns and a tick helper that calls existing egress begin/return methods. Do not add PTY, SSH, shell, or command execution.

- [ ] **Step 4: Verify targeted tests pass**

Run the same targeted command. Expected: all `sandbox_framed_byte` tests pass.

## Task 4: First Producer Wiring

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/framed_byte_egress.rs`
- Modify: `bionic_v0/src/language_binding.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Test: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing tests**

Add tests proving admitted loops create route-local adaptive samples and learned-bias deltas without external setters.

```rust
#[test]
fn paired_action_consequence_sediments_adaptive_and_learned_bias_samples() {
    let vp = Viewport::new();
    make_paired_window(&vp, 1_000, 1_300, 0.9);
    vp.tick_life_closure_producers(1_400);
    assert!(vp.read_adaptive_stability().n_routes > 0);
    assert!(vp.read_learned_bias().summary().recent_event_count > 0);
}

#[test]
fn framed_byte_return_sediments_code_bias_without_meaning_fields() {
    let vp = Viewport::new();
    vp.begin_framed_byte_egress(1_000, 3, b"ab", 0.5);
    vp.observe_framed_byte_return(1_200, 3, b"ab", 0.7, 0.0);
    vp.tick_life_closure_producers(1_300);
    let summary = vp.read_learned_bias().summary();
    assert!(summary.recent_event_count > 0);
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test life_closure_hardening life_closure_producers -- --nocapture
```

Expected: fail because `tick_life_closure_producers` is missing.

- [ ] **Step 3: Implement minimal code**

Add `Viewport::tick_life_closure_producers(now_us)` that reads action-consequence pairs and framed-byte return traces, then writes only `AdaptiveRouteSample` and `LearnedBiasEventInput` numeric events. Track consumed producer cursor so repeated ticks do not duplicate events.

- [ ] **Step 4: Verify targeted tests pass**

Run the same targeted command. Expected: producer tests pass.

## Task 5: Full Verification

**Files:**
- All modified files.

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml
```

- [ ] **Step 2: Full tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all unit, integration, and doc tests pass.

- [ ] **Step 3: Source hygiene scan**

Run:

```bash
rg -n "semantic_readiness|SymbolizerState|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding|println!|eprintln!" bionic_v0/src
```

Expected: no body-side rejected architecture or body diagnostics are introduced. Observer binaries may still contain observer-side `eprintln!` usage.

## Self-Review Notes

This plan intentionally excludes real PTY/SSH/shell carrier admission, responsibility-owner design, brain-police design, and self/goal evolution implementation. Those are v1-triggered items and require separate frozen specs before implementation.
