# Character I/O As Disturbance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move early training/debugging from live camera/microphone ritual into reproducible character/byte disturbance loops without creating chat, token ontology, semantic gates, or request/response behavior.

**Architecture:** Character input is an external framed-byte disturbance that enters the existing `code_ingress` trace path. Character output is an endogenous framed-byte egress trace driven by existing pressure/replay/mismatch signals, not by a prompt. Observer-side compact rendering and session CSV provide evidence; body remains silent and numeric/time/byte-shaped.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `code_ingress`, `code_ingress_adapter`, `framed_byte_egress`, `framed_byte_carrier`, `snapshot`, `observer`, integration tests via `cargo test`.

---

## Frozen Boundary

This plan freezes the current discussion into executable constraints.

Allowed:

- External character files as byte disturbance material.
- Byte/frame sequence traces.
- Timing, prediction error, pressure, recurrence, ambiguity, action-consequence pairs.
- Binding events only when backed by evidence pointers.
- Observer-side human-readable rendering.
- Session CSV as an external research artifact.

Forbidden:

- `prompt -> response`.
- Chat turns, roles, assistant/user message state.
- Token ids, token embeddings, word-meaning tables.
- `semantic_readiness`, `symbolizer`, `concept_id`, or replacements under another name.
- Internal "knowledge import" from current large models.
- Command execution, shell control, browser/tool agent behavior.
- Body-side `println!` / `eprintln!` / human-readable diagnostics.

Core rule:

```text
character input = perturbation, not command
character output = endogenous stream, not answer
training material = environment, not knowledge injection
observer = research tool, not body consciousness
```

## File Structure

- Modify: `bionic_v0/src/bin/observer.rs`
  - Add compact observer mode.
  - Add optional session CSV sink.
  - Keep full observer mode available for existing tests.

- Create: `bionic_v0/src/bin/observer_compact.rs`
  - Pure observer-side helpers for compact header/row formatting.
  - Reads only `SnapshotData`.
  - No body access, no recomputation of body truth.

- Create: `bionic_v0/src/bin/observer_session.rs`
  - Pure observer-side CSV header/row formatting and append helper.
  - Writes external research artifact only.

- Create: `bionic_v0/src/bin/code_replay.rs`
  - Operator-side replay executable.
  - Reads a local byte file and feeds fixed-size frames into existing `CodeIngressAdapter` / `ScriptedCodeIngressCarrier` against a temporary `Viewport` for deterministic tests first.
  - Later runtime integration must remain a carrier, not stdin prompt handling.

- Modify: `bionic_v0/src/framed_byte_carrier.rs`
  - Add minimal deterministic helper for scripted egress loop only if current sandbox carrier cannot express a staged return.
  - Do not add external carrier policy, shell, network, or command execution.

- Test: `bionic_v0/tests/character_io_boundary.rs`
  - Source-level guard against chat/token/semantic drift.

- Test: `bionic_v0/tests/observer_compact.rs`
  - Compact observer formatting tests.

- Test: `bionic_v0/tests/session_trace.rs`
  - Session CSV formatting tests.

- Test: `bionic_v0/tests/code_replay.rs`
  - Character replay ingress tests.

- Test: `bionic_v0/tests/endogenous_byte_stream.rs`
  - Endogenous egress behavior tests.

---

## Phase 0: Boundary Guard First

### Task 0.1: Add Character I/O Boundary Test

**Files:**

- Create: `bionic_v0/tests/character_io_boundary.rs`

- [ ] **Step 1: Write the failing boundary test**

Add:

