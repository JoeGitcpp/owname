# Active Reasoning Persistence v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement numeric active-gap persistence so unclosed reasoning pressure can persist, drive probe output, close through paired return evidence, and re-enter from history.

**Architecture:** Add `active_reasoning.rs` as a focused Viewport extension. Store one `ActiveReasoningGapState` in `Viewport`, feed its pressure into existing `probe_law`, and expose it through a read API for tests and observer-side evidence. Do not add personality, identity, goal, language semantics, or a new output path.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `Viewport`, `probe_law`, `action_output`, and `action_consequence` substrates.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-self-as-persistent-active-gap-boundary-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-active-reasoning-persistence-v0-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-21-endogenous-activity-and-probe-laws-design.md`
5. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md`

Hard bans:

- no personality module or parameter bundle;
- no internal self/identity/owner/ethics fields;
- no goal planner or task queue;
- no source identity;
- no prompt-answer or language semantics;
- no body-side `println!` or `eprintln!`.

## Files

- Create: `bionic_v0/src/active_reasoning.rs`
  - `ActiveReasoningGapState`
  - `Viewport::refresh_active_reasoning_gap`
  - `Viewport::read_active_reasoning_gap`
- Modify: `bionic_v0/src/viewport.rs`
  - import `ActiveReasoningGapState`;
  - add `active_reasoning_gap: RwLock<ActiveReasoningGapState>`;
  - initialize it in `Viewport::new`.
- Modify: `bionic_v0/src/lib.rs`
  - export `active_reasoning`.
- Modify: `bionic_v0/src/probe_law.rs`
  - include active-gap pressure in existing probe drive.
- Create: `bionic_v0/tests/active_reasoning_persistence.rs`
  - behavior tests and source hygiene.
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-active-reasoning-persistence-v0-evidence.md`

## Task 1: RED Active Gap Persistence And Probe

**Files:**

- Create: `bionic_v0/tests/active_reasoning_persistence.rs`

- [ ] **Step 1: Write failing test**

Expected behavior:

```rust
#[test]
fn active_gap_persists_after_pressure_falls_and_drives_probe() {
    let vp = Viewport::new();
    seed_reasoning_pressure(&vp, 0.8, 0.6, 2, 3);

    for t in [1_000_000, 1_010_000, 1_020_000] {
        vp.refresh_active_reasoning_gap(t);
    }
    let active = vp.read_active_reasoning_gap();
    assert!(active.gap_pressure > 0.2);
    assert!(active.persistence_score > 0.0);

    seed_reasoning_pressure(&vp, 0.0, 0.0, 2, 3);
    vp.refresh_active_reasoning_gap(1_030_000);
    let carried = vp.read_active_reasoning_gap();
    assert!(carried.gap_pressure > 0.05);

    vp.refresh_probe_law(1_040_000);
    vp.tick_probe_output_from_law(1_040_000);
    assert!(vp.read_action_output().n_write_epochs > 0);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence active_gap_persists_after_pressure_falls_and_drives_probe
```

Expected: compile failure because active reasoning module/API does not exist.

## Task 2: RED Closure And Re-Entry

**Files:**

- Modify: `bionic_v0/tests/active_reasoning_persistence.rs`

- [ ] **Step 1: Add closure test**

Expected behavior:

- form a gap;
- create a legal write epoch and consequence window with similar magnitude;
- pair them;
- refresh active gap;
- assert pressure drops and closure score rises.

- [ ] **Step 2: Add re-entry test**

Expected behavior:

- seed a weak prior gap;
- set reasoning memory recurrence and persistence high;
- refresh active gap;
- assert re-entry score rises and pressure increases.

- [ ] **Step 3: Add source hygiene test**

Scan new source plus `probe_law.rs` for rejected terms and body output drift.

## Task 3: Implement Active Reasoning Gap

**Files:**

- Create: `bionic_v0/src/active_reasoning.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add state**

Fields:

- `gap_pressure: f64`
- `drive_score: f64`
- `persistence_score: f64`
- `closure_score: f64`
- `reentry_score: f64`
- `focus_slot_id: i8`
- `focus_edge_id: i16`
- `active_cycles: u32`
- `last_pair_seq: u64`
- `last_update_t_us: u64`

All bounded scores clamp to `[0,1]`.

- [ ] **Step 2: Add refresh law**

Inputs:

- current reasoning surprise and instability;
- memory novelty, persistence, recurrence, dominant focus persistence;
- newest action-consequence pair similarity.

Rules:

- drive raises pressure;
- carried pressure decays slowly;
- paired return similarity reduces pressure;
- recurrent history can re-enter a weak gap.

- [ ] **Step 3: Add read API**

`read_active_reasoning_gap` returns a copy of state.

## Task 4: Feed Probe Law

**Files:**

- Modify: `bionic_v0/src/probe_law.rs`

- [ ] **Step 1: Include active-gap pressure**

Read `self.read_active_reasoning_gap().gap_pressure` and include it in the
existing max-of-pressure drive. Keep output path unchanged.

- [ ] **Step 2: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence
```

Expected: all active reasoning persistence tests pass.

## Task 5: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-active-reasoning-persistence-v0-evidence.md`

- [ ] **Step 1: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence --test probe_tick --test endogenous_activity
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
