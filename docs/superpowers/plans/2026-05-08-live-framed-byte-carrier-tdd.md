# Live Framed Byte Carrier Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a live framed-byte ingress carrier so a running body can receive external byte disturbances without becoming a chat, stdin, command, token, or request/response system.

**Architecture:** A body-side carrier tails a file under the runtime directory, slices newly appended bytes into bounded frames, and feeds them through the existing `CodeIngressAdapter` into `Viewport.code_ingress`. The carrier is clocked by body time and produces no human-readable body output. Observer/session CSV remains the only human-readable evidence surface.

**Tech Stack:** Rust crate `bionic_v0`; existing `CodeIngressAdapter`, `FramedByteCarrier`, `Viewport`, body tickers, snapshot, observer compact/session CSV; tests via `cargo test`.

---

## Frozen Boundary

Allowed:

- Runtime file bytes as environmental disturbance material.
- Fixed-size byte frames with stream id, timing, byte length, and validity.
- Body-side ticking that samples the file path if present.
- Observer-side compact/session evidence.

Forbidden:

- stdin line handling.
- Prompt/request/response lifecycle.
- Chat turns, roles, assistant/user message state.
- Token ids, token embeddings, word-meaning tables.
- Internal semantic/concept allocation.
- Shell/tool/browser/command execution.
- Body-side `println!` / `eprintln!` / human-readable diagnostics.

Core rule:

```text
live byte input = continuous disturbance, not request
live byte output = later endogenous write-out, not answer
file path = carrier material boundary, not conversation session
observer = research tool, not body consciousness
```

## File Structure

- Create: `bionic_v0/src/live_byte_carrier.rs`
  - Defines a file-tail framed byte ingress carrier.
  - Implements existing `FramedByteCarrier`.
  - Exposes a spawn helper for the body runtime.
  - No text parsing, no stdout/stderr, no command execution.

- Modify: `bionic_v0/src/lib.rs`
  - Export `live_byte_carrier`.

- Modify: `bionic_v0/src/bin/body.rs`
  - Start live byte ingress ticker using `runtime_dir/live_bytes/in.frames`.
  - Join its handle on shutdown.
  - Preserve body silence.

- Test: `bionic_v0/tests/live_byte_carrier.rs`
  - Unit/integration tests for file tailing, frame slicing, body source wiring, and boundary hygiene.

- Modify: `docs/superpowers/plans/2026-05-08-live-framed-byte-carrier-tdd.md`
  - Add final operator runbook after implementation.

## Phase 0: Boundary Guard

### Task 0.1: Guard Live Carrier Against Drift

**Files:**

- Create: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing test**

Add a source-level test that requires `src/live_byte_carrier.rs` to exist and rejects drift terms.

```rust
#[test]
fn live_byte_carrier_source_has_no_request_or_execution_surface() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/live_byte_carrier.rs"
    ))
    .expect("live_byte_carrier.rs must exist");

    for forbidden in [
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
        "command_executor",
        "std::process::Command",
        "stdin",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "live byte carrier must not contain forbidden surface: {forbidden}"
        );
    }
}
```

- [ ] **Step 2: Run and confirm RED**

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test live_byte_carrier live_byte_carrier_source_has_no_request_or_execution_surface -- --nocapture
```

Expected: fail because `src/live_byte_carrier.rs` does not exist.

## Phase 1: File Tail Carrier

### Task 1.1: Read Appended Bytes As Frames

**Files:**

- Create: `bionic_v0/src/live_byte_carrier.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing behavior test**

Add:

```rust
#[test]
fn live_file_carrier_reads_new_bytes_as_bounded_frames() {
    use bionic_v0::code_ingress_adapter::FramedByteCarrier;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_carrier_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");

    let mut carrier = LiveByteFileCarrier::new(path.clone(), 41, 4);
    assert!(carrier.try_recv_frame().is_none());

    std::fs::write(&path, b"abcdef").unwrap();

    let first = carrier.try_recv_frame().expect("first frame");
    let second = carrier.try_recv_frame().expect("second frame");
    assert!(carrier.try_recv_frame().is_none());

    assert_eq!(first.stream_context_id, 41);
    assert_eq!(first.bytes, b"abcd");
    assert_eq!(second.bytes, b"ef");
    assert_eq!(first.framing_validity, 1.0);
}
```

- [ ] **Step 2: Run and confirm RED**

```bash
cargo test --test live_byte_carrier live_file_carrier_reads_new_bytes_as_bounded_frames -- --nocapture
```

Expected: fail because module/type is missing.

- [ ] **Step 3: Implement minimal carrier**

