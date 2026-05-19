# Five-Phase Life Dashboard And Fire Live Closure Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build observer-side five-phase life balance evidence and a strict fire-force live-closure gate without adding five-phase ontology to the body.

**Architecture:** Add a focused observer/report module that derives wood, fire, earth, metal, and water balance from existing `SnapshotData` and live-nursery evidence. The module must never write to `Viewport`; it only maps numeric substrate curves into observer-side acceptance evidence. Fire live closure is accepted only from runtime snapshot evidence, not deterministic fixtures.

**Tech Stack:** Rust, Cargo integration tests, existing `SnapshotData`, `live_human_nursery`, `self_development`, `surface_projection`, and `curriculum_report`.

---

## Scope

This plan implements phases 0 and 1 from the five-phase life design:

1. Phase 0: five-phase life dashboard.
2. Phase 1: fire-force live closure gate.

It does not implement real camera hardware, multi-snapshot session recording, or body-side five-phase state. Those are separate later plans.

## File Structure

- Create: `bionic_v0/src/five_phase_life.rs`
  - Owns observer-side `FivePhaseLifeEvidence`.
  - Derives five bounded force scores from snapshots.
  - Computes insufficient/excess flags and fire live closure gate.
  - Calls existing live-nursery summarization for write/return/closure evidence.
- Modify: `bionic_v0/src/lib.rs`
  - Exports `five_phase_life`.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Adds `--five-phase-life [runtime_dir]`.
  - Adds `--fire-live-closure [runtime_dir]`.
  - Prints deterministic/runtime provenance clearly.
- Create: `bionic_v0/tests/five_phase_life.rs`
  - Tests balance scoring, fire insufficiency, fire runtime gate, source guard.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Tests report output for both new modes.
- Create after implementation: `docs/superpowers/notes/2026-05-20-five-phase-life-dashboard-and-fire-live-closure-evidence.md`
  - Records commands and numeric evidence.

## Chunk 1: Five-Phase Life Evidence

### Task 1: Add Observer-Side Five-Phase Evidence Type

**Files:**
- Create: `bionic_v0/src/five_phase_life.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/five_phase_life.rs`

- [ ] **Step 1: Write the failing bounded evidence test**

Create `bionic_v0/tests/five_phase_life.rs` with a minimal snapshot builder and this test:

