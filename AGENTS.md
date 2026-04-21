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

7. **Implementation plan (executed 2026-04-20)** —
   [docs/superpowers/plans/2026-04-20-bionic-v0-architecture-cleanup.md](docs/superpowers/plans/2026-04-20-bionic-v0-architecture-cleanup.md):
   the plan that removed the rejected layers from the live body /
   snapshot / observer / test path.

8. **Older v0 spec (PARTIALLY DEPRECATED)** —
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
