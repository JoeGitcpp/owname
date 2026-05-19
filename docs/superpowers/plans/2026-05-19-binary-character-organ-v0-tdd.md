# Binary Character Organ v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add body-level binary character ear and mouth organs that separate concurrent byte-character input lanes, organize bounded byte output bursts, and close return/correction loops through existing substrate channels.

**Architecture:** Implement `BinaryCharacterEar` and `BinaryCharacterMouth` inside `/Users/joe/unnamed/bionic_v0`. The ear wraps and enriches `code_ingress`; the mouth wraps and enriches `framed_byte_egress`. Virtual Life Room only supplies virtual providers and report harnesses.

**Tech Stack:** Rust 2021, existing `bionic_v0::Viewport`, `code_ingress`, `framed_byte_egress`, snapshot binary format, Cargo integration tests, Virtual Life Room CLI provider tests.

---

## Locked Spec

Implement exactly:

`/Users/joe/unnamed/docs/superpowers/specs/2026-05-19-binary-character-organ-v0-design.md`

## File Structure

Body repo changes:

- Create: `/Users/joe/unnamed/bionic_v0/src/binary_character_ear.rs`
  - Body-side binary character ingress organ.
- Create: `/Users/joe/unnamed/bionic_v0/src/binary_character_mouth.rs`
  - Body-side binary character egress organ.
- Modify: `/Users/joe/unnamed/bionic_v0/src/lib.rs`
  - Export the two organ modules.
- Modify: `/Users/joe/unnamed/bionic_v0/src/viewport.rs`
  - Mount organ states and expose read/observe/tick APIs.
- Modify: `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`
  - Serialize/deserialize observer-visible numeric organ state.
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`
  - Add mouth drive into the body refresh order.
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs`
  - Add observer-side report mode `--binary-character-organ`.
- Create: `/Users/joe/unnamed/bionic_v0/tests/binary_character_ear.rs`
- Create: `/Users/joe/unnamed/bionic_v0/tests/binary_character_mouth.rs`
- Create: `/Users/joe/unnamed/bionic_v0/tests/binary_character_snapshot.rs`
- Create: `/Users/joe/unnamed/bionic_v0/tests/binary_character_runtime.rs`

Virtual Life Room changes:

- Create: `/Users/joe/virtual_life_room_v1/src/character_organ.rs`
  - Virtual hardware provider for binary character fibers and return/correction.
- Modify: `/Users/joe/virtual_life_room_v1/src/lib.rs`
  - Export `character_organ`.
- Modify: `/Users/joe/virtual_life_room_v1/src/bin/life_room.rs`
  - Add `--character-io`.
- Create: `/Users/joe/virtual_life_room_v1/tests/character_organ.rs`
  - Provider-level concurrent fiber and mouth routing tests.

## Task 1: Ear State And Concurrent Lane Separation

**Files:**
- Create: `/Users/joe/unnamed/bionic_v0/src/binary_character_ear.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/lib.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_ear.rs`

- [ ] **Step 1: Write failing ear lane tests**

Create `/Users/joe/unnamed/bionic_v0/tests/binary_character_ear.rs`:

```rust
use bionic_v0::binary_character_ear::{BinaryCharacterBurstInput, BinaryCharacterEarState};

#[test]
fn ear_separates_concurrent_fibers_into_distinct_lanes() {
    let mut ear = BinaryCharacterEarState::default();

    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 1,
        carrier_id: 10,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"hi",
        burst_energy: 0.4,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 2,
        carrier_id: 10,
        t_start_us: 1_020,
        t_end_us: 1_130,
        bytes: b"ok",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 3,
        carrier_id: 99,
        t_start_us: 1_040,
        t_end_us: 1_150,
        bytes: b"no",
        burst_energy: 0.6,
        framing_validity: 1.0,
    });

    assert_eq!(ear.lane_count, 3);
    assert_eq!(ear.total_bursts, 3);
    assert_eq!(ear.lanes.len(), 3);
    assert!(ear.lanes.iter().all(|lane| lane.burst_count == 1));
}

#[test]
fn ear_repeated_fiber_raises_continuity_and_recurrence() {
    let mut ear = BinaryCharacterEarState::default();
    for idx in 0..4 {
        let t = 1_000 + idx * 1_000;
        ear.observe_burst(BinaryCharacterBurstInput {
            fiber_id: 7,
            carrier_id: 3,
            t_start_us: t,
            t_end_us: t + 100,
            bytes: b"aa",
            burst_energy: 0.3,
            framing_validity: 1.0,
        });
    }

    assert_eq!(ear.lane_count, 1);
    let lane = &ear.lanes[0];
    assert_eq!(lane.burst_count, 4);
    assert!(lane.continuity_score > 0.5);
    assert!(lane.recurrence_score > 0.5);
    assert!(lane.prediction_pressure < 0.5);
}

#[test]
fn ear_collision_preserves_ambiguity_without_merging_lanes() {
    let mut ear = BinaryCharacterEarState::default();
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 1,
        carrier_id: 1,
        t_start_us: 2_000,
        t_end_us: 2_100,
        bytes: b"xx",
        burst_energy: 0.8,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 2,
        carrier_id: 1,
        t_start_us: 2_030,
        t_end_us: 2_120,
        bytes: b"xx",
        burst_energy: 0.8,
        framing_validity: 1.0,
    });

    assert_eq!(ear.lane_count, 2);
    assert!(ear.collision_pressure > 0.0);
    assert!(ear.lanes.iter().any(|lane| lane.collision_pressure > 0.0));
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_ear
```

