# Self-Model Boundary v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `SelfDevelopmentState` with three numeric self-boundary layers — vocal self-correlation, body state awareness, agency pressure — using only existing substrate data paths in `tick_self_development_from_substrate`.

**Architecture:** All changes in `self_development.rs`. Extend `SelfDevelopmentState` (3 layers × 3 fields), extend `SelfDevelopmentSample` (new inputs), extend `tick_self_development_from_substrate` (read new sources), extend `update()` (EMA computation). Snapshot serializer already handles `SelfDevelopmentState` wholesale — just extend the read/write functions. No new modules. No viewport changes.

**Tech Stack:** Rust, existing bionic_v0 viewport/snapshot/test infrastructure.

---

## Task 1: Layer 1 — Self-Vocal Correlation

**Files:** Modify `bionic_v0/src/self_development.rs`, test `bionic_v0/tests/self_development_life_loop.rs`

- [ ] **Step 1: Write failing test**

Add to `bionic_v0/tests/self_development_life_loop.rs`:

```rust
use bionic_v0::action_consequence::{
    ActionConsequencePair, ActionConsequenceState, ActionConsequenceWindow,
    ConsequenceModalityKind, ConsequencePairStatus, ConsequenceWindowPhase,
};
use bionic_v0::action_output::{
    ActionOutputPhase, ActionOutputState, OutputChannelKind, WriteEpochRecord,
};

#[test]
fn self_vocal_correlation_distinguishes_own_voice_from_external_sound() {
    let vp = Viewport::new();

    // Seed vocal write epoch
    {
        let mut output = vp.action_output.write().unwrap();
        output.write_epoch_history.push(WriteEpochRecord {
            write_seq: 1,
            channel_kind: OutputChannelKind::VocalWrite,
            start_magnitude: 0.72,
            start_t_us: 3_000_000,
            last_update_t_us: 3_000_500,
            last_phase: ActionOutputPhase::Active,
        });
        output.n_write_epochs = 1;
    }

    // Seed paired VIB consequence — short delay, high similarity
    {
        let mut c = vp.action_consequence.write().unwrap();
        c.windows.push(ActionConsequenceWindow {
            window_id: 1, modality_kind: ConsequenceModalityKind::VibIngress,
            start_t_us: 3_000_100, end_t_us: 3_000_600, magnitude: 0.68,
            phase: ConsequenceWindowPhase::Closed, last_update_t_us: 3_000_600,
        });
        c.n_windows = 1;
        c.pairs.push(ActionConsequencePair {
            pair_seq: 1, write_seq: 1, window_id: 1, delay_us: 500,
            status: ConsequencePairStatus::Paired, pair_t_us: 3_000_600,
            curve_similarity: 0.88,
        });
        c.n_pairs = 1; c.total_pairs = 1;
    }

    // Baseline reasoning + memory
    {
        let mut r = vp.global_reasoning.write().unwrap();
        r.global_surprise_load = 0.05; r.last_update_t_us = 3_000_000;
    }
    {
        let mut m = vp.global_reasoning_memory_summary.write().unwrap();
        m.pattern_recurrence_score = 0.50; m.routing_stability_score = 0.50;
        m.dominant_focus_persistence = 0.50; m.last_update_t_us = 3_000_000;
    }

    vp.tick_self_development_from_substrate(3_001_000);
    let state = vp.read_self_development();

    assert!(state.self_vocal_correlation > 0.0,
        "self_vocal_correlation > 0 when vocal output pairs with VIB return");
    assert!(state.self_other_separation > 0.0,
        "self_other_separation > 0 when own voice distinguishable");
    assert!(state.external_vocal_correlation >= 0.0);
}
```

- [ ] **Step 2: Run test, confirm FAIL**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test self_development_life_loop self_vocal_correlation 2>&1 | tail -5
```

- [ ] **Step 3: Add fields to SelfDevelopmentState**

In `bionic_v0/src/self_development.rs`, add after `total_updates`:

```rust
    // Layer 1: self-vocal boundary
    pub self_vocal_correlation: f64,
    pub external_vocal_correlation: f64,
    pub self_other_separation: f64,
```

Update `Default` for `SelfDevelopmentState` — since it uses `#[derive(Default)]` on a struct with all `f64`/`u64`/`u32` fields, the new `f64` fields default to `0.0` automatically. No manual Default change needed.

