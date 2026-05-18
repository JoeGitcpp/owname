# Life Principle Implementation Audit - 2026-05-18

## Scope

Reviewed the current `codex/product-cognition-baseline-training` implementation
for the Live Human Nursery v0 evidence path and Organic Surface Expression
substrate against the repository entrypoint, cleanup rules, language axioms,
body/observer separation, and high-risk terminology boundaries.

## Implemented Design Corrections

- Live Human Nursery spec status was updated from `DESIGN CANDIDATE` to
  `FROZEN CANDIDATE`.
- AGENTS first-read entry now records that the v0 evidence path is implemented.
- Live Human Nursery spec now distinguishes implemented v0 artifacts from later
  session metadata. V0 code adds evidence summary, report mode, source guards,
  and runbook only.
- Organic Surface implementation plan checkboxes now reflect the completed TDD
  and verification work.
- Nursery report now prints `deterministic_fixture` and `runtime_snapshot_read`
  so deterministic sample output cannot be mistaken for live evidence.
- Nursery evidence now treats `phot_health=FAULT` as an observable PHOT state
  via `phot_fault_observed`, instead of allowing camera failure to look silent.

## Findings

### No Current Principle Violation Found In New Nursery/Surface Code

- `live_human_nursery` is observer-side only. It reads `SnapshotData` and does
  not write to `Viewport`.
- `surface_projection` and `surface_render` remain observer-side projection and
  rasterization. They do not write back to the body.
- No rejected runtime source symbol was found in `bionic_v0/src` for:
  `semantic_readiness`, `SemanticReadinessState`, `refresh_semantic_readiness`,
  `symbolizer`, `SymbolizerState`, `refresh_symbolizer`, `concept_id`,
  `ObjectConceptBinding`, or `RelationConceptBinding`.
- Body-owned source still has no `println!` / `eprintln!` diagnostic path under
  the focused body-output test.
- Nursery and surface source guards cover fixed response, recognition,
  expression dictionary, and high-risk ontology terms in the new runtime paths.

### P1 Remaining Live-Readiness Blocker: Real Camera Provider

`bionic_v0/src/phot_camera.rs` still contains the intentionally faulted
`StubRealCameraFrameProvider`. The body can surface the failure numerically, so
this is not a silent-gap principle violation. It is still a live nursery
readiness blocker for camera-based teaching until the real PHOT provider is
available in the local environment.

### P2 Evidence Limitation: Runtime Report Reads One Snapshot

`curriculum_report --live-human-nursery runtime` reads `runtime/snapshot.bin`
once. That is acceptable for v0 preflight evidence, but it is not a full
session recorder. A later session runner should collect snapshot sequences and
preserve phase timing externally.

### P2 Manual Live Smoke Not Yet Performed

The deterministic gates are implemented and passing, but the plan keeps manual
live smoke unchecked. Do not claim live communication or stable learning until
observer-visible live evidence is recorded from a real runtime session.

### P3 Observer Diagnostic Output

`observer_surface` still uses an observer-side usage `eprintln!`. This is
allowed because the ban applies to the body process. It should remain outside
body-owned modules.

## Operating Rule Before Live Session

Treat `communication_candidate=1` as an early candidate only when
`deterministic_fixture=0`, `runtime_snapshot_read=1`, and the live run has
observer-visible write/return/closure evidence. A deterministic fixture report
is only a CI/sample proof of the evidence path.
