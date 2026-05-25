# Solitary And Live Closure Training Runbook

This runbook is observer-side. It does not create a body-side training phase
module, command path, speech recognizer, image recognizer, teacher model, or
chat layer.

## Boundary

`Solitary Room`, `Live I/O`, `Nursery Turn-Taking`, `Mark/Text Grounding`,
`fire`, `wood`, `mark`, `text`, `communication`, and `training` are external
runbook and observer descriptions. They are not body ontology units.

The body-side substrate must remain numeric curves, pressure, recurrence,
write epochs, return windows, temporal pairing, closure, residue, reentry,
and bounded traces. This runbook must not introduce token, semantic, concept,
identity, owner, personality, or natural-language-understanding gates.

## One-Shot Runner

Use the observer-side runner to collect the current numeric gate state:

```bash
RUN=/tmp/unnamed_live_train
cargo run --manifest-path bionic_v0/Cargo.toml --bin live_training_runner -- \
  "$RUN" \
  --solitary-secs 1800 \
  --live-secs 600 \
  --sample-interval-secs 30
```

The runner starts `body` with `--solitary-low-power`, samples existing
`curriculum_report` modes, writes reports under `runner_samples/`, and fails if
`body.stdout` or `body.stderr` is non-empty.

## Phase 1: Solitary Room Training

Run the body in a fixed room for 10-30 minutes with only low-change physical
ingress: room light, microphone floor, screen boundaries, and small incidental
changes.

Manual minimal commands:

```bash
RUN=/tmp/unnamed_live_train
cargo run --manifest-path bionic_v0/Cargo.toml --bin body -- "$RUN" 300 --solitary-low-power
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --pre-language-solitary-growth "$RUN"
```

Required evidence:

```text
runtime_snapshot_read=1
prelingual_growth_gate_pass=1
stable_recurrence > cold baseline
room_familiarity > cold baseline
solitary_growth_score > cold baseline
```

This gate is wood-focused: self-maintaining growth while alone. It is not a
word-learning gate.

## Phase 2: Live I/O Training

Keep the runtime live. A human provides simple physical events only:

- short sound, long sound;
- high contour, low contour;
- bright point, dim point;
- object moves, object stops.

Required evidence from the nursery report:

```text
runtime_snapshot_read=1
phot_structure_observed=1
vocal_write_epochs>=1
return_windows>=1
temporal_pairing_observed=1
```

This gate is fire-focused: pressure becomes write-out and return evidence. It
does not claim understanding.

## Phase 3: Nursery Turn-Taking

The human performs one simple event, waits for body output or pressure change,
then repeats or corrects timing/contour. Keep events simple and repeated.

Required evidence:

```text
closure_or_reentry_observed=1
fire_live_closure_gate_pass=1
```

`communication_candidate=0` is acceptable here. This phase is only looking for
live closure or reentry.

## Phase 4: Mark/Text Grounding

Enter this phase only after the previous gates pass. Use dots, lines, blocks,
and character-shaped marks as external disturbances tied to already-visible
object, relation, and time-window traces.

This phase still does not claim natural language understanding. A mark is an
external disturbance shape until observer-side evidence shows it binds to
existing numeric substrate traces through legal windows.

## Stop Conditions

Stop the run if any of these occur:

- `body.stdout` or `body.stderr` is non-empty;
- fixture evidence is treated as live evidence;
- write epochs appear without legal return-window evidence but are still
  counted as closure;
- mark/text work starts before solitary and live closure gates pass;
- success depends on human feeling instead of snapshot/report evidence.
