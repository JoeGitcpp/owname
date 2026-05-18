# Multi-Gap Selection + Pixel Scale Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an observer-side curriculum/report proving that competing gaps select the highest-pressure probe target and that larger prediction scale persists more strongly than small local gaps.

**Architecture:** Reuse existing `Viewport`, active gap, probe law, language binding, mark disturbance, action-consequence, and cross-channel helper paths. Keep scale and selection evidence in curriculum/report structs only; no new body ontology or active reasoning fields.

**Tech Stack:** Rust crate `bionic_v0`, cargo integration tests, existing `curriculum_report` observer binary.

---

## Chunk 1: RED Tests

### Task 1: Curriculum Evidence Test

**Files:**
- Create: `bionic_v0/tests/multi_gap_selection_pixel_scale.rs`

- [ ] **Step 1: Write the failing test**

Import `run_multi_gap_selection_pixel_scale_curriculum` and assert the pass-standard fields from the spec.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml multi_gap_selection_pixel_scale --test multi_gap_selection_pixel_scale`

Expected: FAIL because the function does not exist.

### Task 2: Observer Report Test

**Files:**
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Write failing report test**

Add a test for `curriculum_report --multi-gap-selection-pixel-scale` and require every compact numeric field.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml multi_gap_selection_pixel_scale_report_emits_numeric_evidence --test curriculum_report`

Expected: FAIL because the report flag does not exist.

## Chunk 2: Curriculum Implementation

### Task 3: Add Evidence Struct And Runner

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add `MultiGapSelectionPixelScaleEvidence`**

Fields mirror the observer evidence list in the spec.

- [ ] **Step 2: Add `run_multi_gap_selection_pixel_scale_curriculum`**

Train small/mid/large materials, introduce competing held-out gaps, compute numeric pressure, refresh existing active gap/probe-law paths for the selected gap, close it, and compare persistent vs cold large-scale reentry.

- [ ] **Step 3: Verify GREEN**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml multi_gap_selection_pixel_scale --test multi_gap_selection_pixel_scale`

Expected: PASS.

## Chunk 3: Observer Report

### Task 4: Add Report Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Add import, flag, parse tuple field, and report branch**

Flag: `--multi-gap-selection-pixel-scale`. Mode line:
`mode=multi_gap_selection_pixel_scale`.

- [ ] **Step 2: Verify report**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml multi_gap_selection_pixel_scale_report_emits_numeric_evidence --test curriculum_report`

Expected: PASS.

## Chunk 4: Verification

### Task 5: Full Verification

**Files:**
- Verify all touched files.

- [ ] **Step 1: Run observer binary**

Run: `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --multi-gap-selection-pixel-scale`

Expected: numeric report satisfying the pass standard.

- [ ] **Step 2: Run checks**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all commands exit 0.