- [ ] **Step 4: Add fields to SelfDevelopmentSample**

Add after `average_pair_similarity`:

```rust
    self_vocal_correlation: f64,
    external_vocal_correlation: f64,
```

- [ ] **Step 5: Add compute function**

Add before `impl Viewport`:

```rust
fn compute_vocal_boundary(
    write_epochs: &[WriteEpochRecord],
    windows: &[ActionConsequenceWindow],
    pairs: &[ActionConsequencePair],
) -> (f64, f64) {
    let vocal_writes: Vec<u64> = write_epochs
        .iter()
        .filter(|ep| ep.channel_kind == OutputChannelKind::VocalWrite)
        .map(|ep| ep.write_seq)
        .collect();

    let mut self_corr = 0.0_f64;
    let mut ext_corr = 0.0_f64;

    let paired_window_ids: std::collections::BTreeSet<u64> = pairs
        .iter()
        .filter(|p| p.status == ConsequencePairStatus::Paired)
        .map(|p| p.window_id)
        .collect();

    for pair in pairs {
        if pair.status != ConsequencePairStatus::Paired { continue; }
        let window = match windows.iter().find(|w| w.window_id == pair.window_id) {
            Some(w) => w, None => continue,
        };
        if window.modality_kind != ConsequenceModalityKind::VibIngress { continue; }
        if !vocal_writes.contains(&pair.write_seq) { continue; }

        let delay_norm = unit(1.0 - (pair.delay_us as f64 / 500_000.0).min(1.0));
        let score = delay_norm * unit(pair.curve_similarity) * unit(window.magnitude);
        self_corr = self_corr.max(score);
    }

    for window in windows {
        if window.modality_kind == ConsequenceModalityKind::VibIngress
            && !paired_window_ids.contains(&window.window_id)
        {
            ext_corr = ext_corr.max(unit(window.magnitude));
        }
    }

    (unit(self_corr), unit(ext_corr))
}
```

- [ ] **Step 6: Wire into tick_self_development_from_substrate**

In `tick_self_development_from_substrate`, after reading `action_consequence` and before constructing `sample`, add:

```rust
        let (self_vocal_corr, external_vocal_corr) = compute_vocal_boundary(
            &action_output.write_epoch_history,
            &action_consequence.windows,
            &action_consequence.pairs,
        );
```

In the `SelfDevelopmentSample` construction, add:

```rust
            self_vocal_correlation: self_vocal_corr,
            external_vocal_correlation: external_vocal_corr,
```

- [ ] **Step 7: Add EMA in update()**

At the end of `update()`, before `self.total_updates`, add:

```rust
        self.self_vocal_correlation =
            ema(self.self_vocal_correlation, sample.self_vocal_correlation, 0.22);
        self.external_vocal_correlation =
            ema(self.external_vocal_correlation, sample.external_vocal_correlation, 0.18);
        let sep = self.self_vocal_correlation + self.external_vocal_correlation;
        let raw_sep = if sep > 1e-9 { self.self_vocal_correlation / sep } else { 0.0 };
        self.self_other_separation = ema(self.self_other_separation, raw_sep, 0.30);
```