```rust
fn read(path: &str) -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/", path))
        .unwrap_or_else(|_| String::new())
}

#[test]
fn character_io_sources_do_not_introduce_chat_or_token_ontology() {
    let paths = [
        "src/bin/observer.rs",
        "src/code_ingress.rs",
        "src/code_ingress_adapter.rs",
        "src/framed_byte_egress.rs",
        "src/framed_byte_carrier.rs",
        "src/viewport.rs",
    ];
    let forbidden = [
        "prompt",
        "chat",
        "assistant_message",
        "user_message",
        "system_message",
        "token_id",
        "token_embedding",
        "word_meaning",
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "intent",
        "personality",
        "owner",
        "command_executor",
        "shell",
    ];

    for path in paths {
        let src = read(path);
        for term in forbidden {
            assert!(
                !src.contains(term),
                "{path} must not contain forbidden character-I/O drift term: {term}"
            );
        }
    }
}

#[test]
fn body_owned_sources_still_do_not_print_human_readable_output() {
    let paths = [
        "src/bin/body.rs",
        "src/vocal_adapter.rs",
        "src/code_ingress.rs",
        "src/code_ingress_adapter.rs",
        "src/framed_byte_egress.rs",
        "src/framed_byte_carrier.rs",
        "src/viewport.rs",
    ];
    for path in paths {
        let src = read(path);
        assert!(!src.contains("println!"), "{path} must not call println!");
        assert!(!src.contains("eprintln!"), "{path} must not call eprintln!");
    }
}
```

- [ ] **Step 2: Run the test**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test character_io_boundary -- --nocapture
```

Expected:

- It may fail initially if the new test includes paths/files not yet created or if existing observer contains allowed external terms.
- If it fails because of observer-only words, narrow the scanned body-owned path list rather than weakening body rules.

- [ ] **Step 3: Make the minimal adjustment**

If needed, adjust only the test path list so it checks current body-owned source and planned character I/O substrate files. Do not remove the forbidden list.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test --test character_io_boundary -- --nocapture
```

Expected: both tests pass.

---

## Phase 1: Compact Observer

### Task 1.1: Add Compact Row Formatter

**Files:**

- Create: `bionic_v0/src/bin/observer_compact.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/observer_compact.rs`

- [ ] **Step 1: Write the failing test**

Create `bionic_v0/tests/observer_compact.rs`:

```rust
#[test]
fn observer_compact_source_exposes_only_short_numeric_trace_columns() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer_compact.rs"
    ))
    .expect("observer_compact.rs must exist");

    for required in [
        "compact_header",
        "compact_row",
        "code_frames",
        "egress_frames",
        "pairs",
        "probe_i",
        "evidence_p",
        "voc_tE",
    ] {
        assert!(src.contains(required), "missing compact trace field: {required}");
    }

    for forbidden in ["prompt", "chat", "answer", "meaning", "token_id"] {
        assert!(
            !src.contains(forbidden),
            "compact observer must not introduce {forbidden}"
        );
    }
}
```

- [ ] **Step 2: Run and confirm RED**

Run:

```bash
cargo test --test observer_compact -- --nocapture
```

Expected: fail because `observer_compact.rs` does not exist.

- [ ] **Step 3: Implement compact formatter**

Create `bionic_v0/src/bin/observer_compact.rs`:

```rust
use bionic_v0::snapshot::SnapshotData;

pub fn compact_header() -> &'static str {
    "seq,clock_s,health,phot_h,src_w,code_frames,code_windows,egress_phase,egress_frames,egress_returns,pairs,probe_i,probe_o,evidence_p,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or"
}

pub fn compact_row(snap: &SnapshotData) -> String {
    let clock_s = (snap.clock_us.saturating_sub(snap.start_us)) as f64 * 1e-6;
    format!(
        "{},{:.3},{},{},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
        snap.seq,
        clock_s,
        snap.health_state,
        snap.phot_health,
        snap.source_writes,
        snap.code_ingress.recent_frames.len(),
        snap.code_ingress.sequence_windows.len(),
        snap.framed_byte_egress.phase,
        snap.framed_byte_egress.n_frames,
        snap.framed_byte_egress.n_returns,
        snap.action_consequence.n_pairs,
        snap.probe_law.intensity,
        snap.probe_law.output_intensity,
        snap.evidence_tension.unresolved_pressure,
        snap.vocal_target.active_energy,
        snap.vocal_target.pitch_hint,
        snap.vocal_target.resonance_hint,
        snap.vocal_organ.breath_drive,
        snap.vocal_organ.pitch_tension,
        snap.vocal_organ.resonance_position,
    )
}
```

- [ ] **Step 4: Wire compact mode into observer**

Modify `bionic_v0/src/bin/observer.rs`:

