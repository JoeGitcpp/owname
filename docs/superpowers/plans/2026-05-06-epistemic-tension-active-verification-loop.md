# Epistemic Tension Active Verification Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a substrate-level loop where unresolved evidence tension raises active probe pressure, and later action-consequence return evidence can reduce that pressure without truth labels, source trust lists, or fixed verification targets.

**Architecture:** The implementation adds a numeric evidence-tension state under `Viewport`, backed by a short ring of pressure traces. Existing `EpistemicTensionFeatures` computes tension; the new state preserves unresolved load, contributes to `probe_law` intensity, and consumes new action-consequence pair evidence as closure support. This is not a verifier module: it records time, pressure, support, pair cursors, and bounded numeric curves only.

**Tech Stack:** Rust 2021, `bionic_v0` shared `Viewport`, existing `evolution_priors`, `probe_law`, `action_consequence`, snapshot/observer substrate.

---

## File Structure

- Create `bionic_v0/tests/evidence_tension_active_verification.rs`
  - Contract tests for tension pressure, probe contribution, consequence closure, snapshot visibility, and forbidden ontology strings.
- Create `bionic_v0/src/evidence_tension.rs`
  - Numeric evidence-tension state and trace ring.
  - Functions that absorb `EpistemicTensionFeatures`, compute pressure, and consume new pair evidence.
- Modify `bionic_v0/src/lib.rs`
  - Export `evidence_tension`.
  - Add caps and engineering rates.
  - Bump `SNAPSHOT_VERSION`.
- Modify `bionic_v0/src/viewport.rs`
  - Add `evidence_tension: RwLock<EvidenceTensionState>`.
  - Initialize and expose read/tick helpers.
- Modify `bionic_v0/src/probe_law.rs`
  - Include normalized evidence-tension pressure in existing probe-law intensity drive.
- Modify `bionic_v0/src/bin/body.rs`
  - Tick consequence-return closure before refreshing probe law.
- Modify `bionic_v0/src/snapshot.rs`
  - Serialize and read a snapshot-visible numeric evidence-tension state.
- Modify `bionic_v0/src/bin/observer.rs`
  - Render compact numeric observer trace only.

---

### Task 1: Evidence-Tension State

**Files:**
- Create: `bionic_v0/src/evidence_tension.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/evidence_tension_active_verification.rs`

- [ ] **Step 1: Write the failing test**

Add this test file:

```rust
use bionic_v0::evolution_priors::EpistemicTensionFeatures;
use bionic_v0::viewport::Viewport;

fn tense_features() -> EpistemicTensionFeatures {
    EpistemicTensionFeatures {
        assertion_observation_conflict: 0.9,
        prediction_error: 0.8,
        source_reliability_gap: 0.2,
        cross_modal_disagreement: 0.7,
        reproducibility_gap: 0.8,
    }
}

#[test]
fn evidence_tension_tick_records_unresolved_numeric_pressure() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_features(), 1_000_000);

    let st = vp.read_evidence_tension();
    assert!(st.current_tension > 0.0);
    assert!(st.unresolved_pressure > 0.0);
    assert_eq!(st.history_len, 1);
    assert_eq!(st.last_update_t_us, 1_000_000);
    assert_eq!(st.history[0].pair_cursor, 0);
    assert!(st.history[0].closure_support <= 1.0);
}

#[test]
fn evidence_tension_quiet_features_decay_without_truth_label() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_features(), 1_000_000);
    let high = vp.read_evidence_tension().unresolved_pressure;
    vp.tick_evidence_tension(EpistemicTensionFeatures::default(), 2_000_000);

    let st = vp.read_evidence_tension();
    assert!(st.unresolved_pressure < high);
    assert_eq!(st.history_len, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification evidence_tension_tick_records_unresolved_numeric_pressure -- --exact
```

Expected: FAIL because `evidence_tension` module and `Viewport::tick_evidence_tension` do not exist.

- [ ] **Step 3: Write minimal implementation**

Create `src/evidence_tension.rs`:

```rust
use crate::evolution_priors::{compute_epistemic_tension, BiasPriors, EpistemicTensionFeatures};
use crate::{
    EVIDENCE_TENSION_DECAY_ALPHA, EVIDENCE_TENSION_HISTORY_CAP,
    EVIDENCE_TENSION_PRESSURE_SCALE,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvidenceTensionTrace {
    pub t_us: u64,
    pub tension: f64,
    pub unresolved_pressure: f64,
    pub closure_support: f64,
    pub pair_cursor: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceTensionState {
    pub current_tension: f64,
    pub unresolved_pressure: f64,
    pub last_update_t_us: u64,
    pub last_pair_cursor: u64,
    pub history: [EvidenceTensionTrace; EVIDENCE_TENSION_HISTORY_CAP],
    pub history_head: usize,
    pub history_len: usize,
}

impl Default for EvidenceTensionState {
    fn default() -> Self {
        Self {
            current_tension: 0.0,
            unresolved_pressure: 0.0,
            last_update_t_us: 0,
            last_pair_cursor: 0,
            history: [EvidenceTensionTrace::default(); EVIDENCE_TENSION_HISTORY_CAP],
            history_head: 0,
            history_len: 0,
        }
    }
}

impl EvidenceTensionState {
    pub fn absorb_features(
        &mut self,
        features: EpistemicTensionFeatures,
        priors: &BiasPriors,
        t_us: u64,
    ) {
        let tension = compute_epistemic_tension(&features, priors);
        let normalized = (tension / EVIDENCE_TENSION_PRESSURE_SCALE).clamp(0.0, 1.0);
        if normalized >= self.unresolved_pressure {
            self.unresolved_pressure = normalized;
        } else {
            self.unresolved_pressure = self.unresolved_pressure * (1.0 - EVIDENCE_TENSION_DECAY_ALPHA)
                + normalized * EVIDENCE_TENSION_DECAY_ALPHA;
        }
        self.current_tension = tension;
        self.last_update_t_us = t_us;
        self.push_trace(t_us, tension, 0.0, self.last_pair_cursor);
    }

    fn push_trace(&mut self, t_us: u64, tension: f64, closure_support: f64, pair_cursor: u64) {
        self.history[self.history_head] = EvidenceTensionTrace {
            t_us,
            tension,
            unresolved_pressure: self.unresolved_pressure,
            closure_support: closure_support.clamp(0.0, 1.0),
            pair_cursor,
        };
        self.history_head = (self.history_head + 1) % EVIDENCE_TENSION_HISTORY_CAP;
        if self.history_len < EVIDENCE_TENSION_HISTORY_CAP {
            self.history_len += 1;
        }
    }
}
```

Modify `src/lib.rs`:

```rust
pub mod evidence_tension;

pub const EVIDENCE_TENSION_HISTORY_CAP: usize = 16;
pub const EVIDENCE_TENSION_PRESSURE_SCALE: f64 = 4.0;
pub const EVIDENCE_TENSION_DECAY_ALPHA: f64 = 0.25;
pub const EVIDENCE_TENSION_CLOSURE_ALPHA: f64 = 0.5;
pub const SNAPSHOT_VERSION: u64 = 37;
```

Modify `src/viewport.rs` by importing `EvidenceTensionState`, adding `pub evidence_tension: RwLock<EvidenceTensionState>`, initializing it in `Viewport::new`, and adding:

```rust
pub fn read_evidence_tension(&self) -> EvidenceTensionState {
    self.evidence_tension.read().unwrap().clone()
}

pub fn tick_evidence_tension(&self, features: EpistemicTensionFeatures, t_us: u64) {
    let priors = BiasPriors::default();
    self.evidence_tension
        .write()
        .unwrap()
        .absorb_features(features, &priors, t_us);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification evidence_tension_tick_records_unresolved_numeric_pressure -- --exact
cargo test --test evidence_tension_active_verification evidence_tension_quiet_features_decay_without_truth_label -- --exact
```

Expected: both tests PASS.

---

### Task 2: Probe-Law Pressure Contribution

**Files:**
- Modify: `bionic_v0/src/probe_law.rs`
- Test: `bionic_v0/tests/evidence_tension_active_verification.rs`