- [ ] **Step 8: Run test, confirm PASS**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test self_development_life_loop self_vocal_correlation 2>&1 | tail -3
```

- [ ] **Step 9: Commit**

```bash
git add bionic_v0/src/self_development.rs bionic_v0/tests/self_development_life_loop.rs
git commit -m "feat: add self-vocal correlation layer to self-model boundary"
```

---

## Task 2: Layer 2 + 3 — Body State Awareness + Agency Pressure

**Files:** Modify `bionic_v0/src/self_development.rs`, test `bionic_v0/tests/self_development_life_loop.rs`

- [ ] **Step 1: Write failing test for both layers**

```rust
#[test]
fn body_state_and_agency_respond_to_health_and_pair_patterns() {
    let vp = Viewport::new();

    // Baseline reasoning
    {
        let mut r = vp.global_reasoning.write().unwrap();
        r.global_surprise_load = 0.12; r.last_update_t_us = 4_000_000;
    }
    {
        let mut m = vp.global_reasoning_memory_summary.write().unwrap();
        m.pattern_recurrence_score = 0.35; m.routing_stability_score = 0.35;
        m.dominant_focus_persistence = 0.35; m.last_update_t_us = 4_000_000;
    }

    // Set phot_health to OK
    vp.set_phot_health(bionic_v0::HealthState::Ok);

    // High-agency: vocal write → paired VIB return (pair ratio high)
    {
        let mut output = vp.action_output.write().unwrap();
        output.write_epoch_history.push(WriteEpochRecord {
            write_seq: 1,
            channel_kind: OutputChannelKind::VocalWrite,
            start_magnitude: 0.65,
            start_t_us: 4_000_000, last_update_t_us: 4_000_400,
            last_phase: ActionOutputPhase::Active,
        });
        output.n_write_epochs = 1;
    }
    {
        let mut c = vp.action_consequence.write().unwrap();
        c.windows.push(ActionConsequenceWindow {
            window_id: 1, modality_kind: ConsequenceModalityKind::VibIngress,
            start_t_us: 4_000_200, end_t_us: 4_000_800, magnitude: 0.60,
            phase: ConsequenceWindowPhase::Closed, last_update_t_us: 4_000_800,
        });
        c.n_windows = 1;
        c.pairs.push(ActionConsequencePair {
            pair_seq: 1, write_seq: 1, window_id: 1, delay_us: 400,
            status: ConsequencePairStatus::Paired, pair_t_us: 4_000_800,
            curve_similarity: 0.85,
        });
        c.n_pairs = 1; c.total_pairs = 1;
    }

    vp.tick_self_development_from_substrate(4_001_000);
    let state = vp.read_self_development();

    // Layer 2: body state awareness
    assert!(state.body_state_awareness >= 0.0);
    assert!(state.fatigue_pressure >= 0.0);
    assert!(state.health_surprise >= 0.0);

    // Layer 3: agency
    assert!(state.agency_pressure >= 0.0,
        "agency_pressure >= 0 when write epochs have paired consequences");
    assert!(state.self_caused_event_ratio >= 0.0);
    assert!(state.external_event_ratio >= 0.0);
}

#[test]
fn agency_pressure_high_when_most_writes_pair_and_low_when_unpaired() {
    let paired = Viewport::new();
    {
        let mut r = paired.global_reasoning.write().unwrap();
        r.global_surprise_load = 0.05; r.last_update_t_us = 5_000_000;
    }
    {
        let mut m = paired.global_reasoning_memory_summary.write().unwrap();
        m.pattern_recurrence_score = 0.40; m.routing_stability_score = 0.40;
        m.dominant_focus_persistence = 0.40; m.last_update_t_us = 5_000_000;
    }
    {
        let mut output = paired.action_output.write().unwrap();
        for idx in 0..3 {
            output.write_epoch_history.push(WriteEpochRecord {
                write_seq: idx + 1,
                channel_kind: OutputChannelKind::VocalWrite,
                start_magnitude: 0.7,
                start_t_us: 5_000_000 + idx * 100_000,
                last_update_t_us: 5_000_500 + idx * 100_000,
                last_phase: ActionOutputPhase::Active,
            });
        }
        output.n_write_epochs = 3;
    }
    {
        let mut c = paired.action_consequence.write().unwrap();
        for idx in 0..3 {
            c.windows.push(ActionConsequenceWindow {
                window_id: idx + 1, modality_kind: ConsequenceModalityKind::VibIngress,
                start_t_us: 5_000_200 + idx * 100_000,
                end_t_us: 5_000_800 + idx * 100_000,
                magnitude: 0.55, phase: ConsequenceWindowPhase::Closed,
                last_update_t_us: 5_000_800 + idx * 100_000,
            });
            c.pairs.push(ActionConsequencePair {
                pair_seq: idx + 1, write_seq: idx + 1, window_id: idx + 1,
                delay_us: 30_000,
                status: ConsequencePairStatus::Paired,
                pair_t_us: 5_000_800 + idx * 100_000,
                curve_similarity: 0.90,
            });
        }
        c.n_windows = 3; c.n_pairs = 3; c.total_pairs = 3;
    }
    paired.tick_self_development_from_substrate(5_500_000);
    let paired_state = paired.read_self_development();

    let unpaired = Viewport::new();
    {
        let mut r = unpaired.global_reasoning.write().unwrap();
        r.global_surprise_load = 0.05; r.last_update_t_us = 5_000_000;
    }
    {
        let mut m = unpaired.global_reasoning_memory_summary.write().unwrap();
        m.pattern_recurrence_score = 0.40; m.routing_stability_score = 0.40;
        m.dominant_focus_persistence = 0.40; m.last_update_t_us = 5_000_000;
    }
    {
        let mut output = unpaired.action_output.write().unwrap();
        output.write_epoch_history.push(WriteEpochRecord {
            write_seq: 1, channel_kind: OutputChannelKind::VocalWrite,
            start_magnitude: 0.7, start_t_us: 5_000_000, last_update_t_us: 5_000_500,
            last_phase: ActionOutputPhase::Active,
        });
        output.n_write_epochs = 1;
    }
    // No consequence windows or pairs — completely unpaired
    unpaired.tick_self_development_from_substrate(5_500_000);
    let unpaired_state = unpaired.read_self_development();

    assert!(paired_state.agency_pressure > unpaired_state.agency_pressure + 0.05,
        "high-pair scenario should have higher agency than zero-pair");
    assert!(paired_state.self_caused_event_ratio > unpaired_state.self_caused_event_ratio);
}
```

- [ ] **Step 2: Run test, confirm FAIL**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test self_development_life_loop body_state_and_agency agency_pressure_high 2>&1 | tail -5
```

