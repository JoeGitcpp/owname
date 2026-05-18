# Auditory-Visual Predictability and Expression v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that hearing/vision can support predictable return and non-symbolic expression before any higher language or recognition layer.

**Architecture:** Add a small numeric `auditory_visual` substrate to `Viewport`. It reads existing action-consequence pairs, slot fusion timing, and vocal output paths; observer reports compact numeric evidence only.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, VIB/PHOT fusion, vocal substrate, `curriculum_report`, cargo integration tests.

---

## Files

- Create: `bionic_v0/src/auditory_visual.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Create: `bionic_v0/tests/auditory_visual.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

## Task 1: RED Tests

- [x] Write failing test `vocal_write_with_vib_return_raises_predictable_return_strength`.
- [x] Write failing test `aligned_vib_phot_activity_raises_cross_channel_predictability`.
- [x] Write failing test `cross_channel_pressure_can_drive_non_symbolic_vocal_expression`.
- [x] Write failing test `misaligned_vib_phot_activity_raises_drift_pressure`.
- [x] Write source guard test rejecting speech/word/text/semantic/concept/token/label/recognition ontology.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test auditory_visual` and confirm RED failure from missing API.

## Task 2: Numeric Substrate

- [x] Add `AuditoryVisualState` with numeric fields only.
- [x] Add `observe_self_return`, `observe_channel_alignment`, `tick_decay`, and `drive_expression`.
- [x] Export `auditory_visual` from `lib.rs`.
- [x] Run focused tests and keep failures limited to missing viewport wiring.

## Task 3: Viewport and Body Wiring

- [x] Add `auditory_visual: RwLock<AuditoryVisualState>` to `Viewport`.
- [x] Add `read_auditory_visual`.
- [x] Add `tick_auditory_visual_from_pairs`.
- [x] Add `tick_auditory_visual_from_fusion`.
- [x] Add `tick_auditory_visual_expression`.
- [x] Wire body ticker after fusion/pressure updates and before snapshot observation.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test auditory_visual` and confirm GREEN.

## Task 4: Observer Curriculum Report

- [x] Add `AuditoryVisualPredictabilityExpressionEvidence`.
- [x] Add `run_auditory_visual_predictability_expression_curriculum`.
- [x] Add `auditory_visual_source_order_guard`.
- [x] Add `--auditory-visual-predictability-expression` to `curriculum_report`.
- [x] Add report shape test in `bionic_v0/tests/curriculum_report.rs`.

## Task 5: Verification

- [x] Run `cargo fmt --manifest-path bionic_v0/Cargo.toml`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test auditory_visual`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report auditory_visual_predictability_expression_report_emits_numeric_evidence`.
- [x] Run `cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --auditory-visual-predictability-expression`.
- [x] Run `cargo test --manifest-path bionic_v0/Cargo.toml`.
