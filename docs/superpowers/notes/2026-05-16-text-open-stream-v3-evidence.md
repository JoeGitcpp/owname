# Text Disturbance Open-Stream v3 Evidence

Date: 2026-05-16

## Boundary

This evidence is observer-side. Text-shaped input remains external byte/mark
disturbance evidence. The implementation does not add a body-side text
interpreter, vocabulary, grammar path, naming gate, discrete internal concept,
or human-readable body output.

## TDD RED Evidence

Behavior test command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream text_open_stream_proves_mixed_stream_delay_composition_correction_and_reentry
```

Observed expected failure before implementation:

```text
no `run_text_open_stream_curriculum` in `curriculum`
```

Report test command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_open_stream_report_emits_numeric_stage_evidence
```

Observed expected failure before implementation:

```text
assertion failed: stdout.contains("mode=text_open_stream")
```

## Report Command

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --text-open-stream
```

Observed output:

```text
curriculum_report_v0
mode=text_open_stream
stream_events=67
supported_marks=6
unrelated_marks=4
variant_marks=3
object_targets=3
relation_targets=2
time_window_targets=2
composition_windows=2
delayed_windows=2
train_supported_avg=1.000000
train_supported_weakest=1.000000
unrelated_leakage_strength=0.000000
hold_supported_ratio=0.950000
variant_persistent_avg=0.548344
variant_cold_avg=0.250000
delayed_binding_strength=0.625000
composition_relation_strength=0.250000
conflict_ambiguity_delta=0.250000
wrong_binding_strength=0.000000
correction_recovery_ratio=1.000000
reentry_persistent_avg=1.000000
reentry_cold_avg=0.535844
reentry_persistent_weakest=1.000000
reentry_cold_weakest=0.523344
```

## Focused Verification

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_open_stream_report_emits_numeric_stage_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream --test curriculum_report
```

Focused commands passed after implementation.

## Full Verification

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```

Both commands passed.

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

This command still fails only on pre-existing unrelated formatting drift:

- `/Users/joe/unnamed/bionic_v0/tests/adaptive_stability.rs:82`
- `/Users/joe/unnamed/bionic_v0/tests/after_window_diagnosis.rs:71`
- `/Users/joe/unnamed/bionic_v0/tests/architecture_contraction.rs:124`

## Acceptance Mapping

- Continuous mixed stream: `stream_events=67`.
- Supported, unrelated, variant marks: `6`, `4`, and `3`.
- Multi-target path: `object_targets=3`, `relation_targets=2`,
  `time_window_targets=2`.
- Hold: `hold_supported_ratio=0.950000`.
- Unrelated leakage bounded: `unrelated_leakage_strength=0.000000`.
- Persistent variant advantage: `0.548344 > 0.250000`.
- Delayed evidence: `delayed_binding_strength=0.625000`.
- Adjacent composition: `composition_relation_strength=0.250000`.
- Conflict and correction:
  `conflict_ambiguity_delta=0.250000`,
  `wrong_binding_strength=0.000000`,
  `correction_recovery_ratio=1.000000`.
- Reentry:
  `reentry_persistent_avg=1.000000 > reentry_cold_avg=0.535844`,
  `reentry_persistent_weakest=1.000000 >
  reentry_cold_weakest=0.523344`.
