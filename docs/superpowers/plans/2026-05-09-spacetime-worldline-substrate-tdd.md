# Spacetime Worldline Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Use superpowers:test-driven-development for every code task.

**Goal:** Make object-space state explicitly time-bearing by adding slot-local spatial worldline traces and spatial relation projection to the existing shared viewport.

**Architecture:** Extend the existing `Viewport` object-slot substrate. Do not add `spatial_reasoner`, `temporal_reasoner`, scene-understanding, request/response, or concept modules. Space-time sense remains a numeric body substrate: object slots maintain spatial curves, relation edges project space between slots, prediction error updates are stored with the slot, and action-consequence coupling stays in the existing loop.

**Tech Stack:** Rust, Cargo tests, existing `Viewport`, `GlobalRelationState`, and binary snapshot format.

---

## Alignment Principle

This plan explicitly protects the following architecture principle:

> 要让 AI 具备空间感和时间感，就不能把它设计成“接收输入、输出答案”的模型。它必须是一个持续存在的 body：在共享时间里维持对象世界线，在共享空间里投影对象关系，通过预测误差修正自身，通过动作后果学习自己和世界的耦合。这不是功能模块，而是生命底座。

Test coverage must therefore verify all four substrate properties:

- object-space state is a time curve, not a latest-only coordinate;
- spatial relation is projected inside the shared relation state, not through a new reasoner module;
- spatial prediction error is recorded as numeric substrate state;
- no request/response or named reasoner API is introduced for this capability.

## File Structure

- Modify `bionic_v0/src/viewport.rs`
  - Add `SpatialWorldlineState` inside `ObjectSlot`.
  - Update `write_slot_spatial` to maintain predicted position, velocity, delta time, and error.
  - Add spatial projection fields to `RelationEdgeState`.
  - Update `refresh_global_relations` to project distance/overlap from valid spatial slot states.
- Modify `bionic_v0/src/snapshot.rs`
  - Persist slot-local `SpatialWorldlineState`.
  - Persist relation-edge spatial projection fields.
- Modify `bionic_v0/src/lib.rs`
  - Bump `SNAPSHOT_VERSION`.
- Create `bionic_v0/tests/spacetime_worldline.rs`
  - TDD tests for slot worldline, relation projection, snapshot roundtrip, and principle hygiene.

## Task 1: Slot Spatial Worldline

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/spacetime_worldline.rs`

- [ ] **Step 1: Write failing tests**

Create `bionic_v0/tests/spacetime_worldline.rs` with:

```rust
use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_PHOT, MOD_VIB};

#[test]
fn spatial_writes_form_slot_worldline_with_prediction_error() {
    let vp = Viewport::new();
    let slot = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot");

    vp.write_slot_spatial(slot, MOD_PHOT, 1_000, 0.10, 0.20, 0.10, 0.10, 0.7);
    vp.write_slot_spatial(slot, MOD_PHOT, 11_000, 0.20, 0.25, 0.10, 0.10, 0.8);
    vp.write_slot_spatial(slot, MOD_PHOT, 21_000, 0.31, 0.31, 0.10, 0.10, 0.9);

    let snap = vp.read_slot(slot).expect("slot snapshot");
    let wl = snap.spatial_worldline.expect("worldline");

    assert_eq!(wl.n_updates, 3);
    assert_eq!(wl.prev_t_us, 11_000);
    assert_eq!(wl.last_update_t_us, 21_000);
    assert!(wl.valid);
    assert!(wl.velocity_x_per_s > 0.0);
    assert!(wl.velocity_y_per_s > 0.0);
    assert!(wl.predicted_pos_x > 0.29 && wl.predicted_pos_x < 0.31);
    assert!(wl.predicted_pos_y > 0.29 && wl.predicted_pos_y < 0.31);
    assert!(wl.err_norm > 0.0);
}
```

- [ ] **Step 2: Run test and verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test spacetime_worldline spatial_writes_form_slot_worldline_with_prediction_error -- --nocapture
```

Expected: compile FAIL because `spatial_worldline` does not exist.

- [ ] **Step 3: Implement minimal worldline substrate**

Add `SpatialWorldlineState` with numeric-only fields:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct SpatialWorldlineState {
    pub n_updates: u32,
    pub prev_t_us: u64,
    pub last_update_t_us: u64,
    pub prev_pos_x: f64,
    pub prev_pos_y: f64,
    pub predicted_pos_x: f64,
    pub predicted_pos_y: f64,
    pub velocity_x_per_s: f64,
    pub velocity_y_per_s: f64,
    pub err_x: f64,
    pub err_y: f64,
    pub err_norm: f64,
    pub last_dt_us: u64,
    pub valid: bool,
}
```

Store it in `ObjectSlot` and `ObjectSlotSnapshot`. In `write_slot_spatial`, update it from the previous valid spatial sample:

- first write seeds the worldline with zero velocity and zero error;
- later writes predict current position from previous velocity and `dt`;
- error is `observed - predicted`;
- velocity is `(observed - previous) / dt_seconds`;
- all coordinates are clamped like `SpatialState`;
- no language/string/source identity fields.

- [ ] **Step 4: Run test and verify GREEN**

Run the same command. Expected: PASS.

## Task 2: Spatial Relation Projection

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/spacetime_worldline.rs`

