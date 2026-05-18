# Live Human Nursery v0 Runbook

This runbook is observer-side. It does not create a body-side teaching module,
chat interface, speech recognizer, image recognizer, or command path.

## Deterministic Gate

Before any live teaching session, run the focused deterministic checks:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight
cargo test --manifest-path bionic_v0/Cargo.toml --test auditory_visual
cargo test --manifest-path bionic_v0/Cargo.toml --test surface_projection --test surface_render
cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output
```

Also run the nursery evidence gates:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_human_nursery
cargo test --manifest-path bionic_v0/Cargo.toml --test curriculum_report live_human_nursery_report_emits_numeric_evidence
```

## Live Startup

Use separate terminals from `/Users/joe/unnamed`:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin body -- runtime
```

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin observer -- runtime 20
```

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin observer_surface -- runtime
```

Nursery evidence report:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --live-human-nursery runtime
```

## Teaching Sequence

Keep the first session short. Use simple, repeatable events.

### Phase 0: Life Signs

- Make a short sound near the microphone.
- Move one simple object or bright block in front of the camera.
- Watch PHOT health, VIB activity, snapshot writes, and the pixel surface.

Expected observer signs:

- PHOT health is visible as a state, not silent;
- VIB energy or windows respond to sound;
- snapshot sequence increases;
- surface is nonblank when pressure exists.

### Phase 1: Vocal Contours

- Produce high then low vocal contours.
- Produce short then long contours.
- Repeat a two-pulse contour.
- Pause and repeat the same contour later.

Expected observer signs:

- vocal target energy changes after external sound;
- vocal organ activity may attempt output;
- imitation pressure changes with gap;
- repeated contours improve reusable vocal traces if feedback is available.

### Phase 2: Visual-Audio Coupling

- Pair a visible bright/moving mark with one contour.
- Pair a still or dim mark with a different contour.
- Deliberately misalign one visible event and sound.

Expected observer signs:

- PHOT slots and spatial regions appear;
- aligned VIB/PHOT activity raises coupling evidence;
- misalignment raises drift or ambiguity instead of false certainty.

### Phase 3: Turn-Taking And Repair

- Act, pause, wait for body output or surface pressure change.
- Repeat if the reaction is weak.
- Correct by changing timing or contour.

Expected observer signs:

- action-output epochs appear when body output occurs;
- consequence windows pair only when legal;
- closure lowers pressure;
- reentry restores traces better than cold exposure.

## Stop Conditions

Stop the live session if:

- body emits human-readable diagnostics;
- camera or microphone failure is silent;
- output occurs without write epoch evidence;
- pairs form without earlier write evidence;
- success can only be judged by human feeling and not snapshot evidence;
- the session becomes complex natural conversation before phases 0-3 have
  numeric evidence.

## Reporting Rule

Do not claim communication just because the reaction looks convincing.

Use the report gate:

```text
deterministic_fixture=0
runtime_snapshot_read=1
communication_candidate=1
```

Even then, treat it as an early candidate only. It does not mean adult language,
child-level reasoning, or stable understanding.
