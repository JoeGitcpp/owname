# Multi-Step Chain Continuity v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove multiple local probe-return steps can strengthen one continuing numeric chain while branch pressure remains available and active execution stays single-aperture.

**Architecture:** Extend `ChainMemoryTrace` inside `active_surface`; no new brain/thought module. Same-coordinate local chain traces raise continuity score; different-coordinate traces become bounded branch pressure. Observer-only evidence is exposed through `curriculum_report`.

**Tech Stack:** Rust, existing `bionic_v0` active surface, action-consequence, prediction pressure field, cargo tests.

---

## Files

- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/multi_step_chain_continuity.rs`

## Task 1: Continuity State

- [ ] Write failing tests for same-coordinate multi-step chain continuity.
- [ ] Add continuity fields to `ChainMemoryTrace` and `ActiveSurfaceState`.
- [ ] Update chain memory sedimentation to raise continuity on repeated coordinates.
- [ ] Verify focused test passes.

## Task 2: Branch Pressure

- [ ] Write failing test for nearby branch trace retention.
- [ ] Add branch pressure computation from non-dominant chain memory traces.
- [ ] Verify focused test passes.

## Task 3: Observer Report

- [ ] Add `MultiStepChainContinuityEvidence`.
- [ ] Add `run_multi_step_chain_continuity_curriculum`.
- [ ] Add `--multi-step-chain-continuity` to `curriculum_report`.
- [ ] Add report shape test.

## Task 4: Verification

- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test multi_step_chain_continuity`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report`.
- [ ] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --multi-step-chain-continuity`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