- [ ] **Step 3: Add Layer 2 + 3 fields to SelfDevelopmentState**

```rust
    // Layer 2: body state awareness
    pub body_state_awareness: f64,
    pub fatigue_pressure: f64,
    pub health_surprise: f64,
    // Layer 3: agency pressure
    pub agency_pressure: f64,
    pub self_caused_event_ratio: f64,
    pub external_event_ratio: f64,
```

- [ ] **Step 4: Add to SelfDevelopmentSample**

```rust
    phot_health_ok: f64,
    total_updates: u64,
    paired_write_ratio: f64,
```

- [ ] **Step 5: Wire into tick_self_development_from_substrate**

After the Layer 1 block, add:

```rust
        let phot_ok = match vp.read_phot_health() {
            bionic_v0::HealthState::Ok => 1.0_f64,
            bionic_v0::HealthState::Quiet => 0.5_f64,
            bionic_v0::HealthState::Fault => 0.0_f64,
        };

        let total_writes = action_output.n_write_epochs.max(1) as f64;
        let paired_writes = action_consequence.pairs
            .iter()
            .filter(|p| p.status == ConsequencePairStatus::Paired)
            .map(|p| p.write_seq)
            .collect::<std::collections::BTreeSet<u64>>()
            .len() as f64;
        let paired_write_ratio = unit(paired_writes / total_writes);
```

In sample construction:

```rust
            phot_health_ok: phot_ok,
            total_updates: self.read_self_development().total_updates,
            paired_write_ratio,
```

- [ ] **Step 6: Add Layer 2 + 3 EMA in update()**

In `update()`, after the Layer 1 block, before `total_updates`:

```rust
        // Layer 2: body state awareness
        let health_now = sample.phot_health_ok;
        let health_prev = (self.body_state_awareness * 0.5 + 0.5).clamp(0.0, 1.0);
        self.health_surprise = ema(self.health_surprise, (health_now - health_prev).abs(), 0.40);
        self.body_state_awareness = ema(
            self.body_state_awareness,
            unit(1.0 - health_now + self.health_surprise * 0.6),
            0.25,
        );
        self.fatigue_pressure = ema(
            self.fatigue_pressure,
            unit(sample.total_updates as f64 / 10_000.0),
            0.12,
        );

        // Layer 3: agency
        let agency_input = unit(sample.paired_write_ratio * (1.0 - 0.3 * sample.prediction_gap));
        self.agency_pressure = ema(self.agency_pressure, agency_input, 0.28);
        let total_events = sample.paired_write_ratio.max(1e-9).max(1.0 - sample.paired_write_ratio);
        self.self_caused_event_ratio = ema(
            self.self_caused_event_ratio,
            sample.paired_write_ratio / total_events,
            0.30,
        );
        self.external_event_ratio = ema(
            self.external_event_ratio,
            unit(1.0 - sample.paired_write_ratio) / total_events,
            0.30,
        );
```

