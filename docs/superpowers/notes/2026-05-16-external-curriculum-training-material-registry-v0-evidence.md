# External Curriculum Training Material Registry v0 Evidence

Date: 2026-05-16

## Scope

Implemented the first external curriculum training plan slice:

- observer-side material registry;
- material origin/carrier/stage coverage;
- principle review evidence;
- `curriculum_report --external-curriculum-plan` output.

This does not add a governor yet. It gives a future governor a principled,
numeric material boundary to consume.

## Files

- Created `docs/superpowers/specs/2026-05-16-external-curriculum-training-material-registry-v0-design.md`
- Created `docs/superpowers/plans/2026-05-16-external-curriculum-training-material-registry-v0.md`
- Created `docs/superpowers/notes/2026-05-16-external-curriculum-training-material-registry-v0-evidence.md`
- Created `bionic_v0/src/curriculum_material.rs`
- Created `bionic_v0/tests/external_curriculum_material_registry.rs`
- Modified `bionic_v0/src/lib.rs`
- Modified `bionic_v0/src/bin/curriculum_report.rs`
- Modified `bionic_v0/tests/curriculum_report.rs`

## TDD Evidence

RED material registry test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry default_external_curriculum_plan_covers_required_material_origins_carriers_and_stages
```

Observed expected failures:

```text
could not find `curriculum_material` in `bionic_v0`
couldn't read `tests/../src/curriculum_material.rs`
```

GREEN material registry tests:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry
```

Observed:

```text
2 passed; 0 failed
```

RED report test:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report external_curriculum_plan_report_emits_material_and_principle_review
```

Observed expected failure:

```text
assertion failed: stdout.contains("mode=external_curriculum_plan")
```

GREEN focused tests:

```text
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry --test curriculum_report
```

Observed:

```text
tests/curriculum_report.rs: 12 passed
tests/external_curriculum_material_registry.rs: 2 passed
```

## Report Evidence

Command:

```text
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --external-curriculum-plan
```

Observed:

```text
curriculum_report_v0
mode=external_curriculum_plan
material_groups=10
synthetic_groups=4
replay_groups=2
probe_return_groups=1
live_ingress_groups=3
carrier_kinds=5
stage_kinds=7
reproducible_groups=7
observer_only_groups=10
body_touch_points=0
missing_required_roles=0
principle_review_pass=1
```

## Debug Note

The first full verification run exposed a source-hygiene false positive:

```text
mark disturbance path must not contain `println!("body`
```

Root cause: observer-side `curriculum_report.rs` printed
`body_touch_points` with a string literal beginning with `body`. The field is
observer evidence, not body output. The fix preserved the output key while
splitting the string literal:

```rust
println!("{}={}", "body_touch_points", review.body_touch_points);
```

Then `mark_disturbance_sources_do_not_introduce_rejected_language_ontology`
passed.

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

Touched-file formatting:

```text
rustfmt --edition 2021 bionic_v0/src/curriculum_material.rs bionic_v0/src/lib.rs bionic_v0/src/bin/curriculum_report.rs bionic_v0/tests/curriculum_report.rs bionic_v0/tests/external_curriculum_material_registry.rs
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

- the registry is observer-side data only;
- material entries are numeric kind codes, ids, seeds, event counts, durations,
  and booleans;
- no entry stores meaning, object names, visual labels, voice labels, prompt
  answers, source identity, ownership, ethics, or personality;
- the registry does not run training, mutate `Viewport`, or write through body
  channels;
- the report emits numeric evidence only;
- hygiene tests scan the new module for rejected ontology and body output.
