# Repository Entrypoint — `unnamed/`

This file is the **mandatory first read** for any agent or contributor
touching this repository. It exists so future work does not silently drift
back into rejected architecture.

## Read These First (in order)

1. **Highest principle** — [consciousness-stream-architecture.md](consciousness-stream-architecture.md):
   the system is a silicon-life substrate, not a request/response program.
   Research observation and the AI body must stay separated.

2. **tier 0 base elements** — [tier0-base-elements.md](tier0-base-elements.md):
   tier 0 is the shared coordinate substrate, not language content.
   Objects precede names. Parameters are time-curves, not static labels.

3. **Language-system axioms (FROZEN)** —
   [docs/axioms/2026-04-20-new-axioms-v1.md](docs/axioms/2026-04-20-new-axioms-v1.md):
   defines what the language system is, what it is not, and which
   engineering shortcuts collapse it back into token-based AI.

4. **Architecture cleanup spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md):
   the audit-and-cleanup contract for `bionic_v0`. Module classification,
   violation criteria, processing actions, and admission rules for new
   modules / fields / tests.

5. **Language binding layer spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-20-language-binding-layer-design.md](docs/superpowers/specs/2026-04-20-language-binding-layer-design.md):
   defines the binding-layer boundary, legal state, learning unit
   (`binding event`), and the concrete bans that prevent sliding back
   into naming gates, concept allocators, or token ontology.

6. **Endogenous activity / self-regulation laws spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-20-endogenous-activity-self-regulation-laws-design.md](docs/superpowers/specs/2026-04-20-endogenous-activity-self-regulation-laws-design.md):
   defines the abstract laws behind initiative, self-adjustment,
   self-repair, early exploration, and imitation, and maps those laws
   onto the cleaned substrate without turning examples into modules.

7. **Endogenous activity and probe laws spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-21-endogenous-activity-and-probe-laws-design.md](docs/superpowers/specs/2026-04-21-endogenous-activity-and-probe-laws-design.md):
   defines under what abstract conditions a continuously running system
   naturally transitions from endogenous activity into active probe
   initiation, and what a probe channel must satisfy to be a legal
   substrate for early active learning.

8. **Action output substrate minimal contract spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md](docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md):
   defines the minimal abstract contract a body-side output channel must
   satisfy to support early active probing; covers body laws, learning
   loop prerequisites, and safety boundaries; stops at "legal write-out
   exists" without entering consequence closure or imitation.

9. **Action-consequence loop and imitation laws spec (FROZEN)** —
   [docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md](docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md):
   defines the abstract laws behind consequence return, action-consequence
   time alignment, consequence boundary formation, external pattern
   short-term retention, self-other comparison, and imitation as curve
   approximation; all laws map onto existing cleaned substrate without
   introducing independent modules.

10. **Subconscious/conscious language boundary spec (FROZEN)** —
    [docs/superpowers/specs/2026-04-29-subconscious-conscious-language-boundary-design.md](docs/superpowers/specs/2026-04-29-subconscious-conscious-language-boundary-design.md):
    defines the hard boundary for using "subconscious" and "conscious" as
    external metaphors only, and maps language execution vs language learning
    responsibilities onto existing substrate laws without adding ontology
    modules.

11. **Live I/O preflight life-sign contract (FROZEN CANDIDATE)** —
    [docs/superpowers/specs/2026-05-05-live-io-preflight-life-sign-contract-design.md](docs/superpowers/specs/2026-05-05-live-io-preflight-life-sign-contract-design.md):
    defines Live I/O preflight as a body life-sign gate before active
    probe-feedback closure: write-out, ingress return, temporal pairing,
    ambiguity preservation, and observer-visible evidence without adding
    semantic/source/identity ontology.

12. **Thinking as gap-driven active reasoning (FROZEN CANDIDATE)** —
    [docs/superpowers/specs/2026-05-17-thinking-gap-driven-active-reasoning-design.md](docs/superpowers/specs/2026-05-17-thinking-gap-driven-active-reasoning-design.md):
    repairs older goal/intention/reasoning language by defining active
    thinking as unresolved passive prediction gaps becoming cross-scale
    active closure pressure; "desire", "subconscious", "conscious", "goal",
    and "intention" are external metaphors only, not body ontology modules.

13. **Organic surface expression substrate (FROZEN CANDIDATE)** —
    [docs/superpowers/specs/2026-05-18-organic-surface-expression-substrate-design.md](docs/superpowers/specs/2026-05-18-organic-surface-expression-substrate-design.md):
    supersedes the older predictive visual surface for new pixel-expression
    work; defines a regulated organism-like observer surface driven by
    continuous numeric substrate, not a fixed expression dictionary, display
    controller, or body-side human-readable output path.

