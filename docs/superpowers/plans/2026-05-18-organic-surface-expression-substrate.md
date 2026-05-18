# Organic Surface Expression Substrate Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing observer-side pixel surface with a regulated organism-like expression drive derived from snapshot numeric substrate.

**Architecture:** Keep the body/observer boundary intact. `surface_projection.rs` derives a bounded `SurfaceOrganDrive` from existing `SnapshotData`; `surface_render.rs` renders that drive as a low-resolution pixel-skin layer under existing object, relation, and trail rendering. No snapshot schema, body output channel, text output, emotion dictionary, fixed expression state, semantic gate, concept allocator, token path, or naming layer is added.

**Tech Stack:** Rust, Cargo, existing `bionic_v0` snapshot/surface modules, TDD

---

## File Structure

- Modify: `/Users/joe/unnamed/docs/superpowers/specs/2026-04-23-predictive-visual-surface-design.md`
- Create: `/Users/joe/unnamed/docs/superpowers/specs/2026-05-18-organic-surface-expression-substrate-design.md`
- Modify: `/Users/joe/unnamed/docs/superpowers/plans/2026-04-23-predictive-visual-surface.md`
- Modify: `/Users/joe/unnamed/docs/superpowers/plans/2026-04-23-surface-endogenous-states-visualization-plan.md`
- Create: `/Users/joe/unnamed/docs/superpowers/plans/2026-05-18-organic-surface-expression-substrate.md`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_projection.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_render.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/observer_surface.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_projection.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_render.rs`

## Chunk 1: Documentation Boundary

### Task 1: Freeze the new design authority

**Files:**
- Create: `/Users/joe/unnamed/docs/superpowers/specs/2026-05-18-organic-surface-expression-substrate-design.md`
- Modify: `/Users/joe/unnamed/docs/superpowers/specs/2026-04-23-predictive-visual-surface-design.md`
- Modify: `/Users/joe/unnamed/docs/superpowers/plans/2026-04-23-predictive-visual-surface.md`
- Modify: `/Users/joe/unnamed/docs/superpowers/plans/2026-04-23-surface-endogenous-states-visualization-plan.md`

- [x] **Step 1: Add new spec**

Write the organic surface expression substrate spec with:

- boundary notes for organic / organ / face-like / expression terms;
- explicit bans on fixed expression dictionaries and rejected ontology;
- display-reversed mapping from pixels to numeric substrate;
- `SurfaceOrganDrive` architecture;
- acceptance evidence and TDD entry points.

- [x] **Step 2: Mark older surface documents superseded**

Add deprecation banners to:

- `docs/superpowers/specs/2026-04-23-predictive-visual-surface-design.md`
- `docs/superpowers/plans/2026-04-23-predictive-visual-surface.md`
- `docs/superpowers/plans/2026-04-23-surface-endogenous-states-visualization-plan.md`

## Chunk 2: Projection Contract

### Task 2: Add the organ-drive projection contract

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_projection.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_projection.rs`

- [x] **Step 1: Write failing test for pressure-derived organ drive**

Add a test that sets existing snapshot pressure fields:

```rust
#[test]
fn projection_derives_regulated_organ_drive_from_body_pressure() {
    let mut snap = make_empty_snapshot();
    snap.global_reasoning.global_surprise_load = 0.7;
    snap.global_reasoning.network_instability_score = 0.8;
    snap.global_reasoning_memory.novelty_score = 0.6;
    snap.active_reasoning_gap.gap_pressure = 0.9;
    snap.evidence_tension.unresolved_pressure = 0.8;
    snap.probe_law.intensity = 0.7;
    snap.probe_law.output_intensity = 0.6;
    snap.vocal_target.active_energy = 0.5;
    snap.vocal_organ.breath_drive = 0.4;
    snap.framed_byte_egress.output_pressure = 0.55;

    let frame = project_surface_frame(&snap);
    let drive = frame.organ_drive;

    assert!(drive.activation_drive > 0.0);
    assert!(drive.tension_drive > 0.0);
    assert!(drive.aperture_drive > 0.0);
    assert!(drive.vocal_drive > 0.0);
    assert!(drive.regulated_activation <= drive.activation_drive);
    assert!((0.0..=1.0).contains(&drive.regulation_gain));
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test surface_projection projection_derives_regulated_organ_drive_from_body_pressure
```

Expected: FAIL because `SurfaceFrame` has no `organ_drive` field.

- [x] **Step 3: Implement minimal projection**

In `surface_projection.rs`:

- add `SurfaceOrganDrive`;
- add `organ_drive: SurfaceOrganDrive` to `SurfaceFrame`;
- derive bounded numeric values from existing `SnapshotData`;
- use no new body state and no fixed expression labels.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test surface_projection projection_derives_regulated_organ_drive_from_body_pressure
```

Expected: PASS.

### Task 3: Add regulation and residue contract

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_projection.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_projection.rs`

- [x] **Step 1: Write failing regulation/residue test**

