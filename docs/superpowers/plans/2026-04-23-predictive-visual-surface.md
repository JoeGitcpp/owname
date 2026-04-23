# Predictive Visual Surface Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the first observer-side native visual surface for `bionic_v0` so current object slots, spatial extent, focus bias, and short trajectory history render as a coarse real-time pixel field without changing `body` responsibilities or adding any body-side display channel.

**Architecture:** Add one new observer-side binary, `observer_surface`, plus three focused support modules: projection, short history, and raster rendering. The implementation must stay snapshot-truth-only: read existing `SnapshotData`, derive an external `SurfaceFrame`, keep all smoothing/trail memory inside the observer process, and render through a low-resolution native window. `body`, `snapshot.bin` schema, and current text observer remain unchanged in this phase.

**Tech Stack:** Rust, Cargo, `winit`, `pixels`, Markdown docs

---

## File Structure

- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/surface_projection.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/surface_history.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/surface_render.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer_surface.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_projection.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_history.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_render.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`

## Chunk 1: Projection Contract

### Task 1: Declare the observer-side projection surface in the crate

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_projection.rs`

- [ ] **Step 1: Write the failing projection contract test**

Add tests that describe the first legal projection surface:

```rust
use bionic_v0::surface_projection::{project_surface_frame, SurfaceObjectGlyph};
use bionic_v0::snapshot::{SnapshotData, SnapshotSlot, SnapshotSpatial};

#[test]
fn projection_uses_active_slot_spatial_as_object_center_and_extent() {
    let snap = make_snapshot_with_one_slot(0.25, 0.75, 0.20, 0.10, 0.6, true);
    let frame = project_surface_frame(&snap);

    assert_eq!(frame.objects.len(), 1);
    let g = &frame.objects[0];
    assert!(g.center_x > 0.20 && g.center_x < 0.30);
    assert!(g.center_y > 0.70 && g.center_y < 0.80);
    assert!(g.radius_x > 0.0);
    assert!(g.radius_y > 0.0);
}
```

- [ ] **Step 2: Run the projection test to verify RED**

Run:

```bash
cargo test --test surface_projection projection_uses_active_slot_spatial_as_object_center_and_extent
```

Expected:

```text
FAIL with unresolved import or missing function/type in surface_projection
```

- [ ] **Step 3: Add one more failing test for selection behavior**

Write a second test that proves projection does not render dead or spatially invalid slots:

```rust
#[test]
fn projection_ignores_inactive_or_invalid_slots() {
    let snap = make_snapshot_with_mixed_slots();
    let frame = project_surface_frame(&snap);

    assert_eq!(frame.objects.len(), 1);
}
```

- [ ] **Step 4: Run both projection tests and verify RED**

Run:

```bash
cargo test --test surface_projection
```

Expected:

```text
FAIL with missing surface_projection module/items
```

- [ ] **Step 5: Implement the minimal projection module**

Create `surface_projection.rs` with:

- `SurfaceFrame`
- `SurfaceObjectGlyph`
- `project_surface_frame(&SnapshotData) -> SurfaceFrame`

Minimal rules:

- only active slots with `spatial.valid == true` become glyphs
- `pos_x/pos_y` map to normalized center
- `span_x/span_y` map to normalized extents
- `attention_share`, `precision_bias`, and `high_layer` fields are copied as numeric rendering hints only

- [ ] **Step 6: Export the module from the crate**

Add:

```rust
pub mod surface_projection;
```

to `src/lib.rs`.

- [ ] **Step 7: Run the projection tests and verify GREEN**

Run:

```bash
cargo test --test surface_projection
```

Expected:

```text
PASS with 2 passed; 0 failed
```

- [ ] **Step 8: Commit the projection contract**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/lib.rs \
        /Users/joe/Documents/unnamed/bionic_v0/src/surface_projection.rs \
        /Users/joe/Documents/unnamed/bionic_v0/tests/surface_projection.rs
git commit -m "feat: add predictive visual surface projection contract"
```

## Chunk 2: Observer-Side Short History

### Task 2: Add short-lived trajectory memory outside the body

**Files:**
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/surface_history.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_history.rs`