Create a carrier that:

- stores `path`, `stream_context_id`, `frame_len`, `offset`, and queued frames;
- opens the file if present;
- seeks from `offset`;
- reads newly appended bytes;
- advances `offset`;
- splits bytes with `chunks(frame_len.max(1))`;
- implements `FramedByteCarrier`.

- [ ] **Step 4: Verify**

```bash
cargo test --test live_byte_carrier live_file_carrier_reads_new_bytes_as_bounded_frames -- --nocapture
```

Expected: pass.

### Task 1.2: Preserve Incremental Tail Semantics

**Files:**

- Modify: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing test**

Add:

```rust
#[test]
fn live_file_carrier_only_reads_newly_appended_bytes() {
    use bionic_v0::code_ingress_adapter::FramedByteCarrier;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_tail_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");
    std::fs::write(&path, b"abc").unwrap();

    let mut carrier = LiveByteFileCarrier::new(path.clone(), 42, 8);
    assert_eq!(carrier.try_recv_frame().unwrap().bytes, b"abc");
    assert!(carrier.try_recv_frame().is_none());

    let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(b"def").unwrap();
    drop(file);

    assert_eq!(carrier.try_recv_frame().unwrap().bytes, b"def");
    assert!(carrier.try_recv_frame().is_none());
}
```

- [ ] **Step 2: Run**

```bash
cargo test --test live_byte_carrier live_file_carrier_only_reads_newly_appended_bytes -- --nocapture
```

Expected: pass if Task 1.1 implementation is correct; otherwise fix only tail offset behavior.

## Phase 2: Adapter And Body Wiring

### Task 2.1: Carrier Feeds Existing Code Ingress Adapter

**Files:**

- Modify: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing integration test**

Add:

```rust
#[test]
fn live_file_carrier_feeds_code_ingress_adapter() {
    use std::sync::Arc;

    use bionic_v0::code_ingress_adapter::CodeIngressAdapter;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_adapter_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");
    std::fs::write(&path, b"abcabc").unwrap();

    let vp = Arc::new(Viewport::new());
    let carrier = LiveByteFileCarrier::new(path, 43, 3);
    let mut adapter = CodeIngressAdapter::new(carrier);
    adapter.tick(&vp, 1_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 2);
    assert_eq!(st.sequence_windows.len(), 1);
    assert_eq!(st.recent_frames[0].stream_context_id, 43);
}
```

- [ ] **Step 2: Run and verify**

```bash
cargo test --test live_byte_carrier live_file_carrier_feeds_code_ingress_adapter -- --nocapture
```

Expected: pass after carrier implementation.

### Task 2.2: Body Starts Live Carrier From Runtime Directory

**Files:**

