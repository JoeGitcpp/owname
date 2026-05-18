# Auditory-Visual Predictability and Expression v0 Design

## Purpose

After vocal reflex closure, hearing and vision should be treated as continuous
body channels that help the substrate predict returns and express pressure.
This v0 slice proves two lower laws before any higher binding work:

```text
body vocal write
-> VIB return
-> numeric delay / magnitude closure
-> predictable return strength rises

VIB / PHOT aligned pressure
-> cross-channel coupling pressure
-> non-symbolic vocal write
-> expression evidence becomes observer-visible
```

This is not speech recognition, image recognition, event naming, or question
answering. It is a body-side sensorimotor loop with observer-side numeric
evidence.

## Boundary

"Hearing", "vision", "expression", and "prediction" are external descriptions.
Internal state may only store numeric substrate:

- return delay support
- return magnitude support
- predictable return strength
- VIB/PHOT alignment support
- drift pressure
- expressive pressure
- write epoch count
- closure count
- mismatch pressure

The implementation must not add:

- speech / word / text output gates
- image recognition or visual labels
- semantic / concept / token ontology
- emotion or intention modules
- human-readable output inside body

Observer reports may print numeric evidence only.

## Life-Like Model

In a life-like body, hearing first answers whether self-output returns through
the body world, and vision first answers whether a spatial change is stable and
temporally coupled with other channels.

The lowest useful loop is:

```text
I output a vocal curve
-> some vibration returns after a delay
-> delay and magnitude become expected
-> later output can anticipate return pressure
```

The next loop is:

```text
vibration pressure and visual motion arrive close together
-> coupling support rises
-> if coupling pressure remains active, the body can express through vocal write
```

The body does not need to know what the sound or visual pattern "means". It only
needs to learn whether pressure can be predicted and whether pressure can reach
an output channel.

## v0 Scope

Add a small numeric substrate, `auditory_visual`, held by `Viewport`.

It must support:

1. **Predictable return evidence**
   - Observe a vocal write epoch paired with VIB consequence return.
   - Increase `predictable_return_strength` when delay is short and magnitude is
     present.
   - Increase `return_closure_count`.

2. **Cross-channel pressure evidence**
   - Observe VIB/PHOT aligned activity from existing slot fusion.
   - Increase `cross_channel_predictability` on aligned activity.
   - Increase `drift_pressure` when VIB and PHOT timing separates.

3. **Expression evidence**
   - Convert strong aligned pressure into non-symbolic vocal write.
   - Count only action-output write epochs, not text.
   - Raise `expressive_pressure` and then lower it after closure.

4. **Observer proof**
   - Add `--auditory-visual-predictability-expression` report.
   - Report compact numeric evidence.

## Acceptance Evidence

The report must expose at least:

```text
mode=auditory_visual_predictability_expression
vocal_writes>=3
vib_returns>=3
predictable_return_strength>0.40
return_closure_count>=3
aligned_av_events>=4
cross_channel_predictability>0.40
drift_pressure_after_misalignment>0.05
expressive_pressure_peak>0.35
expression_write_epochs>=1
closure_reduces_expression_pressure=1
wrong_binding_strength<=0.01
body_source_order_guard=1
principle_guard_pass=1
```

## Principle Guard

This slice is admissible only if:

- no body source prints human-readable diagnostics,
- no speech / word / text / semantic / concept / token / label / recognition
  gate is introduced,
- output remains continuous vocal body state and action-output epochs,
- observer evidence is numeric and outside the body.