```rust
use bionic_v0::active_reasoning::ActiveReasoningGapState;
use bionic_v0::five_phase_life::summarize_five_phase_life;
use bionic_v0::self_development::SelfDevelopmentState;
use bionic_v0::snapshot::{SnapshotActionConsequence, SnapshotActionOutput, SnapshotData};
use bionic_v0::viewport::{GlobalReasoningMemorySummary, GlobalReasoningState};

fn empty_snapshot() -> SnapshotData {
    SnapshotData {
        seq: 0,
        master_tick: 0,
        clock_us: 0,
        start_us: 0,
        source_writes: 0,
        predictor_ticks: 0,
        snapshot_fail_count: 0,
        history_cap: 0,
        axis_count: 0,
        hearing_fault: false,
        hearing_fault_count: 0,
        hl_a: 0.0,
        hl_avg_mse: 0.0,
        hl_pred_mse: 0.0,
        hl_meta_err: 0.0,
        hl_modulation: 0.0,
        last_obs_t_us: 0,
        health_state: 0,
        phot_health: 0,
        phot_started_t_us: 0,
        phot_last_frame_t_us: 0,
        axes: Vec::new(),
        ambient_nomination_seq: 0,
        ambient_vib: Default::default(),
        ambient_phot: Default::default(),
        ambient_mech: Default::default(),
        slots: Vec::new(),
        last_claim: Default::default(),
        global_attention: Default::default(),
        global_relations: Default::default(),
        global_reasoning: GlobalReasoningState::default(),
        global_reasoning_memory: GlobalReasoningMemorySummary::default(),
        language_binding: Default::default(),
        vpc_contract: Default::default(),
        action_output: SnapshotActionOutput::default(),
        action_consequence: SnapshotActionConsequence::default(),
        probe_law: Default::default(),
        vocal_target: Default::default(),
        vocal_organ: Default::default(),
        vocal_imitation: Default::default(),
        vocal_memory: Default::default(),
        code_ingress: Default::default(),
        learned_bias: Default::default(),
        adaptive_stability: Default::default(),
        evidence_tension: Default::default(),
        framed_byte_egress: Default::default(),
        active_reasoning_gap: ActiveReasoningGapState::default(),
        self_development: SelfDevelopmentState::default(),
    }
}

#[test]
fn five_phase_life_scores_are_bounded_and_observer_side() {
    let mut snap = empty_snapshot();
    snap.active_reasoning_gap.gap_pressure = 0.72;
    snap.active_reasoning_gap.closure_score = 0.44;
    snap.active_reasoning_gap.reentry_score = 0.38;
    snap.global_reasoning_memory.novelty_score = 0.61;
    snap.global_reasoning_memory.persistence_score = 0.54;
    snap.self_development.exploratory_pressure = 0.58;
    snap.self_development.stable_recurrence = 0.63;
    snap.self_development.familiarity_pressure = 0.57;
    snap.self_development.output_suppression = 0.20;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert!((0.0..=1.0).contains(&evidence.wood_balance));
    assert!((0.0..=1.0).contains(&evidence.fire_balance));
    assert!((0.0..=1.0).contains(&evidence.earth_balance));
    assert!((0.0..=1.0).contains(&evidence.metal_balance));
    assert!((0.0..=1.0).contains(&evidence.water_balance));
    assert_eq!(evidence.principle_guard_pass, 1);
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life five_phase_life_scores_are_bounded_and_observer_side
```

Expected: FAIL because `bionic_v0::five_phase_life` does not exist.

- [ ] **Step 3: Implement minimal evidence module**

Create `bionic_v0/src/five_phase_life.rs`:

