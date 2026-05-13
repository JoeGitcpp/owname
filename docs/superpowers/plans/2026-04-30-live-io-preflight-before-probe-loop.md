# Live I/O Preflight Before Probe Loop Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove vocal output, microphone ingress, visual substrate structure, and write/consequence traces are reliable before entering active probe-feedback closure.

**Architecture:** Add a focused preflight test suite around existing substrate APIs instead of introducing a new body ontology. Deterministic tests validate write epochs, VIB consequence windows, unpaired external ingress, overlapping ambiguity, PHOT slot/relation/prediction structure, and snapshot visibility. Optional live checks remain observer-side and environment-dependent.

**Tech Stack:** Rust, `cargo test`, existing `bionic_v0` viewport/action-output/action-consequence/VIB/PHOT/snapshot modules.

---

## Chunk 1: Deterministic Preflight Tests

### Task 1: Add Live I/O Preflight Test File

**Files:**
- Create: `bionic_v0/tests/live_io_preflight.rs`
- Reference: `bionic_v0/src/action_output.rs`
- Reference: `bionic_v0/src/action_consequence.rs`
- Reference: `bionic_v0/src/viewport.rs`
- Reference: `docs/superpowers/specs/2026-04-30-live-io-preflight-before-probe-loop-design.md`

- [ ] **Step 1: Write failing test for vocal write epoch pairing with VIB consequence**

Create `bionic_v0/tests/live_io_preflight.rs` with imports and the first test:

```rust
use bionic_v0::action_consequence::ConsequenceWindowPhase;
use bionic_v0::action_output::{
    ActionOutputEvent, ActionOutputFeedback, OutputChannelKind,
};
use bionic_v0::viewport::Viewport;

#[test]
fn vocal_write_epoch_can_pair_with_vib_consequence_window() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.7,
        event_t_us: 1_000,
    });

    let window_id = vp.accumulate_ingress_change(1_040, 0.65);
    assert_ne!(window_id, 0);
    assert!(vp.close_action_consequence_window(window_id, 1_080));
    assert!(vp.try_auto_pair_latest_epoch_to_window(1_090));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].window_id, window_id);
    assert_eq!(ac.pairs[0].delay_us, 40);
    assert!(ac.pairs[0].curve_similarity > 0.9);

    let windows = ac.windows;
    assert_eq!(windows[0].phase, ConsequenceWindowPhase::Closed);
}
```

- [ ] **Step 2: Run the test to verify current behavior**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight vocal_write_epoch_can_pair_with_vib_consequence_window
```

Expected:

- If the current substrate already supports this, the test passes.
- If it fails, the failure should point to pairing delay, window lifecycle, or curve-similarity behavior. Do not change semantics to make it pass.

- [ ] **Step 3: Commit if the test passes without implementation**

```bash
git add bionic_v0/tests/live_io_preflight.rs
git commit -m "test: add vocal to vib preflight pairing"
```

### Task 2: Add External Sound Remains Unpaired Test

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`

- [ ] **Step 1: Write failing test for external ingress without write epoch**

Append:

```rust
#[test]
fn external_vib_without_write_epoch_remains_unpaired() {
    let vp = Viewport::new();

    let window_id = vp.accumulate_ingress_change(2_000, 0.8);
    assert_ne!(window_id, 0);
    assert!(vp.close_action_consequence_window(window_id, 2_100));

    assert!(!vp.try_auto_pair_latest_epoch_to_window(2_120));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 1);
    assert_eq!(ac.n_pairs, 0);
}
```

- [ ] **Step 2: Run the single test**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight external_vib_without_write_epoch_remains_unpaired
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add bionic_v0/tests/live_io_preflight.rs
git commit -m "test: keep unowned vib ingress unpaired"
```

### Task 3: Add Overlap/Ambiguity Test Without Ownership Labels

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`

- [ ] **Step 1: Write test for overlapping external and self-coupled windows**

Append:

```rust
#[test]
fn overlapping_vib_windows_do_not_force_ownership_labels() {
    let vp = Viewport::new();

    let external_window = vp.accumulate_ingress_change(4_000, 0.9);
    assert!(vp.close_action_consequence_window(external_window, 4_050));
    assert!(!vp.try_auto_pair_latest_epoch_to_window(4_060));

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.5,
        event_t_us: 5_000,
    });

    let coupled_window = vp.accumulate_ingress_change(5_080, 0.52);
    assert!(vp.close_action_consequence_window(coupled_window, 5_160));
    assert!(vp.try_auto_pair_latest_epoch_to_window(5_170));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 2);
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].window_id, coupled_window);
    assert_ne!(ac.pairs[0].window_id, external_window);
}
```

- [ ] **Step 2: Add source hygiene assertion**

Add a test that checks the preflight test file itself does not introduce forbidden ownership vocabulary as internal field names:

```rust
#[test]
fn preflight_tests_do_not_introduce_ownership_or_semantic_fields() {
    let source = std::fs::read_to_string(file!()).expect("read test source");
    for forbidden in [
        "self_voice:",
        "external_speaker:",
        "semantic_class:",
        "source_name:",
        "owner:",
        "identity:",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden internal field marker found: {forbidden}"
        );
    }
}
```

