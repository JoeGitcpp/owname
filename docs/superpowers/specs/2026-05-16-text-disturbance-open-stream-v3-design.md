# Text Disturbance Open-Stream v3 Design

Date: 2026-05-16
Status: DRAFT v3

## Purpose

Text Grounding Suite v2 proved that a controlled set of text-shaped external
disturbances can support hold, variant generalization, adjacent composition,
conflict correction, and reentry evidence.

v3 moves one level closer to a real external environment: a continuous
observer-side text disturbance stream. The stream contains supported
disturbances, unrelated disturbances, near variants, conflicts, delayed target
evidence, and idle gaps. The acceptance target is not natural language
understanding. It is observer-visible proof that the substrate can form and
retain stable disturbance-to-substrate connections while preserving ambiguity
and rejecting ontology shortcuts.

## Highest Principle Boundary

The system remains a silicon-life substrate, not a request/response program.
The body process must not print human-readable output. Research observation,
training reports, numeric evidence, and debugging stay outside the body.

This design necessarily uses high-risk external terms: `text`, `word`,
`token`, `label`, `name`, `semantic`, and `concept`.

1. External description vs internal ontology:
   these terms describe the human-visible carrier or forbidden shortcuts. They
   are not body-side units.
2. Engineering index use:
   byte payloads, stream ids, mark hashes, time spans, target refs, and report
   fields are bounded engineering indices over disturbance traces.
3. Why not ontological:
   no field records what a text item means, what object it names, whether it
   is a word, or whether a stream segment is true.
4. Why no principle or axiom violation:
   evidence is formed through ordinary mark traces, temporal windows, and
   binding events against existing substrate targets. The observer may report
   compact numbers; the body does not become a text interpreter.

## Rejected Moves

The v3 implementation must not add:

- tokenizer, parser, vocabulary table, dictionary, grammar tree, meaning DB,
  or natural-language-understanding layer;
- token ids, word ids, object names, concept ids, semantic readiness,
  symbolizer, naming gates, identity, owner, ethics, or personality fields;
- any module that maps stable object or relation substrate to a discrete
  internal concept;
- any body-side `println!`, `eprintln!`, or human-readable diagnostic output;
- any fixed internal category saying which stream items are important.

## Core Model

The stream is an observer-side curriculum protocol. Each item is a timed byte
disturbance with a carrier stream id, intensity, optional alignment action,
and optional target evidence delay. The body-visible substrate receives only
the same mark observation and binding-event primitives already used by v2.

Open-stream learning is measured by numeric evidence:

- recurrence and bounded trace count across a mixed stream;
- retained strength after idle gaps;
- persistent-vs-cold advantage for delayed variants;
- suppression of unsupported conflict traces;
- recovery after supported correction;
- composition evidence from adjacent observed windows;
- boundary evidence formed from repeated temporal co-occurrence, not a
  predefined word or phrase boundary.

## v3 Goal

Build `Text Disturbance Open-Stream v3`: an observer-side curriculum/report
that proves all of the following in one persistent `Viewport`:

1. supported disturbances remain stable inside a mixed continuous stream;
2. unrelated disturbances do not steal supported target evidence;
3. near variants route better in the persistent stream than in a cold stream;
4. adjacent observed windows strengthen relation/time-window evidence;
5. delayed target evidence still forms a measurable connection;
6. conflict raises ambiguity while wrong binding remains bounded;
7. correction restores supported evidence after conflict;
8. reentry after idle beats cold baseline;
9. all evidence is numeric and observer-side.

## Curriculum Shape

The observer curriculum should create one persistent `Viewport` and at least:

- three live object slots;
- two live relation targets from those objects;
- two time-window targets;
- one mixed stream with at least twenty observed disturbance events;
- at least six supported base disturbance forms;
- at least four unrelated disturbance forms;
- at least three near-variant disturbance forms;
- at least two conflict events;
- at least two delayed evidence windows;
- at least two adjacent-pair composition windows.

The suite should run these stages:

1. **Open train**
   Present a mixed stream of supported, unrelated, and near-variant
   disturbances. Supported disturbances receive repeated alignment against
   object, relation, or time-window targets. Unrelated disturbances are
   observed without supported target alignment.
2. **Delayed evidence**
   Present supported disturbance bytes first, then bind after a later target
   interval. Measure whether delayed association remains nonzero.
3. **Composition windows**
   Present adjacent disturbance pairs as two observed windows and bind the
   pair evidence to relation/time-window targets.
4. **Hold**
   Advance idle maintenance and compare retained supported strength against
   trained strength.
5. **Conflict**
   Present contrary evidence with deviation feedback. Measure ambiguity rise
   and wrong-binding suppression.
6. **Correction**
   Reintroduce supported evidence and measure recovery.
7. **Reentry**
   Resume mixed stream exposure on the persistent viewport and compare with a
   cold viewport that receives only reentry exposure.

## Evidence Report

Add an observer-side report mode:

```text
curriculum_report --text-open-stream
```

Expected compact numeric fields:

```text
mode=text_open_stream
stream_events=...
supported_marks=...
unrelated_marks=...
variant_marks=...
object_targets=...
relation_targets=...
time_window_targets=...
composition_windows=...
delayed_windows=...
train_supported_avg=...
train_supported_weakest=...
unrelated_leakage_strength=...
hold_supported_ratio=...
variant_persistent_avg=...
variant_cold_avg=...
delayed_binding_strength=...
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

1. At least twenty stream events are observed.
2. At least six supported marks, four unrelated marks, and three variant marks
   appear in the stream.
3. At least three object targets, two relation targets, and two time-window
   targets participate in the evidence path.
4. Supported training average strength is above candidate level and weakest
   supported strength is nonzero.
5. Unrelated leakage remains below supported training strength.
6. Hold preserves at least 70% of supported average strength.
7. Persistent variants beat cold variants on average.
8. Delayed evidence strength is nonzero.
9. Adjacent composition windows produce nonzero relation/time-window strength.
10. Conflict raises ambiguity and keeps wrong-binding strength bounded.
11. Correction restores at least half of pre-conflict supported strength.
12. Persistent reentry beats cold baseline for average and weakest strength.
13. Hygiene tests prove no rejected language ontology or body-side output was
    introduced.
14. Full crate tests and `git diff --check` pass.

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
- No keyboard/chat interface.
- No real camera, microphone, or speech integration in this phase.
- No body-side human-readable output.
