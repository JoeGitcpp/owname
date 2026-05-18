# Prediction-Linked Vocal Reflex and Suppression v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a body-side, prediction-linked vocal reflex substrate that can produce non-symbolic vocal bursts under prediction shock and learn suppression/habituation through numeric closure evidence.

**Architecture:** Add a bounded numeric `vocal_reflex` substrate to `Viewport`, driven by existing prediction/evidence pressure and wired into the existing vocal organ/action-output path. Keep observer evidence in `curriculum_report`; no text, word, emotion, or TTS layer is introduced.

**Tech Stack:** Rust crate `bionic_v0`, existing vocal substrate, prediction pressure field, evidence tension, action output, cargo integration tests.

---

## Files

- Create: `bionic_v0/src/vocal_reflex.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Create: `bionic_v0/tests/vocal_reflex.rs`

## Task 1: RED Tests

- [x] Write failing test proving high prediction shock raises vocal reflex pressure and creates a non-symbolic vocal burst.
- [x] Write failing test proving repeated similar shock raises familiarity and lowers later reflex pressure.
- [x] Write failing test proving successful quiet regulation raises suppression.
- [x] Write failing test proving high unresolved pressure can bypass suppression.
- [x] Run focused test and confirm failure from missing vocal reflex API.

## Task 2: Reflex State and Viewport Wiring

- [x] Add `VocalReflexState` with numeric fields only.
- [x] Add `Viewport::tick_vocal_reflex_from_pressure`.
- [x] Add `Viewport::observe_vocal_reflex_closure`.
- [x] Add `Viewport::read_vocal_reflex`.
- [x] Wire body vocal ticker so reflex updates before target imitation.
- [x] Run focused tests and confirm pass.

## Task 3: Observer Report

- [x] Add `VocalReflexSuppressionEvidence`.
- [x] Add `run_vocal_reflex_suppression_curriculum`.
- [x] Add `--vocal-reflex-suppression` to `curriculum_report`.
- [x] Add report shape test.

## Task 4: Verification

- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_reflex`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report vocal_reflex_suppression_report_emits_numeric_evidence`.
- [x] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --vocal-reflex-suppression`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
