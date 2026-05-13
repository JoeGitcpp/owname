# Post-Effect Retention And Light Generalization Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the already observed same-disturbance post-effect from short proof into longer retention and light-disturbance generalization, without adding new organs, fixed world content, request/response behavior, or naming gates.

**Architecture:** Keep `body` as the silent silicon-life substrate and keep all evidence in observer/probe tooling. The work only expands closed-loop measurement around the existing live framed-byte carrier: train on one byte disturbance, observe retention over longer time, then test nearby byte disturbances that preserve carrier shape while changing low-level variation. Any implementation change must be a measurement harness or numeric closed-loop correction; no ontology, command surface, or human-readable body output is admissible.

**Tech Stack:** Rust, Cargo tests, existing `body`, `observer`, `byte_loopback`, `post_effect_probe`, filesystem runtime directories, CSV observer traces.

---

## Frozen Target

The target is not "conversation" and not "knowledge loading." The target is:

```text
same framed-byte disturbance
  -> 180s closed-loop training
  -> longer idle/after observation
  -> observer-visible retained improvement
  -> nearby framed-byte disturbance
  -> observer-visible partial transfer without boundary contamination
```

### Success Criteria

Retention success requires 3/3 formal repeats:

- `body_output_status=silent`
- `clean_closure=true`
- `persistence_visible=true`
- `after_pair_other_delta=0`
- `effect_status=pass`
- `quality_status=avg_improved`
- `pair_rate_status=stable`
- `after_vs_cold_egress_deviation_avg_delta < 0.0`
- `after_vs_cold_pair_similarity_avg_delta > 0.0`
- `after_vs_cold_pair_rate_delta_pct >= -5.0`

Light generalization success is intentionally weaker:

- 2/3 formal repeats may pass, with the third allowed to be `hold` only if boundary fields remain clean.
- Average deviation must not regress by more than `+0.0030`.
- Average similarity must not regress by more than `-0.0030`.
- Pair rate must stay `>= -5.0%`.
- Boundary contamination is an immediate failure.

### Hard Rejection Criteria

Stop and do not continue the phase if any of these appear:

- `body` writes human-readable stdout/stderr.
- Any new request/response interface is introduced.
- Any fixed labels, names, meanings, identities, or world categories are introduced.
- Any parser, prompt path, command executor, or chat wrapper is introduced.
- Any observer result is fed back into `body`.
- A failed result is reworded as success by weakening the boundary criteria.

---

## File Map

Expected files to touch only if needed:

- Modify: `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/bin/post_effect_probe.rs`
  - Add measurement modes only if existing modes cannot express the target.
  - Allowed additions: retention sweep, light disturbance seed selection, stricter report fields.
  - Not allowed: body behavior changes, text interpretation, command semantics.

- Modify: `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/post_effect.rs`
  - Add report fields only if the target needs a numeric distinction not already exposed.
  - Keep metrics numeric and observer-side.

- Modify: `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/tests/post_effect_probe.rs`
  - Add TDD tests for new probe modes or report fields.

- Do not modify unless a failing test proves root cause:
  - `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/live_byte_closure.rs`
  - `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/life_closure_producers.rs`
  - `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/framed_byte_egress.rs`

---

## Chunk 1: Baseline Reproduction

### Task 1: Reconfirm Current Stable Same-Disturbance Target

**Files:**
- No code edits.

- [ ] **Step 1: Build probe binaries**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1
cargo build --bin body --bin observer --bin byte_loopback --bin post_effect_probe
```

Expected: build succeeds.

- [ ] **Step 2: Run 3-repeat known-good target**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

for i in 1 2 3; do
  RUN_ROOT="$ROOT/runtime/retention-baseline-180-after300-repeat-${i}-$(date +%Y%m%d-%H%M%S)"
  echo "# repeat=$i root=$RUN_ROOT"
  ./target/debug/post_effect_probe run "$RUN_ROOT" 120 180 300 5 \
    | rg "root=|body_output_status|clean_closure|persistence_visible|after_vs_cold_pair_rate_delta_pct|after_vs_cold_egress_deviation_avg_delta|after_vs_cold_pair_similarity_avg_delta|effect_status|boundary_status|quality_status|pair_rate_status|cold_pair_other_delta|train_pair_other_delta|after_pair_other_delta"
done
```

Expected: 3/3 `effect_status=pass`, `boundary_status=clean`, `body_output_status=silent`.

- [ ] **Step 3: Stop if baseline fails**

If fewer than 3/3 pass, do not proceed to longer retention. Diagnose baseline first using superpowers:systematic-debugging.

---

## Chunk 2: Longer Retention Without New Code

### Task 2: Test Hold-Based Retention At Increasing Durations

