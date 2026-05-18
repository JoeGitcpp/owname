# Text Disturbance Primitive Understanding v1 Design

Date: 2026-05-16
Status: DRAFT v1

## Purpose

This design advances the text part of mark disturbance training from "a byte
mark can bind" to observer-visible primitive understanding evidence.

In this v1, primitive understanding means that repeated text-shaped external
disturbances can form stable, revisable bindings to already-live substrate
targets, preserve those bindings across idle time, recover faster on reentry
than a cold baseline, and preserve ambiguity under conflict.

## Boundary Notes

This design necessarily uses `text`, `word`, `token`, `label`, `name`,
`semantic`, and `concept` while defining the boundary.

1. External description vs internal ontology:
   `text` describes the external carrier used by the observer curriculum.
   None of these terms become body-side substrate units.
2. Engineering index use:
   bytes, mark hashes, carrier kind codes, target refs, and evidence refs are
   bounded engineering indices over disturbances and traces.
3. Why not ontological:
   no state records what a text item means, what object it names, whether it is
   a word, or whether an expression is true.
4. Why no highest-principle or language-axiom violation:
   the body substrate remains a continuously running numeric substrate. This
   v1 adds only observer-side curriculum/report evidence and reuses existing
   binding events against existing targets.

Any implementation that adds token ids, word ids, fixed vocabulary,
dictionary meaning, parser output, concept allocation, naming gates, semantic
promotion, or body-side human-readable output is inadmissible.

## Model

Text enters as byte-carrier mark disturbance. A byte trace may become stable
through recurrence, but it still has no target by itself. Binding remains a
separate learned event against an existing target:

- object slot,
- relation edge,
- time window.

This v1 focuses on object-slot binding and a minimal time-window binding probe.
It does not build syntax, composition, grammar, or language generation.

## Curriculum Shape

The observer-side curriculum creates one persistent `Viewport` and two live
object slots. It then drives:

1. repeated text disturbance A aligned with object slot A;
2. repeated text disturbance B aligned with object slot B;
3. one time-window text disturbance aligned with a numeric time window;
4. an idle hold interval with binding maintenance;
5. a conflict event where disturbance A is presented against object slot B as
   deviation evidence, raising ambiguity without rewriting slots;
6. reentry episodes for A and B on the persistent viewport;
7. a cold-baseline viewport that sees only the reentry episodes.

## Evidence

The report must expose only compact numeric observer evidence:

- training episodes;
- reentry episodes;
- distinct text traces;
- recurrence counts for A and B;
- binding strengths for A->slot A and B->slot B;
- cross-target strength for A->slot B;
- hold strength ratio;
- conflict ambiguity delta;
- persistent reentry strength;
- cold reentry strength;
- time-window binding strength.

## Acceptance Criteria

1. Two distinct text disturbances form two bounded mark traces.
2. A->slot A and B->slot B both strengthen above candidate level.
3. A does not become a forced binding to slot B.
4. Hold preserves at least 80% of the trained average strength.
5. Conflict evidence raises ambiguity while both object slots remain live.
6. Reentry on the persistent viewport is stronger than cold reentry.
7. A text disturbance can also bind to a legal time-window target.
8. The report command emits compact numeric evidence.
9. Hygiene tests reject token/word/concept/semantic/naming-gate drift and
   body-side human-readable output.
