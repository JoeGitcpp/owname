# VPC Time And Error Contract Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze and implement the first formal VPC time-and-error contract so predictor timing, err definition, publication boundaries, and unclosed-prediction state become explicit substrate facts instead of scattered implementation assumptions.

**Architecture:** Add one narrow VPC contract layer for metadata and tests, not a new cognition organ. The work proceeds in three bounded pieces: write the frozen sub-spec, add minimal substrate structures that declare observation domain / horizon / publication provenance / open-loop state, and lock the contract with source-backed and behavior-backed tests. Existing predictors, attention, relations, reasoning, language binding, and observer flow stay intact unless a contract test forces a minimal change.

**Tech Stack:** Rust, Cargo tests, Markdown docs

---

## File Structure

- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/vpc.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_time_err_contract.rs`
- Create: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-22-vpc-time-and-error-contract-design.md`
- Modify: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-20-vpc-core-contract-design.md`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/predictor.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`