```rust
use crate::live_human_nursery::summarize_live_human_nursery;
use crate::snapshot::SnapshotData;
use crate::{CONSEQUENCE_WINDOW_CAP, WRITE_EPOCH_HISTORY_CAP};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FivePhaseLifeEvidence {
    pub deterministic_fixture: u8,
    pub runtime_snapshot_read: u8,
    pub snapshots_seen: u32,
    pub wood_balance: f64,
    pub fire_balance: f64,
    pub earth_balance: f64,
    pub metal_balance: f64,
    pub water_balance: f64,
    pub wood_insufficient: u8,
    pub fire_insufficient: u8,
    pub earth_insufficient: u8,
    pub metal_insufficient: u8,
    pub water_insufficient: u8,
    pub wood_excess: u8,
    pub fire_excess: u8,
    pub earth_excess: u8,
    pub metal_excess: u8,
    pub water_excess: u8,
    pub fire_live_closure_gate_pass: u8,
    pub wood_fire_non_suppression: u8,
    pub principle_guard_pass: u8,
}

pub fn summarize_five_phase_life<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
    deterministic_fixture: u8,
    runtime_snapshot_read: u8,
) -> FivePhaseLifeEvidence {
    let snapshots: Vec<&SnapshotData> = snapshots.into_iter().collect();
    let live = summarize_live_human_nursery(snapshots.iter().copied());
    let mut evidence = FivePhaseLifeEvidence {
        deterministic_fixture,
        runtime_snapshot_read,
        snapshots_seen: live.snapshots_seen,
        ..Default::default()
    };

    for snap in snapshots {
        evidence.wood_balance = evidence.wood_balance.max(wood_score(snap));
        evidence.fire_balance = evidence.fire_balance.max(fire_score(snap));
        evidence.earth_balance = evidence.earth_balance.max(earth_score(snap));
        evidence.metal_balance = evidence.metal_balance.max(metal_score(snap));
        evidence.water_balance = evidence.water_balance.max(water_score(snap));
    }

    evidence.wood_insufficient = low(evidence.wood_balance);
    evidence.fire_insufficient = low(evidence.fire_balance);
    evidence.earth_insufficient = low(evidence.earth_balance);
    evidence.metal_insufficient = low(evidence.metal_balance);
    evidence.water_insufficient = low(evidence.water_balance);

    evidence.wood_excess = excess(evidence.wood_balance);
    evidence.fire_excess = excess(evidence.fire_balance);
    evidence.earth_excess = excess(evidence.earth_balance);
    evidence.metal_excess = excess(evidence.metal_balance);
    evidence.water_excess = excess(evidence.water_balance);

    evidence.fire_live_closure_gate_pass = u8::from(
        runtime_snapshot_read == 1
            && deterministic_fixture == 0
            && live.body_write_out_observed == 1
            && live.vocal_write_epochs > 0
            && live.return_windows > 0
            && live.temporal_pairing_observed == 1
            && live.closure_or_reentry_observed == 1,
    );
    evidence.wood_fire_non_suppression = u8::from(
        evidence.metal_balance < 0.92
            || evidence.wood_balance.max(evidence.fire_balance) >= 0.20,
    );
    evidence.principle_guard_pass = u8::from(all_bounded(&evidence));
    evidence
}

fn wood_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap.gap_pressure
            .max(snap.probe_law.intensity)
            .max(snap.global_reasoning_memory.novelty_score)
            .max(snap.self_development.exploratory_pressure),
    )
}

fn fire_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.vocal_target.active_energy
            .max(snap.vocal_organ.breath_drive)
            .max(ratio(snap.action_output.n_write_epochs, WRITE_EPOCH_HISTORY_CAP))
            .max(snap.framed_byte_egress.output_pressure)
            .max(snap.probe_law.output_intensity),
    )
}

fn earth_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .closure_score
            .max(snap.self_development.familiarity_pressure)
            .max(snap.self_development.stable_recurrence)
            .max(snap.global_reasoning_memory.routing_stability_score),
    )
}

fn metal_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.probe_law
            .suppression
            .max(snap.self_development.output_suppression),
    )
}

fn water_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .reentry_score
            .max(snap.self_development.solitary_reentry_score)
            .max(snap.self_development.quiet_replay_pressure)
            .max(snap.global_reasoning_memory.persistence_score),
    )
}

fn low(value: f64) -> u8 {
    u8::from(value < 0.05)
}

fn excess(value: f64) -> u8 {
    u8::from(value > 0.92)
}

fn ratio(value: u32, cap: usize) -> f64 {
    if cap == 0 { 0.0 } else { unit(value as f64 / cap as f64) }
}

fn all_bounded(evidence: &FivePhaseLifeEvidence) -> bool {
    [
        evidence.wood_balance,
        evidence.fire_balance,
        evidence.earth_balance,
        evidence.metal_balance,
        evidence.water_balance,
    ]
    .into_iter()
    .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
}

fn unit(value: f64) -> f64 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}
```

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod five_phase_life;
```

- [ ] **Step 4: Run test to verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life five_phase_life_scores_are_bounded_and_observer_side
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/five_phase_life.rs /Users/joe/unnamed/bionic_v0/src/lib.rs /Users/joe/unnamed/bionic_v0/tests/five_phase_life.rs
git commit -m "feat: add five phase life evidence"
```

## Chunk 2: Balance Flags And Source Guards

### Task 2: Detect Fire Insufficiency And Metal Over-Suppression

**Files:**
- Modify: `bionic_v0/tests/five_phase_life.rs`
- Modify: `bionic_v0/src/five_phase_life.rs`

- [ ] **Step 1: Write failing fire insufficiency test**

Append:

```rust
#[test]
fn fire_is_insufficient_when_pressure_has_no_manifestation() {
    let mut snap = empty_snapshot();
    snap.active_reasoning_gap.gap_pressure = 0.80;
    snap.global_reasoning_memory.novelty_score = 0.50;
    snap.self_development.exploratory_pressure = 0.52;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert!(evidence.wood_balance > 0.40);
    assert_eq!(evidence.fire_insufficient, 1);
    assert_eq!(evidence.fire_live_closure_gate_pass, 0);
}
```

