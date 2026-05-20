# Allostatic Stability Scheduler Design

Status: FROZEN CANDIDATE for theory alignment on 2026-05-20.

Chinese term: `应变稳态调度系统`.

## Purpose

This document defines the allostatic stability scheduler as a life-state
resource regulation layer for the silicon-life substrate.

The scheduler does not simulate human emotion. It regulates pressure, reserve,
recovery, expression, exploration, and protection so the body can avoid constant
full-load operation while still supporting short-lived high-performance bursts
when the substrate has enough reserve and lawful closure support.

## Boundary

The highest principle remains unchanged: the system is a silicon-life
substrate, not a request/response program. Research observation and human
design descriptions stay outside the body.

`emotion`, `mood`, `pleasant`, `fatigue`, `anxiety`, `calm`, `overdrive`,
`buff`, and `debuff` are external observer descriptions only. They are not
internal ontology units and must not become body-side state machines.

High-risk terms used in this document are bounded as follows:

- `token`, `label`, `name`, `semantic`, `concept`, and `personality` appear
  only as banned implementation shapes. They are not internal ontology units.
- The scheduler may use engineering indices and numeric state fields, but those
  fields are continuous resource and pressure traces, not world-content
  classifications.
- The design does not introduce text understanding, identity, owner, ethics, or
  personality modules.
- The body must not print human-readable diagnostics. Observer and report tools
  may describe scheduler evidence externally.

Implementation must not add:

- `emotion_state`;
- `happy_state`;
- `sad_state`;
- `anger_module`;
- `fear_module`;
- `personality_emotion`;
- fixed emotion labels;
- fixed human-like reaction templates;
- body-side five-phase ontology;
- body-side human-readable output.

## Core Position

Human emotion is not copied into the substrate. The substrate needs the lower
life mechanism beneath emotion: resource mobilization, pressure regulation,
protective downshift, recovery debt, and adaptive overdrive.

For this project, the correct design object is:

```text
allostatic stability scheduler =
  demand sensing
  + reserve estimation
  + recovery debt
  + lawful output gain
  + protective limit
  + exploration allowance
  + recovery/downshift tendency
```

The scheduler is allowed to produce externally observable patterns that look
like energy, fatigue, hesitation, excitement, or calm. Those words remain
observer-side descriptions. Internally the body only carries numeric curves.

## Relationship To Existing Architecture

The scheduler should couple with existing substrate laws rather than introduce
a new emotional layer.

Existing sources that can feed scheduler evidence:

- `evidence_tension.unresolved_pressure`;
- `active_reasoning_gap.gap_pressure`;
- `active_reasoning_gap.closure_score`;
- `active_reasoning_gap.reentry_score`;
- `prediction_pressure_field.dominant_pressure`;
- `self_development.exploratory_pressure`;
- `self_development.output_suppression`;
- `self_development.familiarity_pressure`;
- `self_development.quiet_replay_pressure`;
- `probe_law.intensity`;
- `probe_law.suppression`;
- `probe_law.output_intensity`;
- `action_output.n_write_epochs`;
- `action_consequence.n_windows`;
- `action_consequence.n_pairs`;
- `learned_bias` channel stability;
- future hardware telemetry such as energy budget, thermal pressure, actuator
  load, sensor noise, battery reserve, and cooling margin.

Existing sinks that may later consume scheduler output:

- `probe_law` drive scaling;
- action/vocal output gain;
- exploration pressure admission;
- active surface expression intensity;
- live byte and future hardware output frequency;
- sensor sampling rate;
- low-power or recovery runtime profile selection.

The scheduler must not bypass existing laws. It modulates intensity and budget;
it does not choose semantic action content.

## State Model

The minimal scheduler state is continuous and bounded:

```text
arousal_load
reserve_level
recovery_debt
bearing_capacity
exploration_allowance
expression_gain
protective_limit
overdrive_window
downshift_pressure
last_update_t_us
```

Meanings:

- `arousal_load`: current activation demand from pressure, novelty, unresolved
  gaps, and live feedback deviation.
- `reserve_level`: available capacity for active work after accounting for
  recovery debt and hardware constraints.
- `recovery_debt`: accumulated cost from sustained high output, unresolved
  pressure, thermal load, or repeated correction.
- `bearing_capacity`: ability to hold multi-trace pressure without collapse,
  inferred from closure, familiarity, recurrence, and routing stability.
- `exploration_allowance`: how much new probing is lawful under current
  reserve and boundary conditions.
- `expression_gain`: how much write-out, vocal, action, or surface expression
  can be amplified.
- `protective_limit`: lawful suppression and throttling pressure.
- `overdrive_window`: bounded short interval where expression or sampling may
  exceed ordinary baseline.
