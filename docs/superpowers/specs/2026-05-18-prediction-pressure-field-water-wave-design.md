# Prediction Pressure Field Water-Wave v0 Design

## Boundary

This design locks the "water-wave" formulation of active prediction. It does
not create a thought module, mind module, conscious module, subconscious module,
desire module, goal module, idea module, semantic gate, concept allocator, token
path, or question-answering layer.

`潜意识` and `表意识` remain external metaphors:

- `潜意识`: many parallel body-side prediction pressures and residues.
- `表意识`: one dominant active prediction focus selected from those pressures.

The implementation name is `prediction_pressure_field`, a numeric substrate
field. It stores waves, pressure, persistence, resonance, closure affordance,
write risk, inhibition, and a single dominant focus. It does not store what
anything means.

## Core Model

The system should not decide active prediction with a program-shaped branch:

```text
if idle -> think
if event -> reason
if match -> sudden idea
```

Instead, it should maintain a continuous pressure field:

```text
many pressure waves
  -> decay, persistence, resonance, inhibition, closure feedback
  -> one dominant focus emerges
  -> existing probe law may write out
  -> returned closure lowers the focused wave
  -> unclosed waves leave residue for later reentry
```

The field can express three observed active-prediction shapes without separate
modules:

1. quiet reentry: passive load falls and a retained wave rises;
2. event-forced active prediction: a strong event wave overrides idle residue;
3. relation resonance spike: a low-strength relation wave suddenly matches and
   becomes dominant.

These are pressure regimes, not modules.

## V0 Scope

V0 adds the smallest useful substrate:

- `PredictionWaveInput`: numeric input sample for one pressure wave;
- `PredictionWaveTrace`: bounded state for one wave;
- `PredictionPressureFieldState`: bounded collection and dominant focus summary;
- `Viewport::refresh_prediction_pressure_field(now_us, inputs)`;
- `Viewport::apply_prediction_field_closure(feedback)`;
- `Viewport::read_prediction_pressure_field()`;
- `probe_law` includes the field's dominant pressure as one possible drive.

No snapshot schema change is required in v0. Observer evidence comes through
tests and curriculum report. Snapshot integration can be a later step once the
field behavior is stable.

## Dynamics

Each refresh applies:

- decay to all existing waves;
- persistence growth for surviving waves;
- merge or create incoming waves by numeric coordinates;
- resonance boost when a wave matches current/history pressure;
- inhibition from stronger competing waves;
- focus score with inertia for the previous dominant wave;
- bounded focus switching so the field does not become a scheduler.

V0 coordinates are:

- `source_kind`: numeric kind only;
- `slot_id`;
- `edge_id`;
- `window_start_us`;
- `window_end_us`.

The field may record a dominant wave index, but this is an engineering handle,
not an idea, label, target, or concept.

## Observer Evidence

V0 report evidence should prove:

- at least three waves can coexist;
- only one dominant focus is selected;
- focus switching is bounded by inertia;
- quiet reentry can raise a retained wave;
- event pressure can override an idle wave;
- relation resonance can spike a wave;
- closure reduces the focused wave;
- unclosed residue remains after decay;
- dominant pressure can drive existing probe law;
- principle guard passes.

## Pass Standard

- `waves_alive >= 3`
- `dominant_focus_is_single = 1`
- `focus_switches_bounded = 1`
- `quiet_reentry_wave_rises = 1`
- `event_wave_can_override_idle_wave = 1`
- `relation_resonance_spike_observed = 1`
- `closure_reduces_focused_wave = 1`
- `unclosed_wave_residue_retained = 1`
- `dominant_pressure_drives_probe = 1`
- `principle_guard_pass = 1`

## Principle Guard

This is a pressure field, not a symbolic controller. It must not:

- emit human-readable body text;
- store idea/thought/desire/goal labels;
- store word/token/concept ids;
- decide meaning;
- directly write memories as truth;
- choose tasks or answer questions.

It may only contribute bounded numeric pressure into already legal active
prediction/probe substrates.
