# Active Reasoning Persistence v0 Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Implement the first substrate-level proof that active reasoning is not a
single-cycle program trigger.

The system already has passive prediction, relation reasoning, reasoning
history, probe law, and action-consequence pairing. Active Reasoning
Persistence v0 adds a small numeric active-gap substrate that proves an
unclosed structure can:

1. form from prediction residue and relation instability;
2. persist across multiple cycles;
3. drive probe pressure;
4. reduce when paired return evidence supports closure;
5. re-enter when quiet history makes the same unfinished structure active
   again.

## Architecture

Add a focused module:

`bionic_v0/src/active_reasoning.rs`

It defines:

- `ActiveReasoningGapState`
- `Viewport::refresh_active_reasoning_gap`
- `Viewport::read_active_reasoning_gap`

`Viewport` stores one runtime numeric active-gap state next to existing
`global_reasoning`, `global_reasoning_history`, and `probe_law`.

`probe_law` reads the active-gap pressure as one more legal pressure source.
This keeps probe output unified through the existing action-output path.

## Data Flow

Input surfaces:

- `GlobalReasoningState`
  - surprise load;
  - network instability;
  - top routing slot and edge ids.
- `GlobalReasoningMemorySummary`
  - novelty;
  - persistence;
  - recurrence;
  - dominant focus persistence.
- `ActionConsequenceState`
  - newest paired write/return similarity.

Output surface:

- `ActiveReasoningGapState`
  - active gap pressure;
  - persistence score;
  - closure score;
  - re-entry score;
  - focus slot and edge ids;
  - active cycle count;
  - last consumed pair sequence;
  - update time.

All fields are numeric ids, counts, times, or bounded floats.

## Law

Active gap pressure rises when current relation reasoning carries unclosed
prediction residue or when current reasoning history says the pattern is novel,
persistent, or recurrent.

Active gap pressure does not drop to zero immediately when the current input
falls quiet. It decays as carried residue. This is the v0 proof of active
reasoning persistence.

When a new paired action-consequence event arrives with high curve similarity,
closure score rises and active gap pressure is reduced. The pair is not treated
as success or meaning. It is only numeric closure support.

When current drive is low but history recurrence and persistence are high, a
small re-entry score raises active gap pressure again. This is not a queue or a
todo list. It is history making an unfinished numeric structure active again.

## High-Risk Term Boundary

`reasoning` means numeric routing and relation-pressure substrate, not language
reasoning or request/response thought.

`active` means pressure can affect probe law, not free will or command
execution.

`gap` means unclosed numeric prediction structure, not absence of identity or a
symbolic goal.

`closure` means reduced numeric residue after paired return evidence, not truth,
success, or semantic correctness.

`re-entry` means history recurrence reactivates a numeric pressure pattern, not
task scheduling or a hidden plan.

No implementation may add source identity, owner, ethics, personality fields,
concept ids, word ids, token ids, label logic, target-content logic, prompt
answers, or body-side human-readable output.

## Acceptance

Tests must prove:

1. sustained unclosed reasoning pressure forms an active gap;
2. the active gap persists after immediate pressure falls;
3. active gap pressure can drive probe output through the existing probe law;
4. paired action-consequence evidence reduces active gap pressure and raises
   closure score;
5. recurrence and persistence in reasoning history can re-enter a weak gap;
6. source hygiene rejects rejected ontology and body-output drift;
7. focused and full crate tests pass.

## Non-Goals

- No personality module.
- No internal self or identity field.
- No fixed goal system.
- No task planner.
- No prompt-answer reasoning.
- No language semantics.
- No persistent on-disk active-gap state.
- No new action-output channel.
