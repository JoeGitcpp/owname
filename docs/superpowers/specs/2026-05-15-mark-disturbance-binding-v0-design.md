# Mark Disturbance Binding v0 Design

Date: 2026-05-15
Status: DRAFT v0

## Purpose

This design introduces a minimal external mark-disturbance substrate so that
repeated external traces can bind to existing body substrate objects without
becoming language ontology.

The first carrier is byte-shaped input, including human-visible text as an
external research convenience. Future visual glyphs, image marks, vocal
contours, and gesture traces should enter the same law: a repeatable external
trace produces numeric mark evidence; binding to objects, relations, or time
windows remains learned, revisable, and ambiguity-preserving.

## Boundary Notes

This design necessarily mentions `text`, `symbol`, `word`, `label`, `name`,
`token`, and `concept` to define what is forbidden.

1. External description vs internal ontology:
   these terms describe external human artifacts or rejected architectures.
   They are not internal body units.
2. Engineering index use:
   mark hashes, carrier kind codes, frame refs, evidence refs, and target refs
   are engineering locators over disturbance traces only.
3. Why not ontological:
   no field stores what a mark means, what word it is, what object is named,
   or whether an expression is true. Object substrate remains prior.
4. Why no highest-principle or language-axiom violation:
   the body continues to process numeric substrate events. Mark binding is an
   external perturbation and learned binding path, not a request/response
   controller and not a naming gate.

Any implementation that adds token ids, word ids, text understanding, concept
allocation, name readiness, semantic promotion, identity ownership, or
body-side human-readable output is inadmissible.

## Core Model

A mark disturbance is an external trace with enough recurrence to be tracked as
a candidate pattern. It may come from bytes now and other carriers later.

The admissible trace state is:

- `carrier_kind_code`: numeric carrier family code;
- `mark_hash`: stable engineering hash of the external trace;
- time window;
- recurrence count;
- stability;
- ambiguity;
- evidence refs.

The mark trace does not decide its target. Binding happens only through
ordinary `language_binding` events against existing `ObjectSlot`,
`RelationEdge`, or `TimeWindow` references.

## v0 Architecture

1. `mark_disturbance.rs`
   - Records bounded mark traces from byte carrier events.
   - Computes stable hashes and numeric recurrence/stability/ambiguity.
   - Exposes helpers to build `BindingEvent` only when a caller supplies an
     existing substrate target and feedback.

2. `Viewport`
   - Owns `MarkDisturbanceState`.
   - Provides `observe_mark_bytes`, `read_mark_disturbance`, and
     `bind_latest_mark_to_target` methods.

3. Snapshot / observer
   - Snapshot exposes numeric mark summaries only.
   - Observer may render compact numeric values because observer is outside
     the body.

4. Curriculum / report
   - External harness trains byte marks against synthetic object slots.
   - It reports retention, ambiguity, and reentry evidence against cold
     baselines.

## v0 Non-Goals

- No tokenizer.
- No parser.
- No word or character ontology.
- No text understanding.
- No meaning database.
- No object creation from marks alone.
- No binding feedback invented by sensory adapters.
- No body-side printing.

## Acceptance Criteria

1. Repeated byte-carrier marks form one stable mark trace with increasing
   recurrence and stability.
2. A mark can be bound to a live object slot through an explicit binding event.
3. Repeated aligned evidence strengthens `SymbolSeq -> ObjectSlot` binding.
4. Contradictory target evidence weakens or preserves ambiguity without
   rewriting object substrate.
5. Hold preserves a meaningful fraction of mark-binding strength.
6. Reentry after hold strengthens faster or higher than a cold baseline.
7. Report command emits compact observer-side numeric evidence.
8. Hygiene tests reject token, word, concept, semantic, naming-gate, identity,
   owner, ethics, personality, and body-side human-output drift.
