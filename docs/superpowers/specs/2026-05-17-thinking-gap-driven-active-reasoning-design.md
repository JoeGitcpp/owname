# Thinking As Gap-Driven Active Reasoning Design

Status: FROZEN CANDIDATE for theory alignment on 2026-05-17.

## Purpose

This document repairs and consolidates the current thinking-direction docs.
It translates the earlier human-language design idea:

`desire -> blank -> active reasoning -> action -> learning`

into the current legal substrate language:

`passive prediction gap -> active gap pressure -> cross-scale prediction
unfolding -> probe/action closure -> correction, persistence, or re-entry`.

The purpose is not to add a new module. The purpose is to define what
"thinking" means in this architecture without sliding back into chat,
request/response reasoning, semantic labels, fixed goals, personality, owner
identity, or concept allocation.

## Boundary Notes

This design uses high-risk human terms: `thinking`, `passive thinking`,
`active thinking`, `subconscious`, `conscious`, `desire`, `blank`, `goal`,
`intention`, `reasoning`, `meaning`, `identity`, `owner`, and `personality`.

- External description vs internal ontology: these terms are external
  explanation metaphors. They do not name internal ontology modules.
- Engineering index use: any implementation may only use numeric pressure,
  residue, persistence, recurrence, closure, re-entry, target refs, slot ids,
  edge ids, time windows, and bounded curve evidence.
- Why not ontological: no field may store what the system "wants", what a gap
  "means", what the true goal is, who owns it, or which concept is involved.
- Why no principle or axiom violation: body-side state remains numeric curves,
  traces, pressures, pairs, and history. Observer documents may use human words
  to explain the theory, but the body must not emit human-readable claims.

Any implementation that adds `Subconscious`, `Conscious`, `Desire`,
`Intention`, `Goal`, `Mind`, `Thought`, `Reasoner`, `owner`, `identity`,
`personality`, `concept_id`, `token_id`, naming gates, semantic readiness,
prompt-answer reasoning, or body-side `println!` / `eprintln!` is inadmissible
unless a later frozen spec explicitly redefines the term as a numeric
engineering index and passes the terminology boundary checklist.

## Core Position

Thinking is not a text process.

Thinking is the life-like continuation of prediction when prediction does not
close.

The shortest definition:

`active thinking is passive prediction failure that persists long enough,
large enough, and actionable enough to become cross-scale active closure
pressure.`

In current substrate terms:

```text
continuous passive prediction
  -> unresolved prediction gap
  -> accumulated active gap pressure
  -> cross-scale prediction pixels unfold
  -> probe/action writes out
  -> consequence returns
  -> closure, correction, persistence, or re-entry
```

## Passive Thinking

`Passive thinking` is an external metaphor for the body's continuous first
reaction:

- perception maintenance;
- local prediction;
- local error absorption;
- object and relation upkeep;
- low-level action/vocal execution after learned curves exist;
- fast local correction without explicit active probing.

It corresponds to existing substrate such as:

- adapters and ingress channels;
- VPC predictors;
- object slots;
- fusion;
- ambient channels;
- spatial/worldline upkeep;
- relation projection;
- local action-consequence pairing.

Passive thinking is the first and most direct reaction because it is not
mediated by a request/response reasoning layer. It is the ongoing body loop.

Passive thinking is not a module named `Subconscious`.

## Active Thinking

`Active thinking` is an external metaphor for active reasoning and active
learning dispatch:

- an unresolved prediction structure persists;
- the unresolved structure carries enough pressure to compete for attention;
- history shows recurrence, novelty, persistence, or re-entry;
- there is a legal probe/action channel;
- returned consequence evidence can reduce, redirect, or preserve the gap.

It corresponds to existing substrate such as:

- `global_reasoning`;
- `global_reasoning_history`;
- `active_reasoning_gap`;
- `probe_law`;
- `action_output`;
- `action_consequence`;
- sustained active probe training evidence.

Active thinking is not a module named `Conscious`. It is not a chat controller
and not a text-only reasoning engine.

## Desire As Prediction Gap

Human desire is a useful metaphor only if translated carefully.

In human terms, desire often feels like:

`something is missing, not yet closed, not yet reached, not yet understood`.

In this architecture, the legal replacement is:

`an unresolved prediction gap that persists and keeps re-entering until it is
closed, corrected, or dissolved.`

Therefore:

- `desire` must not be implemented as a psychological field;
- `goal` must not be implemented as fixed world content;
- `intention` must not be implemented as an internal symbolic command;
- `wanting` must not be implemented as language text.