- [ ] **Step 1: Add failing relation projection test**

Append:

```rust
#[test]
fn global_relations_project_spatial_distance_from_worldlines() {
    let vp = Arc::new(Viewport::new());
    let a = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, 1_000, 2).expect("slot b");

    seed_attended_slot(&vp, a, 10_000);
    seed_attended_slot(&vp, b, 10_000);
    vp.write_slot_spatial(a, MOD_PHOT, 10_000, 0.10, 0.10, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, 10_000, 0.80, 0.80, 0.10, 0.10, 0.9);

    vp.refresh_global_attention(10_000);
    vp.refresh_global_relations(10_000);
    let far = vp.read_global_relations();
    let far_edge = relation_edge(&far, a, b);

    vp.write_slot_spatial(b, MOD_PHOT, 20_000, 0.18, 0.18, 0.10, 0.10, 0.9);
    seed_attended_slot(&vp, a, 20_000);
    seed_attended_slot(&vp, b, 20_000);
    vp.refresh_global_attention(20_000);
    vp.refresh_global_relations(20_000);
    let near = vp.read_global_relations();
    let near_edge = relation_edge(&near, a, b);

    assert!(near_edge.spatial_distance_score > far_edge.spatial_distance_score);
    assert!(near_edge.spatial_overlap_score > far_edge.spatial_overlap_score);
}
```

The test helpers must write PHOT/VIB families and refresh fusion/high-layer using the existing patterns from `global_relations.rs`.

- [ ] **Step 2: Run test and verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test spacetime_worldline global_relations_project_spatial_distance_from_worldlines -- --nocapture
```

Expected: compile FAIL because `spatial_distance_score` and `spatial_overlap_score` do not exist.

- [ ] **Step 3: Implement relation projection fields**

Add to `RelationEdgeState`:

```rust
pub spatial_distance_score: f64,
pub spatial_overlap_score: f64,
```

In `refresh_global_relations`, collect valid `SpatialState` for each slot and update:

- `spatial_distance_score = 1.0 - normalized center distance`;
- `spatial_overlap_score = normalized 2D bounding-box overlap`;
- decay both fields when either slot lacks active spatial state;
- leave claim/scoring/adapters untouched.

Persist both fields in snapshot relation serialization and bump `SNAPSHOT_VERSION`.

- [ ] **Step 4: Run test and verify GREEN**

Run the same command. Expected: PASS.

## Task 3: Snapshot Roundtrip And Principle Hygiene

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/spacetime_worldline.rs`

- [ ] **Step 1: Add failing snapshot and hygiene tests**

Append:

```rust
#[test]
fn snapshot_preserves_spatial_worldline_and_spatial_relation_projection() {
    let vp = Arc::new(Viewport::new());
    let a = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, 1_000, 2).expect("slot b");

    seed_attended_slot(&vp, a, 10_000);
    seed_attended_slot(&vp, b, 10_000);
    vp.write_slot_spatial(a, MOD_PHOT, 10_000, 0.10, 0.10, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(a, MOD_PHOT, 20_000, 0.20, 0.20, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, 20_000, 0.24, 0.24, 0.20, 0.20, 0.9);
    vp.refresh_global_attention(20_000);
    vp.refresh_global_relations(20_000);

    let dir = std::env::temp_dir().join("bionic_v0_spacetime_worldline_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    let wl = snap.slots[a].spatial_worldline.expect("worldline");
    assert_eq!(wl.n_updates, 2);
    assert!(wl.err_norm >= 0.0);

    let edge = relation_edge(&snap.global_relations, a, b);
    assert!(edge.spatial_distance_score > 0.0);
    assert!(edge.spatial_overlap_score > 0.0);
}

#[test]
fn spacetime_substrate_does_not_become_request_response_reasoner() {
    let vp_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/viewport.rs"))
        .expect("read viewport.rs");

    for forbidden in [
        "spatial_reasoner",
        "temporal_reasoner",
        "scene_understanding",
        "request_response",
        "answer_model",
        "SpatialReasoner",
        "TemporalReasoner",
    ] {
        assert!(
            !vp_src.contains(forbidden),
            "space-time sense must remain a continuous body substrate, not `{}`",
            forbidden
        );
    }
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test spacetime_worldline -- --nocapture
```

Expected before implementation is complete: compile FAIL or assertion FAIL for missing snapshot fields.

- [ ] **Step 3: Implement snapshot roundtrip**

Add binary write/read helpers for `SpatialWorldlineState` immediately after `SpatialState` in each slot payload. Add relation spatial fields after the existing relation confidence fields. Bump `SNAPSHOT_VERSION`.

- [ ] **Step 4: Run focused and full verification**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test spacetime_worldline -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test global_relations -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cleanup_hygiene -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: all pass.

## Self-Review

- Spec coverage: the plan maps object worldlines, shared spatial relation projection, prediction error, action-consequence boundary preservation, and request/response rejection to concrete tests.
- Placeholder scan: no TBD/TODO/later placeholders remain.
- Type consistency: `SpatialWorldlineState`, `spatial_worldline`, `spatial_distance_score`, and `spatial_overlap_score` are named consistently across tests, viewport, and snapshot.
