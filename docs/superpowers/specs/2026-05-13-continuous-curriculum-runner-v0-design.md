# Continuous Curriculum Runner v0 Design

Date: 2026-05-13
Status: draft implementation spec

## Purpose

Create a minimal external curriculum runner that repeatedly perturbs the
existing `Viewport` through legal substrate APIs and reports observer-side
numeric evidence. This is the next step after the spatial worldline and
action-consequence substrate work: prove that repeated courses can produce
measurable curve change before adding more internal machinery.

## Boundary

The runner is an external training harness, not a body-side ontology module.
It must not run inside `body`, must not print from body-owned code, and must
not write object truth directly into snapshots. It may call existing viewport
APIs that adapters already use: slot allocation, spatial writes, relation
refreshes, action-output events, consequence accumulation, pairing, vocal
target observation, life-closure production, and snapshot writing.

High-risk terms used here:

- `training` and `curriculum` are external harness descriptions, not internal
  body ontology units.
- They are not names, labels, concepts, identities, source owners, or semantic
  readiness gates.
- Course kinds are engineering entrypoints for synthetic perturbation streams.
  They do not define what the system is allowed to know.
- The highest principle is preserved because the body substrate remains
  continuous; the harness only supplies repeated external perturbations and
  reads observer-visible evidence.

## v0 Courses

v0 should include three small courses:

1. Single-object motion course
   - Allocates one slot.
   - Writes a bounded spatial curve over shared time.
   - Refreshes attention/relations where needed.
   - Reports worldline update count and prediction error.

2. Two-object relation course
   - Allocates two slots.
   - Seeds each with low-level PHOT/VIB band data through existing slot APIs.
   - Moves one object closer to the other.
   - Reports distance/overlap relation projection change.

3. Action-consequence pulse course
   - Emits a write epoch through action-output.
   - Accumulates a consequence window with matching magnitude.
   - Auto-pairs and ticks life-closure producers.
   - Reports formed pairs and learned-bias summary.

Optional snapshot writing is observer-side evidence only. A caller may provide
`SnapshotPaths`; otherwise the runner returns metrics without filesystem
output.

## Non-Goals

- No token, word, label, concept, semantic, identity, owner, ethics, or
  personality fields.
- No request/response loop.
- No optimizer, model weights, or offline dataset format.
- No fixed claim about what the system is "really" learning.
- No replacement for adapters or live I/O.

## Success Criteria

- Tests show a single-object course produces a valid spatial worldline with
  bounded prediction error.
- Tests show a two-object course increases spatial relation projection when
  objects move closer.
- Tests show an action-consequence course forms at least one pair and produces
  learned-bias evidence.
- Source hygiene tests ensure the runner stays an external numeric harness and
  does not introduce rejected architecture terms as body fields.