- [ ] **Step 2: Write failing metal non-suppression test**

Append:

```rust
#[test]
fn metal_excess_flags_when_suppression_blocks_wood_and_fire() {
    let mut snap = empty_snapshot();
    snap.probe_law.suppression = 0.96;
    snap.self_development.output_suppression = 0.94;

    let evidence = summarize_five_phase_life([snap].iter(), 1, 0);

    assert_eq!(evidence.metal_excess, 1);
    assert_eq!(evidence.wood_fire_non_suppression, 0);
}
```

- [ ] **Step 3: Run tests to verify RED or partial RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life
```

Expected: FAIL until insufficiency/excess logic matches the tests.

- [ ] **Step 4: Adjust thresholds minimally**

If the tests fail because the initial scoring produces unwanted metal baseline from ambiguity, adjust `metal_score` so it uses only explicit suppression/gain-cap signals available in `SnapshotData` for v0:

```rust
fn metal_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.probe_law
            .suppression
            .max(snap.self_development.output_suppression),
    )
}
```

Do not add semantic/source/personality/identity terms.

- [ ] **Step 5: Add source guard test**

Append:

```rust
#[test]
fn five_phase_life_sources_do_not_introduce_body_ontology() {
    let src = include_str!("../src/five_phase_life.rs");
    for forbidden in [
        "semantic_readiness",
        "concept_id",
        "token_id",
        "object_name",
        "meaning",
        "personality",
        "identity",
        "owner",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "five phase life evidence must not contain `{forbidden}`"
        );
    }
}
```

- [ ] **Step 6: Run tests to verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/five_phase_life.rs /Users/joe/unnamed/bionic_v0/tests/five_phase_life.rs
git commit -m "test: cover five phase balance flags"
```

## Chunk 3: Fire Live Closure Gate

### Task 3: Require Runtime Evidence For Fire Closure

**Files:**
- Modify: `bionic_v0/tests/five_phase_life.rs`
- Modify: `bionic_v0/src/five_phase_life.rs`

- [ ] **Step 1: Write failing deterministic fixture refusal test**

Use existing live-nursery sample snapshots:

```rust
use bionic_v0::live_human_nursery::sample_live_human_nursery_snapshots;

#[test]
fn fire_live_closure_refuses_deterministic_fixture_even_when_closure_exists() {
    let snapshots = sample_live_human_nursery_snapshots();
    let evidence = summarize_five_phase_life(snapshots.iter(), 1, 0);

    assert!(evidence.fire_balance > 0.0);
    assert_eq!(evidence.fire_live_closure_gate_pass, 0);
}
```

- [ ] **Step 2: Write failing runtime acceptance test**

Append:

```rust
#[test]
fn fire_live_closure_accepts_runtime_snapshot_with_write_return_and_closure() {
    let snapshots = sample_live_human_nursery_snapshots();
    let evidence = summarize_five_phase_life(snapshots.iter(), 0, 1);

    assert!(evidence.fire_balance > 0.0);
    assert_eq!(evidence.fire_live_closure_gate_pass, 1);
    assert_eq!(evidence.principle_guard_pass, 1);
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life fire_live_closure
```

Expected: PASS after Task 1 logic exists. If not, fix only the provenance gate.