- [ ] **Step 1: Write the failing test**

Append:

```rust
use bionic_v0::probe_law::ProbePhase;

#[test]
fn unresolved_evidence_tension_contributes_to_probe_pressure() {
    let vp = Viewport::new();

    vp.tick_evidence_tension(tense_features(), 1_000_000);
    vp.refresh_probe_law(1_100_000);

    let st = vp.read_probe_law();
    assert!(st.intensity > 0.0);
    assert!(
        st.phase == ProbePhase::P1 || st.phase == ProbePhase::P2,
        "unresolved evidence pressure should be eligible to raise probe phase"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification unresolved_evidence_tension_contributes_to_probe_pressure -- --exact
```

Expected: FAIL because `refresh_probe_law` only reads novelty.

- [ ] **Step 3: Write minimal implementation**

In `src/probe_law.rs`, change the drive source inside `refresh_probe_law`:

```rust
let novelty = self.read_global_reasoning_memory_summary().novelty_score;
let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
let drive = novelty.max(evidence_pressure);
```

Then replace the intensity update input from `novelty` to `drive`.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification unresolved_evidence_tension_contributes_to_probe_pressure -- --exact
```

Expected: PASS.

---

### Task 3: Consequence Return Closure

**Files:**
- Modify: `bionic_v0/src/evidence_tension.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Test: `bionic_v0/tests/evidence_tension_active_verification.rs`

- [ ] **Step 1: Write the failing test**

Append:

```rust
use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};

#[test]
fn new_consequence_pair_reduces_unresolved_evidence_pressure_once() {
    let vp = Viewport::new();
    vp.tick_evidence_tension(tense_features(), 1_000_000);
    let before = vp.read_evidence_tension().unresolved_pressure;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.8,
        event_t_us: 1_100_000,
    });
    let write_seq = vp.read_action_output().write_epoch_history[0].write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_200_000,
        end_t_us: 1_250_000,
        magnitude: 0.8,
    });
    let window_id = vp.read_action_consequence().windows[0].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, 100_000));

    vp.tick_evidence_tension_from_pairs(1_300_000);
    let after_first = vp.read_evidence_tension().unresolved_pressure;
    vp.tick_evidence_tension_from_pairs(1_400_000);
    let after_second = vp.read_evidence_tension().unresolved_pressure;

    assert!(after_first < before);
    assert!(
        (after_second - after_first).abs() < 1e-12,
        "same pair cursor must not apply closure repeatedly"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification new_consequence_pair_reduces_unresolved_evidence_pressure_once -- --exact
```

Expected: FAIL because `tick_evidence_tension_from_pairs` does not exist.

- [ ] **Step 3: Write minimal implementation**

Add to `EvidenceTensionState`:

```rust
pub fn absorb_return_support(&mut self, support: f64, pair_cursor: u64, t_us: u64) {
    if pair_cursor <= self.last_pair_cursor {
        return;
    }
    let closure = support.clamp(0.0, 1.0);
    self.unresolved_pressure *= 1.0 - closure * EVIDENCE_TENSION_CLOSURE_ALPHA;
    self.current_tension = self.unresolved_pressure * EVIDENCE_TENSION_PRESSURE_SCALE;
    self.last_pair_cursor = pair_cursor;
    self.last_update_t_us = t_us;
    self.push_trace(t_us, self.current_tension, closure, pair_cursor);
}
```

Add to `Viewport`:

```rust
pub fn tick_evidence_tension_from_pairs(&self, t_us: u64) {
    let (pair_cursor, support) = {
        let ac = self.action_consequence.read().unwrap();
        let cursor = ac.n_pairs as u64;
        let support = ac
            .pairs
            .iter()
            .skip(self.evidence_tension.read().unwrap().last_pair_cursor as usize)
            .map(|p| p.curve_similarity)
            .fold(0.0_f64, f64::max);
        (cursor, support)
    };
    self.evidence_tension
        .write()
        .unwrap()
        .absorb_return_support(support, pair_cursor, t_us);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification new_consequence_pair_reduces_unresolved_evidence_pressure_once -- --exact
```

