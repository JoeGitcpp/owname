# Learned Bias Delta Persistence Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:executing-plans to implement this plan in the current isolated worktree. Steps use checkbox (`- [ ]`) syntax for tracking. Use superpowers:test-driven-development for every code task.

**Goal:** Implement the first highest-principle-aligned learned-bias delta substrate: base priors remain unchanged, lived numeric evidence sediments as delta events/curves, learned state persists separately, and snapshot exposes numeric summaries only.

**Architecture:** Add one focused module, `bionic_v0/src/learned_bias.rs`, containing numeric event/curve state, cap-clamped updates, effective scalar composition, and binary checkpoint helpers. Integrate the state into `Viewport` and `snapshot` only as a numeric substrate. Do not wire VIB/PHOT/language/action loops as automatic producers in this plan.

**Tech Stack:** Rust, Cargo tests, existing `BiasPriors` / `BiasAxis`, existing snapshot pattern.

---

## Chunk 1: Pure Learned-Bias Substrate

### Task 1: Add learned-bias events, curves, and cap-clamped updates

**Files:**
- Create: `bionic_v0/src/learned_bias.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/learned_bias.rs`

- [ ] **Step 1: Write failing tests**

Create `bionic_v0/tests/learned_bias.rs` with tests for base separation,
protected-axis clamping, support/deviation sedimentation, and effective scalar
composition.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: compile FAIL because `learned_bias` module does not exist.

- [ ] **Step 3: Implement minimal module**

Add:

- `BiasLocator`
- `LearnedBiasEventInput`
- `LearnedBiasEvent`
- `LearnedBiasCurve`
- `LearnedBiasState`
- `LearnedBiasState::apply_event`
- `LearnedBiasState::effective_scalar`

Rules:

- all fields numeric/enums only;
- every event calls `priors.clamp_modification`;
- support applies `+applied_delta`;
- deviation applies `-applied_delta.abs()`;
- state keeps bounded recent events and curves;
- no string fields.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: PASS.

## Chunk 2: Binary Checkpoint Persistence

### Task 2: Add checkpoint roundtrip and version rejection

**Files:**
- Modify: `bionic_v0/src/learned_bias.rs`
- Test: `bionic_v0/tests/learned_bias.rs`

- [ ] **Step 1: Add failing checkpoint tests**

Append tests for checkpoint roundtrip and unknown-version rejection.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: compile FAIL because checkpoint APIs do not exist.

- [ ] **Step 3: Implement checkpoint helpers**

Add:

- `LEARNED_BIAS_MAGIC`
- `LEARNED_BIAS_VERSION`
- `LearnedBiasCheckpointPaths`
- `write_learned_bias_checkpoint`
- `read_learned_bias_checkpoint`

Use binary little-endian fields and atomic tmp-write/rename pattern. Reject
wrong magic, wrong version, and truncated payloads.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: PASS.

## Chunk 3: Viewport And Snapshot Integration

### Task 3: Add viewport ownership and snapshot summary

**Files:**
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/learned_bias.rs`

- [ ] **Step 1: Add failing viewport/snapshot tests**

Append tests for:

- `Viewport::new()` starts with empty learned-bias state.
- snapshot roundtrip exposes numeric learned-bias summary.

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: compile FAIL because viewport/snapshot APIs do not exist.

- [ ] **Step 3: Implement viewport and snapshot wiring**

Add:

- `pub mod learned_bias;`
- `SNAPSHOT_VERSION` bump to `37`
- `Viewport.learned_bias: RwLock<LearnedBiasState>`
- `Viewport::read_learned_bias`
- `Viewport::apply_learned_bias_event`
- `SnapshotLearnedBiasSummary`
- write/read learned-bias summary after vocal substrate in snapshot payload

Use `BiasPriors::default()` inside `apply_learned_bias_event` for now. Runtime
caller-specific priors integration is deferred.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: PASS.

## Chunk 4: Hygiene And Full Verification

### Task 4: Guard highest-principle boundaries

**Files:**
- Modify: `bionic_v0/tests/learned_bias.rs`

- [ ] **Step 1: Add hygiene test**

Append a source hygiene test checking `src/learned_bias.rs` contains no rejected
architecture fields and no body-side print macros.

- [ ] **Step 2: Run targeted tests**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test --test learned_bias
```

Expected: PASS.

- [ ] **Step 3: Run full suite**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence/bionic_v0
cargo test
```

Expected: PASS.

- [ ] **Step 4: Review diff**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/learned-bias-delta-persistence
git diff -- docs/superpowers/specs/2026-05-06-learned-bias-delta-persistence-design.md docs/superpowers/plans/2026-05-06-learned-bias-delta-persistence-tdd.md bionic_v0/src/learned_bias.rs bionic_v0/src/lib.rs bionic_v0/src/viewport.rs bionic_v0/src/snapshot.rs bionic_v0/tests/learned_bias.rs
```

Expected: diff contains only learned-bias substrate, persistence, viewport/snapshot summary, and docs/plan changes.
