# Held-Out Gap Generalization + Pressure-Driven Probe v1 Design

## Boundary

This stage extends post-foundation active-probe training after the current
sustained active probe curriculum. It keeps "thinking", "desire", "goal", and
"intention" as external metaphors only. The implementable substrate is:

- unresolved prediction gap pressure,
- pressure-driven probe write activity,
- closure evidence from action-consequence pairing,
- held-out disturbance reentry evidence,
- observer-side numeric report fields.

No body-side module named for thinking/desire/goal/intention is admissible.
No request/response path, human-readable body output, naming gate, discrete
concept allocator, or token ontology is introduced.

## Hypothesis

After foundation and sustained active probing, a trained persistent viewport
should respond to adjacent held-out disturbances by raising active gap pressure,
increasing probe output while the gap is unresolved, reducing probe output after
closure, and retaining stronger held-out bindings than a cold viewport given a
shorter reentry exposure.

This is a numeric substrate test of pressure and closure dynamics. It is not a
test of word meaning, question answering, labels, or internal symbolic concepts.

## Training Shape

1. Train at least ten recurrent materials against already-formed object,
   relation, and time-window targets.
2. Introduce at least six adjacent held-out materials that were absent from the
   training set.
3. Apply disturbance conflict to held-out materials by preserving ambiguity and
   recording wrong-target deviation pressure without strengthening wrong
   binding.
4. Refresh active-reasoning gap and probe-law state under the held-out conflict.
5. Close the active gap with action-consequence pairing and aligned correction.
6. Compare persistent held-out reentry against a cold viewport.

## Observer Evidence

The report must emit only numeric/boolean evidence:

- `trained_materials`
- `heldout_materials`
- `gap_pressure_before_heldout`
- `gap_pressure_after_heldout`
- `gap_pressure_after_closure`
- `gap_pressure_rises_under_heldout_conflict`
- `probe_writes_before_gap`
- `probe_writes_during_gap`
- `probe_writes_after_closure`
- `probe_count_rises_under_gap`
- `probe_count_falls_after_closure`
- `persistent_heldout_strength`
- `cold_heldout_strength`
- `persistent_reentry_margin`
- `wrong_binding_strength`
- `principle_guard_pass`

## Pass Standard

The stage is considered admissible when observer evidence satisfies:

- `trained_materials >= 10`
- `heldout_materials >= 6`
- `gap_pressure_rises_under_heldout_conflict = 1`
- `probe_count_rises_under_gap = 1`
- `probe_count_falls_after_closure = 1`
- `persistent_heldout_strength > cold_heldout_strength`
- `persistent_reentry_margin > 0.10`
- `wrong_binding_strength <= 0.01`
- `principle_guard_pass = 1`

## Principle Guard

The implementation is legal only if it stays in curriculum/report/test surfaces.
The body remains a continuously running substrate; observer reporting remains
external. New high-risk language must not add internal ontology. If high-risk
terms appear in docs or tests, they are external description or engineering
indices only, not body-side structures.
