# Vocal Imitation and Learning Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build phase 1 of the vocal system: an external-imitation-first, body-side vocal substrate that can hold a heard target contour, drive primitive vowel-like output, compare heard-vs-self, and sediment stable vocal traces into reusable memory without introducing text/TTS/token logic.

**Architecture:** Add four numeric substrates inside `bionic_v0` (`vocal_target`, `vocal_organ`, `vocal_imitation`, `vocal_memory`) plus a speaker-facing adapter path that renders the organ state into real audio. Reuse existing VIB ingress, snapshot/observer boundaries, and closed-loop style from probe/action-consequence, while keeping all display, debug, and device explanation outside the body process.

**Tech Stack:** Rust, existing `bionic_v0` viewport/snapshot/observer architecture, `cpal` for audio I/O, TDD with `cargo test`, manual smoke tests with `cargo run`

---

## File Map

- Create: `bionic_v0/src/vocal_target.rs`
  Responsibility: short-lived external vocal target contour, decay, and target activation rules.
- Create: `bionic_v0/src/vocal_organ.rs`
  Responsibility: continuous primitive vocal trajectory state and update helpers.
- Create: `bionic_v0/src/vocal_imitation.rs`
  Responsibility: target-vs-self comparison, imitation pressure, convergence/failure accounting.
- Create: `bionic_v0/src/vocal_memory.rs`
  Responsibility: stable vocal prototypes, reinforcement/decay, low-cost replay preference.
- Create: `bionic_v0/src/vocal_adapter.rs`
  Responsibility: speaker-facing adapter that turns `vocal_organ` state into real audio output.
- Modify: `bionic_v0/src/lib.rs`
  Responsibility: expose new modules and any shared constants.
- Modify: `bionic_v0/src/viewport.rs`
  Responsibility: own new substrates, read/write/tick APIs, snapshot-facing read access.
- Modify: `bionic_v0/src/snapshot.rs`
  Responsibility: serialize/deserialize new vocal substrates for observer-side visibility.
- Modify: `bionic_v0/src/bin/body.rs`
  Responsibility: register/tick vocal loop inside the body without adding human-readable output.
- Modify: `bionic_v0/src/bin/observer.rs`
  Responsibility: render compact vocal substrate facts as external research output.
- Create: `bionic_v0/tests/vocal_target.rs`
- Create: `bionic_v0/tests/vocal_organ.rs`
- Create: `bionic_v0/tests/vocal_imitation.rs`
- Create: `bionic_v0/tests/vocal_memory.rs`
- Create: `bionic_v0/tests/vocal_runtime.rs`

---

## Chunk 1: Vocal Target Contract

### Task 1: Add failing tests for short-lived heard target traces

**Files:**
- Create: `bionic_v0/tests/vocal_target.rs`
- Create: `bionic_v0/src/vocal_target.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
use bionic_v0::vocal_target::VocalTargetState;

#[test]
fn vocal_target_activates_from_external_contour_and_decays_when_idle() {
    let mut st = VocalTargetState::default();
    st.observe_external_contour(1_000, 0.8, 0.45, 0.30, 0.12);
    assert!(st.active_energy > 0.0);

    st.tick_decay(5_000_000);
    assert!(st.active_energy < 0.8);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_target`

Expected: FAIL because `vocal_target` module and `VocalTargetState` do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement in `bionic_v0/src/vocal_target.rs`:

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalTargetState {
    pub active_energy: f64,
    pub pitch_hint: f64,
    pub resonance_hint: f64,
    pub envelope_hint: f64,
    pub last_update_t_us: u64,
}
```

Add methods:
- `observe_external_contour(...)`
- `tick_decay(now_us: u64)`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_target`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/vocal_target.rs bionic_v0/tests/vocal_target.rs
git commit -m "feat: add vocal target substrate"
```

---

## Chunk 2: Vocal Organ Contract

### Task 2: Add failing tests for primitive vowel-like organ state

**Files:**
- Create: `bionic_v0/tests/vocal_organ.rs`
- Create: `bionic_v0/src/vocal_organ.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
use bionic_v0::vocal_organ::{VocalOrganPhase, VocalOrganState};

