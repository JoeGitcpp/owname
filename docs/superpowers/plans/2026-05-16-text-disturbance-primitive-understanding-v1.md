# Text Disturbance Primitive Understanding v1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an observer-side curriculum/report that proves text-shaped byte disturbances can form stable, distinct, ambiguity-preserving bindings to existing substrate targets.

**Architecture:** Reuse `mark_disturbance` and `language_binding` exactly as substrate primitives. Add only a curriculum evidence struct/function and `curriculum_report` flag; do not add tokenizer, parser, vocabulary, text meaning, object names, or body output.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `curriculum_report` binary, integration tests.

---

## Files

- Modify: `bionic_v0/src/curriculum.rs`
  - Add `TextDisturbanceCurriculumEvidence`.
  - Add `run_text_disturbance_curriculum`.
  - Add a small private helper for repeated byte-mark alignment.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--text-disturbance`.
  - Emit compact numeric observer evidence.
- Create: `bionic_v0/tests/text_disturbance_curriculum.rs`
  - TDD tests for distinct bindings, hold, conflict ambiguity, reentry vs cold, and hygiene.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add report coverage for `--text-disturbance`.

## Task 1: Curriculum API

- [ ] **Step 1: Write failing test**

Create `bionic_v0/tests/text_disturbance_curriculum.rs` with:

```rust
use bionic_v0::curriculum::{run_text_disturbance_curriculum, CurriculumConfig};

#[test]
fn text_disturbance_curriculum_builds_distinct_hold_conflict_and_reentry_evidence() {
    let evidence = run_text_disturbance_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.train_episodes, 4);
    assert_eq!(evidence.reentry_episodes, 2);
    assert!(evidence.distinct_text_traces >= 3);
    assert!(evidence.a_recurrence >= evidence.train_episodes);
    assert!(evidence.b_recurrence >= evidence.train_episodes);
    assert!(evidence.a_to_slot_a_strength > 0.5);
    assert!(evidence.b_to_slot_b_strength > 0.5);
    assert!(evidence.a_to_slot_b_strength <= 0.01);
    assert!(evidence.hold_avg_strength_ratio >= 0.80);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.reentry_persistent_avg_strength > evidence.reentry_cold_avg_strength);
    assert!(evidence.time_window_strength > 0.0);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_disturbance_curriculum text_disturbance_curriculum_builds_distinct_hold_conflict_and_reentry_evidence
```

Expected: compile failure because `run_text_disturbance_curriculum` does not exist.

- [ ] **Step 3: Implement minimum code**

In `curriculum.rs`:

1. Allocate slot A and slot B.
2. Align byte disturbance A to slot A for four episodes.
3. Align byte disturbance B to slot B for four episodes.
4. Align byte disturbance C to a time window once.
5. Hold and measure average strength retention.
6. Present A against slot B with deviation feedback and measure ambiguity delta.
7. Reenter A/B on the persistent viewport for two episodes.
8. Run A/B on a cold viewport for two episodes.
9. Return numeric evidence.

- [ ] **Step 4: Verify GREEN**

Run the same test. Expected: PASS.

## Task 2: Report Flag

- [ ] **Step 1: Write failing report test**

In `bionic_v0/tests/curriculum_report.rs`, add a test for:

```bash
curriculum_report --text-disturbance
```

Expected output includes:

```text
mode=text_disturbance
train_episodes=4
distinct_text_traces=
a_recurrence=
b_recurrence=
hold_avg_strength_ratio=
conflict_ambiguity_delta=
reentry_persistent_avg_strength=
reentry_cold_avg_strength=
time_window_strength=
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_disturbance_report_emits_numeric_stage_evidence
```

Expected: FAIL because the flag is not wired.

- [ ] **Step 3: Implement report flag**

Parse `--text-disturbance`, call `run_text_disturbance_curriculum`, and print numeric lines only.

- [ ] **Step 4: Verify GREEN**

Run the same report test. Expected: PASS.

## Task 3: Hygiene And Full Verification

- [ ] **Step 1: Add hygiene test**

Scan the text-disturbance path for rejected terms such as `semantic_readiness`,
`SymbolizerState`, `concept_id`, `token_id`, `word_id`, `object_name`,
`text_understanding`, `name_gate`, and `eprintln!`.

- [ ] **Step 2: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_disturbance_curriculum
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report
```

- [ ] **Step 3: Run full verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```

- [ ] **Step 4: Capture evidence**

```bash
/Users/joe/unnamed/bionic_v0/target/debug/curriculum_report --text-disturbance
```
