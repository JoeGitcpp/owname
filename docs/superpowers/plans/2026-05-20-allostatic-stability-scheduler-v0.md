# Allostatic Stability Scheduler v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add observer-side v0 evidence and reporting for the `应变稳态调度系统` without adding body-side emotion or personality state.

**Architecture:** Implement a focused `allostatic_stability` module that derives bounded resource, pressure, overdrive, downshift, and recovery evidence from existing `SnapshotData`. Add one `curriculum_report --allostatic-stability [runtime_dir]` mode. Keep this phase observer-side only; no `Viewport` writes and no probe-law modulation yet.

**Tech Stack:** Rust, Cargo integration tests, existing `SnapshotData`, `curriculum_report`, and current source-guard tests.

---

## File Structure

- Create: `bionic_v0/src/allostatic_stability.rs`
  - Defines `AllostaticStabilityEvidence`.
  - Exposes `summarize_allostatic_stability(...)`.
  - Computes bounded numeric evidence from existing snapshot fields only.
- Modify: `bionic_v0/src/lib.rs`
  - Exports `allostatic_stability`.
- Create: `bionic_v0/tests/allostatic_stability.rs`
  - Tests bounded evidence, high-performance burst, protective downshift, and source hygiene.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Adds `--allostatic-stability [runtime_dir]`.
  - Prints deterministic/runtime provenance and scheduler evidence.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Tests report output for the new mode.

## Task 1: Observer-Side Allostatic Evidence

**Files:**
- Create: `bionic_v0/src/allostatic_stability.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/allostatic_stability.rs`

- [ ] **Step 1: Write the failing bounded evidence test**

Create `bionic_v0/tests/allostatic_stability.rs` with tests that import:

```rust
use bionic_v0::allostatic_stability::summarize_allostatic_stability;
use bionic_v0::live_human_nursery::sample_live_human_nursery_snapshots;
```

Add `allostatic_stability_scores_are_bounded_and_observer_side`:

```rust
#[test]
fn allostatic_stability_scores_are_bounded_and_observer_side() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let snap = snapshots.last_mut().expect("sample snapshot");
    snap.evidence_tension.unresolved_pressure = 0.46;
    snap.active_reasoning_gap.gap_pressure = 0.52;
    snap.active_reasoning_gap.closure_score = 0.40;
    snap.self_development.familiarity_pressure = 0.48;
    snap.self_development.exploratory_pressure = 0.42;
    snap.global_reasoning_memory.routing_stability_score = 0.44;
    snap.probe_law.intensity = 0.50;

    let evidence = summarize_allostatic_stability(snapshots.iter(), 1, 0);

    for value in [
        evidence.arousal_load,
        evidence.reserve_level,
        evidence.recovery_debt,
        evidence.bearing_capacity,
        evidence.exploration_allowance,
        evidence.expression_gain,
        evidence.protective_limit,
        evidence.overdrive_window,
        evidence.downshift_pressure,
    ] {
        assert!(value.is_finite());
        assert!((0.0..=1.0).contains(&value));
    }
    assert_eq!(evidence.deterministic_fixture, 1);
    assert_eq!(evidence.runtime_snapshot_read, 0);
    assert_eq!(evidence.principle_guard_pass, 1);
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability allostatic_stability_scores_are_bounded_and_observer_side
```

Expected: FAIL because `bionic_v0::allostatic_stability` does not exist.

- [ ] **Step 3: Implement minimal evidence type and bounded summary**

Create `bionic_v0/src/allostatic_stability.rs` with:

