# Persistent Curriculum Evidence Handoff

Date: 2026-05-15
Status: accepted observer-side evidence pass

## Boundary

This note records observer-side training evidence only. `training`,
`curriculum`, `persistent`, `hold`, `after`, and `reentry` are external
research descriptions of a harness protocol. They are not body ontology units,
not meanings, not labels, not identities, and not request/response control.

The implementation remains outside `body`. It drives existing `Viewport`,
action-output, action-consequence, learned-bias, and adaptive-stability
substrate APIs and reports numeric evidence through `curriculum_report`.

## Acceptance Target

The accepted target was to move persistent curriculum from "can run" to
observer-visible evidence for:

- long-term evidence retention after idle time;
- related after-stage transfer against a cold baseline;
- reentry strength after hold against a cold baseline;
- hygiene against rejected architecture drift.

## Evidence Command

```bash
/Users/joe/unnamed/bionic_v0/target/debug/curriculum_report --long-retention
```

Observed output:

```text
curriculum_report_v0
mode=long_retention
train_episodes=6
after_episodes=3
reentry_episodes=3
train_pairs=6
train_learned_events=6
train_learned_delta=0.300000
train_adaptive_routes=6
hold_learned_delta=0.300000
hold_adaptive_routes=6
hold_learned_delta_ratio=1.000000
hold_adaptive_route_ratio=1.000000
after_persistent_pairs=3
after_cold_pairs=3
after_persistent_learned_delta=0.450000
after_cold_learned_delta=0.150000
reentry_persistent_pairs=3
reentry_cold_pairs=3
reentry_persistent_learned_delta=0.600000
reentry_cold_learned_delta=0.150000
```

## Interpretation

- Train formed repeated closure: `train_pairs=6`,
  `train_learned_events=6`, `train_adaptive_routes=6`.
- Hold preserved observer-visible evidence:
  `hold_learned_delta_ratio=1.000000` and
  `hold_adaptive_route_ratio=1.000000`.
- Related after-stage perturbation retained stronger evidence than cold:
  `0.450000` persistent vs `0.150000` cold.
- Reentry after hold closed stronger than cold:
  `0.600000` persistent vs `0.150000` cold.

This is not a claim of world understanding. It is numeric evidence that the
same body substrate carries learned pressure traces across train, idle hold,
related after perturbation, and reentry.

## Verification

Targeted tests:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_runner -- --nocapture
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report -- --nocapture
```

Full acceptance verification:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```
