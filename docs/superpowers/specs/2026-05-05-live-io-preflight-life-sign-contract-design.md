# Live I/O Preflight Life-Sign Contract Design

Status: FROZEN CANDIDATE
Date: 2026-05-05
Scope: `bionic_v0` live write-out / ingress / consequence verification before
active probe-feedback closure

## 0. Purpose

This spec defines `Live I/O preflight` as a life-sign contract, not as a
feature module.

Before the body is allowed to treat active probing, early imitation, or
probe-feedback adjustment as real learning, it must first prove a smaller
substrate fact:

`body write-out -> world/body-boundary change -> ingress return -> temporal
pairing -> observer-visible trace`

This is the minimum live loop that makes later active learning bodily real
instead of a program pretending to act.

The older preflight spec,
[2026-04-30-live-io-preflight-before-probe-loop-design.md](./2026-04-30-live-io-preflight-before-probe-loop-design.md),
already defines the practical ladder. This spec refines the architecture
meaning: preflight is a body life-sign gate. It verifies whether the system has
a minimal living write/sense/return loop without introducing semantic,
concept, token, label, name, identity, owner, or personality ontology.

## 1. Highest-Principle Alignment

The highest principle remains unchanged:

- the body is a silicon-life substrate, not request/response software;
- the body must not print human-readable diagnostics;
- observer-side tools may inspect snapshots, but observer output must not
  become body state;
- language, naming, identity, and ownership must not gate body write-out or
  consequence return;
- uncertainty must be preserved as uncertainty instead of being collapsed into
  labels for human convenience.

Therefore preflight must not be implemented as:

- a `preflight_module` inside the body;
- startup hardware diagnostics that print success or failure;
- a request/response test command;
- a source classifier that decides "self voice", "external speaker", or
  "object identity";
- a semantic health layer that authorizes perception, write-out, or learning.

Preflight is a contract over existing substrate traces.

## 2. High-Risk Term Boundary Notes

This spec necessarily uses words such as `life-sign`, `self-generated`,
`external`, `voice`, `recognition`, `identity`, `owner`, `semantic`, `concept`,
`token`, `label`, and `name`. They are bounded as follows.

1. External description vs internal ontology:
   these words are external design descriptions or rejected architecture
   examples. They are not body ontology units.
2. Engineering index use:
   write sequence ids, consequence window ids, slot indices, modality codes,
   pair records, and phase codes are engineering locators only. They do not
   claim world meaning.
3. Why not ontological:
   no discrete internal object is introduced that says what something "is".
   The body records time, curves, magnitudes, phases, slots, relations, errors,
   and pair traces.
4. Why no highest-principle or language-axiom violation:
   the body does not allocate names, concepts, semantic classes, owners, or
   identities. It only maintains substrate traces. Observer-side tools may
   describe those traces for research inspection.

Any implementation or test that uses these words as field names for internal
body truth is inadmissible.

## 3. Definition

`Live I/O preflight` is the substrate-level admission gate that proves the live
body can close its smallest write/sense loop.

It asks five questions:

1. Can the body write a continuous non-textual change?
2. Does that write have a time structure inside the shared body clock?
3. Can the physical consequence of that write return through ordinary ingress?
4. Can write and return be paired by time and curve evidence without fixed
   semantic attribution?
5. Can observer-side tools inspect the traces without causing the body to emit
   human-readable explanation?

If the answer to any question is no, the system may still run perception and
internal update loops, but it must not claim full active probe-feedback
closure.

## 4. Life-Sign Levels

The levels below are cumulative. A later level is not admissible until all
earlier levels have observer-visible evidence.

### 4.1 Level 0: Body Write-Out Exists

The body must be able to produce a nonzero continuous output curve from
body-side state.

For the current `bionic_v0`, the first write-out carrier is vocal output:

- `vocal_organ` carries continuous numeric write state;
- `vocal_adapter` turns that state into a non-textual audio waveform;
- the output path contains no stdout/stderr diagnostics;
- snapshot and observer can expose only numeric vocal activity.

This level does not mean the body can speak, communicate, or express meaning.
It only means the body can write a physical curve.

