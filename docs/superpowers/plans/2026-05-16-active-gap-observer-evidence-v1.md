# Active Gap Observer Evidence v1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make active-gap pressure, closure, re-entry, and active-cycle evidence observable as long-run numeric curves.

**Architecture:** Extend active reasoning with a fixed trace ring, serialize it in snapshot v46, and append observer compact/session columns. Keep observer rendering read-only and keep probe output on the existing path.

**Tech Stack:** Rust crate `bionic_v0`, binary observer helpers, snapshot binary format, Cargo integration tests.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-self-as-persistent-active-gap-boundary-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-active-reasoning-persistence-v0-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-active-gap-observer-evidence-v1-design.md`

Hard bans:

- no personality/self/identity fields;
- no owner, ethics, or source identity;
- no names, labels, tokens, words, concepts, or semantic gates;
- no observer-side recomputation of active-gap evidence;
- no body-side `println!` or `eprintln!`.

## Files

- Modify: `bionic_v0/src/active_reasoning.rs`
  - add `ActiveReasoningGapTrace`;
  - add trace ring fields to `ActiveReasoningGapState`;
  - append one sample per refresh.
- Modify: `bionic_v0/src/lib.rs`
  - add `ACTIVE_REASONING_GAP_TRACE_CAP`;
  - bump snapshot version to 46 and update comments.
- Modify: `bionic_v0/src/snapshot.rs`
  - serialize and deserialize active-gap state.
- Modify: `bionic_v0/src/bin/body.rs`
  - call `refresh_active_reasoning_gap` in the serial refresh cycle before probe law.
- Modify: `bionic_v0/src/bin/observer_helpers/compact.rs`
  - append active-gap columns.
- Modify: `bionic_v0/src/bin/observer_helpers/session.rs`
  - append active-gap columns.
- Modify: `docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md`
  - mention snapshot v46.
- Modify/create tests:
  - `bionic_v0/tests/active_reasoning_persistence.rs`
  - `bionic_v0/tests/active_gap_observer_evidence.rs`
  - `bionic_v0/tests/observer_compact.rs`
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-active-gap-observer-evidence-v1-evidence.md`

## Task 1: RED Active-Gap Trace Ring

**Files:**

- Modify: `bionic_v0/tests/active_reasoning_persistence.rs`

- [ ] **Step 1: Add failing trace assertions**

Expect `read_active_reasoning_gap()` to expose:

- `history_len`
- `total_samples`
- `recent_trace(0)`

and verify the recent trace contains bounded `gap_pressure`, `closure_score`,
`reentry_score`, and `active_cycles`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence active_gap_trace_records_observer_curve_samples
```

Expected: compile failure because trace fields/API do not exist.

## Task 2: RED Snapshot And Observer Tests

**Files:**

- Create: `bionic_v0/tests/active_gap_observer_evidence.rs`
- Modify: `bionic_v0/tests/observer_compact.rs`

- [ ] **Step 1: Add snapshot roundtrip test**

Create a viewport, refresh active gap across several cycles, write snapshot,
read snapshot, and assert:

- `snap.active_reasoning_gap.gap_pressure > 0.0`
- `snap.active_reasoning_gap.history_len > 0`
- latest trace has active cycle and gap evidence.

- [ ] **Step 2: Add observer helper test**

Assert compact/session headers contain `gap_p`, `gap_c`, `gap_r`,
`gap_cycles`, `gap_samples`, and rows include active-gap values.

- [ ] **Step 3: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_gap_observer_evidence --test observer_compact
```

Expected: fail until snapshot/observer fields are implemented.

## Task 3: Implement Trace Ring

**Files:**

- Modify: `bionic_v0/src/active_reasoning.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add trace struct and cap**

Use fixed cap `ACTIVE_REASONING_GAP_TRACE_CAP`.

- [ ] **Step 2: Append sample per refresh**

Push after current scores are updated. Keep eviction ring-based and numeric.

- [ ] **Step 3: Add recent trace accessor**

Expose `recent_trace(i)` on `ActiveReasoningGapState` for tests and snapshot
readers.

## Task 4: Implement Snapshot v46

**Files:**

- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md`

- [ ] **Step 1: Add active-gap field to `SnapshotData`**

- [ ] **Step 2: Write active-gap state**

- [ ] **Step 3: Read active-gap state**

- [ ] **Step 4: Bump version and map**

Set snapshot version to 46 and update architecture map text.

## Task 5: Implement Observer Columns And Body Wiring

**Files:**

- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/bin/observer_helpers/compact.rs`
- Modify: `bionic_v0/src/bin/observer_helpers/session.rs`

- [ ] **Step 1: Body refresh**

Call `refresh_active_reasoning_gap` before `refresh_probe_law`.

- [ ] **Step 2: Compact row columns**

Append current active-gap numeric values.

- [ ] **Step 3: Session row columns**

Append the same fields so CSV rows form long-run curves.

## Task 6: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-active-gap-observer-evidence-v1-evidence.md`

- [ ] **Step 1: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence --test active_gap_observer_evidence --test observer_compact
```

- [ ] **Step 2: Run full verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

- [ ] **Step 3: Record evidence**

Record RED/GREEN, focused/full results, diff hygiene, and any pre-existing
formatting issue outside touched files.
