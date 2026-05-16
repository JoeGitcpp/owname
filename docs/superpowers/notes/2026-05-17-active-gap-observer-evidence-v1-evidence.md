# Active Gap Observer Evidence v1 Evidence

Implemented observer-visible active-gap evidence curves.

## Boundary

This is observer evidence only. The body writes numeric/time state into the
viewport and snapshot. Observer helpers render columns and do not recompute gap
pressure, closure, re-entry, or cycle counts.

No personality, owner, source identity, token, label, name, concept, prompt
answer, or body-side human-readable output was added.

## RED

Initial focused verification failed before implementation was complete:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_gap_observer_evidence --test observer_compact
```

Failure:

- `active_reasoning.rs` had a borrow error while appending a trace sample.
- Observer compact/session helpers did not yet expose active-gap columns.

Full verification then exposed an additional snapshot fixture gap:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Failure:

- `tests/surface_projection.rs` manually constructed `SnapshotData` without the
  new `active_reasoning_gap` field.

## GREEN

Focused verification:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_reasoning_persistence --test active_gap_observer_evidence --test observer_compact
```

Result:

- `active_gap_observer_evidence`: 3 passed
- `active_reasoning_persistence`: 5 passed
- `observer_compact`: 1 passed

Full verification:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Result:

- all crate tests and doc-tests passed
- 0 failed
- one existing warning remains from the test-local inclusion of
  `observer_helpers/session.rs`: `append_session_row` is unused in that test
  compilation unit

Diff hygiene:

```bash
git diff --check
```

Result: clean.

Format check:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Result: clean after applying fmt-only hygiene to three pre-existing test diffs:

- `bionic_v0/tests/adaptive_stability.rs`
- `bionic_v0/tests/after_window_diagnosis.rs`
- `bionic_v0/tests/architecture_contraction.rs`

## Landed Behavior

- `ActiveReasoningGapState` trace append path compiles and records samples.
- Snapshot v46 writes and reads current active-gap state plus the trace ring.
- Compact observer rows expose `gap_p`, `gap_c`, `gap_r`, `gap_cycles`, and
  `gap_samples`.
- Session CSV rows expose the same long-run curve columns.
- Numeric focus indices are included beside the curve columns as engineering
  locators for observer-side correlation only.
- `body` refreshes active gap after reasoning history/evidence tension and
  before probe law reads pressure.
- The global architecture map now names snapshot v46 and active-gap observer
  evidence.
