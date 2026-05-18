# Self-Development Predictive Life Loop v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a body-side self-development prediction loop that rewards prediction improvement and stable recurrence while suppressing repetitive write/return loops.

**Architecture:** Create a focused `self_development` module with numeric state and `Viewport` integration. The body ticks this loop after prediction pressure and before probe-law output, so active probing is shaped by growth evidence rather than raw pressure alone. Snapshot/report integration exposes evidence without writing observer interpretation back into the body.

**Tech Stack:** Rust, Cargo integration tests, existing `Viewport`, `snapshot`, `curriculum_report`, and body runtime order.

---

## File Structure

- Create: `bionic_v0/src/self_development.rs`
  - Owns `SelfDevelopmentState` and the substrate update algorithm.
  - Adds `Viewport::tick_self_development_from_substrate` and `Viewport::read_self_development`.
- Modify: `bionic_v0/src/viewport.rs`
  - Adds `self_development: RwLock<SelfDevelopmentState>`.
- Modify: `bionic_v0/src/probe_law.rs`
  - Dampens raw probe drive with `self_development.output_suppression`.
- Modify: `bionic_v0/src/bin/body.rs`
  - Ticks self-development after active surface/auditory-visual updates and before `refresh_probe_law`.
- Modify: `bionic_v0/src/snapshot.rs`
  - Writes/reads snapshot v47 self-development evidence.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Adds `--self-development-life-loop [runtime_dir]` report mode for live evidence.
- Modify: `bionic_v0/src/lib.rs`
  - Exports module and bumps `SNAPSHOT_VERSION` to 47.
- Test: `bionic_v0/tests/self_development_life_loop.rs`
  - RED/GREEN coverage for recurrence, repetition suppression, probe damping, snapshot roundtrip, report output, and source hygiene.

## Task 1: Core Self-Development State

**Files:**
- Create: `bionic_v0/src/self_development.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/self_development_life_loop.rs`

- [x] **Step 1: Write the failing recurrence test**

```rust
use bionic_v0::viewport::Viewport;

#[test]
fn stable_recurrence_sediments_without_external_curriculum_gate() {
    let vp = Viewport::new();
    for step in 0..12 {
        let t_us = 1_000_000 + step * 33_000;
        {
            let mut reasoning = vp.global_reasoning.write().unwrap();
            reasoning.global_surprise_load = 0.08;
            reasoning.routing_focus_slot_ids[0] = 1;
            reasoning.last_update_t_us = t_us;
        }
        {
            let mut memory = vp.global_reasoning_memory_summary.write().unwrap();
            memory.pattern_recurrence_score = 0.82;
            memory.routing_stability_score = 0.88;
            memory.dominant_focus_persistence = 0.90;
            memory.persistence_score = 0.76;
            memory.history_fill_ratio = 0.80;
            memory.last_update_t_us = t_us;
        }
        vp.tick_self_development_from_substrate(t_us);
    }

    let state = vp.read_self_development();
    assert!(state.stable_recurrence > 0.40);
    assert!(state.familiarity_pressure > 0.25);
    assert!(state.repetition_pressure < 0.20);
    assert!(state.output_suppression < 0.20);
}
```

- [x] **Step 2: Run the recurrence test and verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop stable_recurrence_sediments_without_external_curriculum_gate
```

Expected: FAIL because `self_development` APIs do not exist.

- [x] **Step 3: Implement minimal state and Viewport storage**

Add a `SelfDevelopmentState` with bounded scalar fields and a substrate tick that reads global reasoning memory, active gap, evidence tension, action output, action consequence, and prediction pressure.

- [x] **Step 4: Run the recurrence test and verify GREEN**

Run the same command. Expected: PASS.

## Task 2: Repetition Suppression

**Files:**
- Modify: `bionic_v0/src/self_development.rs`
- Test: `bionic_v0/tests/self_development_life_loop.rs`

- [x] **Step 1: Write the failing repetition test**

```rust
#[test]
fn repetitive_write_return_loop_raises_suppression_when_prediction_does_not_improve() {
    let vp = Viewport::new();
    seed_saturated_low_similarity_loop(&vp);

    for step in 0..8 {
        let t_us = 2_000_000 + step * 33_000;
        vp.tick_self_development_from_substrate(t_us);
    }

    let state = vp.read_self_development();
    assert!(state.repetition_pressure > 0.45);
    assert!(state.output_suppression > 0.25);
    assert!(state.exploratory_pressure < state.prediction_gap.max(0.01));
}
```

- [x] **Step 2: Run and verify RED**

Expected: FAIL because repetition pressure is not implemented.

- [x] **Step 3: Implement low-quality repetition detection**

Use write epoch saturation, consequence-window saturation, pair saturation, weak pair similarity, weak closure, and weak prediction improvement. The result must remain bounded `[0,1]`.

- [x] **Step 4: Run and verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop repetitive_write_return_loop_raises_suppression_when_prediction_does_not_improve
```

Expected: PASS.

## Task 3: Probe-Law Damping

