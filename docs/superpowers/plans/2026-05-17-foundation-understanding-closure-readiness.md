# Foundation Understanding Closure Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an observer-side readiness gate for the foundation-understanding closure stage.

**Architecture:** Keep readiness computation in `bionic_v0/src/curriculum_governor.rs`, derived only from existing course reviews and numeric margins. Extend `bionic_v0/src/bin/curriculum_report.rs` to emit compact numeric readiness evidence.

**Tech Stack:** Rust crate `bionic_v0`, Cargo tests, existing curriculum/governor modules.

---

### Task 1: Add Failing Readiness Tests

**Files:**
- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Update governor behavior test**

In `bionic_v0/tests/persistent_curriculum_governor.rs`, change the first test to expect that course pass is not enough for foundation readiness:

```rust
    assert_eq!(evidence.failing_courses, 0);
    assert_eq!(evidence.foundation_margin_floor, 0.10);
    assert_eq!(evidence.foundation_min_margin_ready, 0);
    assert_eq!(evidence.foundation_stage_ready, 0);
    assert_eq!(evidence.stopped_after_pass, 0);
    assert_eq!(
        evidence.rounds_run,
        PersistentCurriculumGovernorConfig::default().max_rounds
    );
```

- [ ] **Step 2: Add readiness field assertions**

In `persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass`, assert:

```rust
    assert!(evidence.foundation_margin_floor > 0.0);
    assert!(evidence.foundation_margin_floor <= 1.0);
    assert!(evidence.foundation_text_grounding_ready <= 1);
    assert!(evidence.foundation_live_io_ready <= 1);
    assert!(evidence.foundation_cross_channel_ready <= 1);
    assert!(evidence.foundation_stage_ready <= 1);
```

- [ ] **Step 3: Update report test**

In `persistent_curriculum_governor_report_emits_decision_trace_evidence`, assert stdout contains:

```rust
    assert!(stdout.contains("foundation_margin_floor="));
    assert!(stdout.contains("foundation_min_margin_ready="));
    assert!(stdout.contains("foundation_text_grounding_ready="));
    assert!(stdout.contains("foundation_live_io_ready="));
    assert!(stdout.contains("foundation_cross_channel_ready="));
    assert!(stdout.contains("foundation_stage_ready="));
```

- [ ] **Step 4: Run focused tests and verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml persistent_curriculum_governor
```

Expected: FAIL because the new fields do not exist yet.

### Task 2: Implement Governor Readiness

**Files:**
- Modify: `bionic_v0/src/curriculum_governor.rs`

- [ ] **Step 1: Extend config and evidence structs**

Add `foundation_margin_floor: f64` to `PersistentCurriculumGovernorConfig` with default `0.10`.

Add these fields to `GovernorDecisionTrace` and `PersistentCurriculumGovernorEvidence`:

```rust
pub foundation_margin_floor: f64,
pub foundation_min_margin_ready: u8,
pub foundation_text_grounding_ready: u8,
pub foundation_live_io_ready: u8,
pub foundation_cross_channel_ready: u8,
pub foundation_stage_ready: u8,
```

- [ ] **Step 2: Compute readiness from course reviews**

Add helper functions in `curriculum_governor.rs`:

```rust
fn course_passed(course_reviews: &[GovernorCourseReview], kind: GovernorCourseKind) -> u8 {
    course_reviews
        .iter()
        .find(|review| review.course_kind == kind as u8)
        .map(|review| review.passed)
        .unwrap_or(0)
}

fn readiness_flag(value: bool) -> u8 {
    if value { 1 } else { 0 }
}
```

Use them inside `decide_next_course` to compute the readiness flags.

- [ ] **Step 3: Stop only when foundation stage is ready**

Change `stopped_after_pass` to:

```rust
stopped_after_pass: foundation_stage_ready,
```

where `foundation_stage_ready == 1` only if all courses pass, the weakest
margin is above the floor, and the required course flags are all `1`.

- [ ] **Step 4: Propagate fields to top-level evidence**

Copy the decision readiness fields into `PersistentCurriculumGovernorEvidence`
inside `run_persistent_curriculum_governor`.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml persistent_curriculum_governor
```

Expected: PASS.

### Task 3: Emit Readiness Report Fields

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Print new readiness lines**

Inside the `persistent_curriculum_governor` report branch, print:

```rust
println!("foundation_margin_floor={:.6}", evidence.foundation_margin_floor);
println!(
    "foundation_min_margin_ready={}",
    evidence.foundation_min_margin_ready
);
println!(
    "foundation_text_grounding_ready={}",
    evidence.foundation_text_grounding_ready
);
println!("foundation_live_io_ready={}", evidence.foundation_live_io_ready);
println!(
    "foundation_cross_channel_ready={}",
    evidence.foundation_cross_channel_ready
);
println!("foundation_stage_ready={}", evidence.foundation_stage_ready);
```

- [ ] **Step 2: Run report test**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Expected: PASS.

### Task 4: Final Verification

**Files:**
- No further edits unless verification fails.

- [ ] **Step 1: Run formatting check**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
```

Expected: PASS.

- [ ] **Step 2: Run full crate tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.

- [ ] **Step 3: Run diff whitespace check**

Run:

```bash
git diff --check
```

Expected: PASS.