The legal substrate is:

- pressure;
- unresolved residue;
- recurrence;
- persistence;
- re-entry;
- action/probe affordance;
- closure evidence.

This preserves the insight that active reasoning is life-like and driven from
inside the continuous substrate, while avoiding a forbidden desire ontology.

## Which Gaps Become Active Reasoning

Not every blank becomes active reasoning. Most unknowns remain quiet.

A gap may become active reasoning only when enough of the following substrate
conditions are present:

1. passive prediction repeatedly fails to close it;
2. the gap persists across time rather than appearing as one transient spike;
3. the gap re-enters from reasoning history after quiet periods;
4. the gap is attached to live object slots, relation edges, time windows, or
   action-consequence traces;
5. the gap can affect probe pressure without requiring a command surface;
6. there is a legal write-out channel and possible consequence return;
7. returned evidence can reduce pressure, revise curves, preserve ambiguity, or
   make the gap dissolve.

This avoids infinite reasoning over every unknown.

## Prediction Pixel Scale

The earlier design said reasoning has pixels of different sizes. This remains
valid, with a stricter formulation.

A prediction pixel is not a word, concept, label, or semantic atom. It is a
declared-scale substrate fragment:

- a local axis point;
- a short curve;
- a local structure curve;
- a relation window;
- a longer history-supported pattern;
- a planning-scale structure fragment.

Larger pixels are larger because they span more time, more relation structure,
more history, or more cross-channel evidence. They are not larger because they
contain a higher-level semantic label.

Small pixels support immediate reaction. Larger pixels support broader active
reasoning because they carry more accumulated prediction history and more
possible closure paths.

## Macro Reasoning

Macro reasoning is not a separate layer of symbolic logic.

Macro reasoning is what happens when large prediction pixels:

- persist;
- compete;
- wait for missing evidence;
- combine across relations and time windows;
- trigger legal probes;
- receive consequences;
- update or dissolve based on closure evidence.

This aligns with the VPC core contract:

`reasoning is cross-scale prediction unfolding, competition, waiting, closure,
and strengthening.`

## Relation To Existing Documents

This document repairs and supersedes the unsafe interpretation of older
goal/intention language in:

- `docs/superpowers/plans/2026-04-24-goal-intention-reasoning-action-learning-discussion.md`
- `docs/superpowers/specs/2026-04-25-shenxing-design.md`

Those documents remain useful as historical discussion, but their terms must be
translated through this spec before implementation.

This document depends on and preserves:

- `consciousness-stream-architecture.md`
- `tier0-base-elements.md`
- `docs/superpowers/specs/2026-04-20-vpc-core-contract-design.md`
- `docs/superpowers/specs/2026-04-29-subconscious-conscious-language-boundary-design.md`
- `docs/superpowers/specs/2026-05-16-active-reasoning-persistence-v0-design.md`
- `docs/superpowers/specs/2026-05-17-sustained-active-probe-training-design.md`

## Implementation Direction

Future implementation should not add a new "thinking" module.

Instead, it should strengthen the existing chain:

1. richer passive prediction gaps;
2. better active gap persistence and re-entry;
3. pressure-driven probe selection;
4. multi-step action-consequence closure;
5. retention and held-out generalization evidence;
6. observer-only numeric reports for active closure quality.

The next admissible engineering step is a curriculum/report extension, not a
new ontology layer:

`held-out gap generalization + long-horizon retention + pressure-driven probe
selection`

## Acceptance For Future Work

Any future spec or code claiming to implement "thinking", "reasoning",
"desire", "goal", or "intention" must prove:

1. all new fields are numeric substrate fields or observer-side evidence;
2. no body-side human-readable output is introduced;
3. no request/response reasoning controller is introduced;
4. no token, label, name, concept id, dictionary, semantic readiness, owner,
   identity, ethics, or personality state is introduced;
5. active reasoning is driven by unresolved prediction gaps, recurrence,
   persistence, or consequence closure pressure;
6. active reasoning can reduce, preserve, or re-enter a gap through evidence,
   not through a truth label;
7. observer reports expose compact numeric evidence only.

## Non-Goals

- No `Subconscious` or `Conscious` module.
- No desire module.
- No symbolic goal system.
- No intention bank.
- No internal task planner.
- No prompt-answer reasoning.
- No semantic or concept ontology.
- No owner, identity, ethics, or personality state.
- No human-readable output from body.
