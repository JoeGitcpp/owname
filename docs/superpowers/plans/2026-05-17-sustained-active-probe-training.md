# Sustained Active Probe Training Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a curriculum-only sustained active probe training course that repeats probe-return-correction closure across several cycles and reports numeric evidence.

**Architecture:** Extend `bionic_v0/src/curriculum.rs` with one observer-side evidence struct and one curriculum function. Extend `curriculum_report` with a dedicated flag and compact numeric output. Tests drive behavior first and keep forbidden ontology terms guarded.

**Tech Stack:** Rust crate `bionic_v0`, Cargo tests, existing viewport/curriculum/report modules.

---

### Task 1: Add Failing Sustained Active Probe Tests

**Files:**
- Modify: `bionic_v0/tests/active_text_probe_closure.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add the curriculum behavior test**

Append a test that imports and calls:

```rust
run_sustained_active_probe_training_curriculum(CurriculumConfig::default())
```

Assert:

```rust
assert_eq!(evidence.cycles_run, 4);
assert!(evidence.conflict_events >= 8);
assert!(evidence.probe_frames >= 8);
assert!(evidence.return_windows >= 8);
assert!(evidence.paired_consequences >= 8);
assert!(evidence.probe_pair_similarity_avg > 0.50);
assert!(evidence.min_ambiguity_reduction_ratio > 0.10);
assert!(evidence.min_correction_recovery_ratio >= 0.80);
assert!(evidence.max_wrong_binding_strength <= 0.01);
assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
```

- [ ] **Step 2: Add the report output test**

Add a `curriculum_report` test invoking:

```bash
curriculum_report --sustained-active-probe-training
```

Assert stdout contains all sustained evidence keys.

- [ ] **Step 3: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe_training_report_emits_numeric_stage_evidence
```

Expected: compile failure or unresolved import because the new function and
report flag do not exist yet.

### Task 2: Implement Sustained Active Probe Curriculum

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add the evidence struct**

Add:

```rust
pub struct SustainedActiveProbeTrainingEvidence {
    pub cycles_run: u32,
    pub conflict_events: u32,
    pub probe_frames: u32,
    pub probe_write_epochs: u32,
    pub return_windows: u32,
    pub paired_consequences: u32,
    pub probe_pair_similarity_avg: f64,
    pub min_ambiguity_reduction_ratio: f64,
    pub min_correction_recovery_ratio: f64,
    pub max_wrong_binding_strength: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
}
```

- [ ] **Step 2: Add the curriculum function**

Implement `run_sustained_active_probe_training_curriculum` by reusing existing
helpers: `setup_text_grounding_targets`, `bind_mark_set`,
`active_probe_bytes`, `symbol_binding_strength`, `mark_ambiguity`,
`average_pair_similarity`, and `ratio_or_zero`.

Use one persistent viewport, four cycles, two conflicts per cycle, two probe
write/return pairs per cycle, two correction alignments per cycle, and a cold
reentry baseline.

- [ ] **Step 3: Verify GREEN for behavior**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe_training_proves_repeated_probe_return_correction_and_reentry
```

Expected: PASS.

### Task 3: Add Report Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Import and parse flag**

Import `run_sustained_active_probe_training_curriculum` and add
`--sustained-active-probe-training`.

- [ ] **Step 2: Print compact evidence**

Print:

```text
mode=sustained_active_probe_training
cycles_run=
conflict_events=
probe_frames=
probe_write_epochs=
return_windows=
paired_consequences=
probe_pair_similarity_avg=
min_ambiguity_reduction_ratio=
min_correction_recovery_ratio=
max_wrong_binding_strength=
reentry_persistent_strength=
reentry_cold_strength=
```

- [ ] **Step 3: Verify report test**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe_training_report_emits_numeric_stage_evidence
```

Expected: PASS.

### Task 4: Final Verification

**Files:**
- No further edits unless verification fails.

- [ ] **Step 1: Run report**

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --sustained-active-probe-training
```

Expected: mode and all numeric evidence keys are printed.

- [ ] **Step 2: Run formatting and diff checks**

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

Expected: both pass.

- [ ] **Step 3: Run full tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.

