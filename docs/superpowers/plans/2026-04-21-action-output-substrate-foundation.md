# Action Output Substrate Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the smallest admissible action-output substrate to `bionic_v0` so output write state becomes a first-class numeric/time substrate without introducing behavior modules, world scripts, or action-consequence learning.

**Architecture:** Introduce a dedicated numeric-only action-output state module and mount it in the shared `Viewport` as a new subtree parallel to existing ingress-side substrate. Expose only write-state, timing, and reversibility facts through snapshot/observer; do not add body-side behavior loops, device drivers, target selection, or consequence pairing.

**Tech Stack:** Rust (`std`, `RwLock`, `AtomicU64`), existing `Viewport`/snapshot binary format, Cargo integration tests.

---

## File Structure

**Create**
- `bionic_v0/src/action_output.rs`
- `bionic_v0/tests/action_output_substrate.rs`

**Modify**
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/snapshot.rs`
- `bionic_v0/src/bin/observer.rs`

**Do Not Modify In This Plan**
- `bionic_v0/src/bin/body.rs`
- any adapter or actuator implementation
- any language-binding, imitation, or action-consequence logic

**Responsibilities**
- `src/action_output.rs`: numeric-only output-write substrate structs, phase/status enums, reversible write events, no target semantics.
- `src/viewport.rs`: first-class shared subtree and minimal read/write/update API for action-output state.
- `src/snapshot.rs`: binary snapshot persistence for action-output state and reader support.
- `src/bin/observer.rs`: external rendering only; no body-side semantics.
- `tests/action_output_substrate.rs`: contract tests for default state, reversibility, timing, snapshot visibility, and banned concepts.

## Chunk 1: Action Output Substrate In Viewport

### Task 1: Add the failing substrate contract tests

**Files:**
- Create: `bionic_v0/tests/action_output_substrate.rs`
- Test: `bionic_v0/tests/action_output_substrate.rs`

- [ ] **Step 1: Write the failing test for default action-output state**

```rust
use bionic_v0::viewport::Viewport;

