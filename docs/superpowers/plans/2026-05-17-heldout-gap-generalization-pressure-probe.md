# Held-Out Gap Generalization + Pressure-Driven Probe Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an observer-side curriculum report that proves held-out disturbance gap pressure drives probe activity and closes through numeric evidence.

**Architecture:** Reuse `Viewport` active gap, probe law, mark disturbance, language binding, and action-consequence substrates. Add no new body module; all orchestration and evidence aggregation lives in `bionic_v0/src/curriculum.rs` and the existing `curriculum_report` observer binary.

**Tech Stack:** Rust crate `bionic_v0`, cargo integration tests, existing observer report binary.

---

## Chunk 1: RED Tests

### Task 1: Curriculum Evidence Test

**Files:**
- Create: `bionic_v0/tests/heldout_gap_generalization.rs`

- [ ] **Step 1: Write failing test**

Add a test importing `run_heldout_gap_generalization_curriculum` and asserting the pass-standard numeric fields.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml heldout_gap_generalization --test heldout_gap_generalization`

Expected: FAIL because the function/evidence type does not exist yet.

### Task 2: Observer Report Test

**Files:**
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Write failing report test**

Add a test for `curriculum_report --heldout-gap-generalization` requiring all numeric evidence keys.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml heldout_gap_generalization_report_emits_numeric_evidence --test curriculum_report`

Expected: FAIL because the report flag/branch does not exist yet.

## Chunk 2: Curriculum Implementation

### Task 3: Add Evidence Struct And Runner

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add `HeldoutGapGeneralizationEvidence`**

Fields mirror the spec's observer evidence list.

- [ ] **Step 2: Add `run_heldout_gap_generalization_curriculum`**

Use ten trained marks, six held-out marks, existing target setup, active gap refresh,
probe law ticks, generic action-consequence closure, held-out correction, and cold
viewport comparison.

- [ ] **Step 3: Run focused test**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml heldout_gap_generalization --test heldout_gap_generalization`

Expected: PASS with all numeric thresholds satisfied.

## Chunk 3: Observer Report

### Task 4: Add Report Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Add import, flag parsing, and report branch**

Flag: `--heldout-gap-generalization`. Mode line:
`mode=heldout_gap_generalization`.

- [ ] **Step 2: Run report test**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml heldout_gap_generalization_report_emits_numeric_evidence --test curriculum_report`

Expected: PASS.

## Chunk 4: Verification

### Task 5: Full Verification

**Files:**
- Verify all touched files.

- [ ] **Step 1: Run observer binary**

Run: `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --heldout-gap-generalization`

Expected: numeric lines satisfying the pass standard.

- [ ] **Step 2: Format/check**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all commands exit 0.