```rust
use crate::snapshot::SnapshotData;
use crate::{CONSEQUENCE_WINDOW_CAP, PAIR_HISTORY_CAP, WRITE_EPOCH_HISTORY_CAP};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AllostaticStabilityEvidence {
    pub deterministic_fixture: u8,
    pub runtime_snapshot_read: u8,
    pub snapshots_seen: u32,
    pub arousal_load: f64,
    pub reserve_level: f64,
    pub recovery_debt: f64,
    pub bearing_capacity: f64,
    pub exploration_allowance: f64,
    pub expression_gain: f64,
    pub protective_limit: f64,
    pub overdrive_window: f64,
    pub downshift_pressure: f64,
    pub principle_guard_pass: u8,
}

pub fn summarize_allostatic_stability<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
    deterministic_fixture: u8,
    runtime_snapshot_read: u8,
) -> AllostaticStabilityEvidence {
    let mut evidence = AllostaticStabilityEvidence {
        deterministic_fixture,
        runtime_snapshot_read,
        ..Default::default()
    };

    for snap in snapshots {
        evidence.snapshots_seen = evidence.snapshots_seen.saturating_add(1);
        let sample = sample_snapshot(snap);
        evidence.arousal_load = evidence.arousal_load.max(sample.arousal_load);
        evidence.recovery_debt = evidence.recovery_debt.max(sample.recovery_debt);
        evidence.bearing_capacity = evidence.bearing_capacity.max(sample.bearing_capacity);
        evidence.exploration_allowance = evidence
            .exploration_allowance
            .max(sample.exploration_allowance);
        evidence.expression_gain = evidence.expression_gain.max(sample.expression_gain);
        evidence.protective_limit = evidence.protective_limit.max(sample.protective_limit);
        evidence.overdrive_window = evidence.overdrive_window.max(sample.overdrive_window);
        evidence.downshift_pressure = evidence.downshift_pressure.max(sample.downshift_pressure);
        evidence.reserve_level = evidence.reserve_level.max(sample.reserve_level);
    }

    evidence.principle_guard_pass =
        u8::from(evidence.snapshots_seen > 0 && all_bounded(&evidence));
    evidence
}
```

Add private helper functions for `sample_snapshot`, `demand`, `support`, `latest_pair_similarity`, `relation_surprise`, `load_ratio`, `all_bounded`, `ratio`, and `unit`.

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod allostatic_stability;
```

- [ ] **Step 4: Run bounded evidence test to verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability allostatic_stability_scores_are_bounded_and_observer_side
```

Expected: PASS.

## Task 2: Overdrive And Downshift Evidence

**Files:**
- Modify: `bionic_v0/tests/allostatic_stability.rs`
- Modify: `bionic_v0/src/allostatic_stability.rs`

- [ ] **Step 1: Add high-performance burst test**

Add `moderate_demand_with_support_opens_overdrive_and_expression_gain`:

```rust
#[test]
fn moderate_demand_with_support_opens_overdrive_and_expression_gain() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let snap = snapshots.last_mut().expect("sample snapshot");
    snap.evidence_tension.unresolved_pressure = 0.58;
    snap.active_reasoning_gap.gap_pressure = 0.56;
    snap.active_reasoning_gap.closure_score = 0.62;
    snap.global_reasoning_memory.routing_stability_score = 0.66;
    snap.global_reasoning_memory.pattern_recurrence_score = 0.61;
    snap.self_development.familiarity_pressure = 0.60;
    snap.self_development.stable_recurrence = 0.64;
    snap.self_development.output_suppression = 0.10;
    snap.probe_law.output_intensity = 0.42;

    let evidence = summarize_allostatic_stability(snapshots.iter(), 1, 0);

    assert!(evidence.reserve_level > 0.35);
    assert!(evidence.expression_gain > evidence.protective_limit);
    assert!(evidence.exploration_allowance > 0.10);
    assert!(evidence.overdrive_window > 0.0);
}
```

- [ ] **Step 2: Add protective downshift test**

Add `low_support_and_recovery_debt_raise_downshift`:

```rust
#[test]
fn low_support_and_recovery_debt_raise_downshift() {
    let mut snapshots = sample_live_human_nursery_snapshots();
    let snap = snapshots.last_mut().expect("sample snapshot");
    snap.evidence_tension.unresolved_pressure = 0.88;
    snap.active_reasoning_gap.gap_pressure = 0.82;
    snap.active_reasoning_gap.closure_score = 0.02;
    snap.framed_byte_egress.correction_deviation = 0.80;
    snap.self_development.repetition_pressure = 0.78;
    snap.self_development.output_suppression = 0.74;
    snap.self_development.familiarity_pressure = 0.03;
    snap.global_reasoning_memory.routing_stability_score = 0.02;
    snap.probe_law.suppression = 0.76;

    let evidence = summarize_allostatic_stability(snapshots.iter(), 1, 0);

    assert!(evidence.recovery_debt > 0.50);
    assert!(evidence.reserve_level < 0.50);
    assert!(evidence.protective_limit > evidence.expression_gain);
    assert!(evidence.downshift_pressure > 0.40);
}
```

