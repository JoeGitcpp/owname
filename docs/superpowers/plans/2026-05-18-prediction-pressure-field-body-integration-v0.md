# Prediction Pressure Field Body Integration v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable the existing prediction pressure field inside the body refresh loop and prove its functional, timing, and stress behavior with numeric observer evidence.

**Architecture:** Add one `Viewport` method that hydrates the existing pressure field from existing numeric substrate state. Wire it into `body.rs` after active gap refresh and before probe law. Extend `curriculum_report` with a compact numeric evidence mode.

**Tech Stack:** Rust crate `bionic_v0`, cargo tests, observer-side `curriculum_report`, existing viewport/probe/action-output substrate.

---

## Files

- Create: `docs/superpowers/specs/2026-05-18-prediction-pressure-field-body-integration-v0-design.md`
- Create: `docs/superpowers/plans/2026-05-18-prediction-pressure-field-body-integration-v0.md`
- Create: `bionic_v0/tests/prediction_pressure_field_body_integration.rs`
- Modify: `bionic_v0/src/prediction_pressure_field.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

## Task 1: Failing Body Integration Tests

- [ ] **Step 1: Write tests first**

Add `bionic_v0/tests/prediction_pressure_field_body_integration.rs` with tests for:

- body substrate pressure hydrates at least gap/event/relation/residue waves,
- quiet substrate pressure leaves probe write epochs at zero,
- pressure-driven field tick can drive probe output without manual wave input,
- stress cycles keep wave count capped and focus switches bounded,
- source order in `body.rs` places field hydration after active gap and before probe law,
- source guard rejects forbidden architecture terms in the touched files.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_body_integration --test prediction_pressure_field_body_integration
```

Expected: FAIL because `tick_prediction_pressure_field_from_substrate` and body source wiring do not exist yet.

## Task 2: Implement Body Field Hydration

- [ ] **Step 1: Add the `Viewport` method**

Implement `Viewport::tick_prediction_pressure_field_from_substrate(now_us)` in
`bionic_v0/src/prediction_pressure_field.rs`.

It should:

- read active gap, evidence tension, relation state, and memory summary,
- map them into up to four `PredictionWaveInput` values,
- clamp all values to `[0,1]`,
- call `refresh_prediction_pressure_field(now_us, &inputs)`,
- avoid any human-readable body output or ontology terms.

- [ ] **Step 2: Wire body refresh order**

Modify `bionic_v0/src/bin/body.rs`:

```rust
vp.refresh_active_reasoning_gap(t);
vp.tick_prediction_pressure_field_from_substrate(t);
vp.refresh_probe_law(t);
```

Update the body refresh comment so it describes the numeric pressure field and keeps the cleanup guard.

- [ ] **Step 3: Verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_body_integration --test prediction_pressure_field_body_integration
```

Expected: PASS.

## Task 3: Numeric Report Mode

- [ ] **Step 1: Write report test first**

Add a test in `bionic_v0/tests/curriculum_report.rs` for:

```bash
curriculum_report --prediction-pressure-field-body-integration
```

Required output keys:

- `mode=prediction_pressure_field_body_integration`
- `body_ticks_run=`
- `quiet_probe_write_epochs=`
- `event_probe_write_epochs=`
- `waves_alive=`
- `dominant_focus_is_single=`
- `gap_wave_observed=`
- `event_wave_observed=`
- `relation_wave_observed=`
- `residue_wave_observed=`
- `closure_reduces_focused_wave=`
- `bounded_wave_count=`
- `focus_switches_bounded=`
- `stress_ticks=`
- `max_dominant_pressure=`
- `body_source_order_guard=`
- `principle_guard_pass=`

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_body_integration_report_emits_numeric_evidence --test curriculum_report
```

Expected: FAIL because the new flag is not parsed yet.

- [ ] **Step 3: Implement evidence runner and report flag**

Add `PredictionPressureFieldBodyIntegrationEvidence` and
`run_prediction_pressure_field_body_integration_curriculum` in
`bionic_v0/src/curriculum.rs`.

Update `bionic_v0/src/bin/curriculum_report.rs` to parse and print the new mode.

- [ ] **Step 4: Verify report GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml prediction_pressure_field_body_integration_report_emits_numeric_evidence --test curriculum_report
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --prediction-pressure-field-body-integration
```

Expected: PASS and compact numeric evidence only.

## Task 4: Full Verification

- [ ] **Step 1: Formatting and whitespace**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

- [ ] **Step 2: Full test suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all tests pass. Existing unrelated warnings are reported as warnings, not failures.

## Acceptance

The slice is enabled only if all are true:

- body hydration runs in the body refresh chain,
- report evidence is observer-side only,
- pressure field can drive probe output after body hydration,
- quiet conditions do not write,
- stress cycles keep wave count and focus switching bounded,
- no rejected architecture terms are introduced in touched pressure/body/report code,
- full `bionic_v0` tests pass.
