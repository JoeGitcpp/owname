# Persistent Curriculum Session Design

Date: 2026-05-13
Status: draft implementation spec

## Purpose

Extend the external curriculum harness from repeated fresh suites into a
persistent session where one `Viewport` experiences multiple episodes. This
is the next step toward training: the same body substrate must carry traces
forward across episodes before any stronger learning rule is added.

## Boundary

The persistent session remains an observer-side training harness. It does not
run inside `body`, does not add a request/response controller, and does not add
emotion, value, ethics, personality, label, concept, or semantic ontology.
It reuses existing body substrate APIs and reports numeric evidence only.

High-risk terms used here:

- `training` is an external harness description, not a body ontology field.
- `persistent` means the same `Viewport` instance is reused across synthetic
  perturbation episodes. It is not identity, owner, or personality.
- Later emotion/value work must be built as learned regulatory curves and
  selection biases, not fixed labels or moral rule structs.

## Required Behavior

- Reuse one `Viewport` across N episodes.
- Reuse fixed slots for motion and relation courses so slot capacity is not
  exhausted.
- Advance synthetic time per episode.
- Produce at least one action-consequence pair per episode.
- Tick life-closure producers so learned-bias evidence accumulates in the
  same body substrate.
- Report first/last/mean worldline error, final worldline update count, total
  pairs formed, final pair count, final learned event count, and max learned
  delta.

## Non-Goals

- No optimizer or gradient training.
- No offline dataset format.
- No emotional labels or value labels.
- No body-side stdout/stderr.
- No observer-to-body backflow.

## Success Criteria

- Tests prove a persistent session runs several episodes on one `Viewport`.
- Tests prove pair count and learned-bias evidence accumulate across episodes.
- Tests prove the report binary can run in persistent mode.
- Hygiene tests continue to reject ontology drift.