14. **Live Human Nursery v0 (FROZEN CANDIDATE; v0 evidence path implemented)** —
    [docs/superpowers/specs/2026-05-18-live-human-nursery-v0-design.md](docs/superpowers/specs/2026-05-18-live-human-nursery-v0-design.md):
    defines human teaching as an external live training room using camera,
    microphone, vocal output, pixel surface, snapshot evidence, and strict
    numeric acceptance gates; does not add a body-side teacher, chat layer,
    speech recognition, visual recognition, or fixed lesson-response script.

15. **Implementation plan (executed 2026-04-20)** —
    [docs/superpowers/plans/2026-04-20-bionic-v0-architecture-cleanup.md](docs/superpowers/plans/2026-04-20-bionic-v0-architecture-cleanup.md):
    the plan that removed the rejected layers from the live body /
    snapshot / observer / test path.

16. **Older v0 spec (PARTIALLY DEPRECATED)** —
    [docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md](docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md):
    read its §0 deprecation banner before trusting any of its content.

## Rejected Architecture (DO NOT REINTRODUCE)

The following are **rejected** by axioms + cleanup spec. They are not
returning. Do not re-add them under any name:

- `semantic_readiness` / `SemanticReadinessState` / `refresh_semantic_readiness`
- `symbolizer` / `SymbolizerState` / `refresh_symbolizer`
- `concept_id` / `ObjectConceptBinding` / `RelationConceptBinding`
- Any "naming/labeling resource gate" above the substrate
- Any module that maps "stable object/edge → discrete internal concept id"
- `eprintln!` / `println!` / human-readable output inside the `body`
  process (research observation belongs in the observer, not the body)

Quarantined contract tests for the rejected architecture live in
[bionic_v0/experimental/](bionic_v0/experimental/) for archaeological
reference only — do not reactivate.

## Subconscious/Conscious Boundary (Hard Rule)

- "潜意识" (`subconscious`) is an external design metaphor for body-side
  continuous execution (passive prediction, perception maintenance,
  low-level action/vocal execution, local closed-loop correction).
  It must not be implemented as an internal module/ontology named
  `Subconscious`.
- "表意识" (`conscious`) is an external design metaphor for active
  reasoning/active learning dispatch (active probes, hypothesis verification,
  learning correction, pronunciation/vocal correction, long-horizon style
  formation). It must not be implemented as a chat/request-response layer
  or a text-only reasoning engine.
- Language execution can run body-side after learned curves exist.
  Language learning/correction/style sedimentation belongs to active
  learning/reasoning loops.
- Style must remain learned and revisable curves, rhythms, selection biases,
  and correction history; never fixed persona labels or personality structs.
- Preserve the highest principle: silicon-life substrate, not
  request/response software; no human-readable output in `body`; observation
  belongs to `observer`.

## Learned Boundaries Only

When discussing initiative, exploration, imitation, self-adjustment, or
self-repair, agents must preserve this hard rule:

- Only learning laws, body laws, update constraints, and safety bounds may
  be predefined.
- Object ranges, target ranges, result boundaries, importance maps, and
  exploration scope must be treated as **learned and revisable**, not as
  fixed programmer-written world content.
- Do not introduce fixed fields or modules that pre-decide what the system
  is “really learning” in the world.
- If a design needs a boundary, it must explain how that boundary is formed,
  updated, drifted, and dissolved through exploration rather than assumed.
- Do not reduce curiosity, exploration timing, deferred exploration, or
  background resurfacing into a few programmer-controlled numbers at the
  theory layer. If implementation later uses numeric approximations, those
  are shadows of the theory, not the theory itself.

## Substrate That Must Be Preserved

These layers form the current core base and must not be removed by
incidental refactors:

- Perception ingress (`adapter`, `predictor`, `phot_predictor`,
  `phot_camera`, `clock`)
- Object slots, fusion, ambient channel, slot claim/reuse, spatial
- `viewport` shared substrate (attention / relations / relation
  predictor / reasoning / reasoning history / memory summary)
- `snapshot` as observation-side substrate
- `observer` as external research tool

## High-Risk Terms Require Boundary Notes

Per cleanup spec §8.3, any new design or code that introduces the words
`semantic`, `concept`, `token`, `label`, `name`, `word`, `text
understanding`, `identity`, `owner`, `ethics`, or `personality` must
explain, in writing:

- whether the term is being used as an external description or as an
  internal ontology unit,
- whether it is purely an engineering index,
- why it is not an ontological structure,
- why it does not violate the highest principle and the language axioms.

If those four points cannot be answered cleanly, the design is not
admissible.

For the correct substrate-level replacements when a high-risk term appears
in a theory-layer context, see the full terminology boundary checklist:
[docs/superpowers/specs/2026-04-22-terminology-boundary-checklist.md](docs/superpowers/specs/2026-04-22-terminology-boundary-checklist.md)
