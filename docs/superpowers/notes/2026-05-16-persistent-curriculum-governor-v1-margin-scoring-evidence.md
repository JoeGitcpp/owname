# Persistent Curriculum Governor v1 Margin Scoring Evidence

Date: 2026-05-16.

## Scope

Implemented observer-side margin scoring for `PersistentCurriculumGovernor`.

The gate score remains the pass/fail surface. Margin score is a separate
bounded observer metric used to rank courses after all gates pass.

No body-side state, body output, persistent automation, material generator, or
ontology layer was added.

## RED

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor persistent_curriculum_governor_ranks_weakest_margin_after_all_courses_pass
```

Result: failed to compile because `PersistentCurriculumGovernorEvidence` did
not yet expose `selected_margin_score`, `weakest_margin_course_kind`,
`weakest_margin_score`, `near_threshold_courses`, `min_margin_score`, or
`max_margin_score`.

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Result: failed because report stdout did not contain
`selected_margin_score=`.

## GREEN

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor
```

Result: 3 passed, 0 failed.

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Result: 1 passed, 0 failed.

## Report Evidence

Command:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor
```

Output:

```text
curriculum_report_v0
mode=persistent_curriculum_governor
rounds_run=1
courses_evaluated=7
material_groups=10
material_principle_pass=1
decision_trace_len=1
selected_course_kind=2
selected_score=1.000000
selected_margin_score=0.000000
weakest_margin_course_kind=2
weakest_margin_score=0.000000
selected_threshold=1.000000
passing_courses=7
failing_courses=0
near_threshold_courses=2
stopped_after_pass=1
body_touch_points=0
min_score=1.000000
max_score=1.000000
min_margin_score=0.000000
max_margin_score=0.464156
```

Interpretation: all seven courses pass gate score, but margin scoring still
ranks course kind `2` as the weakest current course. Two courses are near the
margin floor.

## Verification

Focused:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor --test curriculum_report
```

Result: `curriculum_report.rs` 13 passed; `persistent_curriculum_governor.rs`
3 passed.

Full:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Result: passed, including doc tests.

Diff hygiene:

```bash
git diff --check
```

Result: passed.

Formatting:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Result: failed only on pre-existing unrelated formatting differences:

- `/Users/joe/unnamed/bionic_v0/tests/adaptive_stability.rs:82`
- `/Users/joe/unnamed/bionic_v0/tests/after_window_diagnosis.rs:71`
- `/Users/joe/unnamed/bionic_v0/tests/architecture_contraction.rs:124`