#[test]
fn vocal_organ_tracks_continuous_vowel_like_parameters() {
    let mut st = VocalOrganState::default();
    st.begin_voicing(1_000, 0.6, 0.4, 0.7);
    assert_eq!(st.phase, VocalOrganPhase::Active);
    assert!(st.breath_drive > 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_organ`

Expected: FAIL because `vocal_organ` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `VocalOrganPhase`
- `VocalOrganState`
- `begin_voicing(...)`
- `adjust_toward(...)`
- `tick_decay(...)`

Required fields:
- `breath_drive`
- `pitch_tension`
- `resonance_position`
- `noise_mix`
- `phase`
- `last_update_t_us`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_organ`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/vocal_organ.rs bionic_v0/tests/vocal_organ.rs
git commit -m "feat: add vocal organ substrate"
```

---

## Chunk 3: Imitation Pressure Contract

### Task 3: Add failing tests for heard-vs-self comparison and pressure decay

**Files:**
- Create: `bionic_v0/tests/vocal_imitation.rs`
- Create: `bionic_v0/src/vocal_imitation.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
use bionic_v0::vocal_imitation::VocalImitationState;

#[test]
fn imitation_pressure_rises_with_target_gap_and_drops_on_convergence() {
    let mut st = VocalImitationState::default();
    st.observe_gap(1_000, 0.9, 0.2);
    let high = st.imitation_pressure;
    st.observe_gap(2_000, 0.3, 0.2);
    assert!(st.imitation_pressure < high);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_imitation`

Expected: FAIL because `vocal_imitation` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `VocalImitationState`
- `observe_gap(...)`
- `tick_decay(...)`
- convergence/failure counters

Required fields:
- `imitation_pressure`
- `suppression`
- `last_gap`
- `converge_count`
- `fail_count`
- `last_update_t_us`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_imitation`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/vocal_imitation.rs bionic_v0/tests/vocal_imitation.rs
git commit -m "feat: add vocal imitation pressure substrate"
```

---

## Chunk 4: Vocal Memory Contract

### Task 4: Add failing tests for stable vocal prototype sedimentation

**Files:**
- Create: `bionic_v0/tests/vocal_memory.rs`
- Create: `bionic_v0/src/vocal_memory.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
use bionic_v0::vocal_memory::VocalMemoryState;

#[test]
fn repeated_success_sediments_a_reusable_vocal_prototype() {
    let mut st = VocalMemoryState::default();
    for i in 0..5 {
        st.record_success(1_000 + i, 0.55, 0.40, 0.70);
    }
    assert!(st.prototypes.len() >= 1);
    assert!(st.prototypes[0].stability > 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_memory`

Expected: FAIL because `vocal_memory` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement:
- `VocalPrototype`
- `VocalMemoryState`
- `record_success(...)`
- `tick_decay(...)`

Required fields:
- prototype parameters matching `vocal_organ`
- `stability`
- `reuse_success`
- `last_update_t_us`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_memory`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/vocal_memory.rs bionic_v0/tests/vocal_memory.rs
git commit -m "feat: add vocal memory substrate"
```

---

## Chunk 5: Viewport + Snapshot Integration

### Task 5: Wire vocal substrates into viewport and snapshot with failing runtime tests first

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Create: `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write the failing test**

```rust
use std::sync::Arc;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn snapshot_roundtrip_preserves_vocal_substrate_state() {
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.7, 0.3, 0.4, 0.2);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.snapshot_bin).unwrap();
    assert!(snap.vocal_target.active_energy > 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime snapshot_roundtrip_preserves_vocal_substrate_state`

Expected: FAIL because snapshot and viewport do not yet expose vocal state.

- [ ] **Step 3: Write minimal implementation**

Add to `Viewport`:
- owned `RwLock` state for all four vocal substrates
- `observe_vocal_target`
- `read_vocal_target`
- `read_vocal_organ`
- `read_vocal_imitation`
- `read_vocal_memory`

Add to `snapshot`:
- new snapshot version fields for vocal substrates
- writer/reader helpers
- observer-safe numeric-only representations

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime snapshot_roundtrip_preserves_vocal_substrate_state`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/viewport.rs bionic_v0/src/snapshot.rs bionic_v0/tests/vocal_runtime.rs
git commit -m "feat: expose vocal substrates through viewport and snapshot"
```

---

## Chunk 6: Speaker-Facing Vocal Adapter

### Task 6: Add failing runtime tests for turning organ state into real audio output

**Files:**
- Create: `bionic_v0/src/vocal_adapter.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write the failing test**

```rust
use std::sync::Arc;
use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_adapter::sample_vocal_waveform;

#[test]
fn vocal_adapter_converts_organ_state_into_nonzero_audio_samples() {
    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(1_000, 0.6, 0.4, 0.7);
    let samples = sample_vocal_waveform(&vp, 128);
    assert!(samples.iter().any(|v| v.abs() > 0.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime vocal_adapter_converts_organ_state_into_nonzero_audio_samples`

Expected: FAIL because no vocal adapter exists yet.

- [ ] **Step 3: Write minimal implementation**

Implement in `vocal_adapter.rs`:
- numeric synthesis from `vocal_organ`
- no text/TTS path
- no human-readable output

Integrate into `body.rs`:
- register/start the vocal adapter thread
- keep speaker output external-device facing, not embedded in the substrate itself

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime vocal_adapter_converts_organ_state_into_nonzero_audio_samples`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/vocal_adapter.rs bionic_v0/src/bin/body.rs bionic_v0/tests/vocal_runtime.rs
git commit -m "feat: add speaker-facing vocal adapter"
```

---

## Chunk 7: Closed-Loop Imitation Update

### Task 7: Add failing runtime tests for external-imitation-first vocal behavior

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/vocal_target.rs`
- Modify: `bionic_v0/src/vocal_organ.rs`
- Modify: `bionic_v0/src/vocal_imitation.rs`
- Modify: `bionic_v0/src/vocal_memory.rs`
- Modify: `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write the failing test**

```rust
use std::sync::Arc;
use bionic_v0::viewport::Viewport;

#[test]
fn repeated_target_exposure_drives_organ_toward_target_and_strengthens_memory() {
    let vp = Arc::new(Viewport::new());
    for t in [1_000, 2_000, 3_000, 4_000, 5_000] {
        vp.observe_vocal_target(t, 0.8, 0.35, 0.60, 0.20);
        vp.tick_vocal_imitation_loop(t);
    }
    let organ = vp.read_vocal_organ();
    let mem = vp.read_vocal_memory();
    assert!(organ.breath_drive > 0.0);
    assert!(!mem.prototypes.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime repeated_target_exposure_drives_organ_toward_target_and_strengthens_memory`

Expected: FAIL because the closed-loop update is not wired yet.

- [ ] **Step 3: Write minimal implementation**

Add `Viewport` ticks:
- `tick_vocal_target_decay(now_us)`
- `tick_vocal_imitation_loop(now_us)`
- `tick_vocal_memory(now_us)`

Required loop behavior:
- external target present -> raise imitation pressure
- organ nudges toward target contour
- lower pressure when self-vs-target gap shrinks
- record stable successes into memory
- no fixed if/else “must speak now” command path

- [ ] **Step 4: Run test to verify it passes**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime repeated_target_exposure_drives_organ_toward_target_and_strengthens_memory`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/viewport.rs bionic_v0/src/vocal_target.rs bionic_v0/src/vocal_organ.rs bionic_v0/src/vocal_imitation.rs bionic_v0/src/vocal_memory.rs bionic_v0/tests/vocal_runtime.rs
git commit -m "feat: add closed-loop vocal imitation"
```

---

## Chunk 8: Observer Visibility and End-to-End Verification

### Task 8: Add observer output and final verification

**Files:**
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write the failing test**

```rust
use std::sync::Arc;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn observer_visible_snapshot_contains_vocal_activity_fields() {
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.9, 0.3, 0.5, 0.2);
    vp.tick_vocal_imitation_loop(2_000);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.snapshot_bin).unwrap();
    assert!(snap.vocal_imitation.imitation_pressure >= 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vocal_runtime observer_visible_snapshot_contains_vocal_activity_fields`

Expected: FAIL until snapshot and observer output are complete.

- [ ] **Step 3: Write minimal implementation**

In `observer.rs`, add compact fields for:
- vocal target activity
- vocal organ phase / rough parameters
- imitation pressure / suppression
- memory prototype count / top stability

Do not add words, labels, or “speaking meaning” output.

- [ ] **Step 4: Run full verification**

Run:

```bash
cd /Users/joe/Documents/unnamed/bionic_v0
cargo test
cargo check --bin body
cargo check --bin observer
```

Manual smoke:

```bash
cd /Users/joe/Documents/unnamed/bionic_v0
cargo run --bin body runtime
```

In a second terminal:

```bash
cd /Users/joe/Documents/unnamed/bionic_v0
cargo run --bin observer runtime 5
```

Expected:
- tests pass
- body still has no human-readable output
- observer shows numeric vocal substrate activity

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/Documents/unnamed
git add bionic_v0/src/bin/observer.rs bionic_v0/tests/vocal_runtime.rs
git commit -m "feat: expose vocal imitation state to observer"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-23-vocal-imitation-learning.md`. Ready to execute?
