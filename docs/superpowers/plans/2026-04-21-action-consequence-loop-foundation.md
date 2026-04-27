# Action Consequence Loop Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the smallest admissible action-consequence substrate to `bionic_v0` so output write events can be time-paired with observed consequence windows without introducing targets, result semantics, or imitation logic.

**Architecture:** Build a numeric/time-only `action_consequence` subtree parallel to existing `action_output` and ingress-side substrate. The foundation records write epochs, consequence windows, and pending/paired relationships; it does not classify meaning, choose targets, compare self vs other, or perform imitation.

**Tech Stack:** Rust (`std`, `RwLock`, `AtomicU64`), existing `Viewport`/snapshot binary format, Cargo integration tests.

---

## File Structure

**Create**
- `bionic_v0/src/action_consequence.rs`
- `bionic_v0/tests/action_consequence_loop.rs`

**Modify**
- `bionic_v0/src/action_output.rs`
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/snapshot.rs`
- `bionic_v0/src/bin/observer.rs`

**Do Not Modify In This Plan**
- `bionic_v0/src/bin/body.rs`
- any adapter or actuator implementation
- any language-binding logic
- any imitation/self-other comparison logic
- any fast-path or high-level behavior logic

**Responsibilities**
- `src/action_output.rs`: expose a minimal write epoch / trace handle suitable for downstream consequence pairing.
- `src/action_consequence.rs`: numeric-only consequence windows, pairing state, and reversible pending/paired records.
- `src/viewport.rs`: mount the new subtree and provide the smallest read/write/update API.
- `src/snapshot.rs`: persist and read action-consequence state.
- `src/bin/observer.rs`: render compact external research output only.
- `tests/action_consequence_loop.rs`: contract tests for empty state, pending consequence windows, output pairing, snapshot visibility, and banned concepts.

## Chunk 1: Write Epoch And Consequence Subtree

### Task 1: Add the failing consequence-loop tests

**Files:**
- Create: `bionic_v0/tests/action_consequence_loop.rs`
- Test: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Write the failing default-state test**

```rust
use bionic_v0::viewport::Viewport;

#[test]
fn action_consequence_viewport_exposes_empty_state_by_default() {
    let vp = Viewport::new();
    let st = vp.read_action_consequence();

    assert_eq!(st.n_windows, 0);
    assert!(st.windows.is_empty());
    assert_eq!(st.n_pairs, 0);
    assert!(st.pairs.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test action_consequence_viewport_exposes_empty_state_by_default --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because `action_consequence` state and viewport API do not exist yet.

- [ ] **Step 3: Add the failing pending-window and pairing tests**

```rust
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::action_consequence::{
    ActionConsequenceWindowEvent, ConsequenceModalityKind, ConsequencePairStatus,
};
use bionic_v0::viewport::Viewport;

#[test]
fn action_consequence_records_pending_window_without_result_semantics() {
    let vp = Viewport::new();

    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 140,
        magnitude: 0.3,
    });

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 1);
    assert_eq!(st.windows[0].start_t_us, 100);
    assert_eq!(st.windows[0].end_t_us, 140);
}

#[test]
fn action_consequence_pairs_a_write_epoch_with_a_window() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });

    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });

    vp.pair_action_consequence(write_seq, 0, 40);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
    assert_eq!(st.pairs[0].write_seq, write_seq);
    assert_eq!(st.pairs[0].window_index, 0);
    assert_eq!(st.pairs[0].status, ConsequencePairStatus::Paired);
    assert_eq!(st.pairs[0].delay_us, 40);
}
```

- [ ] **Step 4: Run the focused test set and confirm it fails**

Run: `cargo test action_consequence --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because the new module, types, and viewport methods do not exist yet.

- [ ] **Step 5: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

### Task 2: Implement write epochs and the minimal action-consequence substrate

**Files:**
- Create: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/action_output.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Extend `ActionOutputWriteTrace` with a write epoch handle**

Modify `bionic_v0/src/action_output.rs` so each active write trace carries a monotonic numeric handle:

```rust
pub struct ActionOutputWriteTrace {
    // existing fields...
    pub current_write_seq: u64,
}
```

Implementation rules:
- increment a viewport-global `action_output_write_seq` on every `Begin`
- `Adjust/Decay/Stop` keep the same `current_write_seq`
- this is an engineering pairing handle only, not a target, concept, or meaning field

- [ ] **Step 2: Create `src/action_consequence.rs` with the smallest admissible types**

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConsequenceModalityKind {
    #[default]
    GenericIngress,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConsequencePairStatus {
    #[default]
    Pending,
    Paired,
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequenceWindow {
    pub modality_kind: ConsequenceModalityKind,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequencePair {
    pub write_seq: u64,
    pub window_index: u32,
    pub delay_us: u64,
    pub status: ConsequencePairStatus,
    pub pair_t_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionConsequenceState {
    pub n_windows: u32,
    pub windows: Vec<ActionConsequenceWindow>,
    pub n_pairs: u32,
    pub pairs: Vec<ActionConsequencePair>,
}

#[derive(Clone, Debug)]
pub struct ActionConsequenceWindowEvent {
    pub modality_kind: ConsequenceModalityKind,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
}
```

Do not add:
- target objects
- result labels
- success/failure semantics
- self/other comparison fields

- [ ] **Step 3: Export and mount the subtree in `Viewport`**

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod action_consequence;
pub const SNAPSHOT_VERSION: u64 = 26;
```

Modify `bionic_v0/src/viewport.rs`:

```rust
use crate::action_consequence::{
    ActionConsequenceState, ActionConsequenceWindowEvent,
};

pub struct Viewport {
    // ...
    pub action_output_write_seq: AtomicU64,
    pub action_consequence: RwLock<ActionConsequenceState>,
    // ...
}
```

Initialize in `Viewport::new()`:

```rust
action_output_write_seq: AtomicU64::new(0),
action_consequence: RwLock::new(ActionConsequenceState::default()),
```

Add the minimal API:

```rust
pub fn read_action_consequence(&self) -> ActionConsequenceState { ... }
pub fn push_action_consequence_window(&self, ev: ActionConsequenceWindowEvent) { ... }
pub fn pair_action_consequence(&self, write_seq: u64, window_index: u32, delay_us: u64) { ... }
```

Rules:
- windows are append-only for this phase
- pairing is explicit and numeric-only
- no automatic target inference
- no consequence classification

- [ ] **Step 4: Run the focused action-consequence tests**

Run: `cargo test action_consequence --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS for the new action-consequence tests.

- [ ] **Step 5: Run the existing action-output tests as a guard**

Run: `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS, confirming write-epoch changes did not regress the output substrate.

- [ ] **Step 6: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

## Chunk 2: Snapshot And Observer Boundary

### Task 3: Add the failing snapshot and observer contract tests

**Files:**
- Modify: `bionic_v0/tests/action_consequence_loop.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`

- [ ] **Step 1: Add the failing snapshot roundtrip test**

```rust
use std::sync::Arc;
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn snapshot_carries_action_consequence_windows_and_pairs() {
    let vp = Arc::new(Viewport::new());
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });
    vp.pair_action_consequence(write_seq, 0, 40);

    let dir = std::env::temp_dir().join("bionic_v0_action_consequence_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.action_consequence.n_windows, 1);
    assert_eq!(snap.action_consequence.n_pairs, 1);
    assert_eq!(snap.action_consequence.pairs[0].write_seq, write_seq);
}
```

- [ ] **Step 2: Add the failing observer hygiene test**

```rust
#[test]
fn observer_renders_action_consequence_without_result_semantics() {
    let observer_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer.rs"
    ))
    .expect("read observer.rs");

    assert!(observer_src.contains("acq:"));
    assert!(!observer_src.contains("success"));
    assert!(!observer_src.contains("target_object"));
    assert!(!observer_src.contains("imit"));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test snapshot_carries_action_consequence_windows_and_pairs --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because snapshot and observer do not expose action-consequence state yet.

