# Live I/O Preflight Life-Sign Implementation Plan

> Executed 2026-05-05 in worktree
> `/Users/joe/unnamed/.worktrees/live-io-preflight-life-sign`.

**Goal:** Land the executable part of
`docs/superpowers/specs/2026-05-05-live-io-preflight-life-sign-contract-design.md`:
prove that the current body write path, ingress return, temporal pairing,
PHOT structure, and snapshot visibility form a minimal live I/O life sign.

**Architecture:** This does not add a preflight module. It tightens existing
substrate contracts:

- vocal write-out must also create an `action_output` write epoch;
- active vocal imitation must use that same write path when starting output;
- consequence windows remain numeric ingress windows;
- write/consequence pairs remain time and curve records only;
- observer visibility comes from snapshot facts, not body-side text output.

**High-risk term boundary:** The words in this plan are external engineering
descriptions. No new body ontology field is introduced. Numeric handles,
slot indices, modality bits, and window ids remain engineering locators only.

## Chunk 1: Align Vocal Write-Out With Write Epochs

- [x] Add a failing test proving `Viewport::begin_voicing` must record one
  `action_output` write epoch.
- [x] Implement the bridge by applying a `GenericWrite / Begin` event after
  updating `vocal_organ`.
- [x] Add a failing test proving `tick_vocal_imitation_loop` must record an
  epoch when it starts output from idle.
- [x] Route the idle-to-output branch through `Viewport::begin_voicing`.

## Chunk 2: Close Minimal Write / Ingress Evidence

- [x] Add a deterministic test where a vocal write epoch pairs with a later
  VIB ingress consequence window.
- [x] Add a deterministic test where ingress without any write epoch remains
  unpaired.
- [x] Add a deterministic test where overlapping windows remain independent
  numeric windows and pairing references only the selected window id.

## Chunk 3: Verify PHOT Structure Before Probe Feedback

- [x] Add a synthetic PHOT preflight test through `RealPhotAdapter`.
- [x] Assert multi-region visual input reaches active PHOT slots with valid
  spatial state.
- [x] Refresh attention and relations, then assert relation structure is
  present from snapshot-substrate state.

## Chunk 4: Snapshot And Observer-Side Runbook

- [x] Add a snapshot preflight test covering action output, consequence pair,
  vocal organ state, PHOT slots, and global relations.
- [x] Add an observer-side runbook for deterministic and live hardware checks.
- [x] Run final verification:
  - `cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight`
  - `cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime`
  - `cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop`
  - `cargo test --manifest-path bionic_v0/Cargo.toml --test real_phot_adapter`
  - `cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output`
  - `cargo test --manifest-path bionic_v0/Cargo.toml`

## Files

- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/probe_law.rs`
- Create: `bionic_v0/tests/live_io_preflight.rs`
- Create: `docs/superpowers/plans/2026-05-05-live-io-preflight-runbook.md`

---

## Continuation Chunk 5: Admit Probe Write-Out Into The Physical Carrier

**Goal:** close the remaining live-preflight gap where `probe_law` can create
an `action_output` write epoch but not necessarily drive the physical vocal
carrier read by `vocal_adapter`.

**Boundary:** This is still a body-side substrate bridge, not a speech,
source, ownership, target, success, or identity layer. The first physical
carrier is vocal output because it is the current non-textual write-out path.
The bridge carries numeric magnitude and substrate time only.

### Task 9: Probe Tick Starts Physical Vocal Output

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/probe_law.rs`

- [x] **Step 1: Write the failing test**

Add a test that forces `probe_law` into `P2`, runs
`tick_probe_output_from_law`, and asserts both a write epoch and nonzero
`vocal_organ` activity are visible.

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight probe_law_active_tick_starts_physical_vocal_write -- --nocapture
```

Expected before implementation: FAIL because `tick_probe_output_from_law`
records an `action_output` event without changing `vocal_organ`.

- [x] **Step 3: Implement the smallest bridge**

Add a viewport helper that applies an `ActionOutputEvent` through the vocal
carrier, then records the same event in `action_output`. Route
`tick_probe_output_from_law` through that helper for generated probe events.

- [x] **Step 4: Run test to verify it passes**

Run the same focused command. Expected: PASS.

### Task 10: Delay-Adjusted Probe Magnitude Reaches Physical Output

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/probe_law.rs`

- [x] **Step 1: Write the failing test**

Add a test that seeds a paired write/consequence history with a two-second
delay, runs a later active probe tick, and asserts the physical vocal breath
uses the delayed magnitude rather than the raw intensity.

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight probe_pair_delay_adjusts_physical_vocal_magnitude -- --nocapture
```

Expected before implementation: FAIL because the existing delay adjustment
does not reach `vocal_organ`.

- [x] **Step 3: Implement minimal physical adjustment**

Ensure the vocal carrier helper applies Begin / Adjust / Decay / Stop to
`vocal_organ` with the same magnitude used for `action_output`. Keep pitch and
resonance as existing numeric carrier shape, falling back to neutral midpoint
values only when the carrier is idle.

- [x] **Step 4: Run focused tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight probe_law_active_tick_starts_physical_vocal_write -- --nocapture
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight probe_pair_delay_adjusts_physical_vocal_magnitude -- --nocapture
```

Expected: PASS.

### Task 11: Direct Probe Triggers Use The Same Physical Carrier

**Files:**
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Modify: `bionic_v0/src/probe_law.rs`

- [x] **Step 1: Write and verify RED for surprise trigger**

Add a test asserting `tick_surprise_trigger` creates both an epoch and vocal
activity. Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight surprise_trigger_starts_physical_vocal_write -- --nocapture
```

- [x] **Step 2: Route surprise trigger through the physical carrier**

Replace the direct `apply_action_output_event` call with the probe vocal
carrier bridge and rerun the focused test.

- [x] **Step 3: Write and verify RED for relation instability trigger**

Add a test asserting `tick_relation_instability_trigger` creates both an epoch
and vocal activity. Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight relation_instability_trigger_starts_physical_vocal_write -- --nocapture
```

- [x] **Step 4: Route relation trigger through the physical carrier**

Replace the direct `apply_action_output_event` call with the probe vocal
carrier bridge and rerun the focused test.

- [x] **Step 5: Write and verify RED for attention adjust trigger**

Add a test asserting `tick_attention_trigger` changes the physical vocal
magnitude when an active write carrier exists. Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight attention_trigger_adjusts_physical_vocal_write -- --nocapture
```

- [x] **Step 6: Route attention trigger through the physical carrier**

Replace the direct `apply_action_output_event` call with the probe vocal
carrier bridge and rerun the focused test.

### Task 12: Final Continuation Verification

**Files:**
- Test only unless failures require code changes.

- [x] **Step 1: Run live preflight tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight -- --nocapture
```

- [x] **Step 2: Run probe and vocal guard tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_probe_entry --test probe_tick --test vocal_runtime -- --nocapture
```

- [x] **Step 3: Run full suite**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```
