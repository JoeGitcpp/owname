# VIB To Vocal Target Runtime Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire live VIB ingress into `vocal_target` so external sound contours can enter the running vocal imitation loop without introducing source identity, semantic, label, or word ontology.

**Architecture:** Keep VIB audio as ordinary ingress. A small viewport-side bridge reads numeric VIB ambient/band curves and refreshes `vocal_target` only when there is no recent body write that could make the ingress write-coupled. Vocal write-out is first aligned with `action_output` write epochs so the bridge can preserve ambiguity instead of turning self-return into an external target.

**Tech Stack:** Rust, Cargo integration tests, existing `Viewport` / VIB / vocal substrate.

---

## File Structure

- Modify: `bionic_v0/src/lib.rs`
  - Add engineering constants for the VIB-to-vocal bridge and write-coupled guard.
- Modify: `bionic_v0/src/viewport.rs`
  - Record action-output epochs when vocal write-out begins.
  - Add `tick_vocal_target_from_vib(now_us)` bridge.
  - Route imitation-loop start through `begin_voicing`.
- Modify: `bionic_v0/src/bin/body.rs`
  - Run the VIB-to-target bridge before `tick_vocal_imitation_loop`.
- Modify: `bionic_v0/tests/vocal_runtime.rs`
  - Add TDD tests for vocal/action epoch alignment and VIB-to-target bridge behavior.
- Modify: `bionic_v0/tests/vocal_body_contract.rs`
  - Add runtime wiring assertion for bridge tick order.

## Task 1: Vocal Write-Out Records Action Epochs

**Files:**
- Modify: `bionic_v0/tests/vocal_runtime.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Write the failing tests**

Append these imports to `bionic_v0/tests/vocal_runtime.rs`:

```rust
use bionic_v0::action_output::ActionOutputPhase;
```

Append these tests to `bionic_v0/tests/vocal_runtime.rs`:

```rust
#[test]
fn begin_voicing_records_action_output_epoch_for_vocal_write_out() {
    let vp = Arc::new(Viewport::new());

    vp.begin_voicing(1_000, 0.6, 0.4, 0.7);

    let out = vp.read_action_output();
    assert_eq!(out.n_channels, 1);
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Active);
    assert_eq!(out.write_epoch_history[0].start_t_us, 1_000);
}

