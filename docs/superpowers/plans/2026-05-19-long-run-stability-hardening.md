# Long-Run Stability Hardening Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent overnight solitary runs from collapsing into unbounded observer traces, huge snapshots, and heat-heavy debug behavior.

**Architecture:** Keep body-side state numeric/time/curve based. Bound VPC trace retention at the substrate level, expose compact observer summaries, and add a low-power launch mode that reduces tick/write pressure without adding world content or a request/response controller.

**Tech Stack:** Rust `bionic_v0`, existing `Viewport`, `VpcContractState`, `snapshot`, `observer`, and `body` binaries.

---

## Boundary Notes

- `trace`, `summary`, and `cap` are engineering evidence-management terms, not internal ontology units.
- No body-side human-readable output may be introduced.
- Observer output may summarize numeric snapshot facts, but observer needs must not create new body ontology.
- Low-power mode is an engineering launch mode for long-running substrate stability. It must not predefine what the body should learn.

## Files

- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/vpc.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Test: `bionic_v0/tests/vpc_predictor_contract.rs`
- Test: `bionic_v0/tests/observer_compact.rs`
- Test: `bionic_v0/tests/body_no_human_output.rs`

## Task 1: Bound VPC Trace Retention

- [x] Add a failing test that repeated VPC publications/resolutions keep predictor and publication traces within fixed caps.
- [x] Run the targeted test and confirm it fails because VPC history is currently unbounded.
- [x] Add engineering caps in `lib.rs`.
- [x] Enforce caps inside `VpcContractState` mutation methods.
- [x] Re-run the targeted test.

## Task 2: Compact Observer VPC Rendering

- [x] Add a failing observer test that a full observer row does not explode when VPC has many traces.
- [x] Limit VPC rendering to recent samples plus counts already present in snapshot.
- [x] Re-run observer tests.

## Task 3: Low-Power Body Launch Mode

- [x] Add a failing test/guard proving body accepts a low-power flag without body-readable diagnostics.
- [x] Parse `--solitary-low-power` in `body`.
- [x] Lower long-run rates for predictor, high-level predictor, health, ingress, binding, endogenous, vocal, live-byte closure, checkpoint, orchestrator, and master clock through a body runtime profile.
- [x] Re-run body source hygiene and short runtime smoke.

## Task 4: Verification

- [x] Run targeted VPC tests.
- [x] Run observer tests.
- [x] Run body no-human-output test.
- [x] Run self-development and pre-language report tests.
- [x] Run formatting and `git diff --check`.
- [x] Run a short low-power body smoke and report snapshot size/metrics.
