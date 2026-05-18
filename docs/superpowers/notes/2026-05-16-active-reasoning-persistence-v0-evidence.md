# Active Reasoning Persistence v0 Evidence

Date: 2026-05-16.

## Scope

Implemented runtime numeric active-gap persistence for active reasoning.

This work follows:

- `docs/superpowers/specs/2026-05-16-self-as-persistent-active-gap-boundary-design.md`
- `docs/superpowers/specs/2026-05-16-active-reasoning-persistence-v0-design.md`

The implementation adds a numeric active-gap substrate. It does not add a
personality module, identity field, fixed goal system, planner, language
semantics, or body-side human-readable output.

## RED

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence active_gap_persists_after_pressure_falls_and_drives_probe
```

Result: failed to compile because:

- `bionic_v0/src/active_reasoning.rs` did not exist;
- `Viewport::refresh_active_reasoning_gap` did not exist;
- `Viewport::read_active_reasoning_gap` did not exist;
- `Viewport` did not expose active-gap state.

## GREEN

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence
```

Result: 4 passed, 0 failed.

Tests prove:

- active gap persists after immediate pressure falls;
- active gap pressure drives existing probe output;
- paired return evidence reduces active gap pressure;
- reasoning history can re-enter a weak active gap;
- source hygiene rejects rejected ontology and body-output drift.

## Focused Regression

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence --test probe_tick --test endogenous_activity
```

Result:

- `active_reasoning_persistence.rs`: 4 passed;
- `endogenous_activity.rs`: 7 passed;
- `probe_tick.rs`: 8 passed.

## Full Verification

Command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Result: passed, including doc tests.

Diff hygiene:

```bash
git diff --check
```

Result: passed.

Formatting:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Result: failed only on pre-existing unrelated formatting differences:

- `/Users/joe/unnamed/bionic_v0/tests/adaptive_stability.rs:82`
- `/Users/joe/unnamed/bionic_v0/tests/after_window_diagnosis.rs:71`
- `/Users/joe/unnamed/bionic_v0/tests/architecture_contraction.rs:124`

Touched Rust files were formatted with `rustfmt`.