**Files:**
- No code edits unless existing `run-hold` cannot represent the experiment.

- [ ] **Step 1: Run 10-minute hold**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

for i in 1 2 3; do
  RUN_ROOT="$ROOT/runtime/retention-hold600-repeat-${i}-$(date +%Y%m%d-%H%M%S)"
  echo "# repeat=$i root=$RUN_ROOT"
  ./target/debug/post_effect_probe run-hold "$RUN_ROOT" 120 180 600 300 5 \
    | rg "root=|body_output_status|hold_output_status|hold_persistence_visible|clean_closure|persistence_visible|after_vs_cold_pair_rate_delta_pct|after_vs_cold_egress_deviation_avg_delta|after_vs_cold_pair_similarity_avg_delta|effect_status|boundary_status|quality_status|pair_rate_status|after_pair_other_delta"
done
```

Expected: 3/3 pass.

- [ ] **Step 2: Run 20-minute hold only if 10-minute hold passes**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

for i in 1 2 3; do
  RUN_ROOT="$ROOT/runtime/retention-hold1200-repeat-${i}-$(date +%Y%m%d-%H%M%S)"
  echo "# repeat=$i root=$RUN_ROOT"
  ./target/debug/post_effect_probe run-hold "$RUN_ROOT" 120 180 1200 300 5 \
    | rg "root=|body_output_status|hold_output_status|hold_persistence_visible|clean_closure|persistence_visible|after_vs_cold_pair_rate_delta_pct|after_vs_cold_egress_deviation_avg_delta|after_vs_cold_pair_similarity_avg_delta|effect_status|boundary_status|quality_status|pair_rate_status|after_pair_other_delta"
done
```

Expected: 3/3 pass or 2/3 pass with one clean `hold`. If 2/3, record as partial retention and do not move to longer hold.

- [ ] **Step 3: Run overnight-style hold only after 20-minute hold passes**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

