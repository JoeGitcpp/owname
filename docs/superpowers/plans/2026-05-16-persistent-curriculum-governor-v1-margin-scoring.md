# Persistent Curriculum Governor v1 Margin Scoring Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add observer-side margin scoring so Governor can rank the weakest course even when every curriculum course passes its gate.

**Architecture:** Extend the existing `curriculum_governor` module without adding body-side state or ontology. Keep gate score for pass/fail and add a separate bounded margin score for all-pass ranking. Extend the observer report with numeric margin evidence.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` observer binary, Superpowers TDD/verification workflow.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-persistent-curriculum-governor-v1-margin-scoring-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-persistent-curriculum-governor-v0-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no internal module for will, intention, ethics, owner, personality, source identity, semantic meaning, names, labels, tokens, words, or concepts;
- no prompt-answer datasets, web ingestion, or material generation;
- no background automation restart.

## Files

- Modify: `bionic_v0/src/curriculum_governor.rs`
  - add margin fields to `GovernorCourseReview`, `GovernorDecisionTrace`, and `PersistentCurriculumGovernorEvidence`;
  - add local margin helper functions;
  - compute course margins from existing evidence fields;
  - select by gate score only when failures exist, otherwise by margin score.
- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`
  - add RED behavior coverage for all-pass margin ranking.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - print new numeric margin fields.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - assert report contains new margin keys.
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-persistent-curriculum-governor-v1-margin-scoring-evidence.md`

## Task 1: RED Governor Margin Behavior

**Files:**

- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`

- [ ] **Step 1: Add failing behavior test**

Expected assertions:

```rust
#[test]
fn persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass() {
    let evidence =
        run_persistent_curriculum_governor(PersistentCurriculumGovernorConfig::default())
            .expect("governor runs");

    assert_eq!(evidence.failing_courses, 0);
    assert_eq!(evidence.selected_score, 1.0);
    assert!(evidence.selected_margin_score >= 0.0);
    assert!(evidence.selected_margin_score <= 1.0);
    assert_eq!(
        evidence.selected_course_kind,
        evidence.weakest_margin_course_kind
    );
    assert_eq!(evidence.selected_margin_score, evidence.weakest_margin_score);
    assert!(evidence.min_margin_score < evidence.max_margin_score);
    assert!(evidence.near_threshold_courses <= evidence.courses_evaluated);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass
```

Expected: compile failure because margin fields do not exist.

## Task 2: Implement Margin Evidence

**Files:**

- Modify: `bionic_v0/src/curriculum_governor.rs`

- [ ] **Step 1: Add public evidence fields**

Add:

- `margin_score` on `GovernorCourseReview`;
- `selected_margin_score` on `GovernorDecisionTrace`;
- `selected_margin_score`, `weakest_margin_course_kind`, `weakest_margin_score`, `near_threshold_courses`, `min_margin_score`, `max_margin_score` on `PersistentCurriculumGovernorEvidence`.

- [ ] **Step 2: Add margin helpers**

Implement:

```rust
fn min_margin(value: f64, floor: f64) -> f64
fn max_margin(value: f64, ceiling: f64) -> f64
fn separation_margin(left: f64, right: f64) -> f64
fn course_margin(points: &[f64]) -> f64
```

All helpers return bounded finite values in `[0, 1]`.

- [ ] **Step 3: Compute margins from existing course evidence**

For each existing course, compute a margin from the same evidence points used by
the gate score. Keep pass/fail gates unchanged.

- [ ] **Step 4: Update selection**

If failures exist, select by lowest gate score. If all pass, select by lowest
margin score.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor
```

Expected: all governor tests pass.

## Task 3: Report Margin Evidence

**Files:**

- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Add failing report assertions**

Expect `--persistent-curriculum-governor` to print:

- `selected_margin_score=`
- `weakest_margin_course_kind=`
- `weakest_margin_score=`
- `near_threshold_courses=`
- `min_margin_score=`
- `max_margin_score=`

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Expected: fail until report prints the new fields.

- [ ] **Step 3: Print numeric fields**

Add the new lines in the existing governor report block.

- [ ] **Step 4: Verify GREEN**

Run the same focused report test again.

## Task 4: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-persistent-curriculum-governor-v1-margin-scoring-evidence.md`

- [ ] **Step 1: Run report**

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor
```

- [ ] **Step 2: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor --test curriculum_report
```

- [ ] **Step 3: Run full verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

- [ ] **Step 4: Record evidence**

Record RED/GREEN, report numbers, full verification, and any pre-existing
formatting issue outside the touched files.