- [ ] **Step 7: Run all self_development tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test self_development_life_loop 2>&1 | tail -10
```

Expected: ALL PASS (7 tests).

- [ ] **Step 8: Commit**

```bash
git add bionic_v0/src/self_development.rs bionic_v0/tests/self_development_life_loop.rs
git commit -m "feat: add body state awareness and agency pressure to self-model boundary"
```

---

## Task 3: Snapshot + Source Hygiene

**Files:** Modify `bionic_v0/src/snapshot.rs`, `bionic_v0/tests/self_development_life_loop.rs`

- [ ] **Step 1: Extend snapshot serializer for new fields**

In `bionic_v0/src/snapshot.rs`, find `fn write_self_development` (around line 2594) and add after the last existing field write:

```rust
    buf.extend_from_slice(&s.self_vocal_correlation.to_le_bytes());
    buf.extend_from_slice(&s.external_vocal_correlation.to_le_bytes());
    buf.extend_from_slice(&s.self_other_separation.to_le_bytes());
    buf.extend_from_slice(&s.body_state_awareness.to_le_bytes());
    buf.extend_from_slice(&s.fatigue_pressure.to_le_bytes());
    buf.extend_from_slice(&s.health_surprise.to_le_bytes());
    buf.extend_from_slice(&s.agency_pressure.to_le_bytes());
    buf.extend_from_slice(&s.self_caused_event_ratio.to_le_bytes());
    buf.extend_from_slice(&s.external_event_ratio.to_le_bytes());
```

Update `read_self_development_snapshot` (around line 2614) to read the new fields in the same order. After the existing `total_updates` read, add:

```rust
    let self_vocal_correlation = read_f64(bytes, cur);
    let external_vocal_correlation = read_f64(bytes, cur);
    let self_other_separation = read_f64(bytes, cur);
    let body_state_awareness = read_f64(bytes, cur);
    let fatigue_pressure = read_f64(bytes, cur);
    let health_surprise = read_f64(bytes, cur);
    let agency_pressure = read_f64(bytes, cur);
    let self_caused_event_ratio = read_f64(bytes, cur);
    let external_event_ratio = read_f64(bytes, cur);
```

And add them to the return struct.

- [ ] **Step 2: Add roundtrip test assertion**

In the existing `snapshot_roundtrip_preserves_self_development_evidence` test, add after the existing assertions:

```rust
    assert!(snap.self_development.self_vocal_correlation >= 0.0);
    assert!(snap.self_development.self_other_separation >= 0.0);
    assert!(snap.self_development.body_state_awareness >= 0.0);
    assert!(snap.self_development.agency_pressure >= 0.0);
```

- [ ] **Step 3: Run all tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | tail -5
```

Expected: ALL PASS.

- [ ] **Step 4: Run source hygiene check**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test self_development_life_loop self_development_sources_stay_numeric_and_guarded 2>&1 | tail -3
```

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/src/snapshot.rs bionic_v0/tests/self_development_life_loop.rs
git commit -m "feat: serialize self-model boundary fields through snapshot"
```

---

## Task 4: Full Verification + Principle Gates

- [ ] **Step 1: Run full test suite**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | tail -5
```

- [ ] **Step 2: G1 — No semantic pollution**

```bash
grep -r "identity\|self_model_slot\|ego\|owner_id\|personality\|consciousness" bionic_v0/src/self_development.rs && echo "G1 FAIL" || echo "G1 PASS"
```

- [ ] **Step 3: G4 — No new modules**

```bash
git diff HEAD~3..HEAD --stat | grep "create mode" | grep -v "tests/" | grep -v ".md" && echo "G4 CHECK" || echo "G4 PASS"
```

- [ ] **Step 4: G5 — No forbidden patterns**

```bash
git diff HEAD~3..HEAD -- bionic_v0/src/ | grep -i "chat\|nlp\|tokeniz\|semantic\|println\|request.response" && echo "G5 FAIL" || echo "G5 PASS"
```

- [ ] **Step 5: Run existing nursery evidence**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_human_nursery --test body_no_human_output 2>&1 | tail -5
```

- [ ] **Step 6: Commit verification**

```bash
git add -A && git diff --cached --stat && git commit -m "chore: verify self-model boundary — all tests pass, all gates pass"
```
