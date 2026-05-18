# Chain Memory Sedimentation v0 Design

## Boundary

This design extends Local Probe Chain v0 by letting repeated local
probe-return revision leave a bounded, revisable numeric residue that can later
reenter the active surface.

The design does not add `Thought`, `Goal`, `Plan`, `Question`, `Answer`,
`Desire`, `Mind`, `Subconscious`, or `Conscious` modules. These remain external
metaphors only. Internally, the body stores only numeric/time substrate:

- pair sequence
- source kind
- slot id
- edge id
- return support
- closure support
- mismatch support
- sediment strength
- reentry pressure
- last update time

No human-readable output is emitted by the body. Observation remains in
`curriculum_report`.

## Problem

Local Probe Chain v0 proves that one write-return pair can revise the current
active surface. That is still too local: once the current trace decays, the
system has only short-term evidence. The next step is to let chain evidence
settle into a bounded substrate trace so that a previously worked-through local
chain can reenter under later pressure.

## Design

Add chain memory traces to `ActiveSurfaceState`.

Each new local probe chain trace is consumed once by pair sequence and folded
into a chain memory trace keyed by:

- `source_kind`
- `slot_id`
- `edge_id`

The trace accumulates:

- closure support from high-support returns
- mismatch support from low-support returns
- sediment strength from recurrence and support
- reentry pressure from sediment strength plus unresolved mismatch

Sediment decays over time. A trace is retained only while its numeric pressure
remains above a small floor.

## Reentry

When a chain memory trace has enough sediment strength, it may inject a
`PREDICTION_WAVE_RESIDUE` pressure input into the existing prediction pressure
field. This is not a lookup or symbolic recall. It is a pressure rehydration
from a numeric substrate trace.

The active surface then receives this through the already-existing pressure
field projection path.

## Body Order

The body order becomes:

1. pressure field hydration from substrate
2. background relation resonance
3. active surface refresh
4. local probe chain pair revision
5. chain memory sedimentation / residue reentry
6. probe law refresh
7. probe output

The reentry pressure can influence the next cycle's surface/probe decision
without bypassing the pressure field.

## Observer Evidence

`curriculum_report --chain-memory-sedimentation` must emit compact numeric
evidence:

- `chain_memory_traces_alive`
- `sediment_updates`
- `closure_sediment_strength`
- `mismatch_sediment_strength`
- `reentry_pressure`
- `sediment_reentry_observed`
- `persistent_reentry_pressure`
- `cold_reentry_pressure`
- `persistent_reentry_margin`
- `duplicate_pair_sediment_guard`
- `body_source_order_guard`
- `principle_guard_pass`

## Guardrails

- No new ontology modules.
- No fixed target/result/success labels.
- No token, concept, naming, or question-answer layer.
- No human-readable body output.
- The trace is bounded and decays; it can be revised or dissolved.