- [ ] **Step 4: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/five_phase_life.rs /Users/joe/unnamed/bionic_v0/tests/five_phase_life.rs
git commit -m "feat: require runtime evidence for fire closure"
```

## Chunk 4: Report Modes

### Task 4: Add `curriculum_report` Modes

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Write failing report test for dashboard mode**

Append to `bionic_v0/tests/curriculum_report.rs`:

```rust
#[test]
fn five_phase_life_report_emits_balance_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--five-phase-life")
        .output()
        .expect("run curriculum_report five phase life");

    assert!(
        output.status.success(),
        "five phase life report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=five_phase_life"));
    assert!(stdout.contains("deterministic_fixture=1"));
    assert!(stdout.contains("runtime_snapshot_read=0"));
    assert!(stdout.contains("wood_balance="));
    assert!(stdout.contains("fire_balance="));
    assert!(stdout.contains("earth_balance="));
    assert!(stdout.contains("metal_balance="));
    assert!(stdout.contains("water_balance="));
    assert!(stdout.contains("fire_live_closure_gate_pass=0"));
    assert!(stdout.contains("principle_guard_pass="));
}
```

- [ ] **Step 2: Write failing report test for fire mode**

Append:

```rust
#[test]
fn fire_live_closure_report_refuses_fixture_as_live_pass() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--fire-live-closure")
        .output()
        .expect("run curriculum_report fire live closure");

    assert!(
        output.status.success(),
        "fire live closure report should exit successfully: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=fire_live_closure"));
    assert!(stdout.contains("deterministic_fixture=1"));
    assert!(stdout.contains("runtime_snapshot_read=0"));
    assert!(stdout.contains("fire_balance="));
    assert!(stdout.contains("fire_live_closure_gate_pass=0"));
}
```

- [ ] **Step 3: Run report tests and verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report five_phase_life_report_emits_balance_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass
```

Expected: FAIL because the report modes do not exist.

- [ ] **Step 4: Add argument parsing**

Modify `parse_args()` in `bionic_v0/src/bin/curriculum_report.rs`:

- Add booleans `five_phase_life` and `fire_live_closure`.
- Accept `--five-phase-life`.
- Accept `--fire-live-closure`.
- Return the new booleans in the tuple.
- Keep existing report modes stable.

Add optional runtime dir helpers:

```rust
fn five_phase_runtime_dir_arg(flag: &str) -> Option<PathBuf> {
    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg == flag {
            if let Some(next) = args.peek() {
                if !next.starts_with("--") {
                    return args.next().map(PathBuf::from);
                }
            }
            return None;
        }
    }
    None
}
```

- [ ] **Step 5: Add report rendering helper**

Add:

```rust
fn print_five_phase_life(mode: &str, runtime_dir_arg: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let deterministic_fixture = if runtime_dir_arg.is_some() { 0 } else { 1 };
    let runtime_snapshot_read = if runtime_dir_arg.is_some() { 1 } else { 0 };
    let snapshots = if let Some(runtime_dir) = runtime_dir_arg {
        vec![read_snapshot(&runtime_dir.join("snapshot.bin"))?]
    } else {
        sample_live_human_nursery_snapshots()
    };
    let evidence = bionic_v0::five_phase_life::summarize_five_phase_life(
        snapshots.iter(),
        deterministic_fixture,
        runtime_snapshot_read,
    );

    println!("mode={mode}");
    println!("deterministic_fixture={}", evidence.deterministic_fixture);
    println!("runtime_snapshot_read={}", evidence.runtime_snapshot_read);
    println!("snapshots_seen={}", evidence.snapshots_seen);
    println!("wood_balance={:.6}", evidence.wood_balance);
    println!("fire_balance={:.6}", evidence.fire_balance);
    println!("earth_balance={:.6}", evidence.earth_balance);
    println!("metal_balance={:.6}", evidence.metal_balance);
    println!("water_balance={:.6}", evidence.water_balance);
    println!("wood_insufficient={}", evidence.wood_insufficient);
    println!("fire_insufficient={}", evidence.fire_insufficient);
    println!("earth_insufficient={}", evidence.earth_insufficient);
    println!("metal_insufficient={}", evidence.metal_insufficient);
    println!("water_insufficient={}", evidence.water_insufficient);
    println!("wood_excess={}", evidence.wood_excess);
    println!("fire_excess={}", evidence.fire_excess);
    println!("earth_excess={}", evidence.earth_excess);
    println!("metal_excess={}", evidence.metal_excess);
    println!("water_excess={}", evidence.water_excess);
    println!(
        "fire_live_closure_gate_pass={}",
        evidence.fire_live_closure_gate_pass
    );
    println!(
        "wood_fire_non_suppression={}",
        evidence.wood_fire_non_suppression
    );
    println!("principle_guard_pass={}", evidence.principle_guard_pass);
    Ok(())
}
```

