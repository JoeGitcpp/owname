# Persistent Curriculum Governor v1 Margin Scoring Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Governor v0 proves that an external observer can evaluate the current
curriculum courses, select a weakest course by pass/fail score, and stop when
all courses pass.

The gap is that all passing courses currently collapse to `1.0`. When every
course passes, v0 can only pick by deterministic course order. Governor v1 adds
margin scoring so the same external observer can still answer: among passing
courses, which one is closest to its floor and therefore most worth another
training round.

## Architecture

Keep `bionic_v0/src/curriculum_governor.rs` as the only implementation module.

The module keeps the existing gate score:

- `score` remains the bounded pass/fail gate ratio.
- `threshold` remains `1.0`.
- `passed` remains derived only from `score >= threshold`.

v1 adds a separate bounded margin score:

- `margin_score` is observer-side numeric evidence in `[0, 1]`.
- `0.0` means at or below the course floor.
- larger values mean more room above the floor.
- the course margin is the weakest margin among its checked evidence points.

Selection law:

1. If any course fails, keep v0 behavior: select the lowest gate score.
2. If all courses pass, select the lowest `margin_score`.
3. Record the selected margin score, global margin min/max, and count of
   near-floor courses.

The report binary keeps `--persistent-curriculum-governor` and emits the new
numeric margin fields.

## Margin Normalization

Use small helper functions local to the governor:

- minimum floor: `(value - floor) / max(abs(floor), epsilon)`;
- maximum ceiling: `(ceiling - value) / max(abs(ceiling), epsilon)`;
- positive separation: `(left - right) / max(abs(left), abs(right), 1.0)`.

All helpers clamp into `[0, 1]`. A course margin is the minimum helper result
across the continuous or comparative evidence points used by its gate review.
Binary guard points and zero-count safety guards are not continuous margins;
they contribute `1.0` when satisfied and `0.0` when violated. This keeps the
signal conservative without letting exact structural course counts erase the
retention/generalization/reentry signal.

## High-Risk Term Boundary

- `governor` remains an external observer/training controller, not a body-side
  mind module.
- `decision` means numeric selection among course reviews, not will,
  intention, or consciousness.
- `margin` means distance from an observer-defined course floor, not meaning,
  identity, or world truth.
- `text` appears only in existing external course names and byte/mark
  disturbance paths. It is not a token, word, label, semantic unit, or internal
  ontology.
- `training` means external disturbance-course scheduling, not request/answer
  fitting or prompt learning.

All new public evidence remains numeric ids, counts, `u8` flags, and bounded
floats. No body-side output, source identity, owner, ethics, personality,
concept ids, word ids, token ids, naming gates, or label logic may be added.

## Acceptance

Tests must prove:

1. all current courses can pass while margin scores still differ;
2. selected course equals the weakest margin course when every course passes;
3. selected gate score can remain `1.0` while selected margin score is below
   the strongest margin;
4. near-floor course count is emitted and bounded by evaluated course count;
5. report output includes the new numeric margin fields;
6. source hygiene still rejects forbidden ontology and body output drift;
7. full crate tests remain green, except any separately identified pre-existing
   formatting issue outside the touched files.

## Non-Goals

- No background automation restart.
- No persistent on-disk governor state.
- No shared-body multi-round mutation loop.
- No internet ingestion.
- No new material generator.
- No labels, captions, voice identities, source trust, owners, ethics, or
  personality logic.
