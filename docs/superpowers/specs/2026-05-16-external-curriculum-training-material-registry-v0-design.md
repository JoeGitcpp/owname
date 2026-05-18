# External Curriculum Training Material Registry v0 Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Define the external curriculum training plan at the material boundary before
building a governor.

The current curriculum stages can prove retention, conflict correction, active
probe closure, and cross-channel coupling. They still construct their material
inside each course. This v0 adds an observer-side registry that declares what
disturbance material exists, which carrier it uses, which curriculum stage it
can feed, whether it is reproducible, and whether it stays outside the body.

This is a plan for external training material, not a plan to train a large
language model and not a body-side scheduler.

## Architecture

Add a small `curriculum_material` module under `bionic_v0/src`.

It owns only observer-side data:

- material entries;
- carrier kind codes;
- origin kind codes;
- stage kind codes;
- reproducibility fields;
- principle review evidence.

It does not feed the body by itself. Later curriculum runners and governors can
read the registry and decide which existing course to run. The body still only
receives ordinary disturbances through existing substrate paths.

## Training Material Sources

The plan admits four origin kinds:

1. Synthetic disturbance material:
   deterministic byte/mark, vib, and phot curves used for controlled tests.
2. Replay material:
   previous snapshots, event traces, and report evidence used for hold and
   reentry comparison.
3. Probe-return material:
   framed byte write/return traces produced by active probe closure.
4. Live-ingress material:
   future camera/audio/text ingress streams, admitted only as raw disturbance
   carriers before any world claim is made.

These origins are external curriculum categories. They are not source identity,
trust, ownership, or semantic authority.

## Principle Review

This design is admissible if all of the following remain true:

- Research observation and training planning stay outside the AI body.
- Material entries are engineering indices and numeric kind codes only.
- Material entries do not store meanings, object names, visual labels, voice
  labels, dictionary entries, prompt/answer pairs, command intent, ownership,
  identity, ethics, or personality.
- Boundary formation is still learned through curriculum evidence; the registry
  only says which disturbance carrier can be replayed or generated.
- The first governor will consume numeric evidence from observer-side reports,
  not body-side human-readable output.

## High-Risk Term Boundary

This spec uses high-risk terms only as external descriptions:

- `text` means byte/mark disturbance carrier, not a language ontology.
- `material` means an external disturbance recipe or replay pointer, not
  knowledge content.
- `source` means external origin kind for scheduling, not identity, owner, or
  trust.
- `stage` means curriculum phase code, not a fixed mental state.
- `review` means observer-side admissibility evidence, not internal ethics or
  personality.

All implementation fields must stay numeric or boolean. Any human-readable
explanation belongs in docs/tests/report output, never inside the body process.

## v0 Acceptance

`External Curriculum Training Material Registry v0` is accepted when tests prove:

1. the default plan contains synthetic, replay, probe-return, and live-ingress
   material groups;
2. the default plan covers mark, vib, phot, framed-byte, and snapshot-trace
   carriers;
3. the default plan covers hold, variation, conflict, correction, reentry,
   coupling, and active-probe stages;
4. all material groups are reproducible or explicitly observer-only deferred
   live ingress;
5. no material group touches the body directly;
6. the principle review reports zero missing required roles and passes;
7. the observer report emits numeric plan/review evidence;
8. source hygiene tests reject ontology drift.

## Non-Goals

- No governor decision algorithm yet.
- No automatic background automation restart.
- No body-side module.
- No internet ingestion or web dataset ingestion.
- No labels, classes, captions, dictionaries, prompts, answers, identities,
  owners, ethics, or personalities.