- [ ] **Step 3: Run tests to verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability moderate_demand_with_support_opens_overdrive_and_expression_gain
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability low_support_and_recovery_debt_raise_downshift
```

Expected: FAIL until formulas encode overdrive and downshift behavior.

- [ ] **Step 4: Refine formulas**

Make `sample_snapshot` compute:

```text
demand = max pressure/deviation/output drive
support = max closure/pair/familiarity/stability
stress_load = demand * (1 - support)
recovery_debt = max repetition, correction-load, unresolved high-stress load
reserve_level = 1 - max(recovery_debt, stress_load * 0.70)
protective_limit = max output_suppression/probe_suppression/recovery pressure
expression_gain = demand * reserve * (0.35 + 0.65 * support)
exploration_allowance = exploratory/gap pressure * reserve * (1 - 0.50 * protective_limit)
overdrive_window = demand * reserve * support when demand/support/reserve are all present
downshift_pressure = max(recovery_debt, stress_load) * (1 - reserve + protective_limit) / 2
```

- [ ] **Step 5: Run all allostatic tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability
```

Expected: PASS.

## Task 3: Source Guard

**Files:**
- Modify: `bionic_v0/tests/allostatic_stability.rs`

- [ ] **Step 1: Add source guard test**

Add `allostatic_stability_sources_do_not_introduce_emotion_or_personality_state`:

```rust
#[test]
fn allostatic_stability_sources_do_not_introduce_emotion_or_personality_state() {
    let src = include_str!("../src/allostatic_stability.rs");
    for forbidden in [
        "emotion_state",
        "happy_state",
        "sad_state",
        "anger_module",
        "fear_module",
        "personality_emotion",
        "semantic_readiness",
        "concept_id",
        "token_id",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "allostatic evidence must not contain `{forbidden}`"
        );
    }
}
```

- [ ] **Step 2: Run source guard**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability allostatic_stability_sources_do_not_introduce_emotion_or_personality_state
```

Expected: PASS after module exists and contains no forbidden strings.

## Task 4: Curriculum Report Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Write failing report test**

Add `allostatic_stability_report_emits_numeric_evidence` to `bionic_v0/tests/curriculum_report.rs`:

```rust
#[test]
fn allostatic_stability_report_emits_numeric_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--allostatic-stability")
        .output()
        .expect("run curriculum_report allostatic stability");

    assert!(
        output.status.success(),
        "allostatic stability report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=allostatic_stability"));
    assert!(stdout.contains("deterministic_fixture=1"));
    assert!(stdout.contains("runtime_snapshot_read=0"));
    assert!(stdout.contains("arousal_load="));
    assert!(stdout.contains("reserve_level="));
    assert!(stdout.contains("recovery_debt="));
    assert!(stdout.contains("bearing_capacity="));
    assert!(stdout.contains("exploration_allowance="));
    assert!(stdout.contains("expression_gain="));
    assert!(stdout.contains("protective_limit="));
    assert!(stdout.contains("overdrive_window="));
    assert!(stdout.contains("downshift_pressure="));
    assert!(stdout.contains("principle_guard_pass="));
}
```

- [ ] **Step 2: Run report test to verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report allostatic_stability_report_emits_numeric_evidence
```

Expected: FAIL because the report mode is missing.

- [ ] **Step 3: Implement report mode**

Add helpers:

```rust
fn report_flag_present(flag: &str) -> bool { ... }
fn runtime_dir_arg(flag: &str) -> Option<PathBuf> { ... }
fn print_allostatic_stability(runtime_dir_arg: Option<PathBuf>) -> std::io::Result<()> { ... }
```

`print_allostatic_stability` must use runtime `snapshot.bin` when a runtime dir is supplied, otherwise use `sample_live_human_nursery_snapshots()`.

Add a main dispatch branch:

```rust
} else if report_flag_present("--allostatic-stability") {
    print_allostatic_stability(runtime_dir_arg("--allostatic-stability"))?;
```

- [ ] **Step 4: Run report test to verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report allostatic_stability_report_emits_numeric_evidence
```

Expected: PASS.

## Task 5: Final Verification And Commit

**Files:**
- All changed files.

- [ ] **Step 1: Format**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml
```

Expected: exit 0.

- [ ] **Step 2: Targeted tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test allostatic_stability
cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report allostatic_stability_report_emits_numeric_evidence
cargo run --quiet --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --allostatic-stability
```

Expected: all tests pass and report prints bounded numeric evidence.

- [ ] **Step 3: Full guard check**

Run:

```bash
git diff --check
```

Expected: no output.

- [ ] **Step 4: Commit**

Run:

```bash
git status --short
git add docs/superpowers/plans/2026-05-20-allostatic-stability-scheduler-v0.md bionic_v0/src/allostatic_stability.rs bionic_v0/src/lib.rs bionic_v0/tests/allostatic_stability.rs bionic_v0/src/bin/curriculum_report.rs bionic_v0/tests/curriculum_report.rs
git commit -m "feat: add allostatic stability evidence"
```

Expected: commit succeeds after verification.

