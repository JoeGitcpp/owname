# Persistent Curriculum Governor v0 Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Build the first external curriculum governor.

The current system has multiple observer-side courses that can prove retention,
conflict correction, active probe closure, cross-channel coupling, and material
admissibility. They still run as separate manual checks. Governor v0 evaluates
those existing numeric evidence surfaces, selects the weakest course, records a
decision trace, and stops once all checked courses pass their thresholds.

This is not an internal intention module. It is an external research/training
controller that observes numeric evidence and decides which existing curriculum
course would be scheduled next.

## Architecture

Add `bionic_v0/src/curriculum_governor.rs`.

The module:

- reads the external material registry review;
- runs selected existing curriculum evidence functions;
- converts each course evidence into a bounded score;
- compares scores to per-course thresholds;
- records a compact numeric decision trace;
- returns an observer-side evidence struct;
- never mutates the body outside the existing curriculum course calls.

The report binary gets `--persistent-curriculum-governor`.

## v0 Course Set

Governor v0 covers:

1. material plan admissibility;
2. long retention;
3. mark binding hold/reentry;
4. text grounding suite;
5. text open stream;
6. active text probe closure;
7. cross-channel disturbance coupling.

The course set is intentionally finite. v0 proves scheduling and review
closure. Later versions can add persistence files, history windows, retries,
and background automations.

## Decision Law

For each round:

1. evaluate every configured course;
2. compute `score` and `threshold`;
3. count passing and failing courses;
4. select the lowest score as weakest;
5. if no course fails, stop;
6. otherwise record the selected course as the next training target and stop
   after the configured max round count.

In v0, running the selected course does not modify a shared body. It records
which existing curriculum should be run next. This keeps the system honest:
existing course evidence already proves the learned behavior, and the governor
only proves external scheduling closure.

## High-Risk Term Boundary

- `governor` is an external observer/training controller, not a body-side mind
  module.
- `decision` means numeric selection among course scores, not internal will,
  intention, or consciousness.
- `target` means curriculum course target, not object identity or world truth.
- `training` means external disturbance-course scheduling, not model fitting or
  token learning.
- `weakest` means lowest bounded evidence score, not a semantic diagnosis.

All implementation state must remain numeric course ids, thresholds, scores,
counts, and booleans.

## Principle Review

Governor v0 is admissible only if:

- it runs outside the body;
- it emits observer-side numeric evidence only;
- it does not introduce source identity, owner, ethics, personality, semantic
  meaning, names, labels, token ids, word ids, or concept ids;
- it does not create a new object ontology or map substrate objects to concept
  identifiers;
- it consumes existing curriculum evidence and material registry review;
- it leaves live body stdout/stderr rules untouched.

## Acceptance

Tests must prove:

1. governor evaluates at least seven courses;
2. material registry is included and principle review passes;
3. decision trace contains at least one round;
4. weakest course id is nonzero;
5. selected score is bounded in `[0, 1]`;
6. passing plus failing courses equals evaluated courses;
7. when all courses pass, governor stops within one round;
8. report output emits numeric governor evidence;
9. source hygiene rejects forbidden ontology and body output drift.

## Non-Goals

- No background automation restart.
- No persistent on-disk governor state.
- No shared-body multi-round reinforcement loop.
- No internet material ingestion.
- No labels, prompt-answer pairs, visual captions, voice identities, source
  trust, owners, ethics, or personality logic.
