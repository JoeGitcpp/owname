# 2026-04-29 Subconscious/Conscious Language Boundary Design (FROZEN)

Status: FROZEN
Scope: theory + architecture boundary for `bionic_v0` and successors
Audience: contributors editing body-side execution, active learning/reasoning dispatch, language runtime/learning paths

## 0. Purpose

This spec fixes a recurring drift: turning "潜意识/表意识" into ontology
modules or chat-style software layers.

This repository allows those terms only as external design metaphors.
Internal implementation must remain substrate-based and law-based.

## 1. Highest-Principle Alignment

- The system is a silicon-life substrate, not a request/response program.
- Body and observer are separate roles.
- The body must not emit human-readable research output.
- Observation, explanation, and analysis stay in observer-side tools.

Nothing in this spec authorizes a return to request/response language AI
architecture.

## 2. Boundary Definitions (Metaphor Only)

### 2.1 "潜意识" (`subconscious`)

Allowed meaning (external description only):

- body-side continuous execution:
  - passive prediction
  - perception maintenance
  - low-level action / vocal execution
  - local closed-loop correction

Forbidden interpretation:

- a first-class ontology/module/class named `Subconscious`
- a manager/router that owns "all passive intelligence"
- a semantic identity partition with fixed responsibilities by name

### 2.2 "表意识" (`conscious`)

Allowed meaning (external description only):

- active reasoning / active learning dispatch:
  - active probes
  - hypothesis verification
  - learning correction
  - pronunciation/vocal correction
  - longer-term style formation

Forbidden interpretation:

- chat/request-response controller
- text-only reasoning engine detached from substrate updates
- module that treats "consciousness" as static internal entity

## 3. Language Boundary

### 3.1 Body-Side Language Execution

Admissible:

- body-side language execution after learned curves exist
- low-latency output control using learned and revisable curves
- local runtime correction loops that do not reintroduce naming ontology

### 3.2 Active Learning/Reasoning Side

Admissible:

- language learning and correction dispatch
- style sedimentation through repeated correction history
- long-horizon adjustment of output tendencies via learned updates

Forbidden:

- fixed persona/personality labels as the style substrate
- immutable style presets treated as identity essence
- collapsing learning into a static token/text ontology

## 4. Style Representation Constraint

Style is represented only as learned, revisable substrate dynamics, such as:

- curves and curve families
- rhythms and temporal tendencies
- selection biases
- correction history and its update traces

Style is not represented as personality tags, role cards, or fixed persona
structs.

## 5. Admissible vs Forbidden Implementation Moves

### 5.1 Admissible

- adding/updating laws, constraints, and learning update rules that keep
  boundaries revisable
- extending body-side continuous execution paths without naming gates
- extending active probe and correction dispatch paths without chat wrappers
- adding observer-side diagnostics to study behavior

### 5.2 Forbidden

- adding `Subconscious*` or `Conscious*` modules as internal ontology units
- adding request/response language controller as "conscious layer"
- adding fixed semantic/personality ownership fields for style
- moving research explanation output into body process

## 6. High-Risk Term Boundary Notes (Mandatory)

For terms such as `subconscious`, `conscious`, `style`, `personality`,
`identity`, `semantic`, `concept`, `label`, `name`, `token`:

1. External description vs internal ontology:
   these terms are external descriptions in this spec, not internal ontology
   units.
2. Engineering index use:
   if an index exists, it is an implementation locator only (for routing,
   storage, or retrieval), not a world-meaning object.
3. Why not ontological:
   no discrete entity is introduced that claims intrinsic world meaning.
   Behavior remains in learned/revisable curves and law-governed updates.
4. Why no highest-principle/axiom violation:
   no request/response fallback is introduced, no naming gate is introduced,
   and body/observer separation remains intact.

Any proposal using these terms without all four clarifications is
non-admissible.

## 7. Admission Checklist for New Design/Code

Before accepting new modules/fields/tests/docs:

- Do they avoid introducing `Subconscious` / `Conscious` ontology modules?
- Do they keep language execution vs language learning boundaries from §3?
- Do they represent style as revisable dynamics, not fixed persona labels?
- Do they preserve no-human-readable-output-in-body and observer separation?
- Do they include §6 boundary notes when high-risk terms appear?

If any answer is "no", reject or rewrite before merge.
