# Long Retention Margin Diagnosis v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add observer-side margin breakdown evidence so the governor can identify which selected-course margin point blocks foundation readiness.

**Architecture:** Keep all diagnosis in `bionic_v0/src/curriculum_governor.rs` and `bionic_v0/src/bin/curriculum_report.rs`. Margin points are numeric observer evidence derived from existing course evidence; no body state or ontology is added.

**Tech Stack:** Rust crate `bionic_v0`, Cargo tests, existing curriculum/governor modules.

---

### Task 1: Add Failing Margin Breakdown Tests

**Files:**
- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Assert selected-course margin breakdown exists**

Add assertions to `persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass`:

```rust
    assert!(evidence.selected_margin_points_len > 0);
    assert!(evidence.selected_weakest_margin_point_kind > 0);
    assert_eq!(
        evidence.selected_weakest_margin_point_score,
        evidence.selected_margin_score
    );
    assert!(evidence
        .selected_margin_points
        .iter()
        .all(|point| point.course_kind == evidence.selected_course_kind));
```

- [ ] **Step 2: Assert report exposes the breakdown**

Add stdout assertions in `persistent_curriculum_governor_report_emits_decision_trace_evidence`:

```rust
    assert!(stdout.contains("selected_margin_points_len="));
    assert!(stdout.contains("selected_weakest_margin_point_kind="));
    assert!(stdout.contains("selected_weakest_margin_point_score="));
    assert!(stdout.contains("selected_margin_point_00_kind="));
    assert!(stdout.contains("selected_margin_point_00_score="));
```

- [ ] **Step 3: Run focused tests and verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml persistent_curriculum_governor
```

Expected: FAIL because the selected margin point fields do not exist.

### Task 2: Implement Margin Point Evidence

**Files:**
- Modify: `bionic_v0/src/curriculum_governor.rs`

- [ ] **Step 1: Add point struct and evidence fields**

Add:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GovernorMarginPoint {
    pub course_kind: u8,
    pub point_index: u8,
    pub point_kind: u16,
    pub margin_score: f64,
}
```

Add to `PersistentCurriculumGovernorEvidence`:

```rust
pub selected_margin_points_len: u32,
pub selected_weakest_margin_point_kind: u16,
pub selected_weakest_margin_point_score: f64,
pub selected_margin_points: Vec<GovernorMarginPoint>,
```

- [ ] **Step 2: Generate margin points with course-local indices**

Create helper functions:

```rust
fn review_with_margins(
    course_kind: GovernorCourseKind,
    score: f64,
    margins: &[f64],
) -> (GovernorCourseReview, Vec<GovernorMarginPoint>) {
    let points = margin_points(course_kind, margins);
    let margin_score = points
        .iter()
        .map(|point| point.margin_score)
        .fold(1.0, f64::min);
    (review(course_kind, score, margin_score), points)
}

fn margin_points(course_kind: GovernorCourseKind, margins: &[f64]) -> Vec<GovernorMarginPoint> {
    let course_code = course_kind as u8;
    margins
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, value)| {
            let point_index = (idx + 1) as u8;
            GovernorMarginPoint {
                course_kind: course_code,
                point_index,
                point_kind: course_code as u16 * 16 + point_index as u16,
                margin_score: unit(value),
            }
        })
        .collect()
}
```

- [ ] **Step 3: Return reviews and points together**

Introduce a private `GovernorCourseEvaluation` with `course_reviews` and
`margin_points`. Change `evaluate_governor_courses` to return it and use
`review_with_margins` for each course.

- [ ] **Step 4: Copy selected margin point data into evidence**

After each decision, filter margin points by `decision.selected_course_kind`,
store them in `evidence.selected_margin_points`, set the length, and compute
the weakest point by `margin_score` then `point_kind`.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml persistent_curriculum_governor
```

Expected: PASS.

### Task 3: Emit Margin Breakdown Report Lines

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Print selected margin point summary**

In the persistent governor report branch, print:

```rust
println!(
    "selected_margin_points_len={}",
    evidence.selected_margin_points_len
);
println!(
    "selected_weakest_margin_point_kind={}",
    evidence.selected_weakest_margin_point_kind
);
println!(
    "selected_weakest_margin_point_score={:.6}",
    evidence.selected_weakest_margin_point_score
);
```

- [ ] **Step 2: Print compact per-point lines**

Print selected points with stable indexes:

```rust
for (idx, point) in evidence.selected_margin_points.iter().enumerate() {
    println!("selected_margin_point_{idx:02}_kind={}", point.point_kind);
    println!(
        "selected_margin_point_{idx:02}_score={:.6}",
        point.margin_score
    );
}
```

- [ ] **Step 3: Run report test**

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

- [ ] **Step 2: Run report command**

Run:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor
```

Expected: report includes selected margin point fields.

- [ ] **Step 3: Run full crate tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run diff whitespace check**

Run:

```bash
git diff --check
```

Expected: PASS.
