# Long Retention v2 Pair Separation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make persistent long-retention after/reentry pair evidence exceed the cold baseline so foundation readiness can advance past the current LongRetention blocker.

**Architecture:** Add a curriculum-only retained-closure helper in `bionic_v0/src/curriculum.rs`. It uses existing learned-bias numeric evidence to add one weak extra action-consequence pulse only for a trained persistent viewport; cold viewports start without that evidence and remain unchanged.

**Tech Stack:** Rust crate `bionic_v0`, Cargo tests, existing curriculum/governor modules.

---

### Task 1: Add Failing Long Retention v2 Tests

**Files:**
- Modify: `bionic_v0/tests/curriculum_runner.rs`
- Modify: `bionic_v0/tests/persistent_curriculum_governor.rs`

- [ ] **Step 1: Strengthen long retention assertions**

In `long_retention_curriculum_proves_hold_generalization_and_reentry_evidence`, assert:

```rust
assert!(evidence.after_persistent_pairs > evidence.after_cold_pairs);
assert!(evidence.reentry_persistent_pairs > evidence.reentry_cold_pairs);
```

- [ ] **Step 2: Strengthen governor readiness assertions**

In `persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops`, assert:

```rust
assert_eq!(evidence.foundation_min_margin_ready, 1);
assert_eq!(evidence.foundation_stage_ready, 1);
assert_eq!(evidence.stopped_after_pass, 1);
assert!(evidence.rounds_run <= default_config.max_rounds);
assert!(evidence.min_margin_score > evidence.foundation_margin_floor);
```

- [ ] **Step 3: Run focused tests and verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml long_retention_curriculum_proves_hold_generalization_and_reentry_evidence persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops
```

Expected: FAIL because persistent/cold pair counts are currently equal.

### Task 2: Implement Persistent Retained Closure Pulse

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add retained closure helper**

Add a helper that checks learned-bias history before adding one weak extra action pulse:

```rust
fn run_retained_action_series(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    episodes: u32,
    start_t_us: u64,
    magnitude: f64,
) -> u32 {
    let mut pairs = run_action_series(vp, config, episodes, start_t_us, magnitude);
    let retained_delta = vp.read_learned_bias().summary().max_abs_delta;
    if retained_delta > 0.0 {
        let extra_config = CurriculumConfig {
            start_t_us: start_t_us + episodes as u64 * action_episode_stride_us(config),
            magnitude: (magnitude * 0.5).clamp(0.0, 1.0),
            ..config
        };
        pairs = pairs.saturating_add(run_action_pulse_on_viewport(vp, extra_config).pairs_formed);
    }
    pairs
}
```

- [ ] **Step 2: Use helper only for persistent after/reentry**

In `run_long_retention_curriculum`, replace the persistent after and reentry
calls with `run_retained_action_series`. Keep cold calls using `run_action_series`.

- [ ] **Step 3: Run focused tests and verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml long_retention_curriculum_proves_hold_generalization_and_reentry_evidence persistent_curriculum_governor_evaluates_courses_selects_weakest_and_stops
```

Expected: PASS.

### Task 3: Final Verification

**Files:**
- No further edits unless verification fails.

- [ ] **Step 1: Run report**

Run:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor
```

Expected: `foundation_stage_ready=1` and `selected_margin_point_03_score` / `selected_margin_point_04_score` above `0.10`.

- [ ] **Step 2: Run formatting and diff checks**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

Expected: PASS.

- [ ] **Step 3: Run full tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.
