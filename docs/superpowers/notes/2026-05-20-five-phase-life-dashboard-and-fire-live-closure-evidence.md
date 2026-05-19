# Five-Phase Life Dashboard And Fire Live Closure Evidence

Date: 2026-05-20

## Boundary

This evidence is observer-side only. Five-phase force names are report and
acceptance vocabulary. Body-side state remains numeric substrate curves and
does not store wood/fire/earth/metal/water ontology.

## Commands

```bash
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --test five_phase_life
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --test curriculum_report five_phase_life_report_emits_balance_evidence
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --test curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --test live_human_nursery
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --test self_development_life_loop
cargo run --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --bin curriculum_report -- --five-phase-life
cargo run --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml --bin curriculum_report -- --fire-live-closure
cargo test --manifest-path /Users/joe/unnamed/.worktrees/five-phase-life-dashboard/bionic_v0/Cargo.toml
git diff --check
```

## Test Results

- `five_phase_life`: 6 passed.
- `curriculum_report five_phase_life_report_emits_balance_evidence`: 1 passed.
- `curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass`: 1 passed.
- `live_human_nursery`: 4 passed.
- `self_development_life_loop`: 8 passed.
- Full `cargo test`: passed all unit, integration, and doc tests; one existing
  `append_session_row` dead-code warning was emitted by
  `active_gap_observer_evidence`.
- `git diff --check`: no whitespace errors.

## Numeric Report

`--five-phase-life`:

```text
mode=five_phase_life
deterministic_fixture=1
runtime_snapshot_read=0
snapshots_seen=3
wood_balance=0.000000
fire_balance=0.500000
earth_balance=0.600000
metal_balance=0.000000
water_balance=0.400000
fire_live_closure_gate_pass=0
wood_fire_non_suppression=1
principle_guard_pass=1
```

`--fire-live-closure`:

```text
mode=fire_live_closure
deterministic_fixture=1
runtime_snapshot_read=0
snapshots_seen=3
wood_balance=0.000000
fire_balance=0.500000
earth_balance=0.600000
metal_balance=0.000000
water_balance=0.400000
fire_live_closure_gate_pass=0
wood_fire_non_suppression=1
principle_guard_pass=1
```

## Acceptance

- Five bounded force balances are reported.
- Deterministic fixture evidence does not pass the fire live-closure gate.
- Source guards reject rejected ontology and body-readable output.
- Runtime acceptance remains pending until a real runtime snapshot is supplied.
