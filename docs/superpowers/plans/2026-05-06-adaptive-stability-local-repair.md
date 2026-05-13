# Adaptive Stability Local Repair Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a numeric adaptive-stability substrate for local degradation, reroute, slow relearning, and reversible fast-path traces.

**Architecture:** Add a focused `adaptive_stability.rs` route ledger and expose it through `Viewport` and snapshot. The substrate is observer-visible numeric/time state only; it does not control object existence, action targets, language binding, identity, or global reset.

**Tech Stack:** Rust, `bionic_v0` integration tests, existing `Viewport` / `snapshot` patterns, Cargo.

---

## Files

- Create: `bionic_v0/src/adaptive_stability.rs`
- Create: `bionic_v0/tests/adaptive_stability.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/body.rs`

## Chunk 1: Route Ledger

### Task 1: Add Failing Route-Ledger Tests

**Files:**
- Create: `bionic_v0/tests/adaptive_stability.rs`

- [ ] **Step 1: Write failing tests**

Add tests for:

- local degradation raises `degradation_score`, `bypass_weight`, and `relearn_pressure`;
- unrelated routes are untouched;
- repeated stable samples form `fast_path_strength`;
- drift revokes fast-path strength while keeping `slow_path_weight > 0.0`.

- [ ] **Step 2: Run RED**

Run: `cargo test --test adaptive_stability`

Expected: FAIL because `bionic_v0::adaptive_stability` does not exist.

### Task 2: Implement Minimal Route Ledger

**Files:**
- Create: `bionic_v0/src/adaptive_stability.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add module**

Add `pub mod adaptive_stability;`.

- [ ] **Step 2: Implement numeric state**

Implement:

- `AdaptiveRouteKind`
- `AdaptiveRoutePhase`
- `AdaptiveRouteSample`
- `AdaptiveRouteTrace`
- `AdaptiveStabilityState`
- `observe_sample`
- `tick_decay`

Use numeric/time fields only. No semantic, concept, token, label, name, identity, owner, personality, target, result, success, or failure fields.

- [ ] **Step 3: Run GREEN**

Run: `cargo test --test adaptive_stability`

Expected: PASS.

## Chunk 2: Viewport And Snapshot

### Task 3: Add Failing Viewport And Snapshot Tests

**Files:**
- Modify: `bionic_v0/tests/adaptive_stability.rs`

- [ ] **Step 1: Add tests**

Add tests for:

- `Viewport::read_adaptive_stability()` starts empty;
- `Viewport::observe_adaptive_route_sample(...)` records a route;
- `write_snapshot` / `read_snapshot` preserves adaptive route traces.

- [ ] **Step 2: Run RED**

Run: `cargo test --test adaptive_stability`

Expected: FAIL because viewport/snapshot integration does not exist.

### Task 4: Implement Viewport And Snapshot Integration

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add `Viewport` field**

Add `pub adaptive_stability: RwLock<AdaptiveStabilityState>`.

- [ ] **Step 2: Add `Viewport` methods**

Add:

- `read_adaptive_stability`
- `observe_adaptive_route_sample`
- `tick_adaptive_stability`

- [ ] **Step 3: Add snapshot v37 fields**

Bump `SNAPSHOT_VERSION` to `37`.

Write/read adaptive stability after vocal memory.

- [ ] **Step 4: Run GREEN**

Run: `cargo test --test adaptive_stability`

Expected: PASS.

## Chunk 3: Runtime Wiring And Hygiene

### Task 5: Add Failing Runtime/Hygiene Tests

**Files:**
- Modify: `bionic_v0/tests/adaptive_stability.rs`

- [ ] **Step 1: Add source hygiene tests**

Add tests that:

- body runtime calls `tick_adaptive_stability`;
- `adaptive_stability.rs` source contains none of the rejected architecture terms.

- [ ] **Step 2: Run RED**

Run: `cargo test --test adaptive_stability`

Expected: FAIL because body runtime is not wired.

### Task 6: Wire Maintenance Ticker

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: Add maintenance call**

Call `vp_endo.tick_adaptive_stability(t)` from the existing endogenous ticker.

- [ ] **Step 2: Run GREEN**

Run: `cargo test --test adaptive_stability`

Expected: PASS.

## Chunk 4: Full Verification

### Task 7: Verify Full Suite

- [ ] **Step 1: Run targeted tests**

Run: `cargo test --test adaptive_stability`

Expected: PASS.

- [ ] **Step 2: Run full suite**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 3: Check git diff**

Run: `git status --short` and `git diff --stat`

Expected: only planned files changed.
