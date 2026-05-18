# Active Text Probe Closure v4 Evidence

Date: 2026-05-16

## Boundary

This evidence is observer-side. Text-shaped input remains external byte/mark
disturbance evidence. Framed-byte output remains bounded body write-out and
action-consequence substrate evidence; it is not text generation, a command
interface, or a request/response controller.

## TDD RED Evidence

Behavior test command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure active_text_probe_closure_proves_probe_return_correction_and_reentry
```

Observed expected failure before implementation:

```text
no `run_active_text_probe_closure_curriculum` in `curriculum`
```

Report test command:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report active_text_probe_closure_report_emits_numeric_stage_evidence
```

Observed expected failure before report implementation:

```text
assertion failed: stdout.contains("mode=active_text_probe_closure")
```

## Report Command

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --active-text-probe-closure
```

Observed output:

```text
curriculum_report_v0
mode=active_text_probe_closure
stream_events=44
supported_marks=6
train_supported_avg=1.000000
conflict_events=2
conflict_ambiguity_delta=0.462500
probe_trigger_events=2
probe_frames=2
probe_write_epochs=2
return_windows=2
paired_consequences=2
probe_pair_similarity_avg=0.912813
pre_probe_ambiguity=0.462500
post_probe_ambiguity=0.213860
ambiguity_reduction_ratio=0.537600
wrong_binding_strength=0.000000
pre_probe_supported_strength=0.950000
post_probe_supported_strength=1.000000
correction_recovery_ratio=1.000000
egress_projection_confidence=0.640000
reentry_persistent_strength=1.000000
reentry_cold_strength=0.548344
```

## Focused Verification

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report active_text_probe_closure_report_emits_numeric_stage_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure --test curriculum_report
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

- Mixed stream: `stream_events=44`, `supported_marks=6`.
- Stable supported binding: `train_supported_avg=1.000000`.
- Conflict: `conflict_events=2`,
  `conflict_ambiguity_delta=0.462500`,
  `wrong_binding_strength=0.000000`.
- Active framed-byte probe:
  `probe_trigger_events=2`, `probe_frames=2`,
  `probe_write_epochs=2`.
- Ordinary return and pairing:
  `return_windows=2`, `paired_consequences=2`,
  `probe_pair_similarity_avg=0.912813`.
- Correction:
  `pre_probe_ambiguity=0.462500`,
  `post_probe_ambiguity=0.213860`,
  `ambiguity_reduction_ratio=0.537600`,
  `post_probe_supported_strength=1.000000`,
  `correction_recovery_ratio=1.000000`.
- Reentry:
  `reentry_persistent_strength=1.000000 >
  reentry_cold_strength=0.548344`.

## Re-Verification

Re-verified 2026-05-16 by rerunning:

- `cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --active-text-probe-closure`
- `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml`
- `git diff --check`

Observed report output remained identical to the numeric report above.

Automation rerun re-verified 2026-05-16T08:40:06+0800 (2026-05-16T00:45:14Z):

- Report output remained identical.
- `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml` and `git diff --check` passed.
- `cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check` still fails only on the same pre-existing drift files listed above.

Re-verified 2026-05-16T10:40:38+0800 (2026-05-16T02:40:43Z):

- Report output remained identical to the numeric report above.
- Focused + full `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml` and `git diff --check` passed.
- `cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check` still fails only on the same three pre-existing drift files listed above.