Wire:

```rust
} else if five_phase_life {
    print_five_phase_life("five_phase_life", five_phase_runtime_dir_arg("--five-phase-life"))?;
} else if fire_live_closure {
    print_five_phase_life(
        "fire_live_closure",
        five_phase_runtime_dir_arg("--fire-live-closure"),
    )?;
}
```

- [ ] **Step 6: Run report tests and verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report five_phase_life_report_emits_balance_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs /Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs
git commit -m "feat: report five phase life balance"
```

## Chunk 5: Evidence And Full Verification

### Task 5: Record Evidence And Run Focused Suite

**Files:**
- Create: `docs/superpowers/notes/2026-05-20-five-phase-life-dashboard-and-fire-live-closure-evidence.md`

- [ ] **Step 1: Run focused tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report five_phase_life_report_emits_balance_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass
```

Expected: PASS.

- [ ] **Step 2: Run report commands**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --five-phase-life
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --fire-live-closure
```

Expected:

- `mode=five_phase_life`
- `wood_balance=...`
- `fire_balance=...`
- `earth_balance=...`
- `metal_balance=...`
- `water_balance=...`
- `fire_live_closure_gate_pass=0` for deterministic fixture
- `mode=fire_live_closure`
- `deterministic_fixture=1`
- `runtime_snapshot_read=0`
- `fire_live_closure_gate_pass=0`

- [ ] **Step 3: Run guard and formatting checks**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test live_human_nursery
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test self_development_life_loop
git diff --check
```

Expected: PASS / no output from `git diff --check`.

- [ ] **Step 4: Write evidence note**

Create `docs/superpowers/notes/2026-05-20-five-phase-life-dashboard-and-fire-live-closure-evidence.md`:

````markdown
# Five-Phase Life Dashboard And Fire Live Closure Evidence

Date: 2026-05-20

## Boundary

This evidence is observer-side only. Five-phase force names are report and
acceptance vocabulary. Body-side state remains numeric substrate curves and
does not store wood/fire/earth/metal/water ontology.

## Commands

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test five_phase_life
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report five_phase_life_report_emits_balance_evidence
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report fire_live_closure_report_refuses_fixture_as_live_pass
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --five-phase-life
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --fire-live-closure
git diff --check
```

## Acceptance

- Five bounded force balances are reported.
- Deterministic fixture evidence does not pass the fire live-closure gate.
- Source guards reject rejected ontology and body-readable output.
- Runtime acceptance remains pending until a real runtime snapshot is supplied.
````

- [ ] **Step 5: Optional broader verification**

Run if time allows:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: PASS. If this fails due to unrelated baseline debt, record the exact failing tests and keep focused tests as the implementation gate.

- [ ] **Step 6: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/five_phase_life.rs \
  /Users/joe/unnamed/bionic_v0/src/lib.rs \
  /Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs \
  /Users/joe/unnamed/bionic_v0/tests/five_phase_life.rs \
  /Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs \
  /Users/joe/unnamed/docs/superpowers/notes/2026-05-20-five-phase-life-dashboard-and-fire-live-closure-evidence.md
git commit -m "feat: add five phase life dashboard"
```

## Execution Notes

- Use @superpowers:subagent-driven-development if subagents are available; otherwise use @superpowers:executing-plans.
- Keep all five-phase wording observer-side or in docs.
- Do not add body-side five-phase fields.
- Do not change `SNAPSHOT_VERSION`; this plan reads existing snapshot fields only.
- Do not claim live fire closure from deterministic fixture output.
- A real live pass requires `--fire-live-closure <runtime_dir>` with `runtime_snapshot_read=1` and `fire_live_closure_gate_pass=1`.
