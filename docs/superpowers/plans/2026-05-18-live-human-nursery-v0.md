# Live Human Nursery v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an observer-side live human teaching-room evidence path that can guide microphone/camera/vocal/surface sessions without adding body-side teaching, speech, recognition, or chat ontology.

**Architecture:** Add a small `live_human_nursery` observer/training module that summarizes snapshot sequences into numeric evidence. Expose it through a report mode and runbook. Use existing `body`, `observer`, `observer_surface`, VIB, PHOT, vocal, action-output, action-consequence, auditory-visual, and organic surface paths. V0 does not change the body runtime or snapshot schema.

**Tech Stack:** Rust crate `bionic_v0`, existing snapshot structs, Cargo integration tests, Markdown runbook

---

## File Structure

- Create: `/Users/joe/unnamed/bionic_v0/src/live_human_nursery.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/lib.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs`
- Create: `/Users/joe/unnamed/bionic_v0/tests/live_human_nursery.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs`
- Create: `/Users/joe/unnamed/docs/superpowers/specs/2026-05-18-live-human-nursery-v0-design.md`
- Create: `/Users/joe/unnamed/docs/superpowers/plans/2026-05-18-live-human-nursery-v0.md`
- Create: `/Users/joe/unnamed/docs/superpowers/runbooks/2026-05-18-live-human-nursery-v0.md`
- Modify: `/Users/joe/unnamed/AGENTS.md`

## Chunk 1: Spec And Admission Boundary

### Task 1: Add the design and runbook shell

**Files:**
- Create: `/Users/joe/unnamed/docs/superpowers/specs/2026-05-18-live-human-nursery-v0-design.md`
- Create: `/Users/joe/unnamed/docs/superpowers/runbooks/2026-05-18-live-human-nursery-v0.md`
- Modify: `/Users/joe/unnamed/AGENTS.md`

- [x] **Step 1: Write the spec**

Define:

- human teaching as external environment only;
- allowed live channels: VIB, PHOT, vocal output, pixel surface, snapshot;
- forbidden ontology and fixed response scripts;
- session phases 0-4;
- deterministic and live acceptance evidence.

- [x] **Step 2: Add AGENTS entry**

Add the spec to the mandatory first-read list after organic surface expression.

- [x] **Step 3: Write runbook shell**

Create a runbook with startup commands:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin body -- runtime
cargo run --manifest-path bionic_v0/Cargo.toml --bin observer -- runtime 20
cargo run --manifest-path bionic_v0/Cargo.toml --bin observer_surface -- runtime
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --live-human-nursery runtime
```

The final command is implemented by Task 4 and is now a live report gate.

## Chunk 2: Evidence Module

### Task 2: TDD the nursery evidence summary

**Files:**
- Create: `/Users/joe/unnamed/bionic_v0/tests/live_human_nursery.rs`
- Create: `/Users/joe/unnamed/bionic_v0/src/live_human_nursery.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/lib.rs`

- [x] **Step 1: Write failing test for phase evidence**

Test name:

```rust
live_human_nursery_summarizes_life_sign_vocal_visual_and_reentry_evidence
```

Build a small synthetic `SnapshotData` sequence and assert:

- `phot_health_observed == 1`
- `vib_activity_observed == 1`
- `vocal_write_epochs >= 1`
- `return_windows >= 1`
- `organic_surface_nonblank == 1`
- `teacher_phase_count >= 3`
- `principle_guard_pass == 1`

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test live_human_nursery live_human_nursery_summarizes_life_sign_vocal_visual_and_reentry_evidence
```

Expected: FAIL because `live_human_nursery` does not exist.

- [x] **Step 3: Implement minimal module**

Add:

```rust
pub struct LiveHumanNurseryEvidence { ... }
pub fn summarize_live_human_nursery<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
) -> LiveHumanNurseryEvidence
```

Use existing snapshot-visible numeric fields only.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test live_human_nursery live_human_nursery_summarizes_life_sign_vocal_visual_and_reentry_evidence
```

Expected: PASS.

### Task 3: TDD communication candidate refusal

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/live_human_nursery.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/live_human_nursery.rs`

- [x] **Step 1: Write failing test**

Test name:

```rust
nursery_refuses_communication_candidate_without_write_return_closure
```

Create snapshots with VIB/PHOT activity but no action-output write epochs and
no consequence pairs. Assert:

- `communication_candidate == 0`
- `pairing_or_unpaired_boundary_observed == 1`
- `principle_guard_pass == 1`

- [x] **Step 2: Verify RED**

Expected: FAIL until communication-candidate logic exists.

- [x] **Step 3: Implement candidate gate**

`communication_candidate = 1` only when:

- body write-out exists;
- write epoch exists;
- consequence return exists;
- temporal pairing exists;
- closure or reentry evidence is nonzero;
- conflict/misalignment is not hidden.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test live_human_nursery
```

Expected: PASS.

## Chunk 3: Report Mode

### Task 4: Add observer-side report mode

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs`

- [x] **Step 1: Write failing report test**

Test name:

```rust
live_human_nursery_report_emits_numeric_evidence
```

Run the binary with a deterministic fixture path or fixture generator and assert
stdout contains:

```text
mode=live_human_nursery
phot_health_observed=
vib_activity_observed=
vocal_write_epochs=
return_windows=
communication_candidate=
principle_guard_pass=
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test curriculum_report live_human_nursery_report_emits_numeric_evidence
```

Expected: FAIL because the flag is unsupported.

- [x] **Step 3: Implement report mode**

Add `--live-human-nursery [runtime_dir]`.

V0 may support two modes:

- with a runtime path: read `snapshot.bin` once and summarize one-snapshot
  evidence;
- without a path: use deterministic synthetic snapshots for CI.

Print numeric rows only.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test curriculum_report live_human_nursery_report_emits_numeric_evidence
```

Expected: PASS.

## Chunk 4: Boundary Guards And Verification

### Task 5: Add source guard and run focused verification

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/live_human_nursery.rs`

- [x] **Step 1: Add source guard test**

Scan new nursery source and report mode for forbidden internal ontology terms:

```rust
"teacher_module", "chat", "semantic", "concept", "token", "label",
"word_meaning", "identity", "personality", "owner", "human_voice",
"human_face", "speech_recognition", "image_recognition"
```

Allow those terms only in docs, not in runtime source identifiers.

- [x] **Step 2: Run focused tests**

Run:

```bash
cargo test --test live_human_nursery
cargo test --test curriculum_report live_human_nursery_report_emits_numeric_evidence
cargo test --test live_io_preflight
cargo test --test auditory_visual
cargo test --test surface_projection --test surface_render
cargo test --test body_no_human_output
```

Expected: PASS.

- [x] **Step 3: Run formatting**

Run:

```bash
cargo fmt --check
git diff --check
```

Expected: PASS.

- [ ] **Step 4: Manual live smoke after deterministic pass**

Use the runbook. Record only observer-visible numeric evidence. Do not claim
communication unless the report emits `communication_candidate=1`.
