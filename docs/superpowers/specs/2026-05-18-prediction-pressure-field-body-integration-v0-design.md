# Prediction Pressure Field Body Integration v0 Design

## Boundary

This design enables the prediction pressure field inside the body refresh loop.
It does not add a new brain module, thought module, desire module, intention
module, conscious module, request/response layer, token layer, naming gate, or
question-answer layer.

"Subconscious", "conscious", "thought", "desire", "blank", and "sudden idea"
remain external metaphors. The body implementation only sees numeric substrate
pressure, temporal persistence, relation resonance, closure support, write risk,
and probe output evidence.

## Goal

The body should continuously hydrate the existing prediction pressure field from
already-learned substrate state so active probe pressure can arise without a
caller manually injecting wave inputs.

The target behavior is:

- quiet substrate pressure keeps the field weak and does not cause write-out,
- unresolved gap/evidence/relation pressure raises waves,
- residual recurrence can re-enter during quieter periods,
- event pressure can override idle residue when it is strong enough,
- closure lowers the focused wave while unclosed residue remains available,
- the probe law can be driven by the field after the body tick runs,
- wave count and focus switching remain bounded under stress.

## Input Sources

`Viewport::tick_prediction_pressure_field_from_substrate(now_us)` is the only
new body-side hydration entry point.

It reads existing numeric substrate only:

- `active_reasoning_gap`: gap pressure, persistence, closure, reentry, focus
  slot, and focus edge.
- `evidence_tension`: unresolved pressure and recent tension.
- `global_relations`: highest active relation surprise and its edge index.
- `global_reasoning_memory_summary`: recurrence, persistence, novelty, routing
  stability, and dominant focus persistence.

It writes only `prediction_pressure_field`.

## Wave Mapping

Gap wave:

- amplitude from active gap pressure,
- resonance from persistence/reentry,
- closure affordance from closure score and persistence,
- write risk reduced when closure affordance is high.

Event wave:

- amplitude from unresolved evidence pressure,
- resonance from current tension,
- closure affordance from active gap closure and memory routing stability,
- write risk rises when evidence is unresolved but closure support is low.

Relation wave:

- amplitude from the maximum active relation surprise,
- resonance from relation confidence and memory recurrence,
- closure affordance from relation predictive score and routing stability,
- write risk decreases as relation confidence rises.

Residue wave:

- amplitude from recurrence times persistence during lower immediate pressure,
- resonance from recurrence and dominant focus persistence,
- closure affordance from routing stability,
- write risk remains moderate because residue alone should not dominate every
  cycle.

All mapped values are clamped numeric approximations. They are implementation
shadows of the theory, not theory-layer ontology.

## Body Refresh Order

The body refresh chain becomes:

1. attention
2. relations
3. relation predictor
4. global reasoning
5. reasoning history and memory summary
6. evidence tension
7. active reasoning gap
8. prediction pressure field hydration
9. probe law
10. probe output
11. snapshot write

The pressure field tick must run after active gap refresh and before probe law
so the probe law can observe the latest wave pressure.

## Observer Evidence

Observation remains outside the body. `curriculum_report` gets a compact numeric
mode:

`--prediction-pressure-field-body-integration`

It reports:

- body ticks run,
- quiet write epochs,
- event write epochs,
- waves alive,
- dominant single-focus flag,
- body gap/event/relation/residue wave flags,
- closure drop flag,
- bounded wave count flag,
- bounded focus switch flag,
- stress ticks,
- maximum dominant pressure,
- body source order guard,
- principle guard.

## Admission Guards

The implementation is admissible only if:

- no body-side human-readable output is added,
- no forbidden naming/token/concept/symbolizer architecture is introduced,
- no new ontology module represents thought, desire, mind, consciousness, goal,
  owner, ethics, or personality,
- tests prove quiet pressure does not produce write-out,
- tests prove pressure can drive probe write-out after the body field tick,
- tests prove source order is active gap -> prediction pressure field -> probe
  law,
- stress tests keep wave count and switching bounded.
