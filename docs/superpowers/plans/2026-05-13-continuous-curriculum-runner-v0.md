# Continuous Curriculum Runner v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal external curriculum runner that drives existing viewport substrate loops and returns observer-side numeric training evidence.

**Architecture:** Add `bionic_v0::curriculum` as a harness module, not a body organ. It drives legal viewport APIs for spatial motion, relation projection, and action-consequence pulses, then returns bounded metrics and optionally writes snapshots. It introduces no semantic, token, label, concept, identity, owner, ethics, or personality ontology.

**Tech Stack:** Rust, existing `Viewport`, `snapshot`, action-output/action-consequence APIs, Cargo integration tests.

---

## File Structure

- Create: `bionic_v0/src/curriculum.rs`
  - Owns external course configuration, metrics, and course runner functions.
- Modify: `bionic_v0/src/lib.rs`
  - Export `pub mod curriculum;`.
- Create: `bionic_v0/tests/curriculum_runner.rs`
  - Covers worldline evidence, relation projection evidence, action-consequence learning evidence, and source hygiene.

## Task 1: Single-Object Motion Course

**Files:**
- Create: `bionic_v0/tests/curriculum_runner.rs`
- Create: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add a test that creates `Viewport`, runs `run_single_object_motion_course`, and asserts the summary has `steps_run`, `worldline_updates`, bounded `final_worldline_err`, and snapshot-visible worldline state.

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner single_object_motion_course_produces_worldline_evidence -- --nocapture
```

Expected: compile FAIL because `bionic_v0::curriculum` does not exist.

- [ ] **Step 3: Implement minimal course**

Add `CurriculumConfig`, `CurriculumSummary`, and `run_single_object_motion_course`. The course allocates one slot, writes a moving spatial curve, refreshes global attention/relations, optionally writes snapshots, and returns numeric metrics.

- [ ] **Step 4: Run test to verify GREEN**

Run the same command. Expected: PASS.

## Task 2: Relation Projection Course

**Files:**
- Modify: `bionic_v0/tests/curriculum_runner.rs`
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Write the failing test**

Add a test that runs `run_two_object_relation_course` and asserts spatial distance and overlap projection increase as one object moves closer.

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner two_object_relation_course_tracks_relation_projection_change -- --nocapture
```

Expected: compile FAIL because `run_two_object_relation_course` does not exist.

- [ ] **Step 3: Implement relation course**

Seed two attended slots with low-level PHOT/VIB bands, write spatial states, refresh attention/relations, move one object near the other, and return first/final relation projection scores.

- [ ] **Step 4: Run test to verify GREEN**

Run the same command. Expected: PASS.

## Task 3: Action-Consequence Pulse Course

**Files:**
- Modify: `bionic_v0/tests/curriculum_runner.rs`
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Write the failing test**

Add a test that runs `run_action_consequence_pulse_course` and asserts it forms a pair and learned-bias event.

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner action_consequence_pulse_course_forms_pair_and_learning_evidence -- --nocapture
```

Expected: compile FAIL because `run_action_consequence_pulse_course` does not exist.

- [ ] **Step 3: Implement pulse course**

Emit an action-output Begin, accumulate a consequence window with the same magnitude, auto-pair, tick life-closure producers, and return pair/learned-bias metrics.

- [ ] **Step 4: Run test to verify GREEN**

Run the same command. Expected: PASS.

## Task 4: Hygiene And Verification

**Files:**
- Modify: `bionic_v0/tests/curriculum_runner.rs`

- [ ] **Step 1: Add hygiene test**

Scan `src/curriculum.rs` for rejected architecture terms that would indicate an internal ontology or request/response controller.

- [ ] **Step 2: Run targeted verification**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```

Expected: PASS.
