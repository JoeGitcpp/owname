# 2026-04-30 Live I/O Preflight Before Probe Loop Design

Status: DESIGN APPROVED FOR PLANNING
Scope: `bionic_v0` live camera / microphone / vocal-output validation before
active probe-feedback closure

Superseding refinement:
[2026-05-05-live-io-preflight-life-sign-contract-design.md](./2026-05-05-live-io-preflight-life-sign-contract-design.md)
defines the architecture meaning of this preflight as a body life-sign gate.
Use the 2026-05-05 spec for admission boundaries; use this document for the
earlier practical ladder and test scope.

## 0. Purpose

Before extending the active probe-feedback loop, the body must first prove that
its live instruments are reliable enough to support that loop.

This spec defines the preflight layer for:

- vocal write-out confirmation
- microphone self-audition feedback
- external-sound vs self-output temporal attribution
- camera substrate recognition of visible structure
- admission criteria for entering the next active probe-feedback phase

It intentionally does not implement a full active-learning loop. It verifies
that the body can write, sense, observe consequence traces, and expose those
traces without reintroducing semantic, concept, token, label, name, identity, or
personality ontology.

## 1. Highest-Principle Alignment

- The body remains a continuous substrate, not a request/response program.
- Body-side validation must not print human-readable diagnostics.
- Human-readable interpretation remains observer-side only.
- No `Subconscious` or `Conscious` internal module is introduced.
- "Subconscious" and "conscious" remain external metaphors only:
  - body-side continuous execution corresponds to ongoing perception,
    prediction, vocal execution, local correction, and maintenance;
  - active reasoning corresponds to later probe dispatch and correction loops.

## 2. High-Risk Term Boundary Notes

This spec uses high-risk terms only under the following boundaries.

1. External description vs internal ontology:
   terms such as `recognition`, `self`, `sound`, `voice`, `seen`, `label`,
   `name`, `identity`, `semantic`, `concept`, and `token` are external
   descriptions in this document, not internal body ontology units.
2. Engineering index use:
   write sequence IDs, window IDs, slot indices, modality codes, and pairing
   records are engineering locators only. They do not claim world meaning.
3. Why not ontological:
   no discrete internal object is introduced that says "this is my voice",
   "this is an object name", or "this is the meaning of the scene". The body
   records time, curve, magnitude, slot, relation, error, and consequence traces.
4. Why no highest-principle violation:
   the body does not explain itself, print diagnostics, allocate concepts, or
   use language as a gate. Observer-side tools may render the numeric traces for
   research inspection.

## 3. Current Substrate Starting Point

The current `bionic_v0` body already has the needed starting pieces:

- continuous `body` runtime with ticker-driven perception, prediction,
  endogenous activity, vocal maintenance, and snapshot writes;
- live VIB ingress through the microphone adapter;
- live PHOT ingress through the camera adapter or explicit fault state;
- vocal output through the body-side audio output stream;
- action-output write epochs;
- action-consequence windows;
- live temporal pairing from write epochs to ingress consequence windows;
- observer-side rendering of snapshots.

The missing piece is not "output exists". The missing piece is a disciplined
preflight ladder proving that output, microphone feedback, camera substrate
structure, and observer visibility are working before the system is allowed to
use them as an active probe-feedback loop.

## 4. Preflight Levels

### 4.1 Level 0: Vocal Write-Out Exists

Goal:

- prove the body can produce nonzero vocal output from continuous vocal-organ
  state;
- keep the output path body-side and non-textual;
- expose only numeric vocal activity through snapshot/observer.

Acceptance:

- existing vocal runtime tests remain passing;
- vocal output path contains no stdout/stderr diagnostics;
- generated sample buffers become nonzero when vocal drive is nonzero.

Status:

- already mostly implemented and covered by existing vocal tests.

### 4.2 Level 1: Self-Generated Sound Reaches VIB Ingress

Goal:

- prove a vocal write epoch can produce a measurable microphone-side VIB
  consequence window;
- pair the write epoch and VIB consequence through substrate time;
- expose delay and curve-similarity traces to observer-side inspection.

This level does not assert "the AI knows this is its own voice". It only proves:

`body write-out occurred before a microphone-side consequence window in a
plausible temporal envelope`.

Acceptance:

- a controlled live run can trigger vocal output;
- VIB event energy rises after output begins;
- an action-consequence window opens and closes through quiescence;
- auto-pairing records a write_seq/window_id pair;
- the pair has a bounded delay and numeric curve_similarity;
- snapshot and observer expose the traces without semantic fields.

### 4.3 Level 2: External Sound vs Self-Output Temporal Attribution

Goal:

- distinguish self-coupled sound traces from uncoupled external sound traces
  using time and curve evidence only;
- preserve uncertainty when traces overlap.

Allowed internal state:

- paired write/consequence traces;
- unpaired ingress consequence windows;
- ambiguous or overlapping windows represented as separate numeric traces.

Forbidden internal state:

- "self voice" labels;
- "external speaker" labels;
- source names;
- semantic audio classes;
- forced ownership decisions.

Acceptance:

- external sound without body write-out forms VIB ingress windows but no
  write-linked pair;
- body vocal output forms at least one legal pair when microphone feedback is
  physically available;
- overlapping external sound and self-output does not force a single ownership
  classification;
- observer can show paired vs unpaired numeric traces.

### 4.4 Level 3: Camera Substrate Recognition Of Visible Structure

Goal:

- prove camera input is entering the substrate as visual structure;
- confirm visible changes can be represented through PHOT bands, spatial
  regions, object slots, relations, predictors, errors, and surprise.

"Recognition" at this level means:

`visible structure becomes stable substrate structure`.

It does not mean:

- object naming;
- semantic classification;
- text understanding;
- concept binding;
- person/object identity;
- scene labels.

Acceptance:

- live or synthetic PHOT frames produce motion/region evidence;
- separate visible regions can claim separate object slots when spatially far
  enough;
- stable visible structure can reuse slots instead of constantly reallocating;
- relation and prediction state responds to visible change;
- snapshot and observer expose PHOT health, slots, spatial fields, relation
  state, prediction err, and surprise.

### 4.5 Level 4: Admission To Active Probe-Feedback Work

Only after Levels 0-3 pass may implementation proceed to active
probe-feedback closure.

The first admissible active loop is:

1. internal novelty/surprise raises probe pressure;
2. probe law emits a small vocal write event;
3. microphone and camera ingress record consequence windows;
4. write/consequence timing forms pairs when physically supported;
5. later ticks use learned delay and curve traces to adjust probe timing or
   magnitude.

This next phase may adjust probe timing and magnitude, but it still must not
introduce semantic labels, fixed target objects, fixed result classes, or a
chat-style conscious controller.

## 5. Observer-Side Verification Contract

Because the body must not print diagnostics, all live preflight runs must be
verified through snapshot/observer data.

The observer may render:

- vocal organ/output activity fields;
- VIB energy and health;
- PHOT health and region/slot state;
- action-output channels and write epoch history;
- action-consequence windows and pairs;
- delay and curve-similarity summaries.

The observer must not feed human-readable interpretation back into the body.

## 6. Test Strategy

Tests should be split into deterministic substrate tests and optional live
hardware checks.

Deterministic tests:

- prove action-output epochs pair with synthetic VIB/PHOT consequence windows;
- prove unpaired external ingress remains unpaired;
- prove overlapping windows do not force ownership labels;
- prove camera structure reaches slots/relations/predictor state;
- prove snapshot carries every preflight trace.

Live checks:

- run the body for a bounded duration with vocal output enabled;
- inspect snapshots via observer tooling;
- confirm microphone feedback appears when hardware loopback is physically
  available;
- confirm camera health and visible-region state are observable.

Live checks may be environment-dependent and should report capability status via
snapshot state rather than body stdout/stderr.

## 7. Non-Goals

This spec does not implement:

- word learning;
- object naming;
- speaker identification;
- semantic audio recognition;
- semantic visual recognition;
- fixed self/other identity modules;
- full imitation;
- full active reasoning;
- chat/controller-style consciousness;
- personality or style ontology.

## 8. Completion Boundary

This preflight is complete when the repository has tests and observer-visible
state proving:

- the body can write vocal output;
- microphone ingress can carry sound consequences;
- write/consequence pairing works for self-generated sound when physically
  present;
- external sound can remain unpaired;
- camera input becomes visual substrate structure;
- all evidence remains numeric/time/curve/slot/relation based.

Only then should the project enter the next design/implementation phase:
active probe-feedback adjustment.
