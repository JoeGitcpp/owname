# Long-Horizon Pressure Chain v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that the active reasoning substrate can maintain one pressure chain across long duration, interruptions, delayed reentry, and branch retention.

**Architecture:** Extend existing `active_surface` chain memory with bounded long-horizon numeric fields. Reuse local probe chain, chain memory, and soft arbitration; do not create a new brain/thought/planner module. Expose evidence only through observer-side `curriculum_report`.

**Tech Stack:** Rust, existing `bionic_v0` viewport, active surface, action-consequence, pressure field, cargo tests.

---

## Files

- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/long_horizon_pressure_chain.rs`

## Task 1: RED Tests

- [x] Write failing test proving a single chain reaches >=12 steps and long span.
- [x] Write failing test proving interruption branch pressure is retained.
- [x] Write failing test proving delayed reentry increments recovery.
- [x] Run focused test and confirm failure from missing long-horizon fields.

## Task 2: Long-Horizon Chain State

- [x] Add long-horizon fields to `ChainMemoryTrace`.
- [x] Add aggregate long-horizon fields to `ActiveSurfaceState`.
- [x] Update chain memory sedimentation and branch refresh to compute span, depth, long score, interruption, and recovery.
- [x] Run focused test and confirm pass.

## Task 3: Observer Report

- [x] Add `LongHorizonPressureChainEvidence`.
- [x] Add `run_long_horizon_pressure_chain_curriculum`.
- [x] Add `--long-horizon-pressure-chain` to `curriculum_report`.
- [x] Add report shape test.

## Task 4: Verification

- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test long_horizon_pressure_chain`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report long_horizon_pressure_chain_report_emits_numeric_evidence`.
- [x] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --long-horizon-pressure-chain`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