- Modify: `bionic_v0/src/live_byte_carrier.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing source wiring test**

Add:

```rust
#[test]
fn body_runtime_wires_live_byte_carrier_without_stdio_surface() {
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/body.rs"
    ))
    .expect("read body.rs");

    assert!(body.contains("spawn_live_byte_ingress"));
    assert!(body.contains("live_bytes"));
    assert!(body.contains("in.frames"));

    for forbidden in ["stdin", "std::process::Command", "println!", "eprintln!"] {
        assert!(
            !body.contains(forbidden),
            "body must not expose live byte carrier through {forbidden}"
        );
    }
}
```

- [ ] **Step 2: Run and confirm RED**

```bash
cargo test --test live_byte_carrier body_runtime_wires_live_byte_carrier_without_stdio_surface -- --nocapture
```

Expected: fail because body is not wired.

- [ ] **Step 3: Implement spawn helper**

In `live_byte_carrier.rs`, add:

- `pub const LIVE_BYTE_FRAME_LEN: usize = 8;`
- `pub const LIVE_BYTE_STREAM_ID: u64 = 2;`
- `pub fn spawn_live_byte_ingress(vp: Arc<Viewport>, stop: Arc<AtomicBool>, path: PathBuf) -> JoinHandle<()>`

The helper owns `CodeIngressAdapter<LiveByteFileCarrier>` and ticks it with body clock using existing `spawn_ticker`.

- [ ] **Step 4: Wire body**

In `body.rs`:

- import `spawn_live_byte_ingress`;
- construct `runtime_dir.join("live_bytes").join("in.frames")`;
- start the handle after `vp` and `stop` are created;
- join the handle on shutdown.

- [ ] **Step 5: Verify**

```bash
cargo test --test live_byte_carrier body_runtime_wires_live_byte_carrier_without_stdio_surface -- --nocapture
cargo build --bin body
```

Expected: pass.

## Phase 3: Runtime Evidence

### Task 3.1: Body Observes Appended File Bytes While Running

**Files:**

- Modify: `bionic_v0/tests/live_byte_carrier.rs`

- [ ] **Step 1: Write failing process test**

Add a test that:

- starts `target/debug/body` for a bounded duration;
- creates `runtime_dir/live_bytes/in.frames`;
- appends bytes shortly after start;
- reads final snapshot;
- asserts `code_ingress.recent_frames.len() > 0`;
- asserts captured stdout/stderr are empty.

- [ ] **Step 2: Run**

```bash
cargo test --test live_byte_carrier live_body_ingests_appended_byte_file_without_text_output -- --nocapture
```

Expected: fail until body wiring is present and timing is sufficient.

- [ ] **Step 3: Minimal stabilization**

Use a short bounded duration and a child process:

- build path via `env!("CARGO_BIN_EXE_body")`;
- run `body runtime_dir 2`;
- append bytes after 200 ms;
- assert body exit success;
- assert output buffers empty;
- read snapshot and assert nonzero code frames.

- [ ] **Step 4: Verify**

```bash
cargo test --test live_byte_carrier live_body_ingests_appended_byte_file_without_text_output -- --nocapture
```

Expected: pass.

## Phase 4: Operator Runbook

### Task 4.1: Add Manual Test Command

**Files:**

- Modify: `docs/superpowers/plans/2026-05-08-live-framed-byte-carrier-tdd.md`

- [ ] **Step 1: Add runbook**

Append:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

cargo build --bin body --bin observer

RUN_DIR="$ROOT/runtime/live-byte-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR/live_bytes"

./target/debug/body "$RUN_DIR" 3 >"$RUN_DIR/body.stdout" 2>"$RUN_DIR/body.stderr" &
BODY_PID=$!

sleep 1
printf 'abcabcabc\n(()())\n001001\n' >> "$RUN_DIR/live_bytes/in.frames"

wait "$BODY_PID"

./target/debug/observer "$RUN_DIR" 5 10 compact "$RUN_DIR/session.csv"

head -n 12 "$RUN_DIR/session.csv"
wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr"
```

Expected:

- `code_frames > 0`.
- `body.stdout` and `body.stderr` are 0 bytes.
- no prompt/answer transcript.

## Phase 5: Verification

### Task 5.1: Run Tests And Boundary Scans

- [ ] **Step 1: Run targeted tests**

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test live_byte_carrier -- --nocapture
```

- [ ] **Step 2: Run related tests**

```bash
cargo test \
  --test character_io_boundary \
  --test code_replay \
  --test code_ingress \
  --test body_no_human_output \
  --test cleanup_hygiene \
  -- --nocapture
```

- [ ] **Step 3: Run full suite**

```bash
cargo test
```

- [ ] **Step 4: Run boundary scans**

```bash
rg -n "println!|eprintln!" src --glob '!src/bin/observer.rs' --glob '!src/bin/observer_surface.rs' --glob '!src/bin/code_replay.rs'
rg -n "semantic_readiness|SemanticReadinessState|refresh_semantic_readiness|symbolizer|SymbolizerState|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding" src
rg -n "prompt|chat|assistant_message|user_message|system_message|token_id|token_embedding|word_meaning|command_executor|std::process::Command|stdin" src
```

Expected: no matches in body-owned active source, except test-owned source is not part of these `src` scans.

## Review Checklist

- Live byte input is disturbance only.
- No stdin prompt loop exists.
- No chat/request-response lifecycle exists.
- No token/semantic/concept ontology is introduced.
- Body remains silent.
- Observer/session remains external evidence only.
- Runtime path names are carrier material boundaries, not conversation state.

## Implemented Operator Runbook

Manual live byte disturbance test:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1

cargo build --bin body --bin observer

RUN_DIR="$ROOT/runtime/live-byte-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR/live_bytes"

./target/debug/body "$RUN_DIR" 3 >"$RUN_DIR/body.stdout" 2>"$RUN_DIR/body.stderr" &
BODY_PID=$!

sleep 1
printf 'abcabcabc\n(()())\n001001\n' >> "$RUN_DIR/live_bytes/in.frames"

wait "$BODY_PID"

./target/debug/observer "$RUN_DIR" 5 10 compact "$RUN_DIR/session.csv"

head -n 12 "$RUN_DIR/session.csv"
wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr"
```

Expected evidence:

- `code_frames` is greater than 0.
- `code_windows` is greater than 0.
- `body.stdout` and `body.stderr` are 0 bytes.
- There is no prompt/answer transcript.
