# Persistent Curriculum Session Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persistent curriculum session that runs repeated episodes through the same `Viewport` and reports cross-episode accumulated evidence.

**Architecture:** Keep the runner external to `body`. Add a same-viewport session path in `bionic_v0::curriculum` that reuses slots, advances synthetic time, closes each consequence window, and reports numeric trend metrics. Extend `curriculum_report` with a persistent mode without adding semantic labels or request/response logic.

**Tech Stack:** Rust, existing `Viewport`, action-output/action-consequence APIs, Cargo integration tests.

---

## Task 1: Persistent Session API

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/tests/curriculum_runner.rs`

- [ ] **Step 1: Write failing test**

Add a test for `run_persistent_curriculum_session(CurriculumConfig::default(), 4)` asserting one body runs four episodes, forms four pairs, retains four final pairs, and has learned-bias evidence.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner persistent_curriculum_session_accumulates_evidence_in_one_body -- --nocapture
```

Expected: compile FAIL because the function does not exist.

- [ ] **Step 3: Implement minimal API**

Add `PersistentCurriculumSessionSummary` and `run_persistent_curriculum_session`. Reuse fixed slots for motion/relation, drive a write-return pulse each episode, close the latest consequence window, tick life-closure producers, and return numeric metrics.

- [ ] **Step 4: Verify GREEN**

Run the same command. Expected: PASS.

## Task 2: Report Persistent Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Write failing test**

Add a test invoking:

```bash
curriculum_report --episodes 4 --persistent
```

Assert output includes `mode=persistent`, `episodes_run=4`, `final_pairs=4`, and learned-bias metrics.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report persistent_curriculum_report_emits_same_body_metrics -- --nocapture
```

Expected: FAIL because the binary does not parse `--persistent`.

- [ ] **Step 3: Implement report mode**

Parse `--persistent`. In that mode call `run_persistent_curriculum_session`; otherwise keep the existing fresh-suite behavior.

- [ ] **Step 4: Verify GREEN and full suite**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```
