# Background Relation Resonance / Sudden-Match v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add background relation resonance traces so low-match relation attempts decay and high-match relations can spike into the active surface through the prediction pressure field.

**Architecture:** Extend `active_surface.rs` with numeric relation-resonance traces. Add one `Viewport` tick that reads relation edges and active-surface residue, updates background attempts, injects bounded relation waves on high match, then let the existing active surface aperture/echo rule arbitrate execution.

**Tech Stack:** Rust crate `bionic_v0`, cargo tests, observer-side `curriculum_report`, existing `Viewport`, `GlobalRelationState`, `prediction_pressure_field`, and `active_surface`.

---

## Files

- Create: `docs/superpowers/specs/2026-05-18-background-relation-resonance-sudden-match-v0-design.md`
- Create: `docs/superpowers/plans/2026-05-18-background-relation-resonance-sudden-match-v0.md`
- Create: `bionic_v0/tests/background_relation_resonance.rs`
- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

## Task 1: Failing Tests

- [ ] **Step 1: Write tests first**

Add `bionic_v0/tests/background_relation_resonance.rs` covering:

- background attempts can remain alive,
- low-match attempts decay and do not enter aperture,
- high-match attempts spike,
- sudden match enters active surface aperture or echo,
- wrong-match leakage remains bounded,
- body order is pressure field -> relation resonance -> active surface -> probe law,
- source guard rejects forbidden architecture terms.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml background_relation_resonance --test background_relation_resonance
```

Expected: FAIL because relation resonance state and tick do not exist yet.

## Task 2: Implement Relation Resonance

- [ ] **Step 1: Extend active surface state**

In `bionic_v0/src/active_surface.rs`, add:

- `RelationResonanceTrace`,
- state fields for traces, counts, spike evidence, leakage,
- helper evidence methods.

- [ ] **Step 2: Add body tick method**

Add `Viewport::tick_active_surface_relation_resonance(now_us)`.

It should:

- read active surface, relation edges, and memory summary,
- update background traces,
- decay low-match attempts,
- inject one bounded `PREDICTION_WAVE_RELATION` wave for high match,
- never write human-readable output.

- [ ] **Step 3: Wire body order**

Modify `bionic_v0/src/bin/body.rs`:

```rust
vp.tick_prediction_pressure_field_from_substrate(t);
vp.tick_active_surface_relation_resonance(t);
vp.refresh_active_surface(t);
vp.refresh_probe_law(t);
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml background_relation_resonance --test background_relation_resonance
```

## Task 3: Numeric Report

- [ ] **Step 1: Write report test first**

Add `background_relation_resonance_report_emits_numeric_evidence` to
`bionic_v0/tests/curriculum_report.rs`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml background_relation_resonance_report_emits_numeric_evidence --test curriculum_report
```

- [ ] **Step 3: Add runner and report flag**

Add `BackgroundRelationResonanceEvidence` and
`run_background_relation_resonance_curriculum` in `bionic_v0/src/curriculum.rs`.

Add:

```text
--background-relation-resonance-sudden-match
```

to `curriculum_report`.

- [ ] **Step 4: Verify report**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml background_relation_resonance_report_emits_numeric_evidence --test curriculum_report
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --background-relation-resonance-sudden-match
```

## Task 4: Full Verification

- [ ] **Step 1: Format and diff checks**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

- [ ] **Step 2: Full suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```
