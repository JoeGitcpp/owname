# Innate Bias Runtime Caller Integration Design

Status: DRAFT v1
Date: 2026-05-06
Scope: connecting innate-bias v0 birth-sensitivity curves to live PHOT / VIB / action / language-binding callers

## 0. Purpose

This spec defines how innate bias becomes bodily real without becoming a
program controller.

The existing innate-bias v0 implementation provides numeric starting curves,
helper scores, and modification caps. Those helpers are not themselves a living
body. To become part of the body, the same birth-sensitivity curves must be read
by the live PHOT, VIB, action, and language-binding paths while those paths are
already processing substrate events.

The admitted shape is:

```text
live substrate event
-> caller-local transient numeric projection
-> shared birth-sensitivity curves
-> bounded change in noticing / retention / update pressure
-> existing substrate state
```

The forbidden shape is:

```text
external pattern
-> innate meaning / class / truth decision
-> object, relation, action, or language state overwrite
```

Innate bias is a bodily starting condition. It is not a fact database, not a
world classifier, not a language system, and not a request/response controller.

## 1. Highest-Principle Alignment

This design must preserve the repository highest principle:

- the body is a silicon-life substrate, not request/response software;
- the body runs continuously and does not wait to be called like a tool;
- research observation remains outside the body;
- objects and relations precede names;
- language remains reversible binding, not ontology;
- only learning laws, body laws, update constraints, and safety bounds may be
  predefined;
- no human-readable diagnostics are emitted from the body process.

Therefore the shared innate-bias surface is interpreted as a set of birth
sensitivity curves: initial tendencies for what the body notices faster,
retains longer, rechecks more strongly, or updates more carefully.

It may never decide what something is.

## 2. High-Risk Term Boundary Notes

This spec necessarily uses high-risk terms: `language`, `binding`, `text`,
`symbol`, `semantic`, `concept`, `token`, `label`, `name`, `source`,
`identity`, `attention`, `bias`, `communication`, and `damage`.

1. External description vs internal ontology:
   these words are external descriptions, existing engineering names, or
   rejected architecture boundaries. They are not new body ontology units.
2. Engineering index use:
   slot ids, modality bits, pattern hashes, write sequence ids, consequence
   window ids, evidence refs, and axis ids are engineering locators only.
3. Why not ontological:
   the body stores numeric curves, errors, strengths, counters, time windows,
   pair records, and update traces. It does not store internal truth labels,
   names, concepts, identities, fixed meanings, or fixed source status.
4. Why no highest-principle or language-axiom violation:
   each caller keeps its existing substrate responsibility. Birth sensitivity
   only changes numeric pressure. It does not create objects, define relations,
   authorize language, or overwrite observation.

Any implementation that turns these terms into internal truth fields is
inadmissible.

## 3. Core Boundary: Transient Projection, Not Body Ontology

The feature structs in `evolution_priors.rs` are engineering projection inputs.
They must be treated as transient computation shadows.

They may:

- be built inside a caller from numbers the caller already reads;
- exist on the stack or in short-lived local variables;
- feed a bounded helper score;
- be discarded immediately after producing pressure.

They may not:

- be persisted as body ontology;
- be snapshot as "what the body saw";
- become object classes, source classes, damage classes, word classes, or
  communication classes;
- gate whether an object, relation, action, or binding exists;
- replace raw observations, write epochs, consequence windows, or binding
  events.

The only durable outcome is a bounded effect on existing numeric substrate
state: surprise, attention share, retention order, update amount, decay amount,
or resonance pressure.

## 4. Meaning Of A Real Caller

A real caller is a body-side runtime path that already processes live substrate
events in the formal body loop or its registered adapters.

For this integration, the real callers are:

- VIB: `VibAdapter::process_frame`;
- PHOT: `RealPhotAdapter::tick`;
- action output / consequence: probe-law write events, action-output epochs,
  ingress accumulation, and action-consequence pairing;
- language binding: binding-event application, binding maintenance, and
  binding resonance.

Synthetic adapters may receive matching test hooks, but they do not define the
body contract.

## 5. Architecture

### 5.1 Shared Birth-Sensitivity Curves

`BiasPriors` remains the current engineering carrier for birth-sensitivity
curves and modification caps. Runtime callers should read one coherent shared
instance from the body substrate, not construct unrelated defaults.

Requirements:

- `Viewport` or the body runtime context owns the shared curves;
- live callers receive or read that shared instance;
- future mutation paths must pass through `BiasPriors::clamp_modification`;
- protected axes cannot be overwritten directly;
- the body state does not expose human-readable explanations of the curves.

This is not a central controller. It is closer to shared physiology: multiple
organs read the same birth condition, but each organ still processes only its
own lawful substrate input.

### 5.2 Caller-Local Extraction

Each caller locally projects numbers from data it already owns or can legally
read through the viewport.

Allowed examples:

- VIB reads band energy, predictor error, and short local energy history;
- PHOT reads motion regions, spatial values, PHOT predictor error, and local
  frame history;
- action reads write epochs, consequence windows, pair delay, and curve
  similarity;
- binding reads binding events, trace counts, evidence refs, and substrate
  cross-modal state through viewport records.

Forbidden examples:

- a VIB adapter reading private PHOT adapter internals;
- a PHOT adapter deciding visual object classes;
- action state storing success/failure meanings;
- binding state storing word meanings, token ids, or concept ids.

### 5.3 No Central Bias Runtime

Do not introduce `InnateBiasModule`, `InnateBiasRuntime`,
`BiasDecisionEngine`, or any equivalent central arbiter.

