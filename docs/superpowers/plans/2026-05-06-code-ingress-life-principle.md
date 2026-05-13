# Code Ingress Life-Principle Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the minimal `CodeIngressAdapter` path: dedicated framed byte carrier -> viewport code-ingress trace -> snapshot/observer visibility.

**Architecture:** Add an independent code-ingress substrate, not a VIB/PHOT/MECH modality and not a language-binding shortcut. The adapter drains a dedicated framed byte carrier, stamps body time, writes bounded numeric traces into `Viewport`, and snapshot/observer expose those traces without body text output or command semantics.

**Tech Stack:** Rust `std`, `Arc`, `RwLock`, bounded `Vec` histories, existing `Viewport`/snapshot binary format, Cargo integration tests.

---

## Chunk 1: Minimal Ingress Substrate

### File Structure

- Create: `bionic_v0/src/code_ingress.rs`
  Responsibility: numeric/time-only state structs, bounded history management, byte feature extraction, projection/window update helpers.
- Create: `bionic_v0/src/code_ingress_adapter.rs`
  Responsibility: dedicated framed carrier trait plus `CodeIngressAdapter::tick`, with no command execution and no language binding writes.
- Modify: `bionic_v0/src/lib.rs`
  Responsibility: export new modules, add fixed caps, bump `SNAPSHOT_VERSION` from `36` to `37`.
- Modify: `bionic_v0/src/viewport.rs`
  Responsibility: own `CodeIngressState`, provide focused read/write APIs.
- Modify: `bionic_v0/src/snapshot.rs`
  Responsibility: persist and read code-ingress traces with version gating.
- Modify: `bionic_v0/src/bin/observer.rs`
  Responsibility: render compact observer-side trace summaries from snapshot truth only.
- Create: `bionic_v0/tests/code_ingress.rs`
  Responsibility: contract tests for ingress state, adapter, snapshot, observer rendering, and forbidden architecture terms.

### Task 1: Add Pure Code-Ingress State

**Files:**
- Create: `bionic_v0/src/code_ingress.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/code_ingress.rs`

- [ ] **Step 1: Write the failing state test**

Add `bionic_v0/tests/code_ingress.rs`:

```rust
use bionic_v0::code_ingress::{
    CodeIngressFrameInput, CodeIngressState, CODE_DECODER_BINARY_FRAGMENT,
    CODE_DECODER_BYTE_UNIT,
};

#[test]
fn code_ingress_state_records_bounded_byte_disturbance() {
    let mut st = CodeIngressState::default();

    st.observe_frame(CodeIngressFrameInput {
        stream_context_id: 7,
        t_start_us: 1_000,
        t_end_us: 1_050,
        bytes: &[0x41, 0x42, 0x0a, 0xff],
        framing_validity: 1.0,
    });

    assert_eq!(st.stream_count, 1);
    assert_eq!(st.recent_frames.len(), 1);
    let frame = &st.recent_frames[0];
    assert_eq!(frame.frame_seq, 1);
    assert_eq!(frame.stream_context_id, 7);
    assert_eq!(frame.byte_len, 4);
    assert_eq!(frame.byte_sample_len, 4);
    assert!(frame.byte_hash != 0);
    assert!(frame.byte_transition_energy > 0.0);
    assert!(frame.evidence_ref != 0);

    assert!(
        st.projection_candidates
            .iter()
            .any(|p| p.decoder_kind_code == CODE_DECODER_BYTE_UNIT)
    );
    assert!(
        st.projection_candidates
            .iter()
            .any(|p| p.decoder_kind_code == CODE_DECODER_BINARY_FRAGMENT)
    );
    assert_eq!(st.sequence_windows.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress code_ingress_state_records_bounded_byte_disturbance -- --nocapture
```

Expected: FAIL because `code_ingress` module and types do not exist.

- [ ] **Step 3: Add module constants and export**

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod code_ingress;
pub mod code_ingress_adapter;

pub const CODE_INGRESS_FRAME_HISTORY_CAP: usize = 64;
pub const CODE_INGRESS_PROJECTION_HISTORY_CAP: usize = 128;
pub const CODE_INGRESS_SEQUENCE_WINDOW_CAP: usize = 32;
pub const CODE_INGRESS_BYTE_SAMPLE_CAP: usize = 32;

