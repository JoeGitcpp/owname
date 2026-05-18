# Mark Disturbance Binding v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an external mark-disturbance binding path so byte-shaped marks can train reversible bindings to existing object substrate without becoming text/token ontology.

**Architecture:** Add `bionic_v0::mark_disturbance` as a numeric mark trace substrate owned by `Viewport`, then use existing `language_binding` events for binding. Extend external curriculum/report evidence to prove mark retention and reentry against cold baseline while keeping marks outside `body` semantics.

**Tech Stack:** Rust 2021, existing `Viewport`, `language_binding`, `snapshot`, `curriculum_report`, Cargo tests.

---

## File Structure

- Create: `bionic_v0/src/mark_disturbance.rs`
  - Owns mark trace structs, byte carrier observation, recurrence/stability updates, and safe binding-event construction.
- Modify: `bionic_v0/src/lib.rs`
  - Export `pub mod mark_disturbance;` and add snapshot version note if snapshot changes.
- Modify: `bionic_v0/src/viewport.rs`
  - Add `RwLock<MarkDisturbanceState>` and viewport methods.
- Modify: `bionic_v0/src/curriculum.rs`
  - Add mark-binding curriculum evidence helper.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add observer-side `--mark-binding` mode.
- Create: `bionic_v0/tests/mark_disturbance.rs`
  - Unit/integration tests for mark traces, binding, retention, ambiguity, and hygiene.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Report-output test.

Deferred from this v0 plan:

- `bionic_v0/src/snapshot.rs`
  - Numeric mark-disturbance snapshot summary can be added once mark traces
    need persisted observer snapshots. The first acceptance loop uses direct
    observer-side curriculum/report evidence only.

## Chunk 1: Mark Trace Substrate

### Task 1: Mark Trace State

**Files:**
- Create: `bionic_v0/tests/mark_disturbance.rs`
- Create: `bionic_v0/src/mark_disturbance.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Write failing test**

Add `byte_mark_observation_records_recurrence_stability_and_hash`:

```rust
let vp = Arc::new(Viewport::new());
let first = vp.observe_mark_bytes(7, 1_000, 1_100, b"alpha", 1.0);
let second = vp.observe_mark_bytes(7, 2_000, 2_100, b"alpha", 1.0);
let st = vp.read_mark_disturbance();
assert_eq!(st.n_traces, 1);
assert_eq!(first, second);
assert_eq!(st.traces[0].recurrence_count, 2);
assert!(st.traces[0].stability > 0.5);
assert!(st.traces[0].ambiguity < 0.5);
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test mark_disturbance byte_mark_observation_records_recurrence_stability_and_hash -- --nocapture
```

Expected: compile failure because mark APIs do not exist.

- [ ] **Step 3: Implement minimal substrate**

Add numeric carrier kind codes, `MarkTrace`, `MarkDisturbanceState`,
`observe_bytes`, and viewport wrappers. No text parser, tokenizer, or word
fields.

- [ ] **Step 4: Verify GREEN**

Run the same test. Expected: PASS.

## Chunk 2: Binding And Ambiguity

### Task 2: Bind Mark To Existing Object

**Files:**
- Modify: `bionic_v0/tests/mark_disturbance.rs`
- Modify: `bionic_v0/src/mark_disturbance.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Write failing test**

Add `aligned_mark_events_strengthen_symbol_binding_for_live_object_slot`.
Claim a live slot, observe the same mark several times, call
`bind_latest_mark_to_target(..., BindingFeedback::Align, ...)`, and assert
`language_binding` contains one `PatternKind::SymbolSeq` trace whose strength
increases.

- [ ] **Step 2: Verify RED**

Run the focused test. Expected: compile failure for missing binding helper.

- [ ] **Step 3: Implement minimal binding helper**

Add a viewport method that reads the latest mark trace and applies a
`BindingEvent` to an existing target. The helper must not bind inactive targets
and must not invent feedback.

- [ ] **Step 4: Verify GREEN**

Run the focused test. Expected: PASS.

### Task 3: Contradictory Evidence Preserves Ambiguity

**Files:**
- Modify: `bionic_v0/tests/mark_disturbance.rs`
- Modify: `bionic_v0/src/mark_disturbance.rs`

- [ ] **Step 1: Write failing test**

Add `contradictory_mark_target_evidence_raises_ambiguity_without_rewriting_slots`.
Bind one mark to slot A, then submit deviating evidence for slot B. Assert
binding strength for A weakens or mark ambiguity rises, and both slots remain
active.

- [ ] **Step 2: Verify RED**

Run the focused test. Expected: fail until ambiguity update is added.

- [ ] **Step 3: Implement ambiguity update**

Keep ambiguity numeric and trace-local. Do not store target names or labels.

- [ ] **Step 4: Verify GREEN**

Run the focused test. Expected: PASS.

## Chunk 3: Curriculum And Report

### Task 4: Mark Binding Curriculum Evidence

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/tests/mark_disturbance.rs`

- [ ] **Step 1: Write failing test**

Add `mark_binding_curriculum_proves_hold_and_reentry_against_cold_baseline`.
Assert train forms binding, hold keeps strength ratio, and reentry persistent
strength exceeds cold baseline.

- [ ] **Step 2: Verify RED**

Run focused test. Expected: missing curriculum function.

- [ ] **Step 3: Implement minimal curriculum**

Use a same-Viewport synthetic object slot and byte mark carrier. Report train,
hold, reentry, cold baseline, binding strength, recurrence, and ambiguity.

- [ ] **Step 4: Verify GREEN**

Run focused test. Expected: PASS.

### Task 5: Report Mode And Hygiene

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/tests/mark_disturbance.rs`

- [ ] **Step 1: Write failing report test**

Add `mark_binding_report_emits_numeric_stage_evidence` using
`curriculum_report --mark-binding`.

- [ ] **Step 2: Verify RED**

Run focused report test. Expected: missing mode.

- [ ] **Step 3: Implement report mode**

Print compact numeric lines only.

- [ ] **Step 4: Add hygiene test**

Scan `mark_disturbance.rs`, `curriculum.rs`, and `curriculum_report.rs` for
rejected architecture terms and body-side output drift.

- [ ] **Step 5: Full verification**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test mark_disturbance -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```

Expected: PASS.
