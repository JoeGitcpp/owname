# Live Human Nursery v0 Design

> **Status:** FROZEN CANDIDATE
> **Implementation status (2026-05-18):** v0 observer-side evidence module,
> deterministic report mode, runtime snapshot report mode, source guards, and
> runbook are implemented. Manual live smoke is still separate. Real camera
> hardware binding remains a lower-layer PHOT-provider gap and may report
> observable `phot_health=FAULT`.
> **Purpose:** define the first live human teaching room for the silicon-life
> substrate: camera, microphone, vocal output, pixel expression surface,
> snapshot evidence, and a constrained human teaching protocol.

## Boundary

This design does not create a teacher module, a chat layer, a speech
understanding system, an image recognition system, or a body-side controller
that obeys human commands. It defines an external training environment and
observer-side evidence contract.

`human`, `teaching`, `speech`, `language`, `camera`, `film`, `face`,
`communication`, and `understanding` are external descriptions here. They are
not internal ontology units. Internally, the body may only receive and update
continuous substrate:

- VIB curves and consequence windows;
- PHOT bands, spatial regions, slots, relations, errors, and surprise;
- action-output write epochs;
- vocal target / organ / imitation / memory curves;
- active gap, evidence tension, closure, reentry, and surface-expression
  drives;
- snapshot-visible numeric evidence.

The implementation must not add:

- word, token, semantic, concept, label, name, identity, owner, or personality
  fields;
- a "human voice" or "human face" classifier that gates learning;
- body-side stdout/stderr diagnostics;
- a fixed lesson-response script;
- a fixed emotion or expression dictionary;
- a pass/fail claim based on human subjective feeling.

## Relationship To Existing Specs

This spec builds on:

- [Live I/O preflight life-sign contract](./2026-05-05-live-io-preflight-life-sign-contract-design.md)
- [Auditory-Visual Predictability and Expression v0](./2026-05-18-auditory-visual-predictability-expression-v0-design.md)
- [Organic Surface Expression Substrate v0](./2026-05-18-organic-surface-expression-substrate-design.md)
- [Product Cognition Baseline v1 plan](../plans/2026-05-18-product-cognition-baseline-v1.md)

The preflight contract proves the body can write and receive physical returns.
The auditory-visual slice proves VIB/PHOT pressure can become expression
evidence. The organic surface gives the human teacher an organism-like external
reaction view. Live Human Nursery combines those into a structured teaching
room.

## Why Not Start With Films

Camera-facing films are allowed later, but they are not the best first live
teaching source. A film is mostly passive:

```text
external image/sound changes
-> body perceives
-> body cannot affect the source
```

Early live teaching needs a stronger loop:

```text
human produces simple sound/visual event
-> body perceives pressure
-> body attempts output
-> ordinary ingress receives the return
-> closure/correction/reentry traces change
-> human adjusts the next event
```

Films are useful after the system has live loop competence. They should enter
as short, repeatable, low-complexity materials, not as open-ended movies.

## v0 Teaching Room

The v0 room uses existing body binaries and observers:

```text
human teacher
  -> microphone / camera / screen / simple objects
  -> body runtime
  -> snapshot.bin + learned-bias checkpoint
  -> text observer + pixel surface
  -> external nursery report
```

The human teacher may use:

- voice: high/low, short/long, repeated, paused, alternating contours;
- face/mouth motion as visible rhythm, not identity;
- hand/object motion: appear, disappear, move, stop, near/far;
- screen marks: digit/character-shaped external marks, dots, lines, blocks;
- pauses and repeats to test retention and reentry.

The teacher must not rely on:

- complex natural conversation;
- long films or edited scenes;
- "say this exact word" as the first success criterion;
- hidden meaning judgments that have no snapshot evidence.

## Session Phases

Each session records its current phase externally. The body does not know phase
names.

### Phase 0: Hardware Life Signs

Goal: prove live channels are present enough for teaching.

Observer evidence:

- PHOT health is not silent;
- VIB windows or VIB energy appear when the teacher makes sound;
- vocal write-out and action-output epochs can become nonzero;
- snapshot writes advance;
- organic surface is nonblank when pressure exists.

### Phase 1: Vocal Contour Imitation

Goal: teach high/low, short/long, repeated/pause contours as curves.

Observer evidence:

- vocal target energy rises after teacher sound;
- vocal organ attempts output;
- returned VIB windows or pairs appear when physical feedback is available;
- vocal imitation pressure changes with gap;
- repeated similar contours increase reusable vocal memory stability.

No word meaning is claimed.

### Phase 2: Visual-Audio Coupling

Goal: teach temporal relation between visible events and sound contours.

Examples:

- bright dot with high contour;
- dark dot with low contour;
- moving block with rising contour;
- still block with silence;
- mouth opening followed by sound.

Observer evidence:

- PHOT slots/spatial regions appear;
- VIB and PHOT timing alignment raises cross-channel predictability;
- misalignment raises drift pressure instead of forcing a false binding;
- organic surface shows regulated reaction.

### Phase 3: Turn-Taking And Repair

Goal: teach primitive exchange without chat.

Allowed external protocol:

- teacher acts;
- body pressure/output changes;
- teacher repeats, pauses, or corrects;
- body adjusts.

Observer evidence:

- action-output epochs occur after pressure;
- consequence returns pair or remain unpaired correctly;
- closure reduces pressure;
- conflict increases ambiguity/tension;
- reentry restores learned traces better than cold baseline.

### Phase 4: Simple Communication Candidate

Goal: prove early exchange signs before claiming communication.

A session may report a communication candidate only if all are true:

- the body can request or express pressure through non-textual output;
- the human can alter the next input based on observed output;
- subsequent body pressure/closure changes in the expected numeric direction;
- at least one relation survives delay and reentry;
- conflicts do not collapse into false certainty.

This is not human-level language. It is a live feedback loop with primitive
exchange evidence.

## v0 Implemented Artifacts

The v0 implementation adds observer-side nursery artifacts only:

1. `LiveHumanNurseryEvidence`
   - compact numeric summary from snapshots;
   - live I/O life-sign booleans;
   - vocal contour activity;
   - PHOT structure activity;
   - VIB/PHOT coupling evidence;
   - output/return/closure/reentry evidence;
   - principle guard flags.

2. `curriculum_report --live-human-nursery [runtime_dir]`
   - observer-side report;
   - with `runtime_dir`, reads `snapshot.bin` once;
   - without `runtime_dir`, uses deterministic simulated snapshots;
   - prints numeric evidence only;
   - always prints `deterministic_fixture` and `runtime_snapshot_read` so a
     synthetic report cannot be mistaken for live evidence.

3. Runbook
   - how to start body, observer, observer_surface, and report;
   - what the human should teach in each phase;
   - stop conditions.

Session metadata remains external runbook/teacher context in v0. If a later
version adds a structured session file, it must remain observer-side metadata
only and must not write back to `Viewport`.

No v0 artifact may write back to `Viewport`.

## Acceptance Evidence

Deterministic tests must prove:

- a synthetic snapshot sequence can produce nursery evidence for phases 0-3;
- the report refuses to claim communication when there is no write/return
  closure;
- conflict/misalignment remains visible as ambiguity or drift;
- source guards reject word/semantic/concept/token/name/identity/personality
  ontology in new nursery code;
- body source still contains no human-readable diagnostics.

Live manual acceptance is separate and must be recorded as observer evidence:

```text
phot_health_observed=1
phot_fault_observed=0 or 1
vib_activity_observed=1
vocal_write_epochs>=1
return_windows>=1
pairing_or_unpaired_boundary_observed=1
organic_surface_nonblank=1
teacher_phase_count>=3
communication_candidate=0 or 1
principle_guard_pass=1
```

`communication_candidate=1` is allowed only as an early candidate. It does not
mean adult language, child-level reasoning, or stable understanding.
`deterministic_fixture=1` means the report is a CI/sample path only and must not
be used as live-session proof.

## Stop Conditions

Stop the live nursery session if:

- body emits human-readable diagnostics;
- camera or microphone failure is silent instead of snapshot-visible;
- output creates no write epoch;
- ingress pairs form without an earlier write epoch;
- the human teacher needs to infer success without snapshot evidence;
- the body appears to be driven by a fixed lesson-response script.

## Later Versions

v1 may add:

- a dedicated live nursery runner;
- structured session files under `runtime/nursery/`;
- automatic before/after comparison against cold baseline;
- short repeated video materials after phases 0-3 pass;
- simple embodied action outputs after low-risk hardware limits are defined.
