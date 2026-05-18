# Soft Arbitration Drift Under Load v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that chain memory can hold, drift, switch, and reenter under competing load while preserving branch traces and single active aperture execution.

**Architecture:** Extend `active_surface` chain memory with soft numeric arbitration. Reuse existing chain memory traces; do not add a new brain/thought/planner module. Expose only observer numeric evidence through `curriculum_report`.

**Tech Stack:** Rust, existing `bionic_v0` active surface, action-consequence, pressure field, cargo tests.

---

## Files

- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/soft_arbitration_drift_under_load.rs`

## Task 1: RED Tests

- [x] Write failing tests for hold under small challenger margin.
- [x] Write failing tests for switch under sustained challenger pressure.
- [x] Write failing tests for old-chain reentry after pressure returns.
- [x] Run focused test and confirm it fails because arbitration fields are missing.

## Task 2: Soft Arbitration State

- [x] Add soft arbitration fields to `ActiveSurfaceState`.
- [x] Update chain memory branch state to apply inertia and margin.
- [x] Count hold, switch, and reentry.
- [x] Verify focused tests pass.

## Task 3: Observer Report

- [x] Add `SoftArbitrationDriftUnderLoadEvidence`.
- [x] Add `run_soft_arbitration_drift_under_load_curriculum`.
- [x] Add `--soft-arbitration-drift-under-load` to `curriculum_report`.
- [x] Add report shape test.

## Task 4: Verification

- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test soft_arbitration_drift_under_load`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report`.
- [x] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --soft-arbitration-drift-under-load`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
