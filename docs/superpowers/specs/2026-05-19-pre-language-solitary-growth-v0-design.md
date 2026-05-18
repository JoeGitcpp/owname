# Pre-Language Solitary Growth v0 Design

> **Status:** DESIGN CANDIDATE
> **Purpose:** define the hard gate before language training: the body must
> continue growing while alone, without a teacher, prompt, curriculum, or
> explicit human feedback.

## Core Principle

A life substrate must not be idle just because nobody is teaching it. If it is
alone in a room, it should still grow from continuous physical ingress:

```text
room light / shadow / camera noise / microphone floor / screen changes
-> passive prediction
-> local gap or stability
-> quiet observation, replay, or small probe
-> prediction improvement or residue
-> sedimented familiarity with the room
```

If there is no external change at all, the body cannot learn new world facts.
It should still reorganize existing traces: replay, reentry, compression,
stability improvement, and suppression of useless output.

This is the difference between a life-like substrate and a request/response
program. A program waits. A life substrate continues regulating, predicting,
repairing, and sedimenting.

## Boundary Notes

`language`, `speech`, `word`, and `talking` are external observer descriptions
in this document. They are not body goals for this gate. Early vocal output is
allowed only as physical disturbance and action-consequence probing.

`person`, `face`, `back of head`, `gesture`, `family conversation`, and
`recognition` are also external descriptions. Internally they must remain
continuous PHOT/VIB/relation curves, timing recurrence, prediction improvement,
and reentry. No fixed person ID, identity field, owner model, personality
field, semantic label, or concept allocator is admissible.

`familiarity` is legal only as a bounded numeric sediment: repeated recurrence
plus lower prediction gap plus reentry stability. It is not identity.

## Relationship To Current Work

This design follows:

- [Self-Development Predictive Life Loop v0](./2026-05-19-self-development-predictive-life-loop-v0-design.md)
- [Live Human Nursery v0](./2026-05-18-live-human-nursery-v0-design.md)
- [Thinking as gap-driven active reasoning](./2026-05-17-thinking-gap-driven-active-reasoning-design.md)

`SelfDevelopmentState` is the first body-side substrate that can express this
direction: prediction gap, improvement, stable recurrence, familiarity
pressure, repetition pressure, exploratory pressure, and output suppression.

Pre-Language Solitary Growth v0 adds a stricter interpretation: those signals
must be useful when nobody is actively teaching the body.

Live Human Nursery remains an environment and observer report. It is not the
source of growth. Product or language checkpoints may only run after this gate
shows that the body can grow from ordinary continuous existence.

## What Must Grow While Alone

### 1. Room Stability

The body should accumulate stable recurrence around persistent background
structure:

- static visual regions;
- lighting gradients;
- screen boundaries;
- microphone noise floor;
- repeated low-energy environmental pulses.

The observer should see stable recurrence and familiarity pressure rise without
teacher phases or curriculum commands.

### 2. Physical Continuity

The body should maintain prediction across:

- stillness;
- small motion;
- disappearance and reappearance;
- near/far changes;
- sound onset and offset;
- repeated timing rhythms.

This is not object naming. It is continuity of curves and lower future error.

### 3. Quiet Internal Work

During low external change, the body should not stall. It should show:

- quiet replay;
- reentry of recent traces;
- stronger stability around repeated curves;
- suppression of useless write-output;
- lower exploratory pressure when no unresolved gap remains.

### 4. Self-Generated Probe Discipline

The body may make small vocal or digital probes, but only under unresolved
pressure. If no feedback returns, repeated probing should self-suppress.

If a vocal probe changes the room, for example a human turns toward the screen,
that becomes ordinary action-consequence evidence. It is not language yet.

## What Must Not Happen

This gate fails if:

- the body only changes when a curriculum runner drives it;
- the body only looks alive when a human teaches it;
- no-teacher runtime keeps `stable_recurrence`, `prediction_improvement`, and
  `familiarity_pressure` near zero;
- no-feedback runtime saturates write epochs or return windows;
- vocal output repeats without useful prediction improvement;
- a proposed implementation introduces fixed person, identity, owner,
  personality, word, label, token, concept, or semantic structures.

## Observer Evidence

The observer report for this gate must compare at least two windows:

```text
baseline window
-> solitary runtime window
-> optional reentry window after disturbance
```

Required numeric evidence:

- `runtime_snapshot_read=1`;
- self-development `total_updates` increases;
- `stable_recurrence` rises above cold baseline;
- `familiarity_pressure` rises above cold baseline;
- `prediction_gap` either lowers after repeated stability or remains bounded
  without causing output saturation;
- `repetition_pressure` remains bounded during quiet periods;
- `output_suppression` rises when low-quality repetition appears;
- action-output write epochs do not saturate in quiet no-feedback runtime;
- no body-side human-readable output appears.

## Later Language Gate

Language training may begin only after solitary growth shows:

1. the room can become familiar without a teacher;
2. repeated external sources can reduce prediction error;
3. self-generated output is bounded and feedback-sensitive;
4. no-feedback repetition is suppressed;
5. observer evidence can distinguish growth from mechanical activity.

The first spoken word should be treated as a late emergent consequence of
living with voices, faces, gestures, turns, and feedback. It must not be the
first training objective.

## v0 Acceptance

The implementation plan should prove, with TDD:

1. a solitary low-change input sequence increases recurrence and familiarity
   without curriculum phase flags;
2. quiet replay/reentry can update growth evidence while external pressure is
   low;
3. no-feedback repeated output increases suppression instead of continuing
   indefinitely;
4. a later disturbance can reenter prior room familiarity better than cold
   baseline;
5. the report refuses language-readiness claims when solitary growth evidence
   is missing;
6. source guards reject all fixed ontology shortcuts listed above.
