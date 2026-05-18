# Multi-Step Chain Continuity v0 Design

## Boundary

This design extends chain memory sedimentation from "a chain can leave residue"
to "several local probe-return steps can be recognized as one continuing
numeric chain, while nearby branch pressure remains alive".

The design does not add `Thought`, `Goal`, `Plan`, `Question`, `Answer`,
`Desire`, `Mind`, `Subconscious`, or `Conscious` modules. Those words remain
external metaphors only. The body stores only numeric/time substrate:

- pair sequence
- source kind
- slot id
- edge id
- sediment strength
- recurrence count
- continuity score
- branch pressure
- reentry pressure

No body-side human-readable output is introduced.

## Problem

Chain Memory Sedimentation v0 can preserve residue from one local chain. The
next requirement is continuity: a later probe-return pair on the same substrate
coordinate should strengthen the same chain rather than forming unrelated
isolated residue.

At the same time, "multiple foci can coexist" means adjacent chains must not be
deleted when one chain becomes dominant. They should remain as bounded branch
pressure. Execution still uses one active aperture at a time.

## Design

Extend `ChainMemoryTrace` with:

- `continuity_score`
- `branch_pressure`

Extend `ActiveSurfaceState` with observer-visible counters:

- `chain_memory_continuity_steps`
- `chain_memory_parallel_branch_count`
- `chain_memory_dominant_chain_index`
- `chain_memory_dominant_continuity_score`
- `chain_memory_branch_pressure`

When a local probe chain trace sediments:

- if its substrate coordinate (`slot_id`, `edge_id`) already exists, update the
  existing trace and raise continuity score; pressure source form may shift
  between gap/residue without becoming a new chain
- if its coordinate is new, create a new branch trace
- after updates, compute the dominant chain by continuity/sediment pressure
- compute branch pressure from non-dominant traces

This is not a plan graph and not a symbolic reasoning chain. It is a bounded
numeric continuity field over existing substrate coordinates.

## Observer Evidence

`curriculum_report --multi-step-chain-continuity` must emit:

- `chain_steps`
- `same_chain_continuity_steps`
- `dominant_continuity_score`
- `parallel_branch_count`
- `branch_pressure`
- `single_aperture_preserved`
- `branch_trace_retained`
- `dominant_chain_reentry_pressure`
- `wrong_chain_binding`
- `body_source_order_guard`
- `principle_guard_pass`

## Guardrails

- No labels, concepts, tokens, task IDs, planner, or question-answer layer.
- Coordinates are substrate indices only.
- Branches are pressure traces, not symbolic alternatives.
- Continuity is learned/revisable numeric recurrence, not fixed identity.