```rust
mod observer_compact;
```

Then in `main`, parse a fourth arg:

```rust
let mode = args.get(4).map(String::as_str).unwrap_or("full");
let compact = mode == "compact";
```

After the start line, print:

```rust
if compact {
    println!("{}", observer_compact::compact_header());
} else {
    println!("existing full header...");
}
```

Inside the snapshot read branch:

```rust
if compact {
    println!("{}", observer_compact::compact_row(&snap));
} else {
    print_row(&snap);
}
```

Keep the existing full observer path intact.

- [ ] **Step 5: Verify compact tests**

Run:

```bash
cargo test --test observer_compact -- --nocapture
```

Expected: pass.

- [ ] **Step 6: Verify observer still builds**

Run:

```bash
cargo build --bin observer
```

Expected: build succeeds.

---

## Phase 2: Session CSV

### Task 2.1: Add Session Trace Formatter

**Files:**

- Create: `bionic_v0/src/bin/observer_session.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Test: `bionic_v0/tests/session_trace.rs`

- [ ] **Step 1: Write failing test**

Create `bionic_v0/tests/session_trace.rs`:

```rust
#[test]
fn session_trace_source_defines_csv_without_chat_fields() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer_session.rs"
    ))
    .expect("observer_session.rs must exist");

    assert!(src.contains("session_header"));
    assert!(src.contains("session_row"));
    assert!(src.contains("append_session_row"));
    assert!(src.contains("code_frames"));
    assert!(src.contains("egress_frames"));
    assert!(src.contains("pairs"));

    for forbidden in ["prompt", "answer", "role", "message", "meaning", "intent"] {
        assert!(
            !src.contains(forbidden),
            "session trace must not contain chat field: {forbidden}"
        );
    }
}
```

- [ ] **Step 2: Run and confirm RED**

Run:

```bash
cargo test --test session_trace -- --nocapture
```

Expected: fail because `observer_session.rs` does not exist.

- [ ] **Step 3: Implement session helper**

Create `bionic_v0/src/bin/observer_session.rs`:

```rust
use std::fs::OpenOptions;
use std::io::{Result, Write};
use std::path::Path;

use bionic_v0::snapshot::SnapshotData;

pub fn session_header() -> &'static str {
    "seq,clock_s,source_writes,code_frames,code_windows,egress_phase,egress_frames,egress_returns,pairs,probe_i,probe_o,evidence_p,learned_axes,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or"
}

pub fn session_row(snap: &SnapshotData) -> String {
    let clock_s = (snap.clock_us.saturating_sub(snap.start_us)) as f64 * 1e-6;
    format!(
        "{},{:.3},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
        snap.seq,
        clock_s,
        snap.source_writes,
        snap.code_ingress.recent_frames.len(),
        snap.code_ingress.sequence_windows.len(),
        snap.framed_byte_egress.phase,
        snap.framed_byte_egress.n_frames,
        snap.framed_byte_egress.n_returns,
        snap.action_consequence.n_pairs,
        snap.probe_law.intensity,
        snap.probe_law.output_intensity,
        snap.evidence_tension.unresolved_pressure,
        snap.learned_bias.n_axes,
        snap.vocal_target.active_energy,
        snap.vocal_target.pitch_hint,
        snap.vocal_target.resonance_hint,
        snap.vocal_organ.breath_drive,
        snap.vocal_organ.pitch_tension,
        snap.vocal_organ.resonance_position,
    )
}

pub fn append_session_row(path: &Path, snap: &SnapshotData, write_header: bool) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if write_header {
        writeln!(file, "{}", session_header())?;
    }
    writeln!(file, "{}", session_row(snap))?;
    Ok(())
}
```

- [ ] **Step 4: Wire session path into observer**

Modify `bionic_v0/src/bin/observer.rs`:

```rust
mod observer_session;
```

Parse fifth arg:

```rust
let session_csv = args.get(5).map(PathBuf::from);
let mut session_header_written = false;
```

When a new snapshot row is observed:

```rust
if let Some(path) = session_csv.as_ref() {
    let _ = observer_session::append_session_row(path, &snap, !session_header_written);
    session_header_written = true;
}
```

This is observer-side only. Do not write CSV from body.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test --test session_trace -- --nocapture
cargo build --bin observer
```

