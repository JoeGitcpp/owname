# Text Grounding Suite v2 Evidence

Date: 2026-05-16

## Boundary Statement

Text remains an external byte-carrier mark disturbance. This suite does not add
tokenization, parsing, vocabulary, dictionary meaning, concept allocation,
semantic readiness, naming gates, or body-side human-readable output. Evidence
is collected through observer-side curriculum/report code against existing
object-slot, relation-edge, and time-window targets.

## Commands

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_grounding_suite
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_grounding_suite_report_emits_numeric_stage_evidence
/Users/joe/unnamed/bionic_v0/target/debug/curriculum_report --text-grounding-suite
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
```

## Numeric Report

```text
curriculum_report_v0
mode=text_grounding_suite
train_marks=6
object_targets=3
relation_targets=2
time_window_targets=1
variant_marks=3
composition_windows=2
train_avg_strength=1.000000
train_weakest_strength=1.000000
hold_avg_ratio=0.950000
hold_weakest_ratio=0.950000
variant_generalization_score=0.025000
variant_distinctness_ratio=1.000000
composition_relation_strength=0.250000
conflict_ambiguity_delta=0.250000
wrong_binding_strength=0.000000
correction_recovery_ratio=1.000000
reentry_persistent_avg=1.000000
reentry_cold_avg=0.535844
reentry_persistent_weakest=1.000000
reentry_cold_weakest=0.523344
```

## Acceptance Checklist

- Six base disturbances form supported bindings: `train_marks=6`.
- Three object-slot targets are used: `object_targets=3`.
- Two relation-edge targets are used: `relation_targets=2`.
- One time-window target is used: `time_window_targets=1`.
- Training average and weakest strengths are above threshold:
  `train_avg_strength=1.000000`, `train_weakest_strength=1.000000`.
- Hold preserves average and weakest strength:
  `hold_avg_ratio=0.950000`, `hold_weakest_ratio=0.950000`.
- Variants show shared-target routing advantage over a cold variant baseline
  while remaining distinct:
  `variant_generalization_score=0.025000`,
  `variant_distinctness_ratio=1.000000`.
- Composition precursor windows formed from adjacent disturbance pairs
  strengthen relation targets:
  `composition_relation_strength=0.250000`.
- Conflict raises ambiguity without creating a wrong binding:
  `conflict_ambiguity_delta=0.250000`,
  `wrong_binding_strength=0.000000`.
- Correction restores supported binding:
  `correction_recovery_ratio=1.000000`.
- Persistent reentry beats cold baseline on average and weakest strength:
  `reentry_persistent_avg=1.000000`,
  `reentry_cold_avg=0.535844`,
  `reentry_persistent_weakest=1.000000`,
  `reentry_cold_weakest=0.523344`.

## Residual Risk

This is still a deterministic observer-side curriculum. It proves substrate
behavior for text-shaped byte disturbances, not natural-language understanding.
The next meaningful increase would be longer noisy runs, broader variants, and
composition windows that are generated from live interaction traces rather than
fixed curriculum episodes.