Expected: FAIL because `binary_character_ear` does not exist.

- [ ] **Step 3: Implement minimal ear state**

Create `/Users/joe/unnamed/bionic_v0/src/binary_character_ear.rs`:

```rust
use crate::code_ingress::CodeIngressFrameInput;

pub const BINARY_CHARACTER_LANE_CAP: usize = 16;

pub struct BinaryCharacterBurstInput<'a> {
    pub fiber_id: u64,
    pub carrier_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub bytes: &'a [u8],
    pub burst_energy: f64,
    pub framing_validity: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterEarState {
    pub lane_count: u32,
    pub total_bursts: u64,
    pub last_update_t_us: u64,
    pub collision_pressure: f64,
    pub active_lane_id: u64,
    pub lanes: Vec<BinaryCharacterLaneTrace>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterLaneTrace {
    pub lane_id: u64,
    pub fiber_id: u64,
    pub carrier_id: u64,
    pub last_burst_ref: u64,
    pub last_t_us: u64,
    pub byte_hash: u64,
    pub byte_len: u32,
    pub burst_count: u32,
    pub continuity_score: f64,
    pub recurrence_score: f64,
    pub collision_pressure: f64,
    pub prediction_pressure: f64,
    pub return_correlation: f64,
}

impl BinaryCharacterEarState {
    pub fn observe_burst(&mut self, input: BinaryCharacterBurstInput<'_>) -> u64 {
        self.total_bursts = self.total_bursts.saturating_add(1);
        let burst_ref = nonzero_ref(hash_bytes(input.bytes) ^ self.total_bursts, self.total_bursts);
        let byte_hash = hash_bytes(input.bytes);
        let lane_idx = self
            .lanes
            .iter()
            .position(|lane| lane.fiber_id == input.fiber_id && lane.carrier_id == input.carrier_id)
            .unwrap_or_else(|| {
                let lane_id = self.total_bursts;
                self.lanes.push(BinaryCharacterLaneTrace {
                    lane_id,
                    fiber_id: input.fiber_id,
                    carrier_id: input.carrier_id,
                    prediction_pressure: 1.0,
                    ..BinaryCharacterLaneTrace::default()
                });
                self.lanes.len() - 1
            });

        let overlap_collision = self
            .lanes
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != lane_idx)
            .any(|(_, lane)| {
                lane.carrier_id == input.carrier_id
                    && input.t_start_us <= lane.last_t_us.saturating_add(200)
                    && lane.byte_hash == byte_hash
            });

        let lane = &mut self.lanes[lane_idx];
        let same_hash = lane.byte_hash == byte_hash && lane.burst_count > 0;
        lane.last_burst_ref = burst_ref;
        lane.last_t_us = input.t_end_us;
        lane.byte_hash = byte_hash;
        lane.byte_len = input.bytes.len().min(u32::MAX as usize) as u32;
        lane.burst_count = lane.burst_count.saturating_add(1);
        lane.continuity_score = (lane.continuity_score + 0.25).clamp(0.0, 1.0);
        if same_hash {
            lane.recurrence_score = (lane.recurrence_score + 0.35).clamp(0.0, 1.0);
        }
        if overlap_collision {
            lane.collision_pressure = (lane.collision_pressure + 0.5).clamp(0.0, 1.0);
            self.collision_pressure = (self.collision_pressure + 0.5).clamp(0.0, 1.0);
        }
        lane.prediction_pressure =
            (1.0 - (lane.continuity_score * 0.4 + lane.recurrence_score * 0.6)).clamp(0.0, 1.0);
        self.active_lane_id = lane.lane_id;
        self.last_update_t_us = input.t_end_us;
        self.evict_lanes();
        self.lane_count = self.lanes.len() as u32;
        burst_ref
    }

    pub fn lane_stream_context_id(lane_id: u64) -> u64 {
        0xB1C0_0000_0000_0000_u64 ^ lane_id
    }

    fn evict_lanes(&mut self) {
        while self.lanes.len() > BINARY_CHARACTER_LANE_CAP {
            self.lanes.remove(0);
        }
    }
}

pub fn to_code_ingress_frame<'a>(
    lane_id: u64,
    input: &BinaryCharacterBurstInput<'a>,
) -> CodeIngressFrameInput<'a> {
    CodeIngressFrameInput {
        stream_context_id: BinaryCharacterEarState::lane_stream_context_id(lane_id),
        t_start_us: input.t_start_us,
        t_end_us: input.t_end_us,
        bytes: input.bytes,
        framing_validity: input.framing_validity,
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |acc, b| {
        acc.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(*b)
    })
}

fn nonzero_ref(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback.max(1) } else { value }
}
```