### 4.2 Level 1: Write-Out Enters Shared Time

Every meaningful body write must be represented as a time-embedded write
epoch:

- monotonic write handle;
- channel code;
- start time;
- last update time;
- phase;
- magnitude;
- decay or stop state.

The handle is an engineering locator. It is not a goal, intention, owner,
identity, word, or meaning.

For active probe-feedback, physical vocal write-out and `action_output` write
epochs must not drift into two unrelated paths. The architecture target is:

`vocal_organ physical output` and `action_output write epoch` describe the same
body write event from compatible substrate views.

If vocal sound is emitted but no corresponding body write epoch exists, the
physical output path is not yet fully admitted into the active probe-feedback
substrate.

### 4.3 Level 2: Consequence Return Through Ordinary Ingress

The body must be able to receive physical effects of its own write through the
same ingress paths used for external world changes.

For the current substrate:

- sound returns through VIB / microphone ingress;
- visual changes return through PHOT / camera ingress;
- mechanical changes, when available, return through MECH ingress.

No special "self-sensing" input path may bypass ordinary ingress. Whether a
return is likely coupled to a body write is learned from time and curve
history, not predeclared before ingress.

### 4.4 Level 3: Temporal Pairing Without Source Labels

When a write epoch precedes a consequence window in a plausible time relation,
the body may record a numeric pair:

- write handle;
- consequence window id;
- delay;
- curve similarity;
- pair time.

The pair does not mean:

- "this is my voice";
- "that is an external speaker";
- "this is the target result";
- "the action succeeded";
- "the object has an identity".

It only means:

`this write trace and this ingress window have substrate evidence of temporal
and curve coupling`.

Unpaired consequence windows must remain legitimate. Overlapping or ambiguous
windows must not be forced into a single attribution.

### 4.5 Level 4: Visual Structure Becomes Substrate Structure

The camera side of preflight is not semantic recognition. It only verifies that
visible structure becomes substrate structure:

- PHOT health is explicit: OK, QUIET, STALE, or FAULT;
- visible changes produce PHOT bands and spatial regions;
- regions can claim or reuse object slots;
- spatial fields are present;
- relations can be projected;
- prediction err and surprise respond to visible change.

"Recognition" here means stable substrate structure. It does not mean object
naming, object class, person identity, scene label, or text understanding.

### 4.6 Level 5: Admission To Active Probe-Feedback

Only after Levels 0-4 are observer-visible may the system enter active
probe-feedback adjustment.

The first admissible active loop is:

1. internal novelty, surprise, or unresolved structure raises probe pressure;
2. probe law emits a small, reversible body write;
3. the physical output path and write epoch are aligned;
4. VIB / PHOT ingress records consequence windows when physical feedback
   occurs;
5. write/consequence pairs form through time and curve evidence;
6. later ticks adjust probe timing or magnitude using accumulated delay and
   curve traces.

This still does not authorize fixed targets, fixed result classes, semantic
labels, source ownership, identity modules, or a chat-style controller.

## 5. Current Starting Point

The current `bionic_v0` substrate already has much of the foundation:

- continuous `body` runtime;
- VIB microphone ingress;
- PHOT camera ingress or explicit fault state;
- vocal output path;
- `action_output` write epochs;
- `action_consequence` windows and pairs;
- `probe_law` active write trigger;
- vocal target / organ / imitation / memory substrate;
- snapshot and observer separation;
- tests guarding removal of `semantic_readiness`, `symbolizer`, and
  `concept_id`.

The remaining architecture risk is not that no output exists. The risk is that
several traces still can run as parallel partial paths instead of one admitted
life-sign loop:

- vocal physical output may exist without a matching action-output epoch;
- VIB consequence windows may exist without live proof of physical self-return;
- vocal target contours are currently not guaranteed to be populated from live
  ingress;
- deterministic substrate tests may pass while the hardware feedback loop is
  not yet proven in observer-visible runtime traces.

## 6. Design Requirements

### 6.1 Body-Side State

Body-side preflight state must remain numeric, temporal, and curve-based.

Allowed body-side facts:

