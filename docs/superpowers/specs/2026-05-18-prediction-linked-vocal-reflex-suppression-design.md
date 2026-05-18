# Prediction-Linked Vocal Reflex and Suppression Design

## Purpose

This spec defines how non-planned vocal output can emerge as a body reflex under
prediction shock, and how the reflex can weaken over growth through prediction
stabilization and suppression learning.

The target is not speech, text output, or a communicative response layer. The
target is a body-side vocal reflex substrate:

```text
prediction error spike
→ vocal reflex pressure
→ breath / pitch / resonance / noise / envelope perturbation
→ vocal output
→ VIB return / consequence
→ correction, suppression, or sedimentation
```

## Boundary

"Surprise", "anger", "sadness", "fear", "maturity", and "emotion" are external
observer descriptions only. They are not internal ontology modules.

The body must not add:

- `Emotion`
- `Anger`
- `Sadness`
- `Fear`
- `Surprise`
- `Maturity`
- `Desire`
- `Thought`
- `Conscious`
- `Subconscious`
- text, word, concept, semantic, or question-answer output gates

Body-side state may only use continuous numeric substrate such as:

- prediction error pressure
- suddenness pressure
- unresolved gap pressure
- familiarity / habituation pressure
- learned suppression pressure
- vocal reflex pressure
- breath burst
- pitch tension
- resonance displacement
- noise mix
- envelope impulse
- return closure / mismatch evidence

Human-readable interpretation belongs only to observer-side documents or
reports.

## Human Reference Model

In humans, sudden vocalization under shock is not usually a planned action.
The simplified chain is:

```text
sudden external or internal event
→ large prediction failure
→ alert / startle activation
→ respiratory and laryngeal perturbation
→ short uncontrolled vocal emission
→ auditory / bodily / social consequence
→ later regulation
```

A child may vocalize easily because:

- world prediction is less stable
- many events produce high prediction error
- motor inhibition is immature
- consequence history is shallow
- social suppression curves are not yet sedimented

An adult may vocalize less because:

- familiar events produce lower prediction error
- top-down inhibition is stronger
- repeated consequence teaches suppression
- body regulation routes pressure into breath hold, posture, facial tension, or
  silent internal activity instead of vocal output

The reflex does not disappear. Its output threshold changes.

## Core Law

Vocal reflex is linked to prediction, not to emotion labels.

The abstract pressure relation is:

```text
vocal_reflex_pressure rises with:
  prediction_error_spike
  suddenness
  unresolved pressure
  body energy available for output
  recent failed closure

vocal_reflex_pressure falls with:
  familiarity
  learned suppression
  successful quiet regulation
  repeated harmless exposure
  low energy / shutdown pressure
```

Implementation may approximate this relation with numbers, but the theory is
not "if surprised then make sound". The theory is continuous pressure passing
through a body channel.

## Growth / Habituation

Growth is not a fixed maturity flag. It is accumulated prediction and
consequence history.

Repeated exposure to similar shock patterns should form:

```text
similar event recurrence
→ familiarity rises
→ prediction error spike falls
→ vocal reflex pressure falls
```

Repeated successful non-vocal regulation should form:

```text
high shock pressure
→ no vocal output or reduced output
→ stable consequence / closure
→ suppression support rises
→ later vocal reflex threshold rises
```

Repeated harmful or high-urgency unresolved events may do the opposite:

```text
shock pressure remains high
→ closure absent
→ suppression cannot fully gate output
→ reflex vocal pressure remains available
```

This keeps the design life-like: the system does not merely become silent. It
learns when direct vocal leakage is unnecessary, and when pressure remains too
high to suppress.

## Fast / Slow Channels

The reflex should use a fast/slow split.

Fast channel:

- rises from a single high prediction shock
- can drive immediate vocal perturbation
- decays quickly if not reinforced
- allows "startle-like" output without planned language

Slow channel:

- sediments repeated exposure, closure, and suppression history
- changes future threshold
- supports long-term reduction of unnecessary reflex leakage
- remains revisable if later evidence changes

This is analogous to short-term reflex amplification and long-term regulation
learning. It is not a programmed response table.

## Vocal Body Mapping

Vocal output should remain continuous:

```text
vocal_reflex_pressure
→ breath_drive impulse
→ pitch_tension perturbation
→ resonance_position displacement
→ noise_mix increase
→ short envelope burst
```

Different pressure profiles may externally resemble different human states:

- high suddenness + high energy may sound like shock
- high unresolved pressure + high breath drive may sound forceful
- low energy + high closure loss may sound weak or broken
- high suppression + high internal pressure may produce no sound or only a
  small breath/noise trace

These are observer interpretations. The body only carries numeric curves.

## Interaction With Existing Substrate

The design should attach to existing body substrate:

- prediction pressure field
- active surface pressure
- evidence tension
- action-consequence return
- VIB ingress
- vocal target
- vocal imitation
- vocal organ
- vocal memory

Preferred flow:

```text
prediction pressure / evidence tension spike
→ vocal reflex pressure trace
→ vocal organ perturbation
→ action output epoch
→ VIB / consequence return
→ suppression or reflex support update
→ vocal memory / regulation sediment
```

The reflex must not bypass existing action-output and consequence pairing
contracts.

## Observer Evidence

A later v0 implementation should expose only numeric observer evidence, for
example:

- `shock_events`
- `reflex_pressure_peak`
- `vocal_burst_epochs`
- `vocal_burst_energy`
- `familiarity_after_repeats`
- `suppression_strength`
- `reflex_pressure_after_habituation`
- `quiet_regulation_successes`
- `urgent_pressure_bypasses_suppression`
- `vib_return_pairs`
- `principle_guard_pass`

Possible acceptance for v0:

- first high prediction shock raises reflex pressure
- high reflex pressure can produce a non-symbolic vocal burst
- repeated similar shock lowers later reflex pressure
- successful quiet regulation raises suppression support
- high unresolved pressure can still bypass suppression
- VIB return/consequence pairs are recorded
- no text, word, concept, semantic, or emotion ontology is introduced
- body emits no human-readable output

## Rejected Designs

The following are not allowed:

```text
if angry -> loud voice
if sad -> low voice
if surprised -> say a fixed sound
if adult -> suppress output
```

Also rejected:

- fixed emotion categories
- fixed vocal response labels
- direct text-to-speech behavior
- symbolic speech planning in the body reflex layer
- social rule tables
- response templates

## Summary

Automatic vocalization is a prediction-linked body reflex. It emerges when
prediction shock exceeds the current suppression and familiarity regulation.
With repeated exposure and successful regulation, the same stimulus produces
less reflex leakage. Under unresolved or urgent pressure, the reflex can still
surface.

For silicon-life design, this means vocal output should become a living body
channel: pressure-driven, consequence-shaped, suppressible, revisable, and
observable only through numeric evidence.