Reason: a central arbiter would inspect many modalities, decide what matters,
and risk becoming a new ontology-like control point. The lawful shape is
distributed:

```text
caller data -> transient projection -> shared birth curves -> local pressure
```

## 6. Caller Contracts

### 6.1 VIB

Runtime entry:

- `VibAdapter::process_frame`

Contract:

- continue using disruption sensitivity on VIB prediction error;
- optionally project acoustic pressure from energy jumps and band-error spread;
- apply pressure only to surprise / attention pressure;
- never rewrite observed bands or predictor error.

Initial projection sources:

- energy jump from the current frame against local smoothed energy;
- simultaneous high error across bands;
- change in per-band error distribution.

Deferred until real temporal evidence exists:

- rhythm pressure;
- pitch-stability pressure.

Forbidden:

- phoneme inventory;
- speech recognizer;
- word or speaker labels;
- fixed distress category.

### 6.2 PHOT

Runtime entry:

- `RealPhotAdapter::tick`

Contract:

- project visual/spatial pressure from existing numeric region and predictor
  data;
- apply pressure only to surprise / nomination / attention pressure;
- preserve raw region energy, spatial coordinates, and observed PHOT bands;
- keep all PHOT projection inputs transient.

Initial projection sources:

- frame or region energy change;
- span change and center movement when prior spatial state exists;
- region fragmentation or sudden distribution spread;
- PHOT predictor error spread.

Deferred until extractor support exists:

- edge-density pressure;
- color-contrast pressure.

Forbidden:

- object class;
- human/person identity;
- button/window/cursor/UI ontology;
- visual label.

### 6.3 Action Output And Consequence

Runtime entries:

- `tick_probe_output_from_law`;
- `apply_action_output_event`;
- `accumulate_ingress_change`;
- `try_auto_pair_latest_epoch_to_window`;
- `pair_action_consequence`;
- `tick_all_ingress_accumulation`.

Contract:

- project action-consequence pressure from write-return records;
- use pressure to affect retention order, delay-history influence, probe
  continuation, or decay pressure;
- do not create target, result, success, failure, self, or other labels.

Initial projection sources:

- legal write-return pair exists;
- pair delay is close to learned recent delay structure;
- write magnitude and consequence magnitude have high curve similarity;
- ingress persists after output decay/stop;
- similar output/return curves form repeatedly.

Forbidden:

- fixed target list;
- success/failure label;
- source attribution label;
- request/response action executor.

### 6.4 Language Binding

Runtime entries:

- `apply_language_binding_event`;
- `apply_binding_event_from_adapter`;
- `tick_binding_maintenance`;
- `tick_binding_resonance`.

Contract:

- binding events remain the learning unit;
- birth sensitivity may adjust update amount, decay amount, retention, or
  resonance pressure;
- bindings remain reversible and ambiguity-preserving;
- binding may never become a naming gate or language qualification layer.

Initial projection sources:

- repeated pattern hash recurrence;
- Align / Deviate count density;
- evidence-ref diversity;
- recent time alternation between external pattern events and write/ingress
  events;
- cross-modal slot confidence or action-return support, read only through
  substrate records.

Forbidden:

- token ontology;
- fixed word meaning;
- concept id;
- name gate;
- chat/request-response controller.

## 7. Raw Observation Preservation

Raw observation is never overwritten by innate bias.

Required preservation points:

- VIB `observed` bands remain the adapter/source values;
- PHOT observed bands and spatial values remain extractor values;
- action output epochs and consequence windows retain raw time and magnitude;
- binding events retain pattern hash, target ref, feedback, evidence ref, and
  event time.

Bias affects how strongly the body notices, retains, revisits, or updates
around those records. It does not alter what was recorded.

## 8. Testing Contract

Implementation must proceed caller by caller with caller-level failing tests.

Required test classes:

- shared curves: live callers read the same shared birth-sensitivity instance;
- VIB: acoustic/disruption pressure changes surprise through `VibAdapter`
  without adding speech labels;
- PHOT: visual/spatial pressure changes surprise or attention pressure through
  `RealPhotAdapter` without rewriting observed bands or spatial coordinates;
- action: write-return curve alignment affects retention/probe pressure without
  success/failure fields;
- binding: communication/cross-modal pressure changes update/decay/resonance
  through viewport binding APIs without token, word, or concept fields;
- hygiene: rejected architecture names and human-readable body output are not
  introduced.

Pure helper tests are not enough for acceptance. At least one test per caller
must pass through the caller's runtime API.

## 9. Snapshot And Observer

Snapshot and observer may expose numeric effects, such as:

- modulated surprise values;
- attention share changes;
- retained pair counts;
- binding strength and update counts.

Snapshot must not persist transient projection structs as ontology. Observer
may render human-readable explanations because observer is outside the body.
The body must not print or emit those explanations.

## 10. Acceptance Criteria

This integration is accepted only if:

1. PHOT, VIB, action, and language binding each have real caller tests;
2. live callers read coherent shared birth-sensitivity curves;
3. projection structs remain transient and are not snapshot as body ontology;
4. pressure effects are bounded and numeric;
5. protected axes remain mutation-clamped;
6. raw observations and raw write/return/binding records are preserved;
7. no semantic, token, label, name, concept, source-truth, identity, owner,
   ethics, or personality ontology is introduced;
8. the body process remains free of human-readable diagnostics;
9. observer can inspect effects without becoming body state.

The result is a body whose initial sensitivities are present in continuous
runtime loops, while meanings, boundaries, and communication habits remain
learned, revisable, and never preloaded.