```rust
#[test]
fn projection_keeps_reentry_residue_visible_while_regulating_surge() {
    let mut snap = make_empty_snapshot();
    snap.global_reasoning.global_surprise_load = 1.0;
    snap.global_reasoning.network_instability_score = 1.0;
    snap.global_reasoning_memory.persistence_score = 0.8;
    snap.global_reasoning_memory.pattern_recurrence_score = 0.7;
    snap.active_reasoning_gap.reentry_score = 0.65;
    snap.active_reasoning_gap.closure_score = 0.5;
    snap.probe_law.suppression = 0.75;

    let drive = project_surface_frame(&snap).organ_drive;

    assert!(drive.residue_drive > 0.0);
    assert!(drive.reentry_drive > 0.0);
    assert!(drive.closure_drive > 0.0);
    assert!(drive.regulation_gain < 1.0);
    assert!(drive.regulated_activation < drive.activation_drive);
}
```

- [x] **Step 2: Verify RED or expected failure**

Run:

```bash
cargo test --test surface_projection projection_keeps_reentry_residue_visible_while_regulating_surge
```

Expected: FAIL until regulation/residue fields exist.

- [x] **Step 3: Implement minimal regulation/residue derivation**

Use numeric-only rules:

- `residue_drive = max(persistence_score, pattern_recurrence_score, dominant_focus_persistence)`;
- `reentry_drive = active_reasoning_gap.reentry_score`;
- `closure_drive = active_reasoning_gap.closure_score`;
- `regulation_gain` decreases with `probe_law.suppression`, unresolved tension, and closure compression;
- `regulated_activation = activation_drive * regulation_gain`.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test surface_projection projection_keeps_reentry_residue_visible_while_regulating_surge
```

Expected: PASS.

## Chunk 3: Pixel Skin Rendering

### Task 4: Render organ drive without objects

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_render.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_render.rs`

- [x] **Step 1: Write failing render test**

Add a frame with no objects and active `organ_drive`:

```rust
#[test]
fn renderer_draws_organ_skin_from_drive_without_objects() {
    let frame = SurfaceFrame {
        objects: Vec::new(),
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
        organ_drive: active_organ_drive(),
    };
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    assert!(count_lit_pixels(&img) > 0);
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --test surface_render renderer_draws_organ_skin_from_drive_without_objects
```

Expected: FAIL because renderer ignores `organ_drive` or the field is missing.

- [x] **Step 3: Implement background organ layer**

In `surface_render.rs`:

- call `apply_organ_skin_background` after global background and before trails;
- draw bounded central glow from `regulated_activation`;
- add diffuse side/tension color from `tension_drive` and `ambiguity_drive`;
- add low-intensity residue from `residue_drive` and `reentry_drive`;
- keep all values clamped.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test surface_render renderer_draws_organ_skin_from_drive_without_objects
```

Expected: PASS.

### Task 5: Prove brightness regulation

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_render.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/surface_render.rs`

- [x] **Step 1: Write failing regulation test**

```rust
#[test]
fn renderer_caps_organ_skin_brightness_under_high_pressure() {
    let mut drive = active_organ_drive();
    drive.activation_drive = 1.0;
    drive.regulated_activation = 0.35;
    drive.tension_drive = 1.0;
    drive.ambiguity_drive = 1.0;

    let frame = frame_with_organ_drive(drive);
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    let saturated = img
        .iter()
        .filter(|px| px[0] > 245 || px[1] > 245 || px[2] > 245)
        .count();
    assert_eq!(saturated, 0);
}
```

- [x] **Step 2: Verify RED or expected failure**

Run:

```bash
cargo test --test surface_render renderer_caps_organ_skin_brightness_under_high_pressure
```

Expected: FAIL until the organ layer respects `regulated_activation`.

- [x] **Step 3: Implement cap**

Ensure organ rendering uses `regulated_activation`, not raw activation, and
keeps maximum channel contribution well below full saturation.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test --test surface_render renderer_caps_organ_skin_brightness_under_high_pressure
```

Expected: PASS.

## Chunk 4: Boundary Hygiene and Verification

### Task 6: Add source guard for the surface expression layer

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/tests/surface_render.rs`

- [x] **Step 1: Write source guard test**

Add a test that scans `surface_projection.rs` and `surface_render.rs` for
forbidden surface-expression ontology/dictionary terms:

```rust
#[test]
fn surface_expression_sources_avoid_fixed_expression_ontology() {
    let sources = [
        include_str!("../src/surface_projection.rs"),
        include_str!("../src/surface_render.rs"),
    ];
    let forbidden = [
        "happy", "sad", "angry", "emotion", "mood", "personality",
        "face_expression", "semantic", "concept", "token", "label",
        "word", "identity",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "surface expression source contains forbidden fixed ontology term: {term}"
            );
        }
    }
}
```

- [x] **Step 2: Run full surface tests**

Run:

```bash
cargo test --test surface_projection --test surface_render
```

Expected: PASS.

- [x] **Step 3: Run formatting and targeted report tests**

Run:

```bash
cargo fmt --check
git diff --check
cargo test --test observer_compact
```

Expected: PASS.

- [x] **Step 4: Record final verification**

Run:

```bash
git status --short --branch
```

Expected: branch remains `codex/product-cognition-baseline-training`; no
unexpected generated files.