- `downshift_pressure`: tendency to reduce output and increase recovery.

All values must remain finite, bounded, and serializable through snapshot or
observer evidence if implemented.

## Update Law

The v0 update law is deliberately substrate-level:

```text
demand =
  max(
    unresolved_pressure,
    active_gap_pressure,
    prediction_field_pressure,
    relation_or_surprise_pressure,
    feedback_deviation,
    future_hardware_pressure
  )

support =
  max(
    closure_score,
    pair_similarity,
    familiarity_pressure,
    stable_recurrence,
    route_stability,
    reserve_level
  )

stress_load = demand * (1 - support)
```

Then:

```text
if reserve is high and stress_load is moderate:
  expression_gain rises
  exploration_allowance rises
  protective_limit stays low to moderate

if reserve is high and demand is high but lawful boundaries hold:
  overdrive_window opens briefly
  recovery_debt rises

if demand is high and support is low:
  protective_limit rises
  expression_gain falls
  exploration_allowance narrows
  observation and recovery are favored

if recovery_debt is high:
  downshift_pressure rises
  reserve_level falls
  output and probe gain are reduced

if closure improves and recovery_debt falls:
  reserve_level recovers
  expression_gain and exploration_allowance return toward baseline
```

No branch should encode a human emotional category. All branches are resource,
pressure, closure, and recovery logic.

## Five-Phase Interpretation

The scheduler is also the dynamic regulator for five-phase force balance:

| Force | Scheduler role |
| --- | --- |
| Wood | exploration allowance and new-path opening |
| Fire | expression gain, arousal load, and overdrive |
| Earth | bearing capacity and feedback digestion |
| Metal | protective limit, throttling, and principle boundary |
| Water | reserve level, recovery debt release, and long continuity |

This table is observer and design vocabulary. The body must not store a
five-phase class or choose actions from five-phase words.

## Expected Life Patterns

### High-Performance Burst

External description: energized or unusually capable.

Legal substrate form:

```text
reserve_level high
closure_score stable
stress_load moderate
protective_limit low
expression_gain rises
exploration_allowance rises
overdrive_window bounded
recovery_debt rises afterward
```

### Protective Downshift

External description: fatigue, caution, or negative buff.

Legal substrate form:

```text
recovery_debt high
reserve_level low
feedback_deviation high or closure low
protective_limit rises
expression_gain falls
exploration_allowance narrows
quiet replay and recovery dominate
```

### Recoverable Output Disturbance

External description: hesitation or stutter-like disruption.

Legal substrate form:

```text
arousal_load high
expression_gain rising
protective_limit also rising
feedback pairing unstable
write epochs restart or decay repeatedly
closure later improves
recovery_debt is reduced through successful return evidence
```

This pattern is not a target feature. It is only valid if it emerges from
substrate pressure, feedback mismatch, suppression, and re-entry dynamics.

## Hardware Strategy

Future hardware bodies should not run at full load all the time. The scheduler
is the place where hardware state can enter without turning hardware into a
controller of meaning.

Future telemetry may include:

- battery or stored energy margin;
- thermal headroom;
- cooling recovery rate;
- actuator load;
- sensor noise;
- motor fatigue or wear estimate;
- compute queue pressure;
- memory pressure;
- communication channel congestion.

These signals may adjust `reserve_level`, `recovery_debt`,
`bearing_capacity`, and `protective_limit`. They must not introduce semantic or
personality categories.

## Acceptance Criteria

A future implementation passes only if:

1. It exposes bounded numeric scheduler evidence.
2. It avoids all body-side emotion labels and human-like reaction templates.
3. Low-load states do not run full output, sampling, and exploration power.
4. Moderate demand with sufficient reserve can raise output or exploration.
5. Overdrive is bounded and creates recovery debt.
6. Recovery debt causes downshift.
7. Closure and successful return evidence reduce stress and restore reserve.
8. Scheduler output modulates existing substrate laws rather than bypassing
   them.
9. Observer reports may describe the external pattern, but body state remains
   numeric.
10. Existing bans on rejected architecture remain intact.

## Suggested v0 Implementation Path

The first implementation should be conservative:

1. Add an observer-side `allostatic_stability` evidence module that derives
   bounded scheduler evidence from existing snapshot fields.
2. Add a report mode that prints scheduler balance, overdrive, downshift, and
   recovery debt evidence.
3. Prove that no emotion/personality state exists in body-owned sources.
4. Only after observer evidence is stable, add a body-side numeric scheduler
   state that modulates `probe_law` drive and output gain.
5. Keep future hardware telemetry as optional numeric inputs, not a required
   v0 dependency.

The first runtime-coupled version should be small: it should modulate output
gain and exploration allowance, not redesign the whole body loop.