Expected: both pass.

---

## Phase 3: Character Replay Ingress

### Task 3.1: Replay File Into Existing Code Ingress Trace

**Files:**

- Create: `bionic_v0/src/bin/code_replay.rs`
- Test: `bionic_v0/tests/code_replay.rs`

- [ ] **Step 1: Write failing test**

Create `bionic_v0/tests/code_replay.rs`:

```rust
#[test]
fn code_replay_source_uses_framed_carrier_not_prompt_parser() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/code_replay.rs"
    ))
    .expect("code_replay.rs must exist");

    assert!(src.contains("ScriptedCodeIngressCarrier"));
    assert!(src.contains("CodeIngressAdapter"));
    assert!(src.contains("frame_bytes"));

    for forbidden in ["prompt", "chat", "answer", "token", "meaning", "parse_sentence"] {
        assert!(
            !src.contains(forbidden),
            "code replay must not introduce {forbidden}"
        );
    }
}
```

- [ ] **Step 2: Run and confirm RED**

Run:

```bash
cargo test --test code_replay -- --nocapture
```

Expected: fail because `code_replay.rs` does not exist.

- [ ] **Step 3: Implement replay binary**

Create `bionic_v0/src/bin/code_replay.rs`:

```rust
use std::path::PathBuf;
use std::sync::Arc;

use bionic_v0::clock::now_us;
use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

pub fn frame_bytes(bytes: &[u8], frame_len: usize) -> Vec<Vec<u8>> {
    let len = frame_len.max(1);
    bytes.chunks(len).map(|chunk| chunk.to_vec()).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        std::process::exit(2);
    }
    let input_path = PathBuf::from(&args[1]);
    let runtime_dir = PathBuf::from(&args[2]);
    let frame_len = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);

    let bytes = std::fs::read(input_path).unwrap_or_default();
    let frames = frame_bytes(&bytes, frame_len);
    let carrier = ScriptedCodeIngressCarrier::new(1, frames);
    let mut adapter = CodeIngressAdapter::new(carrier);
    let vp = Arc::new(Viewport::new());

    adapter.tick(&vp, now_us());
    let paths = SnapshotPaths::new(&runtime_dir);
    let _ = std::fs::create_dir_all(&runtime_dir);
    let _ = write_snapshot(&vp, &paths);
}
```

Add to `bionic_v0/Cargo.toml` if Cargo does not auto-discover this bin:

```toml
[[bin]]
name = "code_replay"
path = "src/bin/code_replay.rs"
```

- [ ] **Step 4: Add behavior test for frame slicing**

Extend `bionic_v0/tests/code_replay.rs` with a source-level check for `chunks(len)` and existing integration coverage:

```rust
#[test]
fn scripted_code_ingress_carrier_already_records_byte_frames() {
    use std::sync::Arc;

    use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
    use bionic_v0::viewport::Viewport;

    let vp = Arc::new(Viewport::new());
    let carrier = ScriptedCodeIngressCarrier::new(7, vec![b"abc".to_vec(), b"def".to_vec()]);
    let mut adapter = CodeIngressAdapter::new(carrier);

    adapter.tick(&vp, 1_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 2);
    assert_eq!(st.recent_frames[0].stream_context_id, 7);
    assert_eq!(st.recent_frames[0].byte_len, 3);
    assert_eq!(st.sequence_windows.len(), 1);
}
```

- [ ] **Step 5: Verify**

Run:

```bash
cargo test --test code_replay -- --nocapture
cargo build --bin code_replay
```

Expected: pass.

---

## Phase 4: Endogenous Framed Byte Egress

### Task 4.1: Egress Can Emit From Existing Pressure Without Input Command

**Files:**

- Test: `bionic_v0/tests/endogenous_byte_stream.rs`
- Modify: `bionic_v0/src/framed_byte_egress.rs` only if existing API cannot express the test.

- [ ] **Step 1: Write failing test**

Create `bionic_v0/tests/endogenous_byte_stream.rs`:

