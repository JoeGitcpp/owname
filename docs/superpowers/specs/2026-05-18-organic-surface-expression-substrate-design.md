# Organic Surface Expression Substrate v0 Design

> **Status:** FROZEN CANDIDATE
> **Supersedes for new work:** [2026-04-23-predictive-visual-surface-design.md](./2026-04-23-predictive-visual-surface-design.md)
> **Purpose:** turn the existing pixel surface from an object/relationship
> visualizer into a regulated, organism-like external expression surface without
> turning it into a request/response output program or a body-side display
> controller.

## Boundary

This design keeps the highest principle intact: the system is a silicon-life
substrate, not a request/response program. Research observation and the AI body
remain separated.

`organic`, `organ`, `face-like`, `expression`, `skin`, and `brain activity` are
external design descriptions. They are not internal ontology units, not fixed
world content, and not evidence that the body contains a display-specific mind
module.

High-risk terms in this document are used only as boundaries:

- `semantic`, `concept`, `token`, `label`, `name`, `word`, `identity`,
  `personality`, and `ethics` are forbidden as surface-expression internals.
- If any of those terms appear, they describe rejected routes or external
  human language only.
- The implementation may store engineering indices such as slot id, edge id,
  trace index, or pixel coordinate.
- Those indices do not decide what the body knows, what an object is called, or
  what a visual pattern means.

The pixel surface may look like a primitive organ from the outside. Internally it
must remain continuous numeric substrate:

- pressure
- tension
- coherence
- ambiguity
- activation
- aperture
- residue
- closure
- reentry
- regulation gain

The implementation must not add emotion dictionaries, expression labels, mood
states, fixed facial expressions, output scripts, text panels, or any body-side
human-readable diagnostics.

## Why The 2026-04-23 Surface Is Deprecated

The older predictive visual surface design correctly established an
observer-side pixel projection, but its center of gravity was object display:

```text
snapshot object/space/relation fields
-> coarse visual surface
-> human observer
```

That is useful, but it is not enough for the next goal. A silicon-life surface
should not feel like a debug window that draws objects. It should feel like an
external body surface that continuously reacts to the substrate's current state,
while remaining regulated and non-authoritative.

The new authority is:

```text
snapshot numeric body state
-> surface expression drive
-> regulated pixel skin rendering
-> human observer
```

The old design remains a valid implementation ancestor for projection, history,
and raster rendering. It is no longer the design authority for new
surface-expression behavior.

## Display-Reversed Model

The display gives us a physical constraint: pixels have position, color,
brightness, and time. Pixel groups have shape, relation, movement, residue, and
rhythm. The design should use those physical dimensions directly instead of
pretending the display is a text output device.

The v0 mapping is:

| Display dimension | Legal substrate source | External appearance |
| --- | --- | --- |
| Brightness | regulated activation drive | liveliness / arousal of the surface |
| Red channel pressure | prediction error, unresolved evidence tension | sharp disturbance |
| Green channel pressure | closure support, coherence, relation support | settling / integration |
| Blue channel pressure | attention, aperture, vocal/body output drive | active focusing / expression |
| Saturation | coherence vs ambiguity | clear vs conflicted reaction |
| Flicker / pulse | active gap pressure and probe output intensity | unresolved closure pressure |
| Shape contraction | routing stability and focus persistence | gathered attention |
| Shape diffusion | network instability and ambiguity | dispersed tension |
| Residue / afterglow | history, reentry, persistence | memory-like trace, not memory truth |

These are not a color dictionary. No single color means a fixed state. Every
visible result is a blend of numeric curves.

## v0 Architecture

V0 adds no new body ontology and no display-specific write channel. It extends
the existing observer-side surface pipeline with one projection artifact:

```text
SnapshotData
-> SurfaceFrame
   -> objects
   -> relations
   -> organ_drive
-> SurfaceHistory
-> render_surface_frame(...)
```

