# Chain Memory Sedimentation v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let local probe-return chains leave bounded numeric sediment that can later reenter the active surface as residue pressure.

**Architecture:** Extend `active_surface` with chain memory traces keyed by existing substrate coordinates. Consume local probe chain traces once, fold closure/mismatch support into decaying sediment, and emit observer-only evidence through a new curriculum report.

**Tech Stack:** Rust, existing `bionic_v0` viewport/action-consequence/action-output/prediction-pressure-field substrate, cargo tests.

---

## Files

- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/chain_memory_sedimentation.rs`

## Task 1: Sediment State and RED Test

- [ ] Write a failing test proving local probe chain traces sediment into bounded chain memory traces.
- [ ] Run the focused test and verify it fails because chain memory state/API is missing.
- [ ] Add `ChainMemoryTrace` and sediment counters to `ActiveSurfaceState`.
- [ ] Add `Viewport::tick_active_surface_chain_memory(now_us)`.
- [ ] Run focused test and verify it passes.

## Task 2: Reentry Pressure

- [ ] Write a failing test proving sediment can reenter as residue pressure after the live surface quiets.
- [ ] Implement residue pressure injection through `refresh_prediction_pressure_field`.
- [ ] Run focused test and verify it passes.

## Task 3: Persistent vs Cold Baseline

- [ ] Write a failing test proving persistent chain memory reentry pressure exceeds cold baseline.
- [ ] Tune only local numeric gains/decay floors required for the test.
- [ ] Run focused test and verify it passes.

## Task 4: Body and Report

- [ ] Add body source-order assertion for chain memory after local probe chain and before probe law.
- [ ] Add `ChainMemorySedimentationEvidence` and report runner.
- [ ] Add `--chain-memory-sedimentation` to `curriculum_report`.
- [ ] Add report shape test.

## Task 5: Verification

- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test chain_memory_sedimentation`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report`.
- [ ] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --chain-memory-sedimentation`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
