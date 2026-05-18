# Local Probe Chain v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect active surface pressure, probe write epochs, paired returns, pressure revision, and next-probe pressure into one numeric local chain.

**Architecture:** Extend the existing `active_surface` substrate rather than creating a new brain/thought module. Consume new `ActionConsequencePair` records by monotonic `pair_seq`, revise the current active aperture numerically, and expose observer-only evidence through `curriculum_report`.

**Tech Stack:** Rust, existing `bionic_v0` viewport/action-consequence/action-output/probe-law substrate, cargo tests.

---

## Files

- Modify: `bionic_v0/src/active_surface.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/local_probe_chain.rs`

## Task 1: Local Chain State and Tests

- [ ] Write failing test in `bionic_v0/tests/local_probe_chain.rs` proving a high-support return reduces active aperture pressure and records a chain step.
- [ ] Run the focused test and verify it fails because the chain API/state is missing.
- [ ] Add `LocalProbeChainTrace` and chain counters to `ActiveSurfaceState`.
- [ ] Add `Viewport::tick_active_surface_probe_chain(now_us)`.
- [ ] Run focused test and verify it passes.

## Task 2: Mismatch Continues Probe Pressure

- [ ] Add failing test proving a low-support return raises `next_probe_pressure` and does not delete the active trace.
- [ ] Implement mismatch revision in `active_surface.rs`.
- [ ] Run focused test and verify it passes.

## Task 3: Body Integration

- [ ] Add failing source-order assertion for `tick_active_surface_probe_chain`.
- [ ] Insert the chain tick after `refresh_active_surface` and before `refresh_probe_law`.
- [ ] Run focused tests.

## Task 4: Observer Curriculum Report

- [ ] Add `LocalProbeChainEvidence` and `run_local_probe_chain_curriculum`.
- [ ] Add `--local-probe-chain` to `curriculum_report`.
- [ ] Add report shape test in `bionic_v0/tests/curriculum_report.rs`.
- [ ] Run report and verify numeric evidence.

## Task 5: Verification

- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test local_probe_chain`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report`.
- [ ] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
- [ ] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --local-probe-chain`.