- [ ] **Step 1: Write the failing history test for trail accumulation**

```rust
use bionic_v0::surface_history::SurfaceHistory;
use bionic_v0::surface_projection::{SurfaceFrame, SurfaceObjectGlyph};

#[test]
fn history_accumulates_recent_positions_into_trail_points() {
    let mut hist = SurfaceHistory::new(8);
    hist.update(&frame_at(0.2, 0.3, 1));
    hist.update(&frame_at(0.3, 0.35, 2));

    let trails = hist.render_trails();
    assert_eq!(trails.len(), 1);
    assert!(trails[0].points.len() >= 2);
}
```

- [ ] **Step 2: Run the history test to verify RED**

Run:

```bash
cargo test --test surface_history history_accumulates_recent_positions_into_trail_points
```

Expected:

```text
FAIL with missing surface_history module/items
```

- [ ] **Step 3: Add the failing decay/eviction test**

```rust
#[test]
fn history_eviction_drops_stale_points_after_capacity_limit() {
    let mut hist = SurfaceHistory::new(2);
    hist.update(&frame_at(0.1, 0.1, 1));
    hist.update(&frame_at(0.2, 0.2, 2));
    hist.update(&frame_at(0.3, 0.3, 3));

    let trails = hist.render_trails();
    assert_eq!(trails[0].points.len(), 2);
}
```

- [ ] **Step 4: Run the history tests and verify RED**

Run:

```bash
cargo test --test surface_history
```

Expected:

```text
FAIL with missing history implementation
```

- [ ] **Step 5: Implement the minimal observer-side history**

Create `surface_history.rs` with:

- `SurfaceHistory`
- one per-object ring of recent projected positions
- bounded history capacity
- conversion into trail segments with descending external alpha weights

Rules:

- history is keyed by slot identity only for observer rendering
- history never writes back to snapshot/body
- stale entries decay and disappear entirely on the observer side

- [ ] **Step 6: Export the module from the crate**

Add:

```rust
pub mod surface_history;
```

to `src/lib.rs`.

- [ ] **Step 7: Run the history tests and verify GREEN**

Run:

```bash
cargo test --test surface_history
```

Expected:

```text
PASS with 2 passed; 0 failed
```

- [ ] **Step 8: Commit the history layer**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/lib.rs \
        /Users/joe/Documents/unnamed/bionic_v0/src/surface_history.rs \
        /Users/joe/Documents/unnamed/bionic_v0/tests/surface_history.rs
git commit -m "feat: add observer-side surface history"
```

## Chunk 3: Coarse Raster Renderer

### Task 3: Convert projected objects and trails into a low-resolution pixel field

**Files:**
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/surface_render.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/surface_render.rs`

- [ ] **Step 1: Write the failing raster test for object blobs**

```rust
use bionic_v0::surface_render::{render_surface_frame, RenderConfig};

#[test]
fn renderer_marks_pixels_inside_projected_object_extent() {
    let frame = single_object_frame(0.5, 0.5, 0.2, 0.2);
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    assert!(img.iter().any(|px| px[0] > 0));
}
```

- [ ] **Step 2: Run the raster test to verify RED**

Run:

```bash
cargo test --test surface_render renderer_marks_pixels_inside_projected_object_extent
```

Expected:

```text
FAIL with missing surface_render module/items
```

- [ ] **Step 3: Add the failing trail test**

```rust
#[test]
fn renderer_draws_trail_energy_behind_recent_motion() {
    let frame = single_object_frame(0.6, 0.5, 0.1, 0.1);
    let trails = one_trail(vec![(0.2, 0.5), (0.4, 0.5), (0.6, 0.5)]);
    let img = render_surface_frame(&frame, &trails, RenderConfig::new(64, 36));

    assert!(count_lit_pixels(&img) > 0);
}
```

- [ ] **Step 4: Run the raster tests and verify RED**

Run:

```bash
cargo test --test surface_render
```

Expected:

```text
FAIL with missing renderer implementation
```

- [ ] **Step 5: Implement the minimal coarse raster renderer**

Create `surface_render.rs` with:

- `RenderConfig`
- `render_surface_frame(...) -> Vec<[u8; 4]>`