### `SurfaceOrganDrive`

`surface_projection.rs` adds a numeric drive structure:

- `activation_drive`
- `regulated_activation`
- `tension_drive`
- `coherence_drive`
- `ambiguity_drive`
- `aperture_drive`
- `residue_drive`
- `closure_drive`
- `reentry_drive`
- `vocal_drive`
- `regulation_gain`

The projection derives these fields only from existing snapshot-visible
substrate:

- `global_reasoning.global_surprise_load`
- `global_reasoning.network_instability_score`
- `global_reasoning_memory.novelty_score`
- `global_reasoning_memory.persistence_score`
- `global_reasoning_memory.routing_stability_score`
- `global_reasoning_memory.pattern_recurrence_score`
- `global_reasoning_memory.dominant_focus_persistence`
- `active_reasoning_gap.gap_pressure`
- `active_reasoning_gap.closure_score`
- `active_reasoning_gap.reentry_score`
- `evidence_tension.unresolved_pressure`
- `probe_law.intensity`
- `probe_law.output_intensity`
- `probe_law.suppression`
- `vocal_target.active_energy`
- `vocal_organ.breath_drive`
- `framed_byte_egress.output_pressure`
- action-output channel magnitudes

All derived values are clamped to `[0, 1]`.

### Regulation

The surface must be expressive but not uncontrolled. Regulation is not a fake
emotion suppressor and not a policy override. It is a numeric gain cap applied
before pixel rendering:

```text
raw activation + tension + output pressure
-> regulation gain
-> regulated activation
-> bounded brightness and pulse strength
```

The first version derives `regulation_gain` from existing body-side suppression
and closure support:

- higher `probe_law.suppression` lowers visible surge;
- higher closure/coherence lowers unresolved flicker;
- sustained reentry can remain visible as residue without forcing full
  brightness.

Regulation must preserve evidence: it compresses amplitude, not truth. The
observer should still see that pressure exists, but the surface should not
blindly saturate into a white or flashing output.

### Pixel Skin Rendering

`surface_render.rs` uses `organ_drive` as a background body layer below trails,
relations, and object blobs.

The renderer may draw:

- a central low-resolution aperture glow;
- bounded side gradients caused by tension and ambiguity;
- a soft pulse ring caused by active gap and output pressure;
- residual afterglow caused by persistence and reentry;
- relation/object overlays from the existing surface pipeline.

The renderer must not draw:

- text;
- icons that imply fixed states;
- named emotions;
- face parts with fixed meanings;
- hard-coded "if state X, draw expression Y" branches.

The target effect is primitive organismic reaction, not a programmed animation.

## Acceptance Evidence

The implementation is accepted when tests prove:

1. A snapshot with active pressure but no objects produces nonzero `organ_drive`
   and a nonblank pixel field.
2. Regulation caps brightness: a highly pressured snapshot must not saturate the
   whole surface.
3. Closure/coherence changes the drive differently from unresolved tension.
4. Reentry/persistence can produce residue without requiring live object slots.
5. Existing object and relation rendering still works.
6. Source guard tests reject forbidden ontology terms and fixed expression
   dictionaries.

The observer may print or render numeric evidence. The body must not print
human-readable output.

## Non-Goals

This v0 does not implement:

- a body-side display organ;
- a full face;
- emotional expression classification;
- user-controllable manual expression commands;
- text communication through pixels;
- image recognition;
- a new snapshot schema.

Those may be discussed later only if they preserve the same boundary: continuous
curves first, observer rendering second, no naming gate, no internal fixed
state dictionary.

## TDD Entry Point

Implementation begins with tests in:

- `bionic_v0/tests/surface_projection.rs`
- `bionic_v0/tests/surface_render.rs`

The first failing tests must require `SurfaceFrame::organ_drive` and prove the
renderer uses it. No production implementation is admissible before those tests
fail for the expected missing-field/missing-rendering reasons.
