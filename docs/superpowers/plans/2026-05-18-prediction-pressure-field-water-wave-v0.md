# Prediction Pressure Field Water-Wave v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a small numeric pressure-field substrate where multiple waves rise, decay, resonate, compete, and yield one dominant active prediction focus that can drive existing probe law.

**Architecture:** Create a focused `prediction_pressure_field` module, add one `RwLock` state to `Viewport`, and let `probe_law` read the field's dominant pressure as one drive source. Observer evidence lives in tests and `curriculum_report`; no snapshot schema change in v0.

**Tech Stack:** Rust crate `bionic_v0`, cargo integration tests, existing observer-side curriculum report binary.

---

## Chunk 1: RED Tests

### Task 1: Field Dynamics Test

**Files:**
- Create: `bionic_v0/tests/prediction_pressure_field.rs`

- [ ] **Step 1: Write failing tests**

Add tests for:
- multiple waves coexist and produce one dominant focus;
- inertia bounds focus switching;
- quiet reentry raises a retained wave;
- event pressure can override idle residue;
- relation resonance spikes a wave;
- closure reduces focused wave and unclosed residue remains;
- dominant pressure drives existing probe law;
- source hygiene contains no rejected ontology terms.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field --test prediction_pressure_field`

Expected: FAIL because the module and APIs do not exist.

### Task 2: Report Test

**Files:**
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add failing report test**

Add `--prediction-pressure-field-water-wave` report test requiring all pass-standard fields.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_water_wave_report_emits_numeric_evidence --test curriculum_report`

Expected: FAIL because the report mode does not exist.

## Chunk 2: Substrate Implementation

### Task 3: Add Numeric Field Module

**Files:**
- Create: `bionic_v0/src/prediction_pressure_field.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Implement state and update law**

Add bounded wave state, update input, refresh method, closure method, and read method. Use numeric coordinates only.

- [ ] **Step 2: Wire Viewport**

Add `prediction_pressure_field: RwLock<PredictionPressureFieldState>` and initialize it in `Viewport::new()`.

- [ ] **Step 3: Verify focused tests**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field --test prediction_pressure_field`

Expected: PASS.

## Chunk 3: Probe and Report

### Task 4: Connect To Probe Law

**Files:**
- Modify: `bionic_v0/src/probe_law.rs`

- [ ] **Step 1: Include dominant field pressure in drive**

Read `self.read_prediction_pressure_field().dominant_pressure` and include it in the max drive chain.

- [ ] **Step 2: Verify probe test**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_dominant_pressure_drives_probe --test prediction_pressure_field`

Expected: PASS.

### Task 5: Add Observer Report

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add evidence runner**

Add `PredictionPressureFieldWaterWaveEvidence` and `run_prediction_pressure_field_water_wave_curriculum`.

- [ ] **Step 2: Add report flag**

Flag: `--prediction-pressure-field-water-wave`. Mode: `prediction_pressure_field_water_wave`.

- [ ] **Step 3: Verify report test**

Run: `cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_water_wave_report_emits_numeric_evidence --test curriculum_report`

Expected: PASS.

## Chunk 4: Verification

### Task 6: Full Verification

- [ ] **Step 1: Run report**

Run: `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --prediction-pressure-field-water-wave`

Expected: numeric evidence satisfying pass standard.

- [ ] **Step 2: Run checks**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all exit 0.