```rust
use bionic_v0::framed_byte_egress::FramedByteEgressPhase;
use bionic_v0::viewport::Viewport;

#[test]
fn framed_byte_egress_is_write_out_not_answer() {
    let vp = Viewport::new();

    let frame = vp.begin_framed_byte_egress(1_000, 11, &[0x61, 0x62, 0x61], 0.4);

    assert_eq!(frame.stream_id, 11);
    assert_eq!(frame.byte_len, 3);
    assert!(frame.write_seq > 0);

    let st = vp.read_framed_byte_egress();
    assert_eq!(st.phase, FramedByteEgressPhase::Active);
    assert_eq!(st.n_frames, 1);
    assert_eq!(st.output_pressure, 0.4);
}

#[test]
fn framed_byte_return_pairs_with_write_epoch_without_result_label() {
    let vp = Viewport::new();
    let frame = vp.begin_framed_byte_egress(1_000, 11, &[0x61, 0x62], 0.5);

    let window_id = vp.observe_framed_byte_return(1_100, 11, &[0x62, 0x61], 0.5, 0.2);

    let acq = vp.read_action_consequence();
    assert!(window_id > 0);
    assert!(acq.n_pairs >= 1);
    assert_eq!(vp.read_framed_byte_egress().current_write_seq, frame.write_seq);
}
```

- [ ] **Step 2: Run**

Run:

```bash
cargo test --test endogenous_byte_stream -- --nocapture
```

Expected: should pass with existing API. If it fails, fix only `framed_byte_egress` numeric pairing behavior.

- [ ] **Step 3: Add source boundary assertion**

Extend `bionic_v0/tests/endogenous_byte_stream.rs`:

```rust
#[test]
fn framed_byte_egress_source_has_no_chat_or_command_terms() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/framed_byte_egress.rs"
    ))
    .expect("read framed_byte_egress.rs");

    for forbidden in ["prompt", "answer", "chat", "command", "shell", "meaning", "token"] {
        assert!(
            !src.contains(forbidden),
            "framed byte egress must not contain {forbidden}"
        );
    }
}
```

- [ ] **Step 4: Verify**

Run:

```bash
cargo test --test endogenous_byte_stream -- --nocapture
```

Expected: pass.

---

## Phase 5: Compact Runtime Loop

### Task 5.1: Manual Reproducible Session Command

**Files:**

- Modify: `docs/superpowers/plans/2026-05-07-character-io-disturbance-tdd.md`

- [ ] **Step 1: Add runbook block after implementation**

Append this operator command block after code is implemented:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

cargo build --bin body --bin observer --bin code_replay

RUN_DIR="$ROOT/runtime/char-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"

printf 'abcabcabc\n(()())\n001001\n' > "$RUN_DIR/input.txt"

./target/debug/body "$RUN_DIR" 0 >"$RUN_DIR/body.stdout" 2>"$RUN_DIR/body.stderr" &
BODY_PID=$!

./target/debug/code_replay "$RUN_DIR/input.txt" "$RUN_DIR" 8
./target/debug/observer "$RUN_DIR" 5 20 compact "$RUN_DIR/session.csv"

kill "$BODY_PID" 2>/dev/null
wait "$BODY_PID" 2>/dev/null

wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr"
ls -lh "$RUN_DIR/snapshot.bin" "$RUN_DIR/session.csv"
```

- [ ] **Step 2: Validate expected outputs**

Expected:

- `body.stdout` and `body.stderr` are 0 bytes.
- `session.csv` exists and has compact rows.
- Compact rows show nonzero `code_frames`.
- There is no prompt/answer transcript.

---

## Phase 6: Full Verification

### Task 6.1: Run Targeted Tests

- [ ] **Step 1: Run targeted suite**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test \
  --test character_io_boundary \
  --test observer_compact \
  --test session_trace \
  --test code_replay \
  --test endogenous_byte_stream \
  -- --nocapture
```

Expected: all pass.

- [ ] **Step 2: Run existing relevant suites**

Run:

```bash
cargo test \
  --test code_ingress \
  --test framed_byte_egress \
  --test life_closure_hardening \
  --test body_no_human_output \
  --test cleanup_hygiene \
  -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Run full suite**

Run:

```bash
cargo test
```

Expected: all pass.

- [ ] **Step 4: Run boundary scans**

Run:

```bash
rg -n "println!|eprintln!" src --glob '!src/bin/observer.rs' --glob '!src/bin/observer_surface.rs' --glob '!src/bin/code_replay.rs'
rg -n "semantic_readiness|SemanticReadinessState|refresh_semantic_readiness|symbolizer|SymbolizerState|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding" src
rg -n "prompt|chat|assistant_message|user_message|system_message|token_id|token_embedding|word_meaning|command_executor|shell" src
```

Expected:

- First command: no body-owned output diagnostics.
- Second command: no rejected architecture terms.
- Third command: no chat/token/command execution drift in active source. If `code_replay.rs` contains external operator usage text, remove or rename it to byte/frame terms.

### Task 6.2: Commit Only Plan-Related Implementation

- [ ] **Step 1: Review status**

Run:

```bash
git status --short
git diff --stat
```

Expected:

- Only files from this plan are modified, plus local `runtime/` evidence directories if manually run.
- Do not stage `runtime/`.

- [ ] **Step 2: Commit implementation**

Run:

```bash
git add \
  bionic_v0/src/bin/observer.rs \
  bionic_v0/src/bin/observer_compact.rs \
  bionic_v0/src/bin/observer_session.rs \
  bionic_v0/src/bin/code_replay.rs \
  bionic_v0/src/framed_byte_carrier.rs \
  bionic_v0/tests/character_io_boundary.rs \
  bionic_v0/tests/observer_compact.rs \
  bionic_v0/tests/session_trace.rs \
  bionic_v0/tests/code_replay.rs \
  bionic_v0/tests/endogenous_byte_stream.rs \
  docs/superpowers/plans/2026-05-07-character-io-disturbance-tdd.md
git commit -m "feat: add character byte disturbance loop"
```

If `framed_byte_carrier.rs` is unchanged, omit it from `git add`.

---

## Review Checklist

Before executing implementation, confirm:

- The plan does not add semantics as an internal ontology.
- Character input is framed disturbance only.
- Character output is framed egress only.
- Observer/session CSV are external research tools.
- No body-owned human-readable output is added.
- No chat/request-response lifecycle appears.
- No current large-model knowledge architecture is imported.

## Implemented Operator Runbook

Implementation note:

- Observer helper files are implemented under `bionic_v0/src/bin/observer_helpers/`
  and imported by `observer.rs` with explicit `#[path = ...]` module paths.
  This keeps them observer-side without making Cargo auto-build helper files as
  standalone binaries.

Manual reproducible session:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

cargo build --bin body --bin observer --bin code_replay

RUN_DIR="$ROOT/runtime/char-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"

printf 'abcabcabc\n(()())\n001001\n' > "$RUN_DIR/input.txt"

./target/debug/code_replay "$RUN_DIR/input.txt" "$RUN_DIR" 8
./target/debug/observer "$RUN_DIR" 5 20 compact "$RUN_DIR/session.csv"

./target/debug/body "$RUN_DIR/body-check" 1 \
  >"$RUN_DIR/body.stdout" \
  2>"$RUN_DIR/body.stderr"

wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr"
ls -lh "$RUN_DIR/snapshot.bin" "$RUN_DIR/session.csv"
```

If a longer body silence check is needed:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

RUN_DIR="$ROOT/runtime/body-silence-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR"

./target/debug/body "$RUN_DIR" 0 >"$RUN_DIR/body.stdout" 2>"$RUN_DIR/body.stderr" &
BODY_PID=$!
sleep 5
kill "$BODY_PID" 2>/dev/null
wait "$BODY_PID" 2>/dev/null

wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr"
```

Expected evidence:

- `body.stdout` and `body.stderr` are 0 bytes.
- `session.csv` exists and has compact rows.
- Compact rows show nonzero `code_frames`.
- There is no prompt/answer transcript.
- `code_replay` writes a replay snapshot for deterministic character
  disturbance inspection. It does not mutate a live body process; runtime body
  integration must remain a carrier path, not stdin or request handling.