Minimal rendering rules:

- low-resolution RGBA pixel buffer only
- object blobs are filled ellipses or rounded coarse masks
- trails are stamped as decaying points or short line segments
- no fixed human-emotion color dictionary
- rendering intensities derive only from numeric hints already produced by projection/history

- [ ] **Step 6: Export the renderer module from the crate**

Add:

```rust
pub mod surface_render;
```

to `src/lib.rs`.

- [ ] **Step 7: Run the raster tests and verify GREEN**

Run:

```bash
cargo test --test surface_render
```

Expected:

```text
PASS with 2 passed; 0 failed
```

- [ ] **Step 8: Commit the raster renderer**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/lib.rs \
        /Users/joe/Documents/unnamed/bionic_v0/src/surface_render.rs \
        /Users/joe/Documents/unnamed/bionic_v0/tests/surface_render.rs
git commit -m "feat: add coarse predictive surface renderer"
```

## Chunk 4: Native Window Binary

### Task 4: Wire the projection/history/renderer into a native observer binary

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer_surface.rs`

- [ ] **Step 1: Write the failing binary smoke test as a compile target**

Add the new binary declaration to `Cargo.toml` without implementation first:

```toml
[[bin]]
name = "observer_surface"
path = "src/bin/observer_surface.rs"
```

and add dependencies:

```toml
pixels = "..."
winit = "..."
```

- [ ] **Step 2: Run cargo check to verify RED**

Run:

```bash
cargo check --bin observer_surface
```

Expected:

```text
FAIL because src/bin/observer_surface.rs does not exist yet
```

- [ ] **Step 3: Implement the minimal native observer**

Create `observer_surface.rs` with:

- CLI args: `runtime_dir`, optional internal pixel size, optional poll rate
- event loop driven by `winit`
- fixed-size low-resolution framebuffer via `pixels`
- periodic `read_snapshot(...)`
- `project_surface_frame(...)`
- `SurfaceHistory::update(...)`
- `render_surface_frame(...)`
- framebuffer upload to window

Rules:

- if snapshot read fails, keep the window alive and render an empty/decayed frame
- do not print body-side explanations
- do not open any reverse control path into `body`

- [ ] **Step 4: Run cargo check and verify GREEN**

Run:

```bash
cargo check --bin observer_surface
```

Expected:

```text
PASS with exit code 0
```

- [ ] **Step 5: Run the full test suite plus new binary check**

Run:

```bash
cargo test
cargo check --bin observer_surface
```

Expected:

```text
All existing tests pass
observer_surface compiles cleanly
```

- [ ] **Step 6: Manual smoke test against a live runtime**

Run in terminal 1:

```bash
cargo run --bin body runtime
```

Run in terminal 2:

```bash
cargo run --bin observer_surface runtime
```

Expected:

```text
A native window opens and shows coarse object blobs/trails updating from snapshot.bin
```

- [ ] **Step 7: Commit the native window observer**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml \
        /Users/joe/Documents/unnamed/bionic_v0/src/bin/observer_surface.rs
git commit -m "feat: add native predictive visual surface observer"
```

## Chunk 5: Final Verification

### Task 5: Verify the phase matches the spec boundary

**Files:**
- Review only: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-23-predictive-visual-surface-design.md`
- Review only: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs`
- Review only: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`

- [ ] **Step 1: Re-read the spec and verify scope**

Check that implementation still matches:

- observer-side only
- no body write-back
- no fixed human emotion color dictionary
- no snapshot schema growth unless absolutely necessary

- [ ] **Step 2: Run final verification commands**

Run:

```bash
cargo test
cargo check --bin observer_surface
```

Expected:

```text
0 failing tests
observer_surface builds successfully
```

- [ ] **Step 3: Inspect git diff for accidental boundary violations**

Run:

```bash
git diff -- bionic_v0/src/bin/body.rs bionic_v0/src/snapshot.rs
```

Expected:

```text
No body-side observer coupling
No accidental display-channel backflow
```

- [ ] **Step 4: Final commit if any boundary fixes were needed**

```bash
git add -A
git commit -m "chore: verify predictive visual surface boundaries"
```
