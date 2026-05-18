# Post-Foundation Sustained Active Probe Expansion Design

Status: Draft for TDD execution on 2026-05-17.

## Purpose

Expand the post-foundation sustained active probe training stage after the
foundation readiness gate has passed. The expansion raises the training load
from a short repeated closure proof into a broader observer-side course that
keeps one persistent substrate alive across more cycles, more disturbance
materials, cross-channel coupling, replay ingress, live-like return ingress,
and final reentry against a cold baseline.

The target evidence remains:

`repeated disturbance -> conflict -> active probe write-out -> return pairing -> correction -> cross-channel carryover -> reentry`

This is still a training harness. It does not add a body-side understanding
module, naming gate, concept allocator, parser, question/answer surface, or
human-readable body output.

## Boundary Notes

This design uses `foundation`, `sustained`, `active probe`, `material`,
`cross-channel`, `replay`, `live`, and `understanding` as observer/training
descriptions.

- External description vs internal ontology: these words describe curriculum
  stages and evidence streams. They are not body ontology units.
- Engineering index use: cycle counts, material counts, channel counts, replay
  frame counts, live return counts, and margin-like ratios are observer-side
  numeric indices.
- Why not ontological: no state records what a mark, channel, target, source, or
  result "means"; the body keeps curves, traces, pairs, windows, and numeric
  evidence.
- Why no principle or axiom violation: all pass/fail judgment stays outside the
  body in the observer report and tests. The body remains a continuously running
  numeric substrate and does not emit human-readable claims.

Any implementation that adds token ids, word ids, fixed vocabulary, dictionary
meaning, parser output, concept allocation, naming gates, semantic promotion,
identity/source ownership, ethics/personality state, chat behavior, or body-side
human-readable output is inadmissible.

## Chosen Approach

Use an additive expansion of the existing sustained active probe curriculum in
`bionic_v0/src/curriculum.rs`.

Rejected alternatives:

- A separate new training runner: rejected because it would duplicate the same
  closure loop and make readiness evidence harder to compare.
- A body-side adaptive training controller: rejected because the current stage
  is observer-side curriculum pressure, not a new body module.
- A language/task benchmark: rejected because the stage must remain substrate
  closure evidence, not question answering or text understanding.

## Architecture

Extend `SustainedActiveProbeTrainingEvidence` with observer-only numeric fields:

- expanded cycle count;
- material count;
- replay ingress frame count;
- live return ingress frame count;
- cross-channel VIB observations;
- cross-channel PHOT worldline updates;
- mean text/VIB/PHOT triad coupling;
- persistent-vs-cold reentry margin;
- principle guard pass flag.

Keep one function:

`run_sustained_active_probe_training_curriculum(config)`

The function still runs one persistent `Viewport`, but the loop expands to:

1. train a larger set of byte-mark materials against object, relation, and time
   targets;
2. run twelve conflict/probe/return/correction cycles;
3. route additional scripted byte frames through `CodeIngressAdapter` as replay
   ingress evidence;
4. route direct framed-byte returns as live-like ingress evidence;
5. write VIB and PHOT substrate samples and bind their modality traces against
   the same targets as the byte-mark traces;
6. compute persistent reentry strength and cold reentry strength;
7. emit only compact numeric evidence in `curriculum_report`.

## Pass Criteria

Tests must prove:

1. `cycles_run >= 12`;
2. `materials_seen >= 10`;
3. `conflict_events >= 24`;
4. `probe_frames >= 24`;
5. `return_windows >= 24`;
6. `paired_consequences >= 24`;
7. `replay_ingress_frames >= 6`;
8. `live_return_ingress_frames >= 24`;
9. `cross_channel_vib_observations >= 12`;
10. `cross_channel_phot_worldline_updates >= 3`;
11. `cross_channel_coupling_strength > 0.0`;
12. `min_ambiguity_reduction_ratio > 0.10`;
13. `min_correction_recovery_ratio >= 0.80`;
14. `max_wrong_binding_strength <= 0.01`;
15. `persistent_reentry_margin > 0.10`;
16. `principle_guard_pass == 1`.

The report mode `curriculum_report --sustained-active-probe-training` must emit
all new fields as compact numeric lines.

## Non-Goals

- No new body-side module.
- No tokenizer, parser, dictionary, concept id, label store, naming gate, or
  semantic readiness gate.
- No chat/request-response interface.
- No fixed world-content curriculum theory. The fixed bytes in this harness are
  test disturbance carriers only.
- No body-side `println!` or `eprintln!`.
