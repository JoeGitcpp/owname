# Background Relation Resonance / Sudden-Match v0 Design

## Boundary

This design extends the existing active-surface substrate with background
relation resonance traces. It does not add an association module, idea module,
thought module, desire module, goal module, mind module, consciousness module,
task scheduler, concept allocator, naming gate, token layer, or question/answer
layer.

"Sudden idea" remains an external metaphor. The body implementation only
observes numeric relation pressure, recurrence, residue, match score, decay,
spike, and aperture entry evidence.

## Goal

Let low-strength relationship attempts remain active in the background and
decay naturally, while a high-match relation can suddenly spike into the
prediction pressure field and then be evaluated by the active surface.

This directly follows the active aperture slice:

```text
parallel focus traces
background relation attempts
high relation match spike
prediction pressure field relation wave
active surface aperture / echo decision
```

No attempt trace owns object meaning. It only references slot and edge indices.

## Data Model

Add `RelationResonanceTrace` inside `active_surface.rs`.

Fields are numeric/time/index only:

- edge index,
- slot index,
- attempt pressure,
- match score,
- residue support,
- spike pressure,
- last update time.

Add these fields to `ActiveSurfaceState`:

- `relation_resonance_traces`,
- `relation_resonance_traces_alive`,
- `low_match_decay_count`,
- `sudden_match_count`,
- `sudden_match_enters_surface_count`,
- `wrong_match_leakage`,
- `last_relation_resonance_t_us`.

## Refresh Rule

`Viewport::tick_active_surface_relation_resonance(now_us)` reads:

- active surface traces,
- global relation edges,
- global reasoning memory summary.

For each active relation edge:

1. Compute residue support from active-surface traces matching the same edge or
   nearby slot.
2. Compute match score from relation surprise, relation confidence, predictive
   score, memory recurrence, routing stability, and residue support.
3. Update or create a background relation trace.
4. Low match decays the attempt pressure and contributes to decay evidence.
5. High match spikes the trace and injects one bounded relation wave into
   `prediction_pressure_field`.

The injected relation wave is not an execution decision. It only raises pressure
that the existing active surface must arbitrate.

## Body Refresh Order

The body order becomes:

```text
active_reasoning_gap
prediction_pressure_field_from_substrate
active_surface_relation_resonance
active_surface
probe_law
probe_output
snapshot
```

Relation resonance must run after pressure-field hydration and before active
surface refresh, so sudden-match pressure can enter the same cycle's aperture
decision.

## Observer Evidence

`curriculum_report --background-relation-resonance-sudden-match` reports:

- background attempts alive,
- low-match attempts decay,
- high-match spike observed,
- sudden match enters surface,
- wrong-match leakage,
- aperture relation entry,
- body source order guard,
- principle guard.

## Acceptance

The slice is acceptable only if:

- background attempts can exist without entering aperture,
- low-match attempts decay,
- high-match attempts spike,
- a spike can enter active surface aperture or echo,
- wrong-match leakage remains bounded,
- body order is pressure field -> relation resonance -> active surface -> probe,
- touched sources avoid rejected architecture terms and body human-readable
  output.
