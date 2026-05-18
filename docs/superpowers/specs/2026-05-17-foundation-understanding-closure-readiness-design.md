# Foundation Understanding Closure Readiness Design

Status: Draft for TDD execution on 2026-05-17.

## Purpose

This design turns the current stage goal into an observer-side readiness gate:
the system should not treat the foundation-understanding closure stage as ready
only because every curriculum course has a passing gate score.

The target stage is:

`object continuity -> relation projection -> action consequence -> weak mark binding -> hold/reentry -> conflict correction -> active probe return`

The gate is external training evidence only. It does not add a body-side
reasoning layer, text-understanding module, naming gate, target label, result
label, source identity, owner, ethics, or personality state.

## Boundary Notes

This design uses `understanding`, `text`, and `readiness` as external
observer/training descriptions.

- External description vs internal ontology: these words describe a training
  stage and report evidence. They are not body ontology units.
- Engineering index use: course kinds, readiness flags, margin floors, and
  report fields are observer-side numeric indices.
- Why not ontological: no field records what an object, mark, relation, or
  consequence "means"; the body continues to hold curves, traces, pairs, and
  numeric evidence.
- Why no principle or axiom violation: readiness is computed from existing
  observer curriculum evidence and does not feed human-readable assertions back
  into body state.

## Architecture

Keep the implementation inside `bionic_v0/src/curriculum_governor.rs` and the
observer report binary `bionic_v0/src/bin/curriculum_report.rs`.

The governor already produces:

- pass/fail gate score per course;
- margin score per course;
- selected weakest course;
- min/max score and min/max margin score.

This stage adds a stricter foundation readiness gate:

- all courses must pass;
- the weakest margin must be above a configurable floor, default `0.10`;
- the text-grounding course must pass;
- the active-probe-return course must pass;
- the cross-channel coupling course must pass.

When all courses pass but the weakest margin remains at or below the floor, the
governor must keep selecting the weakest-margin course and must not report the
foundation stage as ready.

## Evidence Fields

Add numeric observer-side fields to `PersistentCurriculumGovernorEvidence` and
`GovernorDecisionTrace`:

- `foundation_margin_floor: f64`
- `foundation_min_margin_ready: u8`
- `foundation_text_grounding_ready: u8`
- `foundation_live_io_ready: u8`
- `foundation_cross_channel_ready: u8`
- `foundation_stage_ready: u8`

The report mode `curriculum_report --persistent-curriculum-governor` emits the
same fields as compact numeric lines.

## Acceptance

Tests must prove:

1. default governor keeps all course pass evidence visible;
2. if weakest margin is at or below `0.10`, `foundation_stage_ready=0`;
3. in that case `stopped_after_pass=0` and the governor runs to `max_rounds`;
4. report output includes the new foundation readiness fields;
5. existing rejected-ontology hygiene remains green;
6. full crate tests and `git diff --check` pass.

## Non-Goals

- No new training material generator.
- No body-side output.
- No mutation loop that changes course evidence across rounds.
- No natural language understanding claim.
- No parser, tokenizer, dictionary, concept id, or naming layer.
