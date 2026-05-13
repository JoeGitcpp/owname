# Learned Bias Delta Persistence Design

Status: DRAFT v1
Date: 2026-05-06
Scope: highest-principle-aligned learned bias sedimentation for `bionic_v0`

## 0. Purpose

This spec rewrites learned bias persistence from the highest principle:

`the body is a silicon-life substrate, not configurable request/response
software.`

The design goal is not to add a settings store. The design goal is to let a
continuously running body carry experience without rewriting its birth shape.

The core invariant is:

```text
base bias remains recoverable
+ lived substrate traces produce delta events
+ repeated compatible events sediment into delta curves
+ runtime effective bias is derived, not persisted as new base
```

`BiasPriors` is the base sensitivity surface. It may define initial numeric
sensitivities and modification caps. It must not be overwritten by experience.
Experience persists only as delta events and delta curves.

## 1. Highest-Principle Compliance

This persistence layer is admissible only if it preserves all of these rules:

1. The body does not accept external requests to set learned bias values.
   Learned deltas are produced only by body-side substrate traces: prediction
   error, correction pressure, binding events, write-return pairing, surprise,
   retention, and other already-admitted numeric loops.
2. The body does not emit human-readable explanation while learning or
   checkpointing deltas.
3. Observer-side descriptions do not flow back into learned-bias body state.
4. Learned deltas never define what an object, relation, source, word, person,
   or task "is".
5. Learned curves remain reversible, weakenable, and driftable. No curve is an
   irreversible stamp.
6. Runtime composition never mutates `BiasPriors`.

If an implementation lets an external caller directly set a learned-bias field,
it has become a tool configuration layer and must be rejected.

## 2. Boundary Notes For High-Risk Terms

This spec necessarily uses words such as `bias`, `learned`, `base`, `delta`,
`evidence`, `source`, `semantic`, `concept`, `token`, `label`, `name`,
`identity`, `owner`, `trust`, `truth`, and `personality`.

They are bounded as follows:

1. External description vs internal ontology:
   these words are external design descriptions or rejected architecture
   examples. They are not body ontology units.
2. Engineering index use:
   axis ids, locators, stream handles, event refs, checkpoint versions, and
   curve positions are engineering locators only.
3. Why not ontological:
   the body records numeric update traces and curve sedimentation. It does not
   store real meanings, identities, trust judgments, names, concepts, or truth
   labels as internal primitives.
4. Why no highest-principle or language-axiom violation:
   this design introduces no naming gate, no concept allocator, no semantic
   readiness layer, no owner field, no identity module, no personality state,
   and no request/response controller.

Any implementation that stores these terms as body truth fields is
inadmissible.

## 3. What This Layer Is Not

Learned bias delta persistence is not:

- a module that decides what is important;
- a mutable personality;
- a long-term semantic memory;
- a source trust database;
- a code-quality classifier;
- a language understanding layer;
- a user-editable preference file;
- a replacement for `BiasPriors`;
- an observer-side dashboard feature.

It is a thin substrate for numeric sensitivity sedimentation.

## 4. Base Bias

`Base bias` is the recoverable birth surface currently represented by
`BiasPriors`.

Allowed base contents:

- starting sensitivity coefficients;
- retention and surprise multipliers;
- update helper coefficients;
- modification caps.

Forbidden base contents:

- facts;
- dictionaries;
- source trust or distrust;
- fixed object classes;
- fixed output targets;
- personality tags;
- language meanings.

The default base must remain restorable even after many learning events.

## 5. Delta Events

A delta event is the smallest persistent record of bias learning.

It is produced by admitted body-side substrate loops. It records the numeric
pressure created by experience, not the human meaning of that experience.

Allowed fields:

- numeric bias axis;
- numeric locator;
- requested delta;
- applied delta after cap enforcement;
- support magnitude;
- deviation magnitude;
- evidence ref;
- event substrate time.

Forbidden fields:

- natural-language string;
- semantic class;
- concept id;
- source identity truth;
- owner;
- success/failure label;
- target result;
- personality or style tag.

Delta events are not permanent conclusions. They are evidence traces from which
curves can be updated, weakened, replayed, or ignored.

## 6. Delta Curves

A delta curve is the sediment of compatible delta events on the same numeric
axis and locator.

Allowed fields:

- numeric bias axis;
- numeric locator;
- current delta value;
- EMA of applied deltas;
- stability;
- support count;
- deviation count;
- last update time.

A delta curve is not a new base prior. It is an overlay.