#[test]
fn vocal_imitation_loop_records_action_epoch_when_target_starts_voicing() {
    let vp = Arc::new(Viewport::new());

    vp.observe_vocal_target(1_000, 0.8, 0.35, 0.60, 0.20);
    vp.tick_vocal_imitation_loop(2_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.write_epoch_history[0].start_t_us, 2_000);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime begin_voicing_records_action_output_epoch_for_vocal_write_out -- --nocapture
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime vocal_imitation_loop_records_action_epoch_when_target_starts_voicing -- --nocapture
```

Expected: FAIL. `begin_voicing` currently only writes `vocal_organ`, and `tick_vocal_imitation_loop` calls `VocalOrganState::begin_voicing` directly.

- [ ] **Step 3: Write minimal implementation**

In `bionic_v0/src/viewport.rs`, change the action-output import to:

```rust
use crate::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, ActionOutputState,
    OutputChannelKind,
};
```

Replace `Viewport::begin_voicing` with:

```rust
pub fn begin_voicing(&self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
    {
        let mut vo = self.vocal_organ.write().unwrap();
        vo.begin_voicing(now_us, breath, pitch, resonance);
    }
    self.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: breath,
        event_t_us: now_us,
    });
}
```

Inside `tick_vocal_imitation_loop`, replace the idle branch that calls `organ.begin_voicing(...)` while holding the organ lock with:

```rust
if organ.phase == crate::vocal_organ::VocalOrganPhase::Idle {
    drop(organ);
    self.begin_voicing(
        now_us,
        target.active_energy * 0.5,
        target.pitch_hint,
        target.resonance_hint,
    );
} else {
    organ.adjust_toward(
        now_us,
        target.active_energy * 0.5,
        target.pitch_hint,
        target.resonance_hint,
    );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime begin_voicing_records_action_output_epoch_for_vocal_write_out -- --nocapture
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime vocal_imitation_loop_records_action_epoch_when_target_starts_voicing -- --nocapture
```

Expected: PASS.

## Task 2: VIB Ingress Refreshes Vocal Target With Write-Coupled Guard

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/tests/vocal_runtime.rs`

- [ ] **Step 1: Write the failing tests**

Append these imports to `bionic_v0/tests/vocal_runtime.rs`:

```rust
use bionic_v0::{MOD_VIB, VIB_BANDS};
```

Append this helper and tests to `bionic_v0/tests/vocal_runtime.rs`:

```rust
fn push_vib_frame(vp: &Arc<Viewport>, t_us: u64, bands: [f64; VIB_BANDS]) {
    for axis in 0..VIB_BANDS {
        vp.push_observation(axis, t_us, bands[axis]);
    }
    let energy = bands.iter().map(|b| b * b).sum::<f64>();
    vp.write_ambient_for(MOD_VIB, energy, 0.05, Some(t_us), true);
}

#[test]
fn vib_ingress_populates_vocal_target_when_no_recent_body_write_exists() {
    let vp = Arc::new(Viewport::new());

    push_vib_frame(&vp, 10_000, [0.05, 0.12, 0.34, 0.18]);
    vp.tick_vocal_target_from_vib(10_000);

    let target = vp.read_vocal_target();
    assert!(target.active_energy > 0.01);
    assert_eq!(target.last_update_t_us, 10_000);
    assert!(target.pitch_hint > 0.0);
    assert!(target.resonance_hint > 0.0);
}

#[test]
fn recent_body_write_coupled_vib_does_not_overwrite_vocal_target() {
    let vp = Arc::new(Viewport::new());

    vp.begin_voicing(20_000, 0.7, 0.3, 0.4);
    push_vib_frame(&vp, 20_050, [0.06, 0.15, 0.28, 0.16]);
    vp.tick_vocal_target_from_vib(20_050);

    let target = vp.read_vocal_target();
    assert_eq!(target.last_update_t_us, 0);
    assert_eq!(target.active_energy, 0.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime vib_ingress_populates_vocal_target_when_no_recent_body_write_exists -- --nocapture
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime recent_body_write_coupled_vib_does_not_overwrite_vocal_target -- --nocapture
```

Expected: FAIL because `tick_vocal_target_from_vib` does not exist.

- [ ] **Step 3: Write minimal implementation**

In `bionic_v0/src/lib.rs`, add below `INGRESS_ACCUMULATION_THRESHOLD`:

```rust
/// Engineering approximation: VIB ambient energy must exceed this before it can
/// refresh the short-lived vocal target contour. Substrate knob, not a
/// theoretical sound boundary.
pub const VOCAL_TARGET_INGRESS_MIN_ENERGY: f64 = INGRESS_ACCUMULATION_THRESHOLD;

/// Engineering caution window: recent body writes can couple into VIB ingress,
/// so those frames must not overwrite `vocal_target` as if they were external
/// target contours. This is not source identity or ownership.
pub const VOCAL_TARGET_WRITE_COUPLED_GUARD_US: u64 = 300_000;
```

In `bionic_v0/src/viewport.rs`, add these constants to the grouped `crate::{ ... }` import:

```rust
VOCAL_TARGET_INGRESS_MIN_ENERGY, VOCAL_TARGET_WRITE_COUPLED_GUARD_US,
```

Then add this method in the vocal substrate API section before `observe_vocal_target`:

```rust
pub fn tick_vocal_target_from_vib(&self, now_us: u64) {
    let ambient = self.read_ambient();
    let vib = ambient.vib;
    if !vib.candidate_flag || vib.event_energy < VOCAL_TARGET_INGRESS_MIN_ENERGY {
        return;
    }
    if now_us.saturating_sub(vib.last_event_t_us) > QUIET_WINDOW_US {
        return;
    }
    if self.has_recent_write_coupled_vib_caution(now_us) {
        return;
    }

    let mut bands = [0.0_f64; VIB_BANDS];
    for (axis, band) in bands.iter_mut().enumerate() {
        let Some(sample) = self.latest_observation(axis) else {
            return;
        };
        if !sample.valid || now_us.saturating_sub(sample.t_us) > QUIET_WINDOW_US {
            return;
        }
        *band = sample.value.abs();
    }

    let band_sum = bands.iter().sum::<f64>();
    if band_sum <= f64::EPSILON {
        return;
    }

    let denom = (VIB_BANDS.saturating_sub(1).max(1)) as f64;
    let spectral_center = bands
        .iter()
        .enumerate()
        .map(|(axis, value)| *value * (axis as f64 / denom))
        .sum::<f64>()
        / band_sum;
    let high_band_sum = bands
        .iter()
        .enumerate()
        .filter(|(axis, _)| *axis >= VIB_BANDS / 2)
        .map(|(_, value)| *value)
        .sum::<f64>();
    let resonance = high_band_sum / band_sum;
    let envelope = (vib.surprise.abs()
        / (vib.event_energy.abs() + vib.surprise.abs() + f64::EPSILON))
        .clamp(0.0, 1.0);

    self.observe_vocal_target(
        now_us,
        vib.event_energy.clamp(0.0, 1.0),
        spectral_center.clamp(0.0, 1.0),
        resonance.clamp(0.0, 1.0),
        envelope,
    );
}

fn has_recent_write_coupled_vib_caution(&self, now_us: u64) -> bool {
    let out = self.action_output.read().unwrap();
    if out.channels.iter().any(|ch| {
        ch.phase != ActionOutputPhase::Idle
            && now_us.saturating_sub(ch.last_update_t_us)
                <= VOCAL_TARGET_WRITE_COUPLED_GUARD_US
    }) {
        return true;
    }
    out.write_epoch_history.iter().rev().any(|ep| {
        now_us.saturating_sub(ep.start_t_us) <= VOCAL_TARGET_WRITE_COUPLED_GUARD_US
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime vib_ingress_populates_vocal_target_when_no_recent_body_write_exists -- --nocapture
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime recent_body_write_coupled_vib_does_not_overwrite_vocal_target -- --nocapture
```

Expected: PASS.

## Task 3: Body Runtime Calls The Bridge Before Imitation

**Files:**
- Modify: `bionic_v0/tests/vocal_body_contract.rs`
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: Write the failing test**

Replace `body_runtime_wires_live_vocal_ticks_and_output_adapter` in `bionic_v0/tests/vocal_body_contract.rs` with:

```rust
#[test]
fn body_runtime_wires_live_vocal_ticks_and_output_adapter() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    let target_tick = body
        .find("tick_vocal_target_from_vib")
        .expect("body.rs must bridge live VIB ingress into vocal_target");
    let imitation_tick = body
        .find("tick_vocal_imitation_loop")
        .expect("body.rs must wire the live vocal imitation loop into runtime");
    assert!(
        target_tick < imitation_tick,
        "vocal target refresh must run before imitation loop consumes the target"
    );
    assert!(
        body.contains("tick_vocal_memory"),
        "body.rs must wire live vocal memory maintenance into runtime"
    );
    assert!(
        body.contains("spawn_vocal_output"),
        "body.rs must start the speaker-facing vocal adapter"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_body_contract body_runtime_wires_live_vocal_ticks_and_output_adapter -- --nocapture
```

Expected: FAIL because `body.rs` does not call `tick_vocal_target_from_vib`.

- [ ] **Step 3: Write minimal implementation**

In `bionic_v0/src/bin/body.rs`, update the vocal ticker to:

```rust
let h_vocal = spawn_ticker(50, stop.clone(), move |_| {
    let t = now_us();
    vp_vocal.tick_vocal_target_from_vib(t);
    vp_vocal.tick_vocal_imitation_loop(t);
    vp_vocal.tick_vocal_memory(t);
});
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_body_contract body_runtime_wires_live_vocal_ticks_and_output_adapter -- --nocapture
```

Expected: PASS.

## Task 4: Verification

**Files:**
- Test only unless failures identify a regression.

- [ ] **Step 1: Run focused vocal and action suites**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime --test vocal_body_contract --test action_output_substrate --test action_consequence_loop -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run related ingress suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test ingress_consequence_loop -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.

## Self-Review

- Spec coverage: Task 1 aligns physical vocal output with `action_output` epochs; Task 2 adds ordinary VIB ingress to `vocal_target` while preserving write-coupled ambiguity; Task 3 wires the body runtime; Task 4 verifies related substrate behavior.
- Placeholder scan: no TBD/TODO placeholders, no unstated test code, no undefined methods after their implementation steps.
- Type consistency: method names and imports match the current Rust API (`Viewport`, `ActionOutputPhase`, `MOD_VIB`, `VIB_BANDS`).
