# Local Probe Chain v0 Design

## Boundary

This design extends the existing pressure-field / active-surface substrate from
"pressure can drive a probe" to "a probe-return pair can revise the local active
surface and influence the next probe pressure".

The design does not introduce `Thought`, `Goal`, `Plan`, `Question`, `Answer`,
`Desire`, `Mind`, `Subconscious`, or `Conscious` modules. Human words such as
"thinking", "desire", "blank", "focus", and "idea" remain external metaphors.
Inside the body, the admissible state is numeric/time evidence only:

- active pressure
- write epoch
- consequence pair
- curve similarity
- closure pressure
- mismatch pressure
- next probe pressure
- reentry / residue evidence

The observer may report these values. The body must not emit human-readable
text.

## Problem

The current body can form pressure waves, keep parallel active traces, expose a
single active aperture, and let aperture pressure influence probe drive.

The missing loop is local continuity:

1. A surface aperture contributes pressure.
2. The probe law emits a write epoch.
3. Ingress returns and is paired with that write epoch.
4. The pair's numeric alignment revises the active surface.
5. The revised surface either closes, keeps residue, or raises pressure for a
   next probe.

Without this loop, active prediction still looks like isolated probe events
rather than a substrate-level chain.

## Design

Add a local probe chain state inside `ActiveSurfaceState`.

Each consumed action-consequence pair creates a `LocalProbeChainTrace`:

- `pair_seq`
- `write_seq`
- `source_kind`
- `slot_id`
- `edge_id`
- `return_support`
- `closure_pressure`
- `mismatch_pressure`
- `next_probe_pressure`
- `last_update_t_us`

The pair is consumed once, by monotonic `pair_seq`.

### Pair Revision Law

For a new paired write-return record:

- `return_support = curve_similarity`
- if support is high, closure pressure rises and the current aperture pressure
  is reduced while residue remains available
- if support is low, mismatch pressure rises and the current aperture pressure
  is reinforced for another probe
- intermediate support produces a weak revision without forcing a hard branch

These are engineering approximations of substrate revision. They are not
success/failure semantics.

### Body Order

The body refresh order becomes:

1. refresh active reasoning gap
2. tick prediction pressure field from substrate
3. tick background relation resonance
4. refresh active surface
5. tick local probe chain from paired returns
6. refresh probe law
7. tick probe output

This lets already-returned evidence revise the surface before the next probe
decision.

## Observer Evidence

`curriculum_report --local-probe-chain` must print compact numeric evidence:

- `chain_steps`
- `closure_steps`
- `mismatch_steps`
- `return_revises_surface_pressure`
- `closed_focus_does_not_repeat_probe`
- `unclosed_focus_continues_probe`
- `next_probe_pressure_after_mismatch`
- `closure_pressure_after_return`
- `wrong_chain_binding`
- `body_source_order_guard`
- `principle_guard_pass`

## Guardrails

- No new semantic/token/concept/name/label gate.
- No human-readable body output.
- No fixed object ontology.
- No task planner.
- No request/response reasoning layer.
- Consequence alignment remains numeric curve similarity only.