pub const SNAPSHOT_VERSION: u64 = 37;
```

Keep existing constants in place; only bump the existing `SNAPSHOT_VERSION` assignment instead of adding a duplicate.

- [ ] **Step 4: Implement numeric state**

Create `bionic_v0/src/code_ingress.rs`:

```rust
use crate::{
    CODE_INGRESS_BYTE_SAMPLE_CAP, CODE_INGRESS_FRAME_HISTORY_CAP,
    CODE_INGRESS_PROJECTION_HISTORY_CAP, CODE_INGRESS_SEQUENCE_WINDOW_CAP,
};

pub const CODE_DECODER_BYTE_UNIT: u8 = 0;
pub const CODE_DECODER_BINARY_FRAGMENT: u8 = 1;
pub const CODE_DECODER_ASCII_SHAPE: u8 = 2;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeIngressState {
    pub stream_count: u32,
    pub recent_frames: Vec<CodeIngressFrameTrace>,
    pub projection_candidates: Vec<CodeProjectionCandidate>,
    pub sequence_windows: Vec<CodeSequenceWindowTrace>,
    pub last_update_t_us: u64,
    next_frame_seq: u64,
    next_projection_ref: u64,
    next_window_id: u64,
}

pub struct CodeIngressFrameInput<'a> {
    pub stream_context_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub bytes: &'a [u8],
    pub framing_validity: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeIngressFrameTrace {
    pub frame_seq: u64,
    pub stream_context_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub byte_len: u32,
    pub byte_hash: u64,
    pub byte_transition_energy: f64,
    pub byte_distribution_skew: f64,
    pub framing_validity: f64,
    pub evidence_ref: u64,
    pub byte_sample: [u8; CODE_INGRESS_BYTE_SAMPLE_CAP],
    pub byte_sample_len: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeProjectionCandidate {
    pub projection_ref: u64,
    pub frame_seq: u64,
    pub decoder_kind_code: u8,
    pub confidence: f64,
    pub validity: f64,
    pub error_magnitude: f64,
    pub code_unit_count: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeSequenceWindowTrace {
    pub window_id: u64,
    pub stream_context_id: u64,
    pub start_frame_seq: u64,
    pub end_frame_seq: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub recurrence_score: f64,
    pub boundary_confidence: f64,
    pub ambiguity_level: f64,
}
```

Implement `observe_frame`, bounded push helpers, deterministic byte hash, transition energy, distribution skew, projection creation, and simple window extension. All fields must remain numeric/byte arrays; do not introduce `String`.

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress code_ingress_state_records_bounded_byte_disturbance -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd /Users/joe/unnamed
git add bionic_v0/src/lib.rs bionic_v0/src/code_ingress.rs bionic_v0/tests/code_ingress.rs
git commit -m "feat: add code ingress trace state"
```

### Task 2: Add Dedicated Framed Carrier Adapter

**Files:**
- Create: `bionic_v0/src/code_ingress_adapter.rs`
- Modify: `bionic_v0/tests/code_ingress.rs`

- [ ] **Step 1: Write the failing adapter test**

Append to `bionic_v0/tests/code_ingress.rs`:

```rust
use std::sync::Arc;

use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
use bionic_v0::viewport::Viewport;

#[test]
fn code_ingress_adapter_drains_dedicated_framed_carrier_into_viewport() {
    let vp = Arc::new(Viewport::new());
    let carrier = ScriptedCodeIngressCarrier::new(9, vec![vec![0x7b, 0x7d]]);
    let mut adapter = CodeIngressAdapter::new(carrier);

    adapter.tick(&vp, 2_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 1);
    assert_eq!(st.recent_frames[0].stream_context_id, 9);
    assert_eq!(st.recent_frames[0].byte_len, 2);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress code_ingress_adapter_drains_dedicated_framed_carrier_into_viewport -- --nocapture
```

Expected: FAIL because adapter and viewport API do not exist.

- [ ] **Step 3: Implement the carrier and adapter**

Create `bionic_v0/src/code_ingress_adapter.rs`:

```rust
use std::collections::VecDeque;
use std::sync::Arc;

use crate::code_ingress::CodeIngressFrameInput;
use crate::viewport::Viewport;

pub struct CarrierFrame {
    pub stream_context_id: u64,
    pub bytes: Vec<u8>,
    pub framing_validity: f64,
}

pub trait FramedByteCarrier {
    fn try_recv_frame(&mut self) -> Option<CarrierFrame>;
}

pub struct CodeIngressAdapter<C> {
    carrier: C,
}

impl<C: FramedByteCarrier> CodeIngressAdapter<C> {
    pub fn new(carrier: C) -> Self {
        Self { carrier }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        while let Some(frame) = self.carrier.try_recv_frame() {
            vp.observe_code_ingress_frame(CodeIngressFrameInput {
                stream_context_id: frame.stream_context_id,
                t_start_us: now_us,
                t_end_us: now_us,
                bytes: &frame.bytes,
                framing_validity: frame.framing_validity,
            });
        }
    }
}

pub struct ScriptedCodeIngressCarrier {
    stream_context_id: u64,
    frames: VecDeque<Vec<u8>>,
}
```

Implement `ScriptedCodeIngressCarrier::new` and `FramedByteCarrier` for it. This carrier is synthetic/dedicated; do not add shell, terminal, or SSH carriers.

- [ ] **Step 4: Add viewport API**

Modify `bionic_v0/src/viewport.rs`:

- import `CodeIngressFrameInput` and `CodeIngressState`;
- add `pub code_ingress: RwLock<CodeIngressState>` to `Viewport`;
- initialize it in `Viewport::new`;
- add:

```rust
pub fn read_code_ingress(&self) -> CodeIngressState {
    self.code_ingress.read().unwrap().clone()
}

pub fn observe_code_ingress_frame(&self, input: CodeIngressFrameInput<'_>) {
    self.code_ingress.write().unwrap().observe_frame(input);
}
```

Do not route this through `ModalityAdapter`, object slots, language binding, action output, or action consequence.

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress code_ingress_adapter_drains_dedicated_framed_carrier_into_viewport -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd /Users/joe/unnamed
git add bionic_v0/src/code_ingress_adapter.rs bionic_v0/src/viewport.rs bionic_v0/tests/code_ingress.rs
git commit -m "feat: add dedicated code ingress adapter"
```

### Task 3: Expose Code Ingress Through Snapshot

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/tests/code_ingress.rs`

- [ ] **Step 1: Write the failing snapshot test**

Append:

```rust
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};

#[test]
fn snapshot_roundtrip_preserves_code_ingress_traces() {
    let dir = std::env::temp_dir().join("bionic_v0_code_ingress_snap");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());

    let carrier = ScriptedCodeIngressCarrier::new(3, vec![vec![0x61, 0x62, 0x63]]);
    let mut adapter = CodeIngressAdapter::new(carrier);
    adapter.tick(&vp, 3_000);

    write_snapshot(&vp, &paths).expect("write snapshot");
    let snap = read_snapshot(&paths.final_path).expect("read snapshot");

    assert_eq!(snap.code_ingress.recent_frames.len(), 1);
    assert_eq!(snap.code_ingress.recent_frames[0].stream_context_id, 3);
    assert_eq!(snap.code_ingress.recent_frames[0].byte_len, 3);
    assert!(snap.code_ingress.projection_candidates.len() >= 2);
    assert_eq!(snap.language_binding.n_bindings, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress snapshot_roundtrip_preserves_code_ingress_traces -- --nocapture
```

Expected: FAIL because `SnapshotData` does not expose `code_ingress`.

- [ ] **Step 3: Extend snapshot writer**

Modify `bionic_v0/src/snapshot.rs`:

- import `CodeIngressState`, `CodeIngressFrameTrace`, `CodeProjectionCandidate`, and `CodeSequenceWindowTrace`;
- add `pub code_ingress: CodeIngressState` to `SnapshotData`;
- after existing v36 vocal writes, add v37 write section:

```rust
write_code_ingress(&mut buf, &vp.read_code_ingress());
```

Add writer helpers that store:

- frame count and all frame fields;
- bounded byte sample length and bytes;
- projection count and all projection fields;
- sequence-window count and all window fields;
- stream count and last update time.

- [ ] **Step 4: Extend snapshot reader with version gate**

In `read_snapshot`, after v36 reads:

```rust
let code_ingress = if version >= 37 {
    read_code_ingress_snapshot(&bytes, &mut cur)
} else {
    CodeIngressState::default()
};
```

Return it in `SnapshotData`.

- [ ] **Step 5: Run snapshot test and full snapshot regression**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress snapshot_roundtrip_preserves_code_ingress_traces -- --nocapture
cargo test phot_snapshot_roundtrip -- --nocapture
```

Expected: both PASS.

- [ ] **Step 6: Commit**

```bash
cd /Users/joe/unnamed
git add bionic_v0/src/snapshot.rs bionic_v0/tests/code_ingress.rs
git commit -m "feat: expose code ingress through snapshot"
```

### Task 4: Render Observer Trace And Add Hygiene Guards

**Files:**
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/code_ingress.rs`

- [ ] **Step 1: Write observer/hygiene tests**

Append:

```rust
#[test]
fn observer_source_renders_code_ingress_without_command_semantics() {
    let observer = include_str!("../src/bin/observer.rs");
    assert!(observer.contains("code:n"));
    for forbidden in [
        "command meaning",
        "shell state",
        "terminal prompt",
        "source identity",
        "owner",
    ] {
        assert!(
            !observer.contains(forbidden),
            "observer code should not render `{}` as body truth",
            forbidden
        );
    }
}

#[test]
fn code_ingress_sources_contain_no_rejected_architecture() {
    let files = [
        include_str!("../src/code_ingress.rs"),
        include_str!("../src/code_ingress_adapter.rs"),
    ];
    for src in files {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "println!",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "code ingress source must not contain `{}`",
                forbidden
            );
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress observer_source_renders_code_ingress_without_command_semantics -- --nocapture
cargo test --test code_ingress code_ingress_sources_contain_no_rejected_architecture -- --nocapture
```

Expected: observer test FAILS until rendering is added. Hygiene test should PASS if earlier tasks respected the boundary; if it fails, fix the source before continuing.

- [ ] **Step 3: Add compact observer rendering**

Modify `bionic_v0/src/bin/observer.rs` near existing snapshot columns:

```rust
let code = &snap.code_ingress;
let code_latest = code
    .recent_frames
    .last()
    .map(|f| {
        format!(
            "f{}|s{}|len{}|h{:x}|p{}|w{}",
            f.frame_seq,
            f.stream_context_id,
            f.byte_len,
            f.byte_hash,
            code.projection_candidates.len(),
            code.sequence_windows.len()
        )
    })
    .unwrap_or_else(|| "-".to_string());
row.push_str(&format!(
    "\tcode:n{}|{}",
    code.recent_frames.len(),
    code_latest
));
```

Observer may render the hash and lengths. Do not render command/source/owner labels.

- [ ] **Step 4: Run tests**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /Users/joe/unnamed
git add bionic_v0/src/bin/observer.rs bionic_v0/tests/code_ingress.rs
git commit -m "feat: render code ingress observer trace"
```

### Task 5: Final Verification

**Files:**
- Review only: `bionic_v0/src/bin/body.rs`
- Review only: `bionic_v0/src/adapter.rs`
- Review only: new code-ingress modules and tests

- [ ] **Step 1: Run targeted code-ingress tests**

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test code_ingress -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run boundary regressions**

```bash
cd /Users/joe/unnamed/bionic_v0
cargo test --test language_binding_runtime -- --nocapture
cargo test --test cleanup_hygiene -- --nocapture
cargo test phot_snapshot_roundtrip -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Verify no body stdout/stderr diagnostics were added**

Run:

```bash
cd /Users/joe/unnamed
rg -n "println!|eprintln!" bionic_v0/src/bin/body.rs bionic_v0/src/code_ingress.rs bionic_v0/src/code_ingress_adapter.rs
```

Expected: no matches.

- [ ] **Step 4: Verify code ingress was not wired into rejected paths**

Run:

```bash
cd /Users/joe/unnamed
rg -n "apply_language_binding_event|apply_binding_event_from_adapter|write_slot_|begin_action|push_action|pair_action|semantic_readiness|symbolizer|concept_id|token_id|word_id" bionic_v0/src/code_ingress.rs bionic_v0/src/code_ingress_adapter.rs
```

Expected: no matches.

- [ ] **Step 5: Inspect final diff**

```bash
cd /Users/joe/unnamed
git diff --stat
git diff --check
```

Expected: only planned files changed; no whitespace errors.

- [ ] **Step 6: Commit any verification-only fixes**

If verification required small fixes:

```bash
cd /Users/joe/unnamed
git add <changed-files>
git commit -m "test: guard code ingress life principle boundary"
```

If no fixes were needed, do not create an empty commit.