Delta curves must support:

- strengthening;
- weakening;
- sign reversal;
- decay;
- bounded eviction;
- deterministic checkpoint roundtrip.

## 7. Effective Bias

Runtime readers may derive:

```text
effective_bias(axis, locator, t) =
    base_bias(axis, locator)
  + learned_delta_curve(axis, locator, t)
```

The effective value may be clamped by the caller's legal range. It must not be
written back into the base layer.

This distinction is mandatory:

- `base` answers "what sensitivity shape was the body born with?"
- `delta` answers "what has lived experience added or subtracted?"
- `effective` answers "what should a runtime reader use now?"

Only `delta` persists as learned experience.

## 8. Update Law

Every event must clamp through the base cap:

```text
applied_delta = BiasPriors::clamp_modification(axis, requested_delta)
```

This preserves protected Layer 0 axes. Repeated experience may refine them, but
short-term pressure cannot erase breakage sensitivity, irreversible disruption
sensitivity, or evidence-overwrite prevention.

Support and deviation both affect the curve:

- support pushes the curve in the applied direction and increases stability;
- deviation pushes the curve back, lowers stability, or reverses direction;
- zero-support/zero-deviation events are no-ops except for bounded recent trace
  retention if callers need timing evidence.

The theory is curve sedimentation, not assignment.

## 9. Persistence Contract

Learned bias state persists in a separate body checkpoint:

```text
runtime/learned_bias.bin
runtime/learned_bias.bin.tmp
```

The checkpoint must contain only binary numeric state:

- magic;
- version;
- curve count;
- bounded curve records;
- recent event count;
- bounded event records;
- last update time.

Writes must follow the existing safe pattern:

1. write `.tmp`;
2. sync;
3. rename to final path.

Unknown versions and corrupt payloads must be rejected.

This checkpoint is not observer output. It is body-resume state.

## 10. Snapshot Boundary

Snapshot may expose numeric learned-bias facts for observer inspection:

- curve count;
- recent event count;
- maximum absolute delta;
- protected-axis curve count;
- ordinary-axis curve count;
- last update time.

Snapshot must not expose or require human-readable body explanations.

Observer may render these fields as text. That rendering remains outside the
body and must not be written back as learned-bias state.

## 11. First Implementation Scope

The first implementation is intentionally narrow:

1. Add the numeric learned-bias state and pure update helpers.
2. Add checkpoint write/read helpers.
3. Add viewport ownership and snapshot summary.
4. Add effective-bias helpers for generic scalar axes.

The first implementation does not wire VIB, PHOT, language binding, code I/O,
or action-consequence loops as automatic producers. Those integrations require
separate caller-level tests proving the delta originates from existing body
traces.

This order prevents accidental creation of a broad self-modification path
before the life-design boundary is tested.

## 12. Test Contract

Required deterministic tests:

1. learned updates do not mutate `BiasPriors`;
2. ordinary-axis deltas are clamped by ordinary caps;
3. protected-axis deltas are clamped by protected caps;
4. repeated support sediments into a stronger curve;
5. deviation weakens or reverses an existing curve;
6. effective value equals base scalar plus learned delta;
7. checkpoint roundtrip preserves numeric events and curves;
8. unknown checkpoint version is rejected;
9. viewport starts with empty learned-bias state;
10. snapshot exposes numeric learned-bias summary;
11. source hygiene prevents human-readable output and rejected architecture
    terms in learned-bias body code.

## 13. Admission Checklist

An implementation is admissible only if all answers are yes:

- Is `BiasPriors` kept recoverable and unmodified by learned updates?
- Are learned deltas produced by body substrate events rather than external
  request setters?
- Does every event pass through `clamp_modification`?
- Are support and deviation both represented numerically?
- Can a learned curve weaken, reverse, decay, or be evicted?
- Does checkpointing store binary numeric body state only?
- Does snapshot expose observer-visible numeric facts only?
- Are semantic readiness, symbolizers, concept ids, source identity truth,
  owner fields, trust labels, and personality labels absent?

If any answer is no, reject or rewrite.

## 14. Closing Position

This layer is life-compatible only as experience sedimentation.

It is not a configuration surface. It is not a personality surface. It is not a
truth surface.

The body keeps its birth shape and carries lived curves over it:

```text
base is preserved
experience sediments as delta
future behavior reads base + delta
correction can still revise delta
```

That is the minimal persistence form that lets experience matter without
turning experience into fixed ontology.
