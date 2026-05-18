# Multi-Gap Selection + Pixel Scale v1 Design

## Boundary

This stage extends held-out gap generalization from one unresolved gap to
multiple competing gaps. It stays observer-side and curriculum-side. It does not
add a body-side brain module, desire module, goal module, thought module, mind
module, request/response layer, naming gate, concept allocator, or token
ontology.

`Prediction pixel scale` is an external design phrase. In implementation it may
only appear as numeric scale evidence: local object-slot disturbance, relation
window disturbance, longer time/cross-channel disturbance, pressure, persistence,
closure, reentry, and retained binding/coupling strength.

## Purpose

The previous stage proved that a held-out blank can raise gap pressure, drive
probe writes, close, and reenter better on a persistent viewport than on a cold
baseline.

This stage asks a stricter question:

When several unresolved gaps coexist, can the observer prove that the substrate
selects the highest-pressure gap for active probing, and that larger prediction
scale produces longer persistence and stronger dependence on persistent history?

This remains a numeric evidence test. It is not a language meaning, label,
question-answering, or symbolic reasoning test.

## Training Shape

1. Train recurrent small-scale materials against object-slot targets.
2. Train recurrent mid-scale materials against relation-edge targets.
3. Train recurrent large-scale materials against time-window and cross-channel
   targets.
4. Introduce held-out disturbances at all three scales in one competing window.
5. Compute observer-side scale pressure from existing substrate evidence:
   ambiguity, recurrence, cross-channel support, relation/time span, and
   persistence.
6. Feed the selected gap into existing active gap and probe-law refresh paths.
7. Close the selected gap through normal action-consequence pairing and aligned
   correction.
8. Compare large-scale held-out reentry on persistent vs cold viewports.

## Observer Evidence

The report must emit compact numeric evidence:

- `small_gap_events`
- `mid_gap_events`
- `large_gap_events`
- `competing_gaps`
- `small_gap_pressure`
- `mid_gap_pressure`
- `large_gap_pressure`
- `selected_gap_index`
- `selected_gap_pressure`
- `selected_gap_pressure_is_max`
- `probe_writes_before_selection`
- `probe_writes_after_selection`
- `probe_bias_tracks_selected_gap`
- `small_gap_closure_ratio`
- `small_gap_persistence_score`
- `large_gap_persistence_score`
- `large_gap_persistent_strength`
- `large_gap_cold_strength`
- `large_gap_reentry_margin`
- `wrong_binding_strength`
- `principle_guard_pass`

## Pass Standard

- `small_gap_events >= 4`
- `mid_gap_events >= 4`
- `large_gap_events >= 4`
- `competing_gaps >= 3`
- `selected_gap_pressure_is_max = 1`
- `probe_bias_tracks_selected_gap = 1`
- `small_gap_closure_ratio > 0.40`
- `large_gap_persistence_score > small_gap_persistence_score`
- `large_gap_persistent_strength > large_gap_cold_strength`
- `large_gap_reentry_margin > 0.10`
- `wrong_binding_strength <= 0.01`
- `principle_guard_pass = 1`

## Principle Guard

Selection is an observer-side report over numeric substrate evidence, not a
body-side symbolic controller. Pixel scale is measured as pressure and
persistence across existing targets and channels; it must not become fixed world
content or a hard-coded theory of what the system is "really thinking".
