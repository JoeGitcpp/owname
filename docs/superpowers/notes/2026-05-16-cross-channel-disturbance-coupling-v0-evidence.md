# Cross-Channel Disturbance Coupling v0 Evidence

Date: 2026-05-16

## Scope

Implemented observer-side `Cross-Channel Disturbance Coupling v0` for
`bionic_v0`.

This stage proves that text mark disturbance, vib disturbance, and phot
disturbance can accumulate compatible numeric binding curves against the same
existing substrate targets, then survive conflict/correction and reenter better
than cold baseline.

No body-side output path, request/response layer, naming gate, concept
allocator, parser, tokenizer, dictionary, visual labeler, voice labeler,
identity field, owner field, ethics field, or personality field was added.

## Files

- Created `docs/superpowers/specs/2026-05-16-cross-channel-disturbance-coupling-v0-design.md`
- Created `docs/superpowers/plans/2026-05-16-cross-channel-disturbance-coupling-v0.md`
- Created `bionic_v0/tests/cross_channel_disturbance_coupling.rs`
- Modified `bionic_v0/src/curriculum.rs`
- Modified `bionic_v0/src/bin/curriculum_report.rs`
- Modified `bionic_v0/tests/curriculum_report.rs`

## TDD Evidence

RED behavior test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling cross_channel_disturbance_coupling_proves_shared_targets_conflict_recovery_and_reentry
```

Expected failure observed:

```text
error[E0432]: unresolved import `bionic_v0::curriculum::run_cross_channel_disturbance_coupling_curriculum`
```

RED report test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report cross_channel_disturbance_coupling_report_emits_numeric_stage_evidence
```

Expected failure observed:

```text
assertion failed: stdout.contains("mode=cross_channel_disturbance_coupling")
```

GREEN focused test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling --test curriculum_report
```

Observed:

```text
tests/cross_channel_disturbance_coupling.rs: 2 passed
tests/curriculum_report.rs: 11 passed
```

## Report Evidence

Command:

```text
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --cross-channel-disturbance-coupling
```

Observed:

```text
curriculum_report_v0
mode=cross_channel_disturbance_coupling
stream_events=26
text_marks=4
vib_observations=16
phot_worldline_updates=13
object_targets=3
relation_targets=2
time_window_targets=2
text_binding_strength=1.000000
vib_slot_confidence=1.000000
phot_spatial_confidence=0.860000
text_vib_coupling_strength=1.000000
text_phot_coupling_strength=1.000000
vib_phot_coupling_strength=1.000000
conflict_coupling_drop=0.345938
wrong_coupling_strength=0.000000
correction_recovery_ratio=1.000000
reentry_persistent_coupling=1.000000
reentry_cold_coupling=0.535844
```

## Verification

Full test command:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Observed: pass, including doc tests.

Whitespace check:

```text
git diff --check
```

Observed: pass.

Formatting:

```text
rustfmt --edition 2021 bionic_v0/src/curriculum.rs bionic_v0/src/bin/curriculum_report.rs bionic_v0/tests/curriculum_report.rs bionic_v0/tests/cross_channel_disturbance_coupling.rs
```

Observed: pass.

Repository-wide formatting check:

```text
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Observed: fails only on pre-existing unrelated formatting differences in:

- `bionic_v0/tests/adaptive_stability.rs`
- `bionic_v0/tests/after_window_diagnosis.rs`
- `bionic_v0/tests/architecture_contraction.rs`

Those files were not changed for this stage.

## Principle Check

The implementation remains admissible because:

- coupling is measured as numeric trace compatibility against existing
  substrate targets;
- text marks remain external byte/mark disturbance traces;
- vib and phot remain ordinary modality pattern kinds and slot-side
  disturbance evidence;
- no stable object or relation is promoted to a concept id;
- no carrier is translated into another carrier;
- report output is observer-side numeric evidence only;
- the hygiene test scans production/observer sources for rejected terms.