#[test]
fn action_output_viewport_exposes_empty_write_state_by_default() {
    let vp = Viewport::new();
    let st = vp.read_action_output();

    assert_eq!(st.n_channels, 0);
    assert!(st.channels.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test action_output_viewport_exposes_empty_write_state_by_default --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because `read_action_output` and the new test file do not exist yet.

- [ ] **Step 3: Add the failing reversibility and timing tests**

```rust
use bionic_v0::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, ActionOutputState,
    ActionOutputWriteTrace, OutputChannelKind,
};

#[test]
fn action_output_state_tracks_write_phase_without_targets_or_words() {
    let st = ActionOutputState::default();
    assert_eq!(st.n_channels, 0);
    let dbg = format!("{:?}", st);
    assert!(!dbg.contains("target"));
    assert!(!dbg.contains("word"));
}

#[test]
fn action_output_events_are_small_step_and_reversible() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.1,
        event_t_us: 100,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Adjust,
        magnitude: 0.25,
        event_t_us: 120,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Decay,
        magnitude: 0.05,
        event_t_us: 180,
    });

    let st = vp.read_action_output();
    assert_eq!(st.n_channels, 1);
    assert_eq!(st.channels[0].phase, ActionOutputPhase::Decaying);
    assert!(st.channels[0].current_magnitude <= 0.25);
    assert_eq!(st.channels[0].start_t_us, 100);
    assert_eq!(st.channels[0].last_update_t_us, 180);
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because `action_output` module and viewport API do not exist yet.

- [ ] **Step 5: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

### Task 2: Implement the minimal numeric action-output substrate

**Files:**
- Create: `bionic_v0/src/action_output.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/action_output_substrate.rs`

- [ ] **Step 1: Create `src/action_output.rs` with the smallest admissible state**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputChannelKind {
    GenericWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionOutputPhase {
    Idle,
    Active,
    Decaying,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionOutputFeedback {
    Begin,
    Adjust,
    Decay,
    Stop,
}

#[derive(Clone, Debug)]
pub struct ActionOutputEvent {
    pub channel_kind: OutputChannelKind,
    pub feedback: ActionOutputFeedback,
    pub magnitude: f64,
    pub event_t_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionOutputWriteTrace {
    pub channel_kind: OutputChannelKind,
    pub phase: ActionOutputPhase,
    pub current_magnitude: f64,
    pub peak_magnitude: f64,
    pub start_t_us: u64,
    pub last_update_t_us: u64,
    pub last_transition_t_us: u64,
    pub reinforce_count: u32,
    pub decay_count: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ActionOutputState {
    pub n_channels: u32,
    pub channels: Vec<ActionOutputWriteTrace>,
}
```

Requirements:
- keep it numeric/time-only
- no target object fields
- no goal/result/meaning fields
- no language or naming terms

- [ ] **Step 2: Export the module and bump snapshot version placeholder**

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod action_output;
pub const SNAPSHOT_VERSION: u64 = 25;
```

Do not change any unrelated constants.

- [ ] **Step 3: Mount the subtree in `Viewport`**

Add to `bionic_v0/src/viewport.rs`:

```rust
use crate::action_output::{ActionOutputEvent, ActionOutputState};

pub struct Viewport {
    // ...
    pub action_output: RwLock<ActionOutputState>,
    // ...
}
```

Initialize it in `Viewport::new()`:

```rust
action_output: RwLock::new(ActionOutputState::default()),
```

Add the minimal API:

```rust
pub fn read_action_output(&self) -> ActionOutputState { ... }
pub fn apply_action_output_event(&self, event: ActionOutputEvent) { ... }
```

Implementation rule:
- one generic channel is enough for this phase
- update only write-state and timing fields
- preserve reversibility via `Begin/Adjust/Decay/Stop`
- do not touch slots, relations, reasoning, or body orchestration

- [ ] **Step 4: Run the new focused tests**

Run: `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS for the new action-output tests.

- [ ] **Step 5: Run the existing language-binding tests as a guard**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS to confirm the new subtree did not regress the binding layer.

- [ ] **Step 6: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

## Chunk 2: Snapshot And Observer Boundary

### Task 3: Add the failing snapshot and observer contract tests

**Files:**
- Modify: `bionic_v0/tests/action_output_substrate.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`

- [ ] **Step 1: Add the failing snapshot persistence test**

```rust
use std::sync::Arc;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};

#[test]
fn snapshot_carries_action_output_write_state() {
    let vp = Arc::new(Viewport::new());
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });

    let dir = std::env::temp_dir().join("bionic_v0_action_output_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.action_output.n_channels, 1);
    assert_eq!(snap.action_output.channels[0].start_t_us, 1000);
}
```

- [ ] **Step 2: Add the failing observer hygiene test**

```rust
#[test]
fn observer_renders_action_output_without_human_semantics() {
    let observer_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer.rs"
    ))
    .expect("read observer.rs");

    assert!(observer_src.contains("out:"));
    assert!(!observer_src.contains("speak"));
    assert!(!observer_src.contains("target_object"));
    assert!(!observer_src.contains("word"));
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test snapshot_carries_action_output_write_state --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because snapshot reader/writer and observer do not expose action-output state yet.

- [ ] **Step 4: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

### Task 4: Persist and render the substrate without adding behavior semantics

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/action_output_substrate.rs`

- [ ] **Step 1: Extend snapshot binary format for action-output state**

Modify `bionic_v0/src/snapshot.rs` to:
- import `ActionOutputState`, `ActionOutputWriteTrace`, `ActionOutputPhase`, `OutputChannelKind`
- write `vp.read_action_output()` after existing language-binding block
- read the same section into `SnapshotData`

Minimum serialized fields per channel:

```rust
n_channels: u32
for each channel:
  channel_kind: u64
  phase: u64
  current_magnitude: f64
  peak_magnitude: f64
  start_t_us: u64
  last_update_t_us: u64
  last_transition_t_us: u64
  reinforce_count: u32
  decay_count: u32
```

Do not serialize any target/result/meaning fields.

- [ ] **Step 2: Add observer rendering for output write state**

Modify `bionic_v0/src/bin/observer.rs`:
- add a compact `out:` column after existing `bind:` column
- render numeric/time write-state only

Example shape:

```rust
out:n|ch[kind|phase|mag|peak|start_age_ms|last_age_ms]
```

Rules:
- render only snapshot truth
- no body-side computation
- no human-readable semantics beyond channel kind/phase abbreviations
- no interpretation as speech, gesture, goal, or target

- [ ] **Step 3: Run the focused action-output test suite**

Run: `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Run the snapshot roundtrip regression test**

Run: `cargo test phot_snapshot_roundtrip --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS, confirming the snapshot format still round-trips.

- [ ] **Step 5: Run the no-human-output body guard**

Run: `cargo test body_no_human_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS, confirming no forbidden human-readable output was added to the body process.

- [ ] **Step 6: Run the full test suite**

Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`

Expected: PASS.

- [ ] **Step 7: Record repository state**

Run: `git rev-parse --show-toplevel`

Expected: FAIL with `fatal: not a git repository...`

Note in task log: cannot commit in this workspace because `/Users/joe/Documents/unnamed` is not a git repository.

## Out Of Scope

- any actuator/device implementation
- any body loop that autonomously emits output
- any action-consequence pairing or learned consequence windows
- any imitation, curve matching, or self/other comparison
- any fast-path formation
- any target-object, target-region, or success-result semantics
- any language-mediated output

## Verification Checklist

- `cargo test action_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test phot_snapshot_roundtrip --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test body_no_human_output --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
- `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`
- `git rev-parse --show-toplevel` (expected to fail because this workspace is not a git repository)

## Handoff Notes

- Keep all new state numeric/time-oriented and observer-rendered only outside the body.
- Do not introduce target semantics, result classes, curiosity numbers, or behavior scripts.
- If the parallel spec session changes the admissible contract, reconcile this plan to the final frozen spec before execution.