Modify `/Users/joe/unnamed/bionic_v0/src/lib.rs`:

```rust
pub mod binary_character_ear;
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_ear
```

Expected: PASS.

## Task 2: Mount Ear In Viewport And Forward To Code Ingress

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/src/viewport.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_ear.rs`

- [ ] **Step 1: Add failing viewport integration test**

Append to `/Users/joe/unnamed/bionic_v0/tests/binary_character_ear.rs`:

```rust
use bionic_v0::viewport::Viewport;

#[test]
fn viewport_observes_binary_character_burst_and_forwards_code_ingress() {
    let vp = Viewport::new();

    vp.observe_binary_character_burst(BinaryCharacterBurstInput {
        fiber_id: 11,
        carrier_id: 4,
        t_start_us: 10_000,
        t_end_us: 10_100,
        bytes: b"hi",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });

    let ear = vp.read_binary_character_ear();
    let code = vp.read_code_ingress();
    assert_eq!(ear.lane_count, 1);
    assert_eq!(code.recent_frames.len(), 1);
    assert_eq!(
        code.recent_frames[0].stream_context_id,
        BinaryCharacterEarState::lane_stream_context_id(ear.active_lane_id)
    );
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_ear viewport_observes_binary_character_burst_and_forwards_code_ingress
```

Expected: FAIL because `Viewport` has no ear field or methods.

- [ ] **Step 3: Mount ear in Viewport**

Modify `/Users/joe/unnamed/bionic_v0/src/viewport.rs`:

```rust
use crate::binary_character_ear::{
    to_code_ingress_frame, BinaryCharacterBurstInput, BinaryCharacterEarState,
};
```

Add field to `Viewport`:

```rust
pub binary_character_ear: RwLock<BinaryCharacterEarState>,
```

Initialize in `Viewport::new()`:

```rust
binary_character_ear: RwLock::new(BinaryCharacterEarState::default()),
```

Add methods in `impl Viewport`:

```rust
pub fn read_binary_character_ear(&self) -> BinaryCharacterEarState {
    self.binary_character_ear.read().unwrap().clone()
}

pub fn observe_binary_character_burst(&self, input: BinaryCharacterBurstInput<'_>) {
    let lane_id = {
        let mut ear = self.binary_character_ear.write().unwrap();
        ear.observe_burst(BinaryCharacterBurstInput {
            fiber_id: input.fiber_id,
            carrier_id: input.carrier_id,
            t_start_us: input.t_start_us,
            t_end_us: input.t_end_us,
            bytes: input.bytes,
            burst_energy: input.burst_energy,
            framing_validity: input.framing_validity,
        });
        ear.active_lane_id
    };
    self.observe_code_ingress_frame(to_code_ingress_frame(lane_id, &input));
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_ear
```

Expected: PASS.

## Task 3: Mouth State And Bounded Byte Output

**Files:**
- Create: `/Users/joe/unnamed/bionic_v0/src/binary_character_mouth.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/lib.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_mouth.rs`

- [ ] **Step 1: Write failing mouth tests**

Create `/Users/joe/unnamed/bionic_v0/tests/binary_character_mouth.rs`:

```rust
use bionic_v0::binary_character_mouth::{
    BinaryCharacterMouthState, BinaryCharacterOutputDrive,
};

#[test]
fn mouth_organizes_bounded_output_burst_from_lane_pressure() {
    let mut mouth = BinaryCharacterMouthState::default();

    let burst = mouth.begin_output(BinaryCharacterOutputDrive {
        lane_id: 3,
        t_us: 20_000,
        pressure: 0.7,
        seed_hash: 0x1234,
        requested_len: 64,
    });

    assert_eq!(mouth.active_lane_id, 3);
    assert_eq!(mouth.output_count, 1);
    assert_eq!(burst.bytes.len(), bionic_v0::LIVE_BYTE_EGRESS_FRAME_LEN);
    assert!(mouth.output_pressure > 0.0);
    assert!(mouth.rhythm_score >= 0.0);
}

#[test]
fn mouth_return_updates_match_and_deviation_without_answer_state() {
    let mut mouth = BinaryCharacterMouthState::default();
    let burst = mouth.begin_output(BinaryCharacterOutputDrive {
        lane_id: 4,
        t_us: 30_000,
        pressure: 0.6,
        seed_hash: 0x77,
        requested_len: 8,
    });

    mouth.observe_return(4, 31_000, &burst.bytes, 0.5, 0.0);

    assert_eq!(mouth.return_count, 1);
    assert!(mouth.return_match > 0.5);
    assert_eq!(mouth.correction_deviation, 0.0);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_mouth
```

Expected: FAIL because `binary_character_mouth` does not exist.

- [ ] **Step 3: Implement mouth state**

Create `/Users/joe/unnamed/bionic_v0/src/binary_character_mouth.rs`:

```rust
use crate::live_byte_closure::LIVE_BYTE_EGRESS_FRAME_LEN;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterMouthState {
    pub active_lane_id: u64,
    pub output_pressure: f64,
    pub burst_length: u32,
    pub rhythm_score: f64,
    pub predicted_byte_hash: u64,
    pub last_output_ref: u64,
    pub output_count: u64,
    pub return_count: u64,
    pub return_match: f64,
    pub correction_deviation: f64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryCharacterOutputDrive {
    pub lane_id: u64,
    pub t_us: u64,
    pub pressure: f64,
    pub seed_hash: u64,
    pub requested_len: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterOutputBurst {
    pub output_ref: u64,
    pub lane_id: u64,
    pub t_us: u64,
    pub bytes: Vec<u8>,
    pub pressure: f64,
}

impl BinaryCharacterMouthState {
    pub fn begin_output(&mut self, drive: BinaryCharacterOutputDrive) -> BinaryCharacterOutputBurst {
        self.output_count = self.output_count.saturating_add(1);
        let len = drive
            .requested_len
            .clamp(1, LIVE_BYTE_EGRESS_FRAME_LEN);
        let pressure = drive.pressure.clamp(0.0, 1.0);
        let mut seed = drive.seed_hash
            ^ drive.lane_id.rotate_left(7)
            ^ self.output_count.rotate_left(19)
            ^ ((pressure * 65_535.0) as u64);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            seed = seed.rotate_left(9).wrapping_mul(0x9E37_79B1_85EB_CA87);
            bytes.push((seed & 0xff) as u8);
        }
        let output_ref = nonzero_ref(hash_bytes(&bytes) ^ self.output_count, self.output_count);
        self.active_lane_id = drive.lane_id;
        self.output_pressure = pressure;
        self.burst_length = bytes.len() as u32;
        self.rhythm_score = (self.rhythm_score + pressure * 0.25).clamp(0.0, 1.0);
        self.predicted_byte_hash = hash_bytes(&bytes);
        self.last_output_ref = output_ref;
        self.last_update_t_us = drive.t_us;
        BinaryCharacterOutputBurst {
            output_ref,
            lane_id: drive.lane_id,
            t_us: drive.t_us,
            bytes,
            pressure,
        }
    }

    pub fn observe_return(
        &mut self,
        lane_id: u64,
        t_us: u64,
        returned_bytes: &[u8],
        magnitude: f64,
        correction_deviation: f64,
    ) {
        self.active_lane_id = lane_id;
        self.return_count = self.return_count.saturating_add(1);
        self.return_match = (1.0 - correction_deviation.clamp(0.0, 1.0))
            .max(magnitude.clamp(0.0, 1.0) * 0.5)
            .clamp(0.0, 1.0);
        self.correction_deviation = correction_deviation.clamp(0.0, 1.0);
        self.predicted_byte_hash = hash_bytes(returned_bytes);
        self.last_update_t_us = t_us;
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |acc, b| {
        acc.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(*b)
    })
}

fn nonzero_ref(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback.max(1) } else { value }
}
```

Modify `/Users/joe/unnamed/bionic_v0/src/lib.rs`:

```rust
pub mod binary_character_mouth;
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_mouth
```

Expected: PASS.

## Task 4: Mount Mouth In Viewport And Forward Through Framed Byte Egress

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/src/viewport.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_mouth.rs`

- [ ] **Step 1: Add failing viewport mouth integration tests**

Append to `/Users/joe/unnamed/bionic_v0/tests/binary_character_mouth.rs`:

```rust
use bionic_v0::binary_character_ear::BinaryCharacterBurstInput;
use bionic_v0::viewport::Viewport;

#[test]
fn viewport_binary_character_mouth_writes_framed_byte_egress() {
    let vp = Viewport::new();
    vp.observe_binary_character_burst(BinaryCharacterBurstInput {
        fiber_id: 1,
        carrier_id: 1,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"aa",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });

    let wrote = vp.tick_binary_character_mouth_from_ear(2_000);

    assert!(wrote);
    let mouth = vp.read_binary_character_mouth();
    let egress = vp.read_framed_byte_egress();
    assert!(mouth.output_count >= 1);
    assert!(egress.total_frames >= 1);
    assert_eq!(egress.active_stream_id, mouth.active_lane_id);
}

#[test]
fn viewport_binary_character_return_updates_mouth_and_pairs_consequence() {
    let vp = Viewport::new();
    vp.observe_binary_character_burst(BinaryCharacterBurstInput {
        fiber_id: 2,
        carrier_id: 1,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"bb",
        burst_energy: 0.7,
        framing_validity: 1.0,
    });
    assert!(vp.tick_binary_character_mouth_from_ear(2_000));
    let lane_id = vp.read_binary_character_mouth().active_lane_id;

    vp.observe_binary_character_return(lane_id, 2_100, b"bb", 0.6, 0.1);

    let mouth = vp.read_binary_character_mouth();
    let egress = vp.read_framed_byte_egress();
    assert_eq!(mouth.return_count, 1);
    assert!(mouth.return_match > 0.0);
    assert!(egress.total_returns >= 1);
    assert!(vp.read_action_consequence().total_framed_byte_pairs >= 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_mouth viewport_binary_character_mouth_writes_framed_byte_egress
```

Expected: FAIL because `Viewport` mouth APIs do not exist.

- [ ] **Step 3: Mount mouth and add APIs**

Modify `/Users/joe/unnamed/bionic_v0/src/viewport.rs`:

```rust
use crate::binary_character_mouth::{BinaryCharacterMouthState, BinaryCharacterOutputDrive};
```

Add field to `Viewport`:

```rust
pub binary_character_mouth: RwLock<BinaryCharacterMouthState>,
```

Initialize in `Viewport::new()`:

```rust
binary_character_mouth: RwLock::new(BinaryCharacterMouthState::default()),
```

Add methods:

```rust
pub fn read_binary_character_mouth(&self) -> BinaryCharacterMouthState {
    self.binary_character_mouth.read().unwrap().clone()
}

pub fn tick_binary_character_mouth_from_ear(&self, now_t_us: u64) -> bool {
    let ear = self.read_binary_character_ear();
    let Some(lane) = ear.lanes.iter().find(|lane| lane.lane_id == ear.active_lane_id) else {
        return false;
    };
    let pressure = (lane.prediction_pressure
        .max(self.read_evidence_tension().unresolved_pressure)
        .max(self.read_active_reasoning_gap().gap_pressure))
        .clamp(0.0, 1.0);
    if pressure <= 0.0001 {
        return false;
    }
    let burst = self.binary_character_mouth.write().unwrap().begin_output(
        BinaryCharacterOutputDrive {
            lane_id: lane.lane_id,
            t_us: now_t_us,
            pressure,
            seed_hash: lane.byte_hash ^ lane.last_burst_ref,
            requested_len: crate::live_byte_closure::LIVE_BYTE_EGRESS_FRAME_LEN,
        },
    );
    self.begin_framed_byte_egress(now_t_us, burst.lane_id, &burst.bytes, burst.pressure);
    true
}

pub fn observe_binary_character_return(
    &self,
    lane_id: u64,
    t_us: u64,
    returned_bytes: &[u8],
    magnitude: f64,
    correction_deviation: f64,
) {
    self.binary_character_mouth.write().unwrap().observe_return(
        lane_id,
        t_us,
        returned_bytes,
        magnitude,
        correction_deviation,
    );
    self.observe_framed_byte_return(
        t_us,
        lane_id,
        returned_bytes,
        magnitude,
        correction_deviation,
    );
}
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_mouth
```

Expected: PASS.

## Task 5: Snapshot Roundtrip For Organ State

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_snapshot.rs`

- [ ] **Step 1: Write failing snapshot test**

Create `/Users/joe/unnamed/bionic_v0/tests/binary_character_snapshot.rs`:

```rust
use std::sync::Arc;

use bionic_v0::binary_character_ear::BinaryCharacterBurstInput;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn snapshot_roundtrip_preserves_binary_character_organ_state() {
    let vp = Arc::new(Viewport::new());
    vp.observe_binary_character_burst(BinaryCharacterBurstInput {
        fiber_id: 9,
        carrier_id: 1,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"zz",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });
    vp.tick_binary_character_mouth_from_ear(2_000);
    let lane_id = vp.read_binary_character_mouth().active_lane_id;
    vp.observe_binary_character_return(lane_id, 2_100, b"zz", 0.6, 0.1);

    let dir = std::env::temp_dir().join("bionic_v0_binary_character_snap");
    std::fs::create_dir_all(&dir).unwrap();
    write_snapshot(&vp, &SnapshotPaths::new(&dir)).unwrap();
    let snap = read_snapshot(&dir.join("snapshot.bin")).unwrap();

    assert_eq!(snap.binary_character_ear.lane_count, 1);
    assert!(snap.binary_character_ear.total_bursts >= 1);
    assert!(snap.binary_character_mouth.output_count >= 1);
    assert!(snap.binary_character_mouth.return_count >= 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_snapshot
```

Expected: FAIL because snapshot has no binary character fields.

- [ ] **Step 3: Serialize and deserialize state**

Modify `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`:

1. Import:

```rust
use crate::binary_character_ear::{BinaryCharacterEarState, BinaryCharacterLaneTrace};
use crate::binary_character_mouth::BinaryCharacterMouthState;
```

2. Increment `SNAPSHOT_VERSION` in `/Users/joe/unnamed/bionic_v0/src/lib.rs`.

3. Add fields to `SnapshotData`:

```rust
pub binary_character_ear: BinaryCharacterEarState,
pub binary_character_mouth: BinaryCharacterMouthState,
```

4. In `write_snapshot`, after framed-byte egress or near other I/O substrates:

```rust
write_binary_character_ear(&mut buf, &vp.read_binary_character_ear());
write_binary_character_mouth(&mut buf, &vp.read_binary_character_mouth());
```

5. In `read_snapshot`, read these fields when version is high enough, otherwise default.

Implement helpers:

```rust
fn write_binary_character_ear(buf: &mut Vec<u8>, s: &BinaryCharacterEarState) {
    buf.extend_from_slice(&s.lane_count.to_le_bytes());
    buf.extend_from_slice(&s.total_bursts.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
    buf.extend_from_slice(&s.collision_pressure.to_le_bytes());
    buf.extend_from_slice(&s.active_lane_id.to_le_bytes());
    buf.extend_from_slice(&(s.lanes.len() as u32).to_le_bytes());
    for lane in &s.lanes {
        buf.extend_from_slice(&lane.lane_id.to_le_bytes());
        buf.extend_from_slice(&lane.fiber_id.to_le_bytes());
        buf.extend_from_slice(&lane.carrier_id.to_le_bytes());
        buf.extend_from_slice(&lane.last_burst_ref.to_le_bytes());
        buf.extend_from_slice(&lane.last_t_us.to_le_bytes());
        buf.extend_from_slice(&lane.byte_hash.to_le_bytes());
        buf.extend_from_slice(&lane.byte_len.to_le_bytes());
        buf.extend_from_slice(&lane.burst_count.to_le_bytes());
        buf.extend_from_slice(&lane.continuity_score.to_le_bytes());
        buf.extend_from_slice(&lane.recurrence_score.to_le_bytes());
        buf.extend_from_slice(&lane.collision_pressure.to_le_bytes());
        buf.extend_from_slice(&lane.prediction_pressure.to_le_bytes());
        buf.extend_from_slice(&lane.return_correlation.to_le_bytes());
    }
}

fn write_binary_character_mouth(buf: &mut Vec<u8>, s: &BinaryCharacterMouthState) {
    buf.extend_from_slice(&s.active_lane_id.to_le_bytes());
    buf.extend_from_slice(&s.output_pressure.to_le_bytes());
    buf.extend_from_slice(&s.burst_length.to_le_bytes());
    buf.extend_from_slice(&s.rhythm_score.to_le_bytes());
    buf.extend_from_slice(&s.predicted_byte_hash.to_le_bytes());
    buf.extend_from_slice(&s.last_output_ref.to_le_bytes());
    buf.extend_from_slice(&s.output_count.to_le_bytes());
    buf.extend_from_slice(&s.return_count.to_le_bytes());
    buf.extend_from_slice(&s.return_match.to_le_bytes());
    buf.extend_from_slice(&s.correction_deviation.to_le_bytes());
    buf.extend_from_slice(&s.last_update_t_us.to_le_bytes());
}
```

Mirror those helpers with `read_binary_character_ear_snapshot` and
`read_binary_character_mouth_snapshot` using existing `rd_u64`, `rd_u32`, and
`rd_f64` helpers in `snapshot.rs`.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_snapshot
```

Expected: PASS.

## Task 6: Body Runtime Tick Order And Observer Report

**Files:**
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`
- Modify: `/Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs`
- Test: `/Users/joe/unnamed/bionic_v0/tests/binary_character_runtime.rs`

- [ ] **Step 1: Write failing runtime/report tests**

Create `/Users/joe/unnamed/bionic_v0/tests/binary_character_runtime.rs`:

```rust
#[test]
fn body_runtime_ticks_binary_character_mouth_after_code_pressure_before_snapshot() {
    let source = include_str!("../src/bin/body.rs");
    let code_pos = source
        .find("tick_evidence_tension_from_code_ingress")
        .expect("code pressure tick");
    let mouth_pos = source
        .find("tick_binary_character_mouth_from_ear")
        .expect("binary character mouth tick");
    let snapshot_pos = source.find("write_snapshot").expect("snapshot write");

    assert!(code_pos < mouth_pos);
    assert!(mouth_pos < snapshot_pos);
}

#[test]
fn binary_character_report_emits_numeric_organ_evidence() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--binary-character-organ")
        .output()
        .expect("run report");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mode=binary_character_organ"));
    assert!(stdout.contains("ear_lanes="));
    assert!(stdout.contains("ear_bursts="));
    assert!(stdout.contains("mouth_outputs="));
    assert!(stdout.contains("mouth_returns="));
    assert!(stdout.contains("framed_byte_pairs="));
    assert!(stdout.contains("principle_guard_pass=1"));
}

#[test]
fn binary_character_sources_do_not_add_rejected_ontology() {
    let sources = [
        include_str!("../src/binary_character_ear.rs"),
        include_str!("../src/binary_character_mouth.rs"),
        include_str!("../src/viewport.rs"),
        include_str!("../src/bin/body.rs"),
    ];
    let forbidden = [
        "semantic_readiness",
        "SymbolizerState",
        "concept_id",
        "token_id",
        "word_id",
        "object_name",
        "name_gate",
        "meaning_db",
        "dictionary",
        "parser",
        "tokenizer",
        "request_response",
        "answer_model",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "eprintln!",
    ];
    for source in sources {
        for term in forbidden {
            assert!(!source.contains(term), "rejected term: {term}");
        }
    }
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_runtime
```

Expected: FAIL because body order and report mode are absent.

- [ ] **Step 3: Add body tick**

Modify `run_global_refresh_cycle` in `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`.
Insert after `vp.tick_prediction_pressure_field_from_substrate(t);` and before
probe/action output:

```rust
vp.tick_binary_character_mouth_from_ear(t);
```

This is body organ drive. It must not print, write files, or inspect external
human-readable text.

- [ ] **Step 4: Add observer-side report mode**

Modify `/Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs`:

1. Parse `--binary-character-organ`.
2. In that branch, create a `Viewport`, observe several bursts, tick mouth,
   observe one return, and print numeric lines only:

```rust
println!("mode=binary_character_organ");
println!("ear_lanes={}", ear.lane_count);
println!("ear_bursts={}", ear.total_bursts);
println!("ear_collision_pressure={:.6}", ear.collision_pressure);
println!("mouth_outputs={}", mouth.output_count);
println!("mouth_returns={}", mouth.return_count);
println!("mouth_deviation={:.6}", mouth.correction_deviation);
println!("framed_byte_pairs={}", vp.read_action_consequence().total_framed_byte_pairs);
println!("principle_guard_pass=1");
```

Report printing is legal because `curriculum_report` is observer-side.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml --test binary_character_runtime
```

Expected: PASS.

## Task 7: Virtual Life Room Character Provider

**Files:**
- Create: `/Users/joe/virtual_life_room_v1/src/character_organ.rs`
- Modify: `/Users/joe/virtual_life_room_v1/src/lib.rs`
- Modify: `/Users/joe/virtual_life_room_v1/src/bin/life_room.rs`
- Test: `/Users/joe/virtual_life_room_v1/tests/character_organ.rs`

- [ ] **Step 1: Write failing provider tests**

Create `/Users/joe/virtual_life_room_v1/tests/character_organ.rs`:

```rust
use tempfile::tempdir;
use virtual_life_room_v1::character_organ::{
    run_character_io_demo, CharacterFiberBurst, CharacterOrganRoom,
};

#[test]
fn virtual_character_ear_feeds_concurrent_fibers_without_merging() {
    let dir = tempdir().unwrap();
    let mut room = CharacterOrganRoom::new(dir.path()).unwrap();

    room.feed_burst(CharacterFiberBurst {
        fiber_id: 1,
        carrier_id: 1,
        bytes: b"hi".to_vec(),
        t_us: 1_000,
    });
    room.feed_burst(CharacterFiberBurst {
        fiber_id: 2,
        carrier_id: 1,
        bytes: b"ok".to_vec(),
        t_us: 1_020,
    });
    room.tick(2_000).unwrap();

    let snap = bionic_v0::snapshot::read_snapshot(&room.snapshot_path()).unwrap();
    assert_eq!(snap.binary_character_ear.lane_count, 2);
    assert!(snap.code_ingress.recent_frames.len() >= 2);
}

#[test]
fn character_io_demo_reports_organ_level_communication_candidate() {
    let dir = tempdir().unwrap();
    let evidence = run_character_io_demo(dir.path()).unwrap();

    assert_eq!(evidence.protocol_ready, 1);
    assert_eq!(evidence.fibers_seen, 3);
    assert!(evidence.ear_lanes >= 3);
    assert!(evidence.mouth_outputs >= 3);
    assert!(evidence.return_windows >= 3);
    assert!(evidence.framed_byte_pairs >= 3);
    assert_eq!(evidence.communication_candidate, 1);
    assert_eq!(evidence.principle_guard_pass, 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cd /Users/joe/virtual_life_room_v1
cargo test --test character_organ
```

Expected: FAIL because `character_organ` does not exist.

- [ ] **Step 3: Implement provider**

Create `/Users/joe/virtual_life_room_v1/src/character_organ.rs`.
Use `LifeRoomBodyHost::viewport()` to call the new body organ APIs:

```rust
let burst = BinaryCharacterBurstInput {
    fiber_id: fixture.fiber_id,
    carrier_id: fixture.carrier_id,
    t_start_us: fixture.t_us,
    t_end_us: fixture.t_us + fixture.duration_us,
    bytes: &fixture.bytes,
    burst_energy: (fixture.bytes.len() as f64 / 16.0).clamp(0.0, 1.0),
    framing_validity: 1.0,
};

vp.observe_binary_character_burst(burst);

let wrote = vp.tick_binary_character_mouth_from_ear(fixture.t_us + fixture.duration_us + 1_000);
if wrote {
    let lane_id = vp.read_binary_character_mouth().active_lane_id;
    vp.observe_binary_character_return(
        lane_id,
        fixture.t_us + fixture.duration_us + 2_000,
        &fixture.bytes,
        (fixture.bytes.len() as f64 / 16.0).clamp(0.0, 1.0),
        0.0,
    );
}
```

Provider state may track external fixture names, but only numeric
fiber/carrier facts enter the body.

- [ ] **Step 4: Add CLI**

Modify `/Users/joe/virtual_life_room_v1/src/bin/life_room.rs`:

- parse `--character-io`;
- run `run_character_io_demo`;
- print numeric fields:

```text
mode=character_io_v0
protocol_ready=1
fibers_seen=3
ear_lanes=3
mouth_outputs=3
return_windows=3
framed_byte_pairs=3
communication_candidate=1
principle_guard_pass=1
```

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cd /Users/joe/virtual_life_room_v1
cargo test --test character_organ
cargo run --bin life_room -- --character-io --runtime-dir /Users/joe/virtual_life_room_v1/runtime_character_io_smoke
```

Expected: tests PASS and CLI emits `communication_candidate=1`.

## Task 8: Full Verification

**Files:**
- All changed files.

- [ ] **Step 1: Run focused body tests**

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml \
  --test binary_character_ear \
  --test binary_character_mouth \
  --test binary_character_snapshot \
  --test binary_character_runtime
```

Expected: PASS.

- [ ] **Step 2: Run body guard tests**

```bash
cd /Users/joe/unnamed
cargo test --manifest-path bionic_v0/Cargo.toml \
  --test code_ingress \
  --test framed_byte_egress \
  --test live_byte_feedback_closure \
  --test body_no_human_output
```

Expected: PASS.

- [ ] **Step 3: Run virtual life room tests**

```bash
cd /Users/joe/virtual_life_room_v1
cargo fmt --check
cargo test
```

Expected: PASS.

- [ ] **Step 4: Run source boundary scan**

```bash
cd /Users/joe/unnamed
rg -n "semantic_readiness|SemanticReadinessState|refresh_semantic_readiness|symbolizer|SymbolizerState|concept_id|token_id|word_id|object_name|name_gate|meaning_db|dictionary|parser|tokenizer|request_response|answer_model|source_identity|owner|ethics|personality|eprintln!" bionic_v0/src/binary_character_ear.rs bionic_v0/src/binary_character_mouth.rs bionic_v0/src/bin/body.rs
```

Expected: no matches.

- [ ] **Step 5: Run final reports**

```bash
cd /Users/joe/unnamed
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --binary-character-organ

cd /Users/joe/virtual_life_room_v1
cargo run --bin life_room -- --character-io --runtime-dir /Users/joe/virtual_life_room_v1/runtime_character_io_smoke
```

Expected: both reports emit numeric evidence and `principle_guard_pass=1`.

## Explicit Non-Implementation Boundaries

Do not implement:

- chat UI;
- natural language answers;
- tokenizer/parser/dictionary;
- network sockets or real terminal control;
- user identity or topic models;
- command execution;
- any body-side human-readable output.

Those can only be discussed in future external provider specs after this organ
proves primitive construction, prediction, output organization, and return
correction.
