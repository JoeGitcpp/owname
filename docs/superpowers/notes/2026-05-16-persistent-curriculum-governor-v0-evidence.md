# Persistent Curriculum Governor v0 Evidence

Date: 2026-05-16

## Scope

Implemented `Persistent Curriculum Governor v0`.

Governor v0 is an external observer-side scheduler. It evaluates existing
curriculum evidence, checks every course against bounded gates, records the
selected course and decision trace, and stops when all checked courses pass.

It does not restart background automation, add body modules, persist governor
state to disk, generate new training material, or mutate a shared live body.

## Files

- Created `docs/superpowers/specs/2026-05-16-persistent-curriculum-governor-v0-design.md`
- Created `docs/superpowers/plans/2026-05-16-persistent-curriculum-governor-v0.md`
- Created `docs/superpowers/notes/2026-05-16-persistent-curriculum-governor-v0-evidence.md`
- Created `bionic_v0/src/curriculum_governor.rs`
- Created `bionic_v0/tests/persistent_curriculum_governor.rs`
- Modified `bionic_v0/src/lib.rs`
- Modified `bionic_v0/src/bin/curriculum_report.rs`
- Modified `bionic_v0/tests/curriculum_report.rs`

## TDD Evidence

RED governor test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops
```

Observed expected failures:

```text
could not find `curriculum_governor` in `bionic_v0`
couldn't read `tests/../src/curriculum_governor.rs`
```

GREEN governor tests:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor
```

Observed:

```text
2 passed; 0 failed
```

RED report test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_governor_report_emits_decision_trace_evidence
```

Observed expected failure:

```text
assertion failed: stdout.contains("mode=persistent_curriculum_governor")
```

GREEN focused tests:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test persistent_curriculum_governor --test curriculum_report
```

Observed:

```text
tests/curriculum_report.rs: 13 passed
tests/persistent_curriculum_governor.rs: 2 passed
```

## Report Evidence

Command:

```text
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor
```

Observed:

```text
curriculum_report_v0
mode=persistent_curriculum_governor
rounds_run=1
courses_evaluated=7
material_groups=10
material_principle_pass=1
decision_trace_len=1
selected_course_kind=1
selected_score=1.000000
selected_threshold=1.000000
passing_courses=7
failing_courses=0
stopped_after_pass=1
body_touch_points=0
min_score=1.000000
max_score=1.000000
```

## v0 Boundary

All seven courses pass their v0 gates, so the governor stops in one round.

The current score is gate-based: a course receives `1.0` only when all v0
acceptance gates pass. Because every course passes, the selected course is a
deterministic tie selection, not a fine-grained weakness ranking.

The next governor version should add margin-based scoring if we want it to
prioritize "which passing course is weakest" rather than only "which course
fails".

## Verification

Full test command:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Observed: pass, including doc tests.

Whitespace check:

```text
git diff --check
```

Observed: pass.

Touched-file formatting:

```text
rustfmt --edition 2021 bionic_v0/src/curriculum_governor.rs bionic_v0/src/lib.rs bionic_v0/src/bin/curriculum_report.rs bionic_v0/tests/curriculum_report.rs bionic_v0/tests/persistent_curriculum_governor.rs
```

Observed: pass.

Repository-wide formatting check:

```text
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Observed: fails only on pre-existing unrelated formatting differences in:

- `bionic_v0/tests/adaptive_stability.rs`
- `bionic_v0/tests/after_window_diagnosis.rs`
- `bionic_v0/tests/architecture_contraction.rs`

Those files were not changed for this stage.

## Principle Check

Governor v0 remains admissible because:

- it is observer-side only;
- it consumes existing course evidence and material registry review;
- it stores numeric course ids, scores, thresholds, counts, and boolean-as-u8
  pass flags;
- it does not introduce source identity, owner, ethics, personality, semantic
  meaning, labels, names, token ids, word ids, or concept ids;
- it does not add body output;
- it does not create an internal module for intention, will, or request/answer
  behavior.
