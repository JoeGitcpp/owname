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

## Runtime Fire Closure Evidence

After the five-phase dashboard merged to `main`, the existing live body and
byte-loopback path was exercised as a real runtime snapshot source.

```bash
DIR=$(mktemp -d /tmp/bionic_fire_live.XXXXXX)
mkdir -p "$DIR/live_bytes"
/Users/joe/unnamed/bionic_v0/target/debug/body "$DIR" 5 >"$DIR/body.stdout" 2>"$DIR/body.stderr" &
BODY_PID=$!
sleep 0.5
printf 'abcabcabc001001xyzxyz' >> "$DIR/live_bytes/in.frames"
/Users/joe/unnamed/bionic_v0/target/debug/byte_loopback "$DIR" 4 >"$DIR/loop.stdout" 2>"$DIR/loop.stderr" &
LOOP_PID=$!
wait "$BODY_PID"
wait "$LOOP_PID"
cargo run --quiet --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --fire-live-closure "$DIR"
```

Observed report:

```text
curriculum_report_v0
mode=fire_live_closure
deterministic_fixture=0
runtime_snapshot_read=1
snapshots_seen=1
wood_balance=0.456850
fire_balance=1.000000
earth_balance=0.006000
metal_balance=0.540556
water_balance=0.000001
wood_insufficient=0
fire_insufficient=0
earth_insufficient=1
metal_insufficient=0
water_insufficient=1
wood_excess=0
fire_excess=1
earth_excess=0
metal_excess=0
water_excess=0
fire_live_closure_gate_pass=1
wood_fire_non_suppression=1
principle_guard_pass=1
```

This proves the fire gate can be satisfied by a real runtime `snapshot.bin`
produced through body write-out, byte-loopback return, temporal pairing, and
closure/reentry evidence. The runtime artifacts are intentionally kept under
`/tmp` and are not tracked.

## Acceptance

- Five bounded force balances are reported.
- Deterministic fixture evidence does not pass the fire live-closure gate.
- Real runtime snapshot evidence passes the fire live-closure gate.
- Source guards reject rejected ontology and body-readable output.
