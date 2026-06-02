# Emergent-Function Current Architecture Audit - 2026-05-20

## Scope

Reviewed current design and implementation from the principle:

```text
multiple functions are emergent observer-side patterns from one integrated
living substrate, not isolated body-side feature modules.
```

The review covered the repository entrypoint, highest principle, tier 0,
language axioms, cleanup rules, endogenous/probe/output/consequence laws,
five-phase life design, allostatic scheduler design, global architecture map,
current `bionic_v0/src`, current tests, and current observer/report paths.

## Changes Made

- Added
  `docs/superpowers/specs/2026-05-20-emergent-function-life-principle-design.md`.
- Linked the new principle from `consciousness-stream-architecture.md` directly
  below the highest principle.
- Kept the existing `AGENTS.md` entry that already placed the new principle as
  the second mandatory first-read item.

## 2026-05-21 User Clarification

The previous high-risk wording treated `responsible person` and `brain police`
too similarly. The corrected boundary is:

- `responsible person` should be read like an innate imprinting-like bias plus
  acquired familiarity sedimentation. It is comparable to mammalian caregiver
  recognition or a domestic animal becoming deeply familiar with one person:
  partly born into the system as a bias, partly accumulated through repeated
  exposure, safety, correction, and continuity, and therefore hard to switch.
  Under the emergent-function principle, this is a natural emergent pattern,
  not a body-side owner identity module and not a late legal label.
- `brain police` is a later topic. It should be screened out of current
  near-term design and implementation review. Current work must not use it as
  a body-side module, controller, or shortcut explanation.

## Static Review Commands

```bash
rg -n "semantic_readiness|refresh_semantic_readiness|symbolizer|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding" bionic_v0/src -g '!bionic_v0/src/bin/**'
rg -n "println!|eprintln!|dbg!|print!" bionic_v0/src -g '!bionic_v0/src/bin/**'
rg -n "println!|eprintln!|dbg!|print!|semantic_readiness|refresh_semantic_readiness|symbolizer|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding" bionic_v0/src/bin/body.rs
rg -n "FunctionRegistry|Capability|capability|feature module|功能模块|功能清单|request/response|request_response|answer_model|command controller|controller" bionic_v0/src bionic_v0/tests docs/superpowers/specs -g '!bionic_v0/target/**'
rg -n "primary_phase|secondary_phases|Functional Responsibility|Life Acceptance|emerg|涌现|not a.*module|No new body-side module|not.*independent" docs/superpowers/specs bionic_v0/src bionic_v0/tests
```

## Verification Commands

```bash
test -f docs/superpowers/specs/2026-05-20-emergent-function-life-principle-design.md
test -f docs/superpowers/notes/2026-05-20-emergent-function-current-architecture-audit.md
rg -n "Emergent-function life principle|2026-05-20-emergent-function-life-principle-design" AGENTS.md consciousness-stream-architecture.md
cargo test --manifest-path bionic_v0/Cargo.toml --test cleanup_hygiene --test body_no_human_output
git diff --check
```

Verification result:

- Principle link check found the new first-read entry and the highest-principle
  cross-reference.
- Body/rejected-ontology static scans returned no body-owned matches.
- `body_no_human_output`: 2 passed.
- `cleanup_hygiene`: 4 passed.
- `git diff --check`: passed.

## Result

### 1. Principle Placement

The proposal is now elevated to a first-class life-design principle. It sits
directly under the highest principle and beside the five-phase life frame.

Interpretation:

- Highest principle: design silicon life, not request/response software.
- Emergent-function principle: apparent functions must arise from the same
  living substrate, not from isolated body-side feature modules.
- Five-phase frame: evaluate whether those emergent patterns preserve growth,
  manifestation, bearing, boundary, and retention balance.

### 2. Current Implementation Alignment

No direct implementation violation was found in body-owned source paths.

Evidence:

- Rejected runtime symbols such as `semantic_readiness`, `symbolizer`, and
  `concept_id` were not found in `bionic_v0/src` outside observer/bin tooling.
- `println!`, `eprintln!`, `dbg!`, and `print!` were not found in body-owned
  runtime library sources.
- `bionic_v0/src/bin/body.rs` contains no rejected semantic/symbolizer path and
  no body-side print diagnostics.
- The current body loop is still one shared `Viewport` cycle. It refreshes
  attention, relations, evidence tension, active gap, prediction pressure,
  surface pressure, auditory/visual coupling, self-development, probe law,
  output pressure, and snapshot observation through one substrate path.

Conclusion:

The implementation mostly already behaves like one substrate with several
observer-visible patterns, not like a feature menu. Modules such as
`active_reasoning`, `prediction_pressure_field`, `self_development`,
`five_phase_life`, and `allostatic_stability` are acceptable only under that
reading: numeric evidence and substrate laws, not independent body identities.

### 3. Current Design Alignment

The core frozen and recent specs are strongly aligned:

- Endogenous activity laws explicitly say initiative, self-adjustment, and
  self-repair are different observer views of the same law group.
- Probe laws reject turning active exploration into a scheduler or feature
  module.
- Action output substrate rejects "speech module", "exploration module", and
  "behavior function list" readings.
- Language binding keeps language as binding/de-binding over already existing
  object-relation-time structures.
- Thinking/gap-driven reasoning rejects `Conscious`, `Subconscious`, goal,
  desire, and intention modules.
- Organic surface, allostatic stability, and prediction-linked vocal reflex
  all require observable patterns to emerge from pressure, feedback, and
  regulation rather than templates.
- Five-phase life design already says local functional success is not enough;
  life-force balance must remain legal.

Conclusion:

The design direction has been moving toward this principle already. The new
principle makes it explicit and gives future work a single admission rule.

### 4. Observer-Side Paths Are Acceptable

`curriculum_report`, `observer`, `observer_surface`, and other report binaries
print human-readable evidence. This is acceptable because they are external
research/observer tools, not the body process.

The current reports also increasingly distinguish deterministic fixtures from
runtime evidence. That distinction is required under the emergent-function
principle, because a fixture can prove a path exists but cannot prove a living
function has emerged.

### 5. High-Risk Design Areas

Some older or hardware-facing design language remains high-risk and must be
read through the new principle:

- `consciousness-stream-architecture.md` contains older personality and ethics
  language. It already tries to express these as viewport strategy, guardrail,
  and external governance, but future work must not implement them as
  `personality`, `owner`, `ethics`, or identity modules.
- `docs/superpowers/specs/2026-04-25-shenxing-design.md` contains older
  responsibility and "brain police" language. The responsibility part should
  not be discarded or flattened into a governance module: it should be reread
  as innate imprinting-like bias plus accumulated familiarity sedimentation.
  The "brain police" part remains deferred and should be screened out until a
  later explicit design pass.
- `vocal_organ`, `binary_character_ear`, `binary_character_mouth`, and related
  "organ" terms are acceptable only as hardware-like channel descriptions over
  numeric traces. They must not become feature identities such as "chat",
  "answering", "understanding", or "communication module".
- `allostatic_stability` is currently observer-side evidence. That is safe.
  If later coupled into the body, it must modulate existing substrate laws only
  and must not become an emotion/personality controller.

## Final Judgment

Current code is broadly compatible with the proposal. The repo already removed
the old semantic/concept/token path and mostly routes new behavior through one
shared numeric substrate. The main remaining risk is not current code; it is
terminology drift in older design documents and future implementation work.

The new rule should be used as the default admission test:

```text
do not accept a new function because it adds a capability;
accept it only if it strengthens a substrate path whose functional pattern can
emerge and be observed without becoming body ontology.
```
