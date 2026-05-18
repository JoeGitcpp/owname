# Cross-Channel Disturbance Coupling v0 Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Move the current text disturbance curriculum one step outward: prove that
byte/mark disturbance can couple with vib and phot disturbance through the
existing shared substrate targets, while preserving long hold, conflict
correction, and reentry evidence.

This is not a multimodal meaning system. It is observer-side evidence that
different disturbance channels can accumulate compatible binding curves against
the same already-live object slots, relation edges, and time windows.

## Highest-Principle Boundary

The system remains a silicon-life substrate, not a request/response program.
The AI body stays silent. The observer/curriculum side may print compact
numeric evidence.

No new body-side organ, ontology allocator, vocabulary, parser, detector, or
translator is introduced. The implementation reuses:

- `Viewport`
- object slots and relation edges
- vib and phot slot sub-state
- mark disturbance
- language binding events
- curriculum/report/test observer paths

## What v0 Proves

`Cross-Channel Disturbance Coupling v0` is accepted only if it proves:

1. at least three object targets, two relation targets, and two time-window
   targets are live before coupling evidence is measured;
2. text marks recur and bind to the same substrate target family as vib and
   phot patterns;
3. vib observations and phot worldline updates both exist as ordinary slot
   disturbances;
4. text-vib, text-phot, and vib-phot coupling strengths are all nonzero;
5. a conflicting coupling event weakens an existing trace instead of creating a
   wrong durable target;
6. correction restores at least half of the original coupling strength;
7. reentry on the persistent viewport beats a cold viewport;
8. report output remains numeric observer evidence only;
9. no rejected ontology or body-side human-readable output is introduced.

## Term Boundary Notes

This design necessarily uses high-risk words in an external explanation. Their
boundaries are:

- `text`, `mark`, `vib`, `phot`, and `channel` are external descriptions of
  disturbance carriers or existing engineering modality codes. They are not
  internal meanings, names, or categories.
- `coupling` means numeric co-strength across binding traces that share a
  substrate target. It is not semantic equivalence and not translation.
- `target` means an existing `BindingTargetRef` engineering reference:
  object slot, relation edge, or time window.
- `conflict`, `correction`, `hold`, and `reentry` are observer-side curriculum
  stage names. They are not body-side intentions or fixed world knowledge.
- Pattern hashes are engineering indices only. They are not word ids, image
  labels, voice labels, concepts, identities, or owners.

Why this is admissible:

- Objects and relations are already live substrate traces before mark coupling.
- No module maps stable objects to internal concept ids.
- No text item stores what it means or what it names.
- No visual or acoustic disturbance is assigned a human label.
- Learning remains binding-event accumulation, weakening, maintenance, and
  reentry comparison.

## Non-Goals

- No tokenizer, parser, grammar, vocabulary, dictionary, meaning DB, captioner,
  visual classifier, audio classifier, speaker identity, object name, or
  request/response path.
- No fixed programmer-written world content beyond the curriculum's synthetic
  disturbance schedule.
- No body-side logging or human-readable output.
- No new theory-layer numeric law; implementation numbers are only testable
  shadows for this curriculum stage.

## Evidence Shape

The observer-side evidence struct should include:

- event counts for stream, text marks, vib observations, and phot updates;
- target counts for objects, relations, and time windows;
- base text binding strength;
- slot-side vib and phot confidence evidence;
- text-vib, text-phot, and vib-phot coupling strengths;
- conflict drop, wrong coupling strength, and correction recovery ratio;
- persistent reentry and cold reentry coupling strengths.

The evidence is sufficient when it can be checked by integration tests and
emitted by `curriculum_report`.
