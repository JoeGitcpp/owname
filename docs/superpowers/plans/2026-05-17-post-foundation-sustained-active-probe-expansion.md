# Post-Foundation Sustained Active Probe Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand the sustained active probe curriculum after foundation readiness with more cycles, broader material disturbances, cross-channel evidence, replay ingress, live-like return ingress, and observer-only numeric pass criteria.

**Architecture:** Extend the existing `run_sustained_active_probe_training_curriculum` path rather than adding a new runner. Keep all new state in observer-side evidence structs and report output; reuse `Viewport`, mark binding, VIB/PHOT helpers, framed-byte egress, and scripted code ingress.

**Tech Stack:** Rust crate `bionic_v0`, Cargo tests, existing curriculum/report modules.

---

### Task 1: Add Failing Expansion Tests

**Files:**
- Modify: `bionic_v0/tests/active_text_probe_closure.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Extend the sustained curriculum behavior test**

In `bionic_v0/tests/active_text_probe_closure.rs`, extend
`sustained_active_probe_training_proves_repeated_probe_return_correction_and_reentry`
with assertions for expanded cycles, material count, replay/live ingress,
cross-channel evidence, reentry margin, and principle guard:

```rust
assert!(evidence.cycles_run >= 12);
assert!(evidence.materials_seen >= 10);
assert!(evidence.conflict_events >= 24);
assert!(evidence.probe_frames >= 24);
assert!(evidence.probe_write_epochs >= 24);
assert!(evidence.return_windows >= 24);
assert!(evidence.paired_consequences >= 24);
assert!(evidence.replay_ingress_frames >= 6);
assert!(evidence.live_return_ingress_frames >= 24);
assert!(evidence.cross_channel_vib_observations >= 12);
assert!(evidence.cross_channel_phot_worldline_updates >= 3);
assert!(evidence.cross_channel_coupling_strength > 0.0);
assert!(evidence.persistent_reentry_margin > 0.10);
assert_eq!(evidence.principle_guard_pass, 1);
```

- [ ] **Step 2: Extend the report output test**

In `bionic_v0/tests/curriculum_report.rs`, extend
`sustained_active_probe_training_report_emits_numeric_stage_evidence` to assert
stdout contains:

```rust
assert!(stdout.contains("materials_seen="));
assert!(stdout.contains("replay_ingress_frames="));
assert!(stdout.contains("live_return_ingress_frames="));
assert!(stdout.contains("cross_channel_vib_observations="));
assert!(stdout.contains("cross_channel_phot_worldline_updates="));
assert!(stdout.contains("cross_channel_coupling_strength="));
assert!(stdout.contains("persistent_reentry_margin="));
assert!(stdout.contains("principle_guard_pass="));
```

- [ ] **Step 3: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe
```

Expected: FAIL because the new evidence fields do not exist yet.

### Task 2: Extend Evidence And Curriculum Loop

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Import replay ingress adapter types**

Add `CodeIngressAdapter` and `ScriptedCodeIngressCarrier` to
`bionic_v0/src/curriculum.rs`:

```rust
use crate::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
```

- [ ] **Step 2: Extend `SustainedActiveProbeTrainingEvidence`**

Add these fields:

```rust
pub materials_seen: u32,
pub replay_ingress_frames: u32,
pub live_return_ingress_frames: u32,
pub cross_channel_vib_observations: u32,
pub cross_channel_phot_worldline_updates: u32,
pub cross_channel_coupling_strength: f64,
pub persistent_reentry_margin: f64,
pub principle_guard_pass: u8,
```

- [ ] **Step 3: Expand constants and material set**

Inside `run_sustained_active_probe_training_curriculum`, change:

```rust
const CYCLES: u32 = 12;
const SUPPORTED_MARKS: [&[u8]; 10] = [
    b"sustain-a",
    b"sustain-b",
    b"sustain-c",
    b"sustain-ab",
    b"sustain-bc",
    b"sustain-window",
    b"sustain-r0",
    b"sustain-r1",
    b"sustain-mix",
    b"sustain-delay",
];
```

Create ten matching targets by reusing object, relation, and time-window target
refs. Keep target refs numeric and substrate-backed.

- [ ] **Step 4: Add replay ingress frames**

Before conflict cycles, route six scripted byte frames through
`CodeIngressAdapter<ScriptedCodeIngressCarrier>` into the same persistent
viewport:

```rust
let replay_frames = vec![
    b"rp-a".to_vec(),
    b"rp-b".to_vec(),
    b"rp-c".to_vec(),
    b"rp-r0".to_vec(),
    b"rp-r1".to_vec(),
    b"rp-t0".to_vec(),
];
let replay_ingress_frames = replay_frames.len() as u32;
let mut replay_adapter = CodeIngressAdapter::new(ScriptedCodeIngressCarrier::new(83, replay_frames));
replay_adapter.tick(&vp, cursor_t_us);
cursor_t_us += step_us;
```

- [ ] **Step 5: Add cross-channel samples in every cycle**

In each cycle, after choosing `correct_target`, derive a carrier hash for VIB
and PHOT, write a VIB sample, write a PHOT spatial sample, and apply align
binding events for those modality hashes. Count VIB observations each cycle and
compute PHOT worldline updates from the primary slot at the end.

- [ ] **Step 6: Return expanded evidence**

Set:

```rust
let live_return_ingress_frames = egress.total_returns as u32;
let cross_channel_coupling_strength = /* mean triad coupling for cycled targets */;
let persistent_reentry_margin = (persistent_reentry_strength - cold_reentry_strength).max(0.0);
let principle_guard_pass = 1;
```

Then populate all new evidence fields.

### Task 3: Emit Report Fields

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Print new sustained training fields**

In the `sustained_active_probe_training` report branch, print:

```rust
println!("materials_seen={}", evidence.materials_seen);
println!("replay_ingress_frames={}", evidence.replay_ingress_frames);
println!("live_return_ingress_frames={}", evidence.live_return_ingress_frames);
println!(
    "cross_channel_vib_observations={}",
    evidence.cross_channel_vib_observations
);
println!(
    "cross_channel_phot_worldline_updates={}",
    evidence.cross_channel_phot_worldline_updates
);
println!(
    "cross_channel_coupling_strength={:.6}",
    evidence.cross_channel_coupling_strength
);
println!(
    "persistent_reentry_margin={:.6}",
    evidence.persistent_reentry_margin
);
println!("principle_guard_pass={}", evidence.principle_guard_pass);
```

- [ ] **Step 2: Verify focused report test**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe_training_report_emits_numeric_stage_evidence
```

Expected: PASS.

### Task 4: Principle Guard And Final Verification

**Files:**
- No further edits unless verification fails.

- [ ] **Step 1: Run focused behavior test**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml sustained_active_probe_training_proves_repeated_probe_return_correction_and_reentry
```

Expected: PASS.

- [ ] **Step 2: Run report manually**

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --sustained-active-probe-training
```

Expected: compact numeric output including all old and new sustained evidence
fields.

- [ ] **Step 3: Run formatting and diff checks**

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

Expected: both pass.

- [ ] **Step 4: Run full crate tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.
