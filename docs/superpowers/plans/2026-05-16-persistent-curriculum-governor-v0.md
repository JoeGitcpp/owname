# Persistent Curriculum Governor v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an observer-side governor that evaluates existing curriculum evidence, selects the weakest course, records numeric decision trace, and stops when all courses pass.

**Architecture:** Add a focused `curriculum_governor` module that depends on `curriculum` and `curriculum_material`. It does not add body modules, internal ontology, persistent state files, or background automation. `curriculum_report` prints numeric governor evidence.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` observer binary, Superpowers TDD/verification workflow.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-persistent-curriculum-governor-v0-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-external-curriculum-training-material-registry-v0-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-cross-channel-disturbance-coupling-v0-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no internal module for will, intention, ethics, owner, personality, source
  identity, semantic meaning, names, labels, tokens, words, or concepts;
- no prompt-answer datasets or web ingestion;
- no new material generation in governor v0;
- no automated background task restart.

## Files

- Create: `bionic_v0/src/curriculum_governor.rs`
  - `PersistentCurriculumGovernorConfig`
  - `GovernorCourseKind`
  - `GovernorCourseReview`
  - `GovernorDecisionTrace`
  - `PersistentCurriculumGovernorEvidence`
  - `run_persistent_curriculum_governor`
- Modify: `bionic_v0/src/lib.rs`
  - export `curriculum_governor`.
- Create: `bionic_v0/tests/persistent_curriculum_governor.rs`
  - behavior and source hygiene tests.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - add `--persistent-curriculum-governor`.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - add report coverage.
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-persistent-curriculum-governor-v0-evidence.md`

## Task 1: RED Governor Behavior Test

**Files:**

- Create: `bionic_v0/tests/persistent_curriculum_governor.rs`

- [ ] **Step 1: Write failing behavior test**

Expected API:

```rust
use bionic_v0::curriculum_governor::{
    run_persistent_curriculum_governor, PersistentCurriculumGovernorConfig,
};

#[test]
fn persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops() {
    let evidence =
        run_persistent_curriculum_governor(PersistentCurriculumGovernorConfig::default())
            .expect("governor runs");

    assert!(evidence.rounds_run >= 1);
    assert!(evidence.courses_evaluated >= 7);
    assert!(evidence.material_groups >= 8);
    assert_eq!(evidence.material_principle_pass, 1);
    assert_eq!(evidence.body_touch_points, 0);
    assert!(evidence.decision_trace_len >= 1);
    assert!(evidence.selected_course_kind > 0);
    assert!(evidence.selected_score >= 0.0 && evidence.selected_score <= 1.0);
    assert_eq!(
        evidence.passing_courses + evidence.failing_courses,
        evidence.courses_evaluated
    );
    assert_eq!(evidence.stopped_after_pass, 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops
```

Expected: unresolved module or function.

## Task 2: Guard Test

**Files:**

- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`

- [ ] **Step 1: Add source hygiene test**

Scan `src/curriculum_governor.rs` and `src/bin/curriculum_report.rs` for
rejected ontology and body output drift.

- [ ] **Step 2: Verify guard**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor
```

Expected after implementation: pass.

## Task 3: Implement Governor

**Files:**

- Create: `bionic_v0/src/curriculum_governor.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Implement structs**

All public evidence fields are numeric, boolean-as-u8, or bounded floats.

- [ ] **Step 2: Implement course scoring**

Use existing evidence functions:

- material registry review;
- `run_long_retention_curriculum`;
- `run_mark_binding_curriculum`;
- `run_text_grounding_suite`;
- `run_text_open_stream_curriculum`;
- `run_active_text_probe_closure_curriculum`;
- `run_cross_channel_disturbance_coupling_curriculum`.

- [ ] **Step 3: Implement decision selection**

Lowest score wins. If all courses pass thresholds, set `stopped_after_pass=1`.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor
```

Expected: all tests pass.

## Task 4: Report Mode

**Files:**

- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add failing report test**

Expect `--persistent-curriculum-governor` to print:

- `mode=persistent_curriculum_governor`
- `rounds_run=`
- `courses_evaluated=`
- `material_groups=`
- `material_principle_pass=`
- `selected_course_kind=`
- `selected_score=`
- `selected_threshold=`
- `passing_courses=`
- `failing_courses=`
- `stopped_after_pass=`
- `body_touch_points=`

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Expected: fail until flag is wired.

- [ ] **Step 3: Wire report**

Import governor runner, parse flag, print numeric evidence.

- [ ] **Step 4: Verify GREEN**

Run the same report test again.

## Task 5: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-persistent-curriculum-governor-v0-evidence.md`

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
formatting issue.
