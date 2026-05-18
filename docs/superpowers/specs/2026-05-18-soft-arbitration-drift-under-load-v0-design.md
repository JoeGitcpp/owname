# Soft Arbitration Drift Under Load v0 Design

## Boundary

This design extends chain continuity with a soft numeric arbitration field under
load. It does not add `Thought`, `Goal`, `Plan`, `Question`, `Answer`,
`Desire`, `Mind`, `Subconscious`, or `Conscious` modules. Those remain external
metaphors only.

The body stores only numeric/time substrate:

- chain coordinate
- dominance pressure
- challenger pressure
- arbitration margin
- hold count
- switch count
- reentry count
- branch pressure

No body-side human-readable output is introduced.

## Problem

Multi-Step Chain Continuity v0 proves that one chain can strengthen over
several steps and that a branch can remain alive. The next requirement is load:
when multiple chains repeatedly receive pressure, the system should not hard-cut
between them or erase non-current chains.

Instead it should show:

- hold: current dominant chain remains dominant when the challenger margin is
  small
- drift: challenger pressure can rise without immediate deletion or forced
  switch
- switch: sustained challenger pressure can become dominant
- reentry: an older chain can regain dominance when pressure returns

## Design

Extend `ActiveSurfaceState` chain memory arbitration with:

- `chain_memory_soft_arbitration_ticks`
- `chain_memory_soft_hold_count`
- `chain_memory_soft_switch_count`
- `chain_memory_soft_reentry_count`
- `chain_memory_competition_pressure`
- `chain_memory_challenger_pressure`
- `chain_memory_arbitration_margin`
- current/previous dominant substrate coordinates

Dominance selection uses inertia:

- If no dominant exists, choose the strongest chain.
- If a challenger is stronger but margin is small, hold the current dominant.
- If a challenger is stronger by a sufficient margin, switch softly and keep
  old chain as branch pressure.
- If the previous dominant coordinate returns later, count reentry.

The arbitration is not a planner and not a symbolic control loop. It is a
numeric continuity/pressure field over substrate coordinates.

## Observer Evidence

`curriculum_report --soft-arbitration-drift-under-load` must emit:

- `load_ticks`
- `hold_count`
- `switch_count`
- `reentry_count`
- `competition_pressure`
- `challenger_pressure`
- `arbitration_margin`
- `branch_pressure_retained`
- `single_aperture_preserved`
- `dominant_chain_reentered`
- `wrong_chain_binding`
- `body_source_order_guard`
- `principle_guard_pass`

## Guardrails

- No labels, concepts, tokens, task IDs, planner, or question-answer layer.
- Coordinates are substrate indices only.
- Branches remain bounded pressure traces.
- Non-dominant chains must not be deleted merely because they are not current.