**Files:**
- Modify: `bionic_v0/src/probe_law.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Test: `bionic_v0/tests/self_development_life_loop.rs`

- [x] **Step 1: Write the failing probe damping test**

```rust
#[test]
fn self_development_suppression_dampens_probe_output_without_disabling_probe_law() {
    let baseline = Viewport::new();
    seed_probe_pressure(&baseline, 3_000_000, 0.90);
    baseline.refresh_probe_law(3_000_100);
    baseline.tick_probe_output_from_law(3_000_100);
    let baseline_epochs = baseline.read_action_output().n_write_epochs;

    let damped = Viewport::new();
    seed_probe_pressure(&damped, 3_000_000, 0.90);
    seed_saturated_low_similarity_loop(&damped);
    for step in 0..10 {
        damped.tick_self_development_from_substrate(3_000_000 + step * 33_000);
    }
    damped.refresh_probe_law(3_400_000);
    damped.tick_probe_output_from_law(3_400_000);
    let damped_probe = damped.read_probe_law();

    assert!(baseline_epochs > 0);
    assert!(damped_probe.intensity < 0.90);
    assert!(damped.read_self_development().output_suppression > 0.25);
}
```

- [x] **Step 2: Run and verify RED**

Expected: FAIL because `refresh_probe_law` ignores self-development suppression.

- [x] **Step 3: Apply suppression to probe drive**

Compute `effective_drive = raw_drive * (1.0 - output_suppression * 0.75)`. Keep the raw substrate values unchanged.

- [x] **Step 4: Add body order guard**

Assert source order: `tick_self_development_from_substrate` appears before `refresh_probe_law`.

- [x] **Step 5: Run and verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop self_development_suppression_dampens_probe_output_without_disabling_probe_law
```

Expected: PASS.

## Task 4: Snapshot And Report Evidence

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Test: `bionic_v0/tests/self_development_life_loop.rs`
- Test: `bionic_v0/tests/curriculum_report.rs`

- [x] **Step 1: Write snapshot roundtrip test**

```rust
#[test]
fn snapshot_roundtrip_preserves_self_development_evidence() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_self_development_snapshot_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = bionic_v0::snapshot::SnapshotPaths::new(&dir);
    let vp = std::sync::Arc::new(Viewport::new());
    seed_stable_recurrence(&vp, 4_000_000);
    vp.tick_self_development_from_substrate(4_000_000);

    bionic_v0::snapshot::write_snapshot(&vp, &paths).unwrap();
    let snap = bionic_v0::snapshot::read_snapshot(&paths.final_path).unwrap();

    assert!(snap.self_development.familiarity_pressure > 0.0);
    assert_eq!(snap.self_development.total_updates, 1);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [x] **Step 2: Verify RED**

Expected: FAIL because snapshot has no `self_development` field.

- [x] **Step 3: Add snapshot v47 write/read**

Bump `SNAPSHOT_VERSION` to 47. Append self-development after active reasoning gap. Read it only when `version >= 47`.

- [x] **Step 4: Add report mode test and implementation**

Add `--self-development-life-loop [runtime_dir]` output fields:

```text
mode=self_development_life_loop
runtime_snapshot_read=0 or 1
prediction_gap=
prediction_improvement=
stable_recurrence=
familiarity_pressure=
repetition_pressure=
exploratory_pressure=
output_suppression=
total_updates=
suppression_events=
principle_guard_pass=
```

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report self_development_life_loop_report_emits_numeric_evidence
```

Expected: PASS.

## Task 5: Principle Guards And Full Verification

**Files:**
- Modify: `bionic_v0/tests/self_development_life_loop.rs`
- Modify: `docs/superpowers/specs/2026-05-19-self-development-predictive-life-loop-v0-design.md`
- Modify: `docs/superpowers/plans/2026-05-19-self-development-predictive-life-loop-v0.md`

- [x] **Step 1: Add source hygiene test**

Forbid `semantic_readiness`, `SymbolizerState`, `concept_id`, `token_id`, `word_id`, `object_name`, `text_understanding`, `name_gate`, `meaning_db`, `dictionary`, `tokenizer`, `image_label`, `voice_label`, `prompt_answer`, `source_identity`, `owner`, `ethics`, `personality`, `println!("body`, `eprintln!`, `struct Thought`, `struct Desire`, `struct Mind`, `struct Conscious`.

- [x] **Step 2: Run targeted tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test prediction_pressure_field_body_integration
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test live_human_nursery
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test body_no_human_output
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report self_development_life_loop_report_emits_numeric_evidence
```

- [x] **Step 3: Run format and diff checks**

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --check
git diff --check
```

- [x] **Step 4: Commit**

```bash
git add \
  docs/superpowers/specs/2026-05-19-self-development-predictive-life-loop-v0-design.md \
  docs/superpowers/plans/2026-05-19-self-development-predictive-life-loop-v0.md \
  bionic_v0/src/lib.rs \
  bionic_v0/src/self_development.rs \
  bionic_v0/src/viewport.rs \
  bionic_v0/src/probe_law.rs \
  bionic_v0/src/bin/body.rs \
  bionic_v0/src/snapshot.rs \
  bionic_v0/src/bin/curriculum_report.rs \
  bionic_v0/tests/self_development_life_loop.rs \
  bionic_v0/tests/curriculum_report.rs
git commit -m "feat: add self-development predictive life loop"
```

Runtime files under `runtime/` must remain unstaged.
