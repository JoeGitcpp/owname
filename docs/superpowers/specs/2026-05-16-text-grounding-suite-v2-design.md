# Text Grounding Suite v2 Design

Date: 2026-05-16
Status: DRAFT v2

## Purpose

This design sets the next text-disturbance goal: prove that text-shaped
external disturbances can support stable grounding behavior across multiple
objects, variants, primitive composition, conflict correction, hold, and
reentry.

The target is intentionally larger than a single binding course. v2 should
show that the substrate can preserve and revise a small text-disturbance field
without turning text into internal language ontology.

## Highest Principle Boundary

The system remains a silicon-life substrate, not a request/response program.
The body process must not print human-readable output. Research observation,
training reports, numeric evidence, and debugging stay outside the body.

This design necessarily uses high-risk external terms: `text`, `word`,
`token`, `label`, `name`, `semantic`, and `concept`.

1. External description vs internal ontology:
   these terms describe the external human-visible carrier or forbidden
   shortcuts. They are not internal body units.
2. Engineering index use:
   byte payloads, mark hashes, variant hashes, carrier codes, evidence refs,
   target refs, and report fields are bounded engineering indices over
   disturbance traces.
3. Why not ontological:
   no field records what a text item means, what object it names, whether it
   is a word, or whether a composition is true.
4. Why no principle or axiom violation:
   grounding evidence is formed through ordinary mark traces and binding
   events against already-existing substrate targets. The observer may report
   compact numbers; the body does not become a text interpreter.

## Rejected Moves

The v2 implementation must not add:

- tokenizer, parser, vocabulary table, dictionary, grammar tree, or text
  meaning database;
- token ids, word ids, object names, concept ids, semantic readiness,
  symbolizer, naming gates, identity, owner, ethics, or personality fields;
- any module that maps stable object or relation substrate to a discrete
  internal concept;
- any body-side `println!`, `eprintln!`, or human-readable diagnostic output;
- any adapter-side fabricated binding feedback.

## Core Model

Text enters as byte-carrier mark disturbance. Each disturbance produces a
bounded numeric trace with recurrence, stability, ambiguity, and evidence
refs. Binding is a separate learned event against an existing substrate target:

- object slot;
- relation edge;
- time window.

v2 adds no new text ontology. It adds only an external curriculum that creates
enough structured disturbance history to prove larger behavior.

## v2 Goal

Build `Text Grounding Suite v2`: an observer-side curriculum/report proving
all of the following:

1. multiple text disturbances can remain distinct across multiple targets;
2. bindings survive idle hold and recover on reentry;
3. variants can generalize through shared substrate evidence without becoming
   the same internal token;
4. adjacent disturbances can form a primitive composition trace by binding to
   relation/time-window targets;
5. conflict increases ambiguity and weakens wrong evidence without rewriting
   live object substrate;
6. correction restores the supported binding after conflict;
7. persistent reentry beats cold baseline on average and on the weakest item.

## Curriculum Shape

The observer curriculum should create one persistent `Viewport` and at least:

- three live object slots;
- two relation edges or relation-like substrate targets formed from those
  objects;
- one or more time-window targets;
- six base text disturbances;
- at least three variant disturbances sharing substrate context with base
  disturbances;
- at least two adjacent-pair disturbance sequences used as composition
  precursors.

The suite should run these stages:

1. **Train**
   Align six base disturbances across object slots, relation edges, and time
   windows.
2. **Variant**
   Present byte-level variants with shared target evidence and measure whether
   they develop nearby routing behavior without replacing base traces.
3. **Composition**
   Present adjacent disturbance pairs and bind the pair window to relation or
   time-window targets.
4. **Hold**
   Advance maintenance over idle time and measure retained strength ratios.
5. **Conflict**
   Present wrong-target or contrary evidence and measure ambiguity rise plus
   wrong-binding suppression.
6. **Correction**
   Reintroduce supported evidence and measure recovery ratio.
7. **Reentry**
   Resume supported disturbances on the persistent viewport and compare
   against a cold viewport with the same reentry exposure only.

## Evidence Report

Add an observer-side report mode:

```text
curriculum_report --text-grounding-suite
```

Expected compact numeric fields:

```text
mode=text_grounding_suite
train_marks=6
object_targets=3
relation_targets=2
time_window_targets=...
variant_marks=...
composition_windows=...
train_avg_strength=...
train_weakest_strength=...
hold_avg_ratio=...
hold_weakest_ratio=...
variant_generalization_score=...
variant_distinctness_ratio=...
composition_relation_strength=...
conflict_ambiguity_delta=...
wrong_binding_strength=...
correction_recovery_ratio=...
reentry_persistent_avg=...
reentry_cold_avg=...
reentry_persistent_weakest=...
reentry_cold_weakest=...
```

The report must stay numeric. It may print from `curriculum_report` because
that binary is observer-side.

## Acceptance Criteria

1. At least six base disturbances form distinct bounded mark traces.
2. At least three object-slot targets receive supported bindings.
3. At least two relation/time-window targets receive supported bindings.
4. Training average strength is above candidate level and weakest strength is
   nonzero.
5. Hold preserves at least 80% of trained average strength and at least 60% of
   trained weakest strength.
6. Variants show measurable shared-target routing while remaining distinct
   traces from their bases.
7. Composition precursor windows strengthen relation or time-window bindings.
8. Conflict raises ambiguity and keeps wrong-binding strength bounded.
9. Correction restores a meaningful fraction of the pre-conflict supported
   binding.
10. Persistent reentry beats cold baseline for average and weakest strength.
11. Hygiene tests prove no rejected language ontology or body-side output was
    introduced.
12. Full crate tests and `git diff --check` pass.

## Subagent And Skill Governance

Subagents and skills are allowed during implementation, but only under these
rules:

- Every subagent prompt must include the repository entrypoint and the highest
  principle summary.
- No subagent may edit `body` output behavior or introduce internal text
  ontology.
- Worker subagents must own disjoint write sets.
- Explorer/reviewer subagents may audit boundaries, tests, and numeric
  evidence.
- Main agent remains responsible for final integration and verification.
- `superpowers:test-driven-development` is mandatory for behavior changes.
- `superpowers:systematic-debugging` is mandatory for unexpected failures.
- `superpowers:verification-before-completion` is mandatory before claiming
  completion.

## Non-Goals

- No natural language understanding claim.
- No grammar or parser.
- No text generation.
- No live keyboard/chat interface.
- No real camera, microphone, or speech integration in this phase.
- No body-side human-readable output.