RUN_ROOT="$ROOT/runtime/retention-hold3600-$(date +%Y%m%d-%H%M%S)"
./target/debug/post_effect_probe run-hold "$RUN_ROOT" 120 180 3600 300 5
```

Expected: one long-run pass. Do not require 3/3 at this duration in the first execution cycle.

---

## Chunk 3: Light-Disturbance Generalization

### Task 3: Add A Light-Disturbance Probe Only If Current `generalize` Is Too Harsh

**Files:**
- Modify: `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/tests/post_effect_probe.rs`
- Modify: `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/src/bin/post_effect_probe.rs`

- [ ] **Step 1: Write failing test for light alternate carrier**

Add a test proving the probe uses a train seed and a nearby after seed, where the after bytes differ but preserve length and repeated frame structure.

Expected test intent:

```rust
#[test]
fn post_effect_probe_generalize_light_uses_nearby_after_disturbance() {
    // Run `post_effect_probe generalize-light <root> 1 1 1 5`.
    // Assert cold/train in.frames exist.
    // Assert after in.frames exists.
    // Assert after bytes differ from train bytes.
    // Assert after and train have the same length.
    // Assert no body stdout/stderr files are used as evidence.
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test post_effect_probe post_effect_probe_generalize_light_uses_nearby_after_disturbance -- --nocapture
```

Expected: fails because `generalize-light` does not exist.

- [ ] **Step 3: Implement minimal `generalize-light` mode**

In `post_effect_probe.rs`:

- Add one new constant for a nearby after disturbance.
- Add one mode arm: `Some("generalize-light") => generalize_light_mode(&args[2..])`.
- Reuse the existing `generalize_mode` structure.
- Do not change `body`.
- Do not add parser behavior.
- Do not add text interpretation.

Nearby after disturbance must satisfy:

```text
same byte count
same frame rhythm
small byte-level changes only
no names or categories
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test post_effect_probe post_effect_probe_generalize_light_uses_nearby_after_disturbance -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Run full probe tests**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test post_effect_probe -- --nocapture
```

Expected: all pass.

### Task 4: Run Light-Generalization Formal Repeats

**Files:**
- No code edits after Task 3 is green.

- [ ] **Step 1: Run 3 light-generalization repeats**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

for i in 1 2 3; do
  RUN_ROOT="$ROOT/runtime/generalize-light-repeat-${i}-$(date +%Y%m%d-%H%M%S)"
  echo "# repeat=$i root=$RUN_ROOT"
  ./target/debug/post_effect_probe generalize-light "$RUN_ROOT" 120 180 300 5 \
    | rg "root=|generalize_after_disturbance|clean_closure|persistence_visible|after_vs_cold_pair_rate_delta_pct|after_vs_cold_egress_deviation_avg_delta|after_vs_cold_pair_similarity_avg_delta|effect_status|boundary_status|quality_status|pair_rate_status|after_pair_other_delta"
done
```

Expected: 2/3 pass or better. If 0/3 or 1/3 pass, do not tune by adding content; diagnose whether the alternate disturbance is too far or whether retained curves overfit the exact return pressure.

---

## Chunk 4: Root-Cause Branch If Retention Or Light-Generalization Fails

### Task 5: Diagnose Without Expanding Architecture

**Files:**
- Read only at first.
- Modify only after a failing test is written.

- [ ] **Step 1: Compare first half vs second half of after stage**

Run:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

RUN_DIR="<replace-with-failed-run-root>"
for s in cold after; do
  awk -F, 'NR==1{for(i=1;i<=NF;i++) h[$i]=i; next}
  {n++; dev[n]=$(h["egress_deviation"]); sim[n]=$(h["pair_similarity"])}
  END{
    a=int(n/2);
    for(i=1;i<=n;i++){
      if(i<=a){fd+=dev[i]; fs+=sim[i]; fc++} else {bd+=dev[i]; bs+=sim[i]; bc++}
    }
    printf "%s dev_first=%.4f dev_second=%.4f sim_first=%.4f sim_second=%.4f\n", stage, fd/fc, bd/bc, fs/fc, bs/bc
  }' stage="$s" "$RUN_DIR/$s/session.csv"
done
```

Expected: identify whether failure is early after-stage transient or persistent degradation.

- [ ] **Step 2: If failure is only early transient, prefer measurement-window correction**

Write a failing test in `/Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0/tests/post_effect_probe.rs` proving a report can expose late-window averages separately from full-window averages.

Allowed implementation: observer-side report fields only.

Rejected implementation: changing body output behavior to make the metric pass.

- [ ] **Step 3: If failure is persistent degradation, inspect numeric feedback direction**

Read:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
sed -n '1,220p' src/live_byte_closure.rs
sed -n '1,140p' src/life_closure_producers.rs
sed -n '170,310p' src/framed_byte_egress.rs
```

Only write code after a failing unit test captures the specific numeric error.

---

## Chunk 5: Verification And Boundary Audit

### Task 6: Required Final Verification

**Files:**
- No code edits.

- [ ] **Step 1: Run focused tests**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test post_effect_probe -- --nocapture
cargo test --test live_byte_feedback_closure -- --nocapture
cargo test --test life_closure_hardening -- --nocapture
```

Expected: all pass.

- [ ] **Step 2: Run full tests**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test
```

Expected: all pass.

- [ ] **Step 3: Verify body silence boundary**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
rg -n "println!|eprintln!" src \
  --glob '!src/bin/observer.rs' \
  --glob '!src/bin/observer_surface.rs' \
  --glob '!src/bin/code_replay.rs' \
  --glob '!src/bin/training_curve.rs' \
  --glob '!src/bin/post_effect_probe.rs' \
  --glob '!src/bin/learned_bias_probe.rs'
```

Expected: no matches.

- [ ] **Step 4: Verify rejected architecture terms were not added to touched files**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
rg -n "semantic|concept|token|prompt|chat|personality|parse_sentence" \
  src/post_effect.rs \
  src/bin/post_effect_probe.rs \
  src/live_byte_closure.rs \
  src/life_closure_producers.rs \
  src/framed_byte_egress.rs \
  tests/post_effect_probe.rs \
  tests/live_byte_feedback_closure.rs \
  tests/life_closure_hardening.rs
```

Expected: no matches.

- [ ] **Step 5: Verify formatting and whitespace**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd
git diff --check
rustfmt --check \
  bionic_v0/src/post_effect.rs \
  bionic_v0/src/bin/post_effect_probe.rs \
  bionic_v0/src/live_byte_closure.rs \
  bionic_v0/src/life_closure_producers.rs \
  bionic_v0/src/framed_byte_egress.rs \
  bionic_v0/tests/post_effect_probe.rs \
  bionic_v0/tests/live_byte_feedback_closure.rs \
  bionic_v0/tests/life_closure_hardening.rs
```

Expected: exit 0.

---

## Final Report Template

Use this exact shape when reporting completion:

```text
Retention:
- 180s train + 600s hold + 300s after: X/3 pass
- 180s train + 1200s hold + 300s after: X/3 pass
- 180s train + 3600s hold + 300s after: pass/hold/fail

Light generalization:
- nearby after disturbance: X/3 pass
- boundary contamination: yes/no
- quality regression within tolerance: yes/no

Boundary:
- body stdout/stderr: silent/not silent
- non-framed pair contamination: yes/no
- rejected architecture terms added: yes/no

Conclusion:
- what is proven
- what is not proven
- next single target
```