- [ ] **Step 4: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

### Task 4: Persist and render the consequence-loop foundation

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Extend snapshot format for action-consequence state**

Modify `bionic_v0/src/snapshot.rs` to write and read:

```rust
n_windows: u32
for each window:
  modality_kind: u64
  start_t_us: u64
  end_t_us: u64
  magnitude: f64

n_pairs: u32
for each pair:
  write_seq: u64
  window_index: u32
  delay_us: u64
  status: u64
  pair_t_us: u64
```

Place this after the existing action-output block.

Do not serialize:
- target ids
- result labels
- similarity scores
- imitation state

- [ ] **Step 2: Add observer rendering for compact consequence-loop traces**

Modify `bionic_v0/src/bin/observer.rs`:
- add a compact `acq:` column after `out:`
- render only window/pair substrate facts

Example shape:

```rust
acq:wN|w[mod|start|end|mag];pN|p[seq|win|delay|st]
```

Rules:
- render snapshot truth only
- no interpretation as success/failure
- no target or meaning explanation
- no imitation wording

- [ ] **Step 3: Run the focused consequence-loop tests**

Run: `cargo test action_consequence --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Run the action-output guard suite**

Run: `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run snapshot and body guards**

Run: `cargo test phot_snapshot_roundtrip --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS.

Run: `cargo test body_no_human_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run the full test suite**

Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`

Expected: PASS.

- [ ] **Step 7: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

## Out Of Scope

- automatic consequence detection from live ingress
- learned consequence windows
- target region/object inference
- success/failure semantics
- similarity comparison between self output and external patterns
- imitation, babbling, or curve fitting
- fast-path formation
- any body-side autonomous behavior loop

## Verification Checklist

- `cargo test action_consequence --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test phot_snapshot_roundtrip --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test body_no_human_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`
- `git rev-parse --show-toplevel` (expected to fail because this workspace is not a git repository)

## Handoff Notes

- Keep all new state numeric/time-oriented and explicitly lower than imitation logic.
- `write_seq` is an engineering pairing handle only; it must not become a target, concept, or meaning field.
- Do not auto-infer consequences in this phase; explicit window recording is enough for the substrate.
- If a later frozen spec tightens the interface, reconcile this plan to the spec before execution.
