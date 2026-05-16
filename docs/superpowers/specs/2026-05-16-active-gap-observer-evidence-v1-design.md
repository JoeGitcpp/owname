# Active Gap Observer Evidence v1 Design

Status: Draft for TDD execution on 2026-05-16.

## Purpose

Active Reasoning Persistence v0 proves that an active gap can form, persist,
drive probe pressure, close through paired return evidence, and re-enter from
history. The gap is still mostly runtime-local: observer tooling cannot yet
watch its curve over time.

Active Gap Observer Evidence v1 makes that runtime substrate observable as
numeric curves:

- active gap pressure;
- closure score;
- re-entry score;
- active cycle count.

The observer should be able to read these values from snapshots and session CSV
rows without recomputing them.

## Architecture

Extend `ActiveReasoningGapState` with a fixed-capacity trace ring.

Each refresh appends an `ActiveReasoningGapTrace` containing:

- `t_us`;
- `gap_pressure`;
- `closure_score`;
- `reentry_score`;
- `active_cycles`.

Snapshot v46 serializes the current active gap state plus trace ring.

The compact observer and session CSV append active-gap columns:

- `gap_p`
- `gap_c`
- `gap_r`
- `gap_cycles`
- `gap_samples`

The observer does not compute these values. It only renders what the body wrote
into the snapshot.

## Data Flow

`Viewport::refresh_active_reasoning_gap` updates the current active-gap state
and appends one trace sample.

`body::run_global_refresh_cycle` runs active-gap refresh after reasoning history
and evidence-tension updates, before probe-law refresh. This gives probe law the
current gap pressure.

`write_snapshot` writes active-gap evidence after the existing v45 sections.

`read_snapshot` reads the same fields into `SnapshotData.active_reasoning_gap`.

Observer helpers append current active-gap values to their rows. Session CSVs
then become the long-run curve carrier.

## High-Risk Term Boundary

`observer` means external research rendering. It does not compute body state.

`gap` means numeric unclosed prediction pressure, not identity, self, desire,
need, or goal.

`curve` means a numeric trace over time, not a semantic narrative.

No implementation may add personality, owner, ethics, source identity, concept
ids, word ids, token ids, labels, names, prompt answers, or body-side
human-readable output.

## Acceptance

Tests must prove:

1. active-gap refresh appends bounded trace samples;
2. trace samples preserve gap pressure, closure, re-entry, and active cycles;
3. snapshot roundtrip preserves active-gap current state and trace samples;
4. observer compact/session headers and rows expose active-gap numeric columns;
5. body refresh runs active-gap refresh before probe law can read its pressure;
6. source hygiene still rejects rejected ontology and body output drift;
7. focused and full crate tests pass.

## Non-Goals

- No new learning rule for active gaps.
- No persistent personality/self field.
- No target planner.
- No semantic interpretation of gap curves.
- No new output channel.
- No background automation restart.