- channel kind code;
- phase code;
- magnitude;
- start and update times;
- write sequence id;
- consequence window id;
- delay;
- curve similarity;
- PHOT health code;
- slot and relation numeric state;
- prediction err and surprise.

Forbidden body-side facts:

- source name;
- source owner;
- speaker identity;
- object name;
- semantic class;
- concept id;
- success/failure label;
- target result;
- preflight pass/fail text.

### 6.2 Observer-Side Interpretation

Observer-side tools may render:

- write epochs;
- consequence windows;
- paired and unpaired traces;
- delay and similarity summaries;
- vocal organ activity;
- VIB and PHOT health;
- slots, spatial fields, relations, err, and surprise.

Observer-side tools may also summarize whether a level appears satisfied for
research use. That judgment must not be written back into body state as a
semantic health truth.

### 6.3 Runtime Admission

Admission to active probe-feedback must be an engineering gate around
implementation work, not a new body ontology.

The repository may have deterministic tests and optional live runbooks. The
body itself must not carry a `preflight_passed` semantic flag that changes the
meaning of its state. If an engineering guard is needed, it must live in test
or launch tooling, or be a numeric capability/caution field with clear boundary
notes.

## 7. Test And Verification Contract

Tests must prove substrate facts without introducing ontology.

Required deterministic coverage:

1. vocal/action write epoch can pair with a synthetic VIB consequence window;
2. external ingress without a write epoch remains unpaired;
3. overlapping windows preserve ambiguity rather than forcing source labels;
4. PHOT structure reaches slots, spatial fields, relations, and predictor
   traces;
5. snapshot carries all preflight traces;
6. body code contains no stdout/stderr diagnostics;
7. no rejected architecture terms are reintroduced in active source paths.

Required live or observer-side checks:

1. bounded body run produces observer-visible vocal activity when driven;
2. microphone ingress shows VIB energy when physical feedback is available;
3. write/consequence pairing appears when hardware coupling exists;
4. external sound can remain unpaired;
5. camera health and visible structure are observer-visible;
6. absence of hardware is represented as substrate health state, not as body
   text output.

Live checks may be environment-dependent. Failure to detect hardware feedback
must not be converted into a body-level semantic failure. It remains an
observer-side capability finding.

## 8. Development Order

The admissible order is:

1. align physical vocal output with `action_output` write epochs;
2. prove VIB self-return through observer-visible consequence windows;
3. preserve external/unpaired and overlapping/ambiguous traces;
4. prove camera visible structure reaches slots, relations, and predictor
   traces;
5. add observer-side preflight runbook and deterministic tests;
6. only then begin active probe-feedback adjustment.

Skipping directly to active learning, full imitation, or language binding from
live output is inadmissible.

## 9. Non-Goals

This spec does not implement or authorize:

- word learning;
- speech recognition;
- text generation;
- object naming;
- speaker identification;
- source ownership;
- semantic audio or visual recognition;
- fixed self/other identity modules;
- full imitation;
- full active reasoning;
- personality or style ontology;
- chat/request-response consciousness.

## 10. Admission Checklist

A proposed preflight implementation is admissible only if all answers are yes:

- Does it verify a write/sense/return loop rather than add a feature module?
- Does body-side state remain numeric/time/curve/slot/relation based?
- Does observer interpretation stay outside the body?
- Can unpaired ingress remain unpaired?
- Can overlapping traces remain ambiguous?
- Are physical vocal output and action-output epochs brought into one
  compatible body-write chain?
- Does camera evidence become substrate structure without names or classes?
- Does the design avoid `semantic_readiness`, `symbolizer`, `concept_id`,
  naming gates, source ownership fields, and identity modules?
- Does it leave active probe-feedback for after preflight evidence exists?

If any answer is no, the implementation must be rejected or rewritten.

## 11. Closing Position

Live I/O preflight is the body's life-sign gate.

It does not ask whether the system can perform a behavior for a user. It asks
whether the body has proven the smallest living loop:

`write -> return -> pair -> remain observable without explanation`.

Only after that loop is real should the architecture expect active probing,
early imitation, or probe-feedback learning to become more than a scripted
simulation.