- [ ] **Step 3: Run the test file**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight
```

Expected: all live I/O preflight tests pass.

- [ ] **Step 4: Commit**

```bash
git add bionic_v0/tests/live_io_preflight.rs
git commit -m "test: preserve ambiguity in io preflight traces"
```

## Chunk 2: Camera Substrate Preflight

### Task 4: Add PHOT Structure Reaches Slots And Relations Test

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Reference: existing PHOT tests in `bionic_v0/tests/real_phot_adapter.rs`
- Reference: existing relation tests in `bionic_v0/tests/global_relations.rs`
- Reference: existing prediction tests in `bionic_v0/tests/relation_predictor.rs`

- [ ] **Step 1: Inspect existing PHOT helper APIs**

Run:

```bash
cd /Users/joe/unnamed
rg -n "RealPhotAdapter|Synthetic|push_phot|claim_slot|refresh_global_relations|refresh_global_relation_predictor" bionic_v0/tests bionic_v0/src
```

Expected: identify the existing helper path for creating PHOT evidence and slot claims.

- [ ] **Step 2: Write a deterministic PHOT structure test using existing helpers**

Add a test named:

```rust
#[test]
fn phot_visible_structure_reaches_slots_relations_and_prediction() {
    // Use the same PHOT evidence construction path as existing PHOT tests.
    // Arrange two spatially separated PHOT events.
    // Claim slots through the viewport.
    // Refresh global attention, relations, relation predictor, and reasoning.
    // Assert:
    // - at least two slots are active;
    // - spatial fields are valid;
    // - at least one relation edge is active;
    // - relation predictor fields are updated after warmup;
    // - no semantic/name/concept fields are involved.
}
```

Use exact APIs discovered in Step 1 rather than adding a new camera abstraction.

- [ ] **Step 3: Run the PHOT preflight test**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight phot_visible_structure_reaches_slots_relations_and_prediction
```

Expected: PASS after using existing helper paths.

- [ ] **Step 4: Commit**

```bash
git add bionic_v0/tests/live_io_preflight.rs
git commit -m "test: add phot substrate preflight"
```

### Task 5: Add Snapshot Visibility For Preflight Evidence

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Reference: `bionic_v0/src/snapshot.rs`
- Reference: snapshot tests in `bionic_v0/tests/action_output_substrate.rs`
- Reference: snapshot tests in `bionic_v0/tests/action_consequence_loop.rs`
- Reference: snapshot tests in `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write snapshot roundtrip test**

Add a test named:

```rust
#[test]
fn snapshot_exposes_live_io_preflight_traces() {
    // Arrange:
    // - one action-output Begin event;
    // - one ingress consequence window;
    // - one write/consequence pair;
    // - one vocal activity update if existing public APIs allow it;
    // - one PHOT slot or PHOT health state if existing public APIs allow it.
    //
    // Act:
    // - write snapshot using SnapshotPaths in a temp runtime directory;
    // - read snapshot back using existing snapshot roundtrip helpers.
    //
    // Assert:
    // - action-output channel and write epoch are present;
    // - consequence window and pair are present;
    // - vocal activity fields are present when arranged;
    // - PHOT health/slot fields are present when arranged;
    // - no semantic readiness or symbolizer fields appear.
}
```

- [ ] **Step 2: Run the snapshot test**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight snapshot_exposes_live_io_preflight_traces
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add bionic_v0/tests/live_io_preflight.rs
git commit -m "test: expose io preflight traces in snapshot"
```

## Chunk 3: Optional Live Hardware Runbook

### Task 6: Add Observer-Side Live Check Documentation

**Files:**
- Create: `docs/superpowers/plans/2026-04-30-live-io-preflight-runbook.md`
- Reference: `docs/superpowers/specs/2026-04-30-live-io-preflight-before-probe-loop-design.md`

- [ ] **Step 1: Write the runbook**

Create a short runbook with:

```markdown
# Live I/O Preflight Runbook

## Purpose

Validate hardware-dependent microphone, camera, and vocal-output behavior
through snapshot/observer data only.

## Commands

Run bounded body:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo run --bin body -- runtime_live_io_preflight 10
```

Inspect snapshot through existing observer tooling:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo run --bin observer -- runtime_live_io_preflight
```

## Expected Observer-Side Evidence

- VIB health and energy are visible.
- PHOT health is OK or explicit FAULT/STALE.
- Vocal activity fields are visible when vocal drive is nonzero.
- Action-output write epochs are visible when a write is triggered.
- Consequence windows and pairs are visible when physical feedback occurs.

## Boundary

No body stdout/stderr diagnostics are required or allowed for success.
Environment failure is represented as snapshot state, not human-readable body
output.
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/plans/2026-04-30-live-io-preflight-runbook.md
git commit -m "docs: add live io preflight runbook"
```

## Chunk 4: Verification

### Task 7: Run Focused And Full Verification

**Files:**
- Test only unless failures require code changes.

- [ ] **Step 1: Run focused preflight tests**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test live_io_preflight
```

Expected: PASS.

- [ ] **Step 2: Run existing related tests**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test action_output_substrate --test action_consequence_loop --test ingress_consequence_loop --test real_phot_adapter --test vocal_runtime
```

Expected: PASS.

- [ ] **Step 3: Run full suite**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test
```

Expected: PASS.

- [ ] **Step 4: Final status**

Run:

```bash
cd /Users/joe/unnamed
git status --short
```

Expected: clean worktree after commits, or only intentional uncommitted notes if the user requested no commits.
