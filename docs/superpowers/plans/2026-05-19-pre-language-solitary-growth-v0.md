# Pre-Language Solitary Growth v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement the pre-language solitary-growth gate so the body shows measurable growth during quiet no-teacher runtime before any later training is allowed.

**Architecture:** Extend the existing body-side `SelfDevelopmentState` rather than adding a separate teacher or curriculum module. The loop will derive solitary stability, quiet replay pressure, room familiarity, and a prelingual growth gate from existing prediction, recurrence, action-output, and action-consequence substrate. Snapshot/report integration exposes the evidence without writing observer interpretation back into the body.

**Tech Stack:** Rust, Cargo integration tests, existing `Viewport`, `SelfDevelopmentState`, snapshot v47 path, and `curriculum_report`.

---

## File Structure

- Modify: `bionic_v0/src/self_development.rs`
  - Add solitary-growth numeric fields and update logic.
- Modify: `bionic_v0/src/snapshot.rs`
  - Serialize/read the expanded self-development state.
- Modify: `bionic_v0/src/lib.rs`
  - Bump `SNAPSHOT_VERSION` to 48.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--pre-language-solitary-growth [runtime_dir]` report mode.
- Modify: `bionic_v0/tests/self_development_life_loop.rs`
  - Add RED/GREEN tests for solitary growth, quiet replay, reentry advantage, and source guards.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add report contract test.
- Modify: `docs/superpowers/specs/2026-05-19-pre-language-solitary-growth-v0-design.md`
  - Mark implementation status after verification.

## Task 1: Solitary Low-Change Growth

**Files:**
- Modify: `bionic_v0/tests/self_development_life_loop.rs`
- Modify: `bionic_v0/src/self_development.rs`

- [x] **Step 1: Write the failing test**

Add a test that feeds stable low-gap recurrence without write/return saturation and expects `solitary_stability`, `quiet_replay_pressure`, `room_familiarity`, and `solitary_growth_gate_pass` to rise.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop solitary_low_change_runtime_builds_room_familiarity_without_teacher
```

Expected: FAIL because the solitary-growth fields do not exist.

- [x] **Step 3: Implement minimal body-side fields**

Add bounded numeric fields:

```rust
pub solitary_stability: f64,
pub quiet_replay_pressure: f64,
pub room_familiarity: f64,
pub solitary_growth_score: f64,
pub solitary_growth_gate_pass: u8,
```

Update them from recurrence, low prediction gap, low write/return load, and familiarity pressure.

- [x] **Step 4: Verify GREEN**

Run the same test. Expected: PASS.

## Task 2: Quiet Replay And Reentry Advantage

**Files:**
- Modify: `bionic_v0/tests/self_development_life_loop.rs`
- Modify: `bionic_v0/src/self_development.rs`

- [x] **Step 1: Write failing tests**

Add tests proving:

- quiet low-pressure recurrence updates `quiet_replay_pressure`;
- disturbance after prior familiarity keeps a higher reentry/familiarity response than a cold body.

- [x] **Step 2: Verify RED**

Run targeted tests. Expected: FAIL until the reentry contribution is included.

- [x] **Step 3: Implement reentry support**

Let existing `reentry_score` and familiarity carry into room familiarity without creating identity or person fields.

- [x] **Step 4: Verify GREEN**

Run the targeted tests. Expected: PASS.

## Task 3: Snapshot And Report Gate

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/tests/self_development_life_loop.rs`

- [x] **Step 1: Write failing snapshot/report tests**

Add tests for snapshot roundtrip of new fields and report output:

```text
mode=pre_language_solitary_growth
runtime_snapshot_read=
solitary_stability=
quiet_replay_pressure=
room_familiarity=
solitary_growth_score=
solitary_growth_gate_pass=
prelingual_growth_gate_pass=
principle_guard_pass=
```

- [x] **Step 2: Verify RED**

Run the targeted tests. Expected: FAIL because snapshot v48 and report mode do not exist.

- [x] **Step 3: Implement snapshot v48 and report mode**

Append fields to self-development serialization, read them back, and add a deterministic non-runtime sample plus runtime snapshot path.

- [x] **Step 4: Verify GREEN**

Run targeted tests. Expected: PASS.

## Task 4: Guardrails, Verification, Commit

**Files:**
- Modify: `bionic_v0/tests/self_development_life_loop.rs`
- Modify: `docs/superpowers/specs/2026-05-19-pre-language-solitary-growth-v0-design.md`
- Modify: `docs/superpowers/plans/2026-05-19-pre-language-solitary-growth-v0.md`

- [x] **Step 1: Add/extend source guard**

Guard against fixed ontology shortcuts:

```text
semantic_readiness, SymbolizerState, concept_id, token_id, word_id,
object_name, text_understanding, name_gate, meaning_db, dictionary,
tokenizer, image_label, voice_label, prompt_answer, source_identity,
owner, ethics, personality, person_id, identity_id
```

- [x] **Step 2: Run verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report pre_language_solitary_growth_report_emits_numeric_gate_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test body_no_human_output
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --check
git diff --check
```

- [x] **Step 3: Commit**

```bash
git add \
  bionic_v0/src/self_development.rs \
  bionic_v0/src/snapshot.rs \
  bionic_v0/src/lib.rs \
  bionic_v0/src/bin/curriculum_report.rs \
  bionic_v0/tests/self_development_life_loop.rs \
  bionic_v0/tests/curriculum_report.rs \
  docs/superpowers/specs/2026-05-19-pre-language-solitary-growth-v0-design.md \
  docs/superpowers/plans/2026-05-19-pre-language-solitary-growth-v0.md
git commit -m "feat: add pre-language solitary growth gate"
```