Expected: PASS.

---

### Task 4: Snapshot, Observer, And Hygiene

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/evidence_tension_active_verification.rs`

- [ ] **Step 1: Write the failing tests**

Append:

```rust
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use std::sync::Arc;

#[test]
fn evidence_tension_is_snapshot_visible_as_numeric_trace() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_evidence_tension_snapshot_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());

    vp.tick_evidence_tension(tense_features(), 1_000_000);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert!(snap.evidence_tension.current_tension > 0.0);
    assert!(snap.evidence_tension.unresolved_pressure > 0.0);
    assert_eq!(snap.evidence_tension.history_len, 1);

    let _ = std::fs::remove_file(paths.final_path);
    let _ = std::fs::remove_file(paths.tmp_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn evidence_tension_modules_do_not_contain_rejected_ontology_terms() {
    let evidence = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/evidence_tension.rs"
    ))
    .expect("read evidence_tension.rs");
    let probe = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/probe_law.rs"
    ))
    .expect("read probe_law.rs");
    let forbidden = &[
        "truth_label",
        "true_label",
        "false_label",
        "trust_list",
        "distrust_list",
        "target_object",
        "result_label",
        "semantic_gate",
        "concept_id",
        "token",
    ];
    for kw in forbidden {
        assert!(!evidence.contains(kw), "evidence_tension.rs must not contain: {}", kw);
        assert!(!probe.contains(kw), "probe_law.rs must not contain: {}", kw);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification evidence_tension_is_snapshot_visible_as_numeric_trace -- --exact
```

Expected: FAIL because snapshot does not expose `evidence_tension`.

- [ ] **Step 3: Write minimal implementation**

In `snapshot.rs`, import `EvidenceTensionState`, add `SnapshotEvidenceTension`, add `evidence_tension` to `SnapshotData`, write/read the state after `probe_law`, and use default state for older versions if needed. In `observer.rs`, append one compact numeric block:

```rust
let et = &snap.evidence_tension;
row.push_str(&format!(
    "\tetn:t{:.4}|p{:.4}|h{}|pc{}",
    et.current_tension, et.unresolved_pressure, et.history_len, et.last_pair_cursor,
));
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification evidence_tension_is_snapshot_visible_as_numeric_trace -- --exact
cargo test --test evidence_tension_active_verification evidence_tension_modules_do_not_contain_rejected_ontology_terms -- --exact
```

Expected: both tests PASS.

---

### Task 5: Body Wiring And Final Verification

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`
- Test: `bionic_v0/tests/evidence_tension_active_verification.rs`

- [ ] **Step 1: Wire closure tick into body refresh**

In `run_global_refresh_cycle`, call:

```rust
vp.tick_evidence_tension_from_pairs(t);
vp.refresh_probe_law(t);
```

with the closure tick immediately before `refresh_probe_law`.

- [ ] **Step 2: Run focused tests**

Run:

```bash
cd bionic_v0
cargo test --test evidence_tension_active_verification
cargo test --test probe_tick
cargo test --test live_probe_entry
```

Expected: all focused tests PASS.

- [ ] **Step 3: Run full verification**

Run:

```bash
cd bionic_v0
cargo test
```

Expected: full test suite PASS.

- [ ] **Step 4: Review git diff for boundary violations**

Run:

```bash
git diff -- bionic_v0/src/evidence_tension.rs bionic_v0/src/probe_law.rs bionic_v0/src/snapshot.rs bionic_v0/src/bin/body.rs bionic_v0/tests/evidence_tension_active_verification.rs
```

Expected: diff contains only numeric/time/curve substrate state and no truth/source-trust/semantic ontology.

---

## Self-Review

- Spec coverage: the plan covers unresolved evidence tension, active probe pressure contribution, consequence-return closure, observer-visible numeric traces, and forbidden ontology checks.
- Placeholder scan: no unresolved placeholder markers remain.
- Type consistency: the plan uses `EvidenceTensionState`, `EvidenceTensionTrace`, `tick_evidence_tension`, `tick_evidence_tension_from_pairs`, and `SnapshotEvidenceTension` consistently across tasks.
