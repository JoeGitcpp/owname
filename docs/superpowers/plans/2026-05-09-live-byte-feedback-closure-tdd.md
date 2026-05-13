# Live Byte Feedback Closure Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bounded character-side feedback closure where live byte ingress can trigger body-side framed-byte write-out, an external loopback can return bytes, and observer traces show `egress_frames > 0`, `egress_returns > 0`, `pairs > 0`, and `learned_axes > 0`.

**Architecture:** Keep the AI body as a continuous substrate, not a request/response program. Add a body-side numeric byte-closure driver that derives bounded output bytes from fresh code-ingress pressure and records later byte ingress as framed-byte return evidence. Add a separate external loopback binary for experiments; the body never prints, reads stdin, runs commands, or interprets byte content.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `CodeIngressState`, `FramedByteEgressState`, `ActionConsequenceState`, `LearnedBiasState`, file-backed live byte carrier, observer compact/session CSV.

---

## Hard Boundaries

- No chat/request/response behavior.
- No text understanding, no token model, no semantic readiness, no concept allocator.
- No human-readable output from `body`.
- `byte_loopback` is an external experiment tool only; it is not part of the AI body.
- Body output bytes are bounded numeric traces derived from substrate pressure and recent byte-shape evidence, not natural-language answers.
- A successful run is measured only by observer-visible numeric closure: write-out, return, temporal pair, learned bias sediment.

## Files

- Create: `bionic_v0/src/live_byte_closure.rs`
  - Body-side byte closure driver.
  - Owns runtime-local cursors for fresh code frame consumption and return pairing.
  - Writes body-side byte frames to `runtime/live_bytes/out.frames`.
  - Calls existing `begin_framed_byte_egress` and `observe_framed_byte_return`.

- Create: `bionic_v0/src/bin/byte_loopback.rs`
  - External experiment utility.
  - Tails `runtime/live_bytes/out.frames`.
  - Appends bounded transformed bytes to `runtime/live_bytes/in.frames`.
  - Produces no stdout/stderr.

- Modify: `bionic_v0/src/bin/body.rs`
  - Start one ticker for `LiveByteClosureDriver`.
  - Join it on bounded runs.

- Modify: `bionic_v0/src/lib.rs`
  - Export `live_byte_closure`.

- Modify: `bionic_v0/Cargo.toml`
  - Add `byte_loopback` binary.

- Test: `bionic_v0/tests/live_byte_feedback_closure.rs`
  - Driver writes output after fresh code pressure.
  - Later byte ingress closes framed-byte return and sediments learned bias.
  - Body runtime wires the driver without stdio/request surfaces.
  - Loopback copies bounded transformed bytes from out file to in file.

## Chunk 1: Body-Side Byte Closure Driver

### Task 1: RED - output frame from fresh code pressure

- [ ] **Step 1: Write failing test**

Add `bionic_v0/tests/live_byte_feedback_closure.rs`:

```rust
#[test]
fn closure_driver_writes_body_frame_from_fresh_code_pressure() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!("bionic_v0_live_byte_closure_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();
    let out_path = dir.join("live_bytes").join("out.frames");

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(out_path.clone());
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc001",
        framing_validity: 1.0,
    });
    vp.tick_evidence_tension_from_code_ingress(1_100);

    assert!(driver.tick(&vp, 1_200));
    assert_eq!(vp.read_framed_byte_egress().n_frames, 1);
    assert!(std::fs::metadata(out_path).unwrap().len() > 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd /Users/joe/unnamed/.worktrees/architecture-contraction-tdd/bionic_v0
cargo test --test live_byte_feedback_closure closure_driver_writes_body_frame_from_fresh_code_pressure -- --nocapture
```

Expected: FAIL because `live_byte_closure` does not exist.

- [ ] **Step 3: Implement minimal driver**

Create `LiveByteClosureDriver` with:

- output file path
- last consumed code frame cursor
- last paired egress frame cursor
- last return code frame cursor
- `tick(&mut self, vp: &Viewport, now_t_us: u64) -> bool`

The first implementation only writes one output frame when fresh code ingress exists and evidence pressure is nonzero.

- [ ] **Step 4: Run test to verify it passes**

Run the same test. Expected: PASS.

## Chunk 2: Return Pairing and Sediment

### Task 2: RED - later ingress closes framed-byte return

- [ ] **Step 1: Write failing test**

Add:

```rust
#[test]
fn closure_driver_pairs_later_code_frame_as_return_and_sediments_bias() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!("bionic_v0_live_byte_return_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(dir.join("live_bytes").join("out.frames"));
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc001",
        framing_validity: 1.0,
    });
    vp.tick_evidence_tension_from_code_ingress(1_100);
    assert!(driver.tick(&vp, 1_200));

    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_500,
        t_end_us: 1_500,
        bytes: b"001abcabc",
        framing_validity: 1.0,
    });
    assert!(driver.tick(&vp, 1_600));
    vp.tick_life_closure_producers(1_700);

    assert_eq!(vp.read_framed_byte_egress().n_returns, 1);
    assert!(vp.read_action_consequence().n_pairs > 0);
    assert!(vp
        .read_learned_bias()
        .recent_events
        .iter()
        .any(|event| event.axis == BiasAxis::CrossChannelGain));
}
```

- [ ] **Step 2: Run test to verify it fails**

Expected: FAIL because return pairing is not implemented.

- [ ] **Step 3: Implement return observation**

When an egress frame is active and a fresh later code frame arrives, copy its bounded byte sample into `observe_framed_byte_return`. Use numeric magnitude/deviation from byte-shape traces only.

- [ ] **Step 4: Run test to verify it passes**

Expected: PASS.

## Chunk 3: External Loopback Tool and Runtime Wiring

### Task 3: RED - loopback moves output bytes to input file

- [ ] **Step 1: Write failing test**

Add a test that calls a helper function from `byte_loopback` support code or validates the source/bin behavior by spawning `byte_loopback` for a bounded duration.

- [ ] **Step 2: Implement `byte_loopback`**

Create a no-output binary:

```bash
byte_loopback <runtime_dir> <duration_secs>
```

It tails `runtime/live_bytes/out.frames`, transforms bytes with a reversible bounded operation, and appends them to `runtime/live_bytes/in.frames`.

- [ ] **Step 3: Wire body runtime**

In `body.rs`, start `LiveByteClosureDriver` at 20 Hz:

```rust
let h_live_byte_closure = spawn_ticker(20, stop.clone(), move |_| {
    driver.lock().unwrap().tick(&vp_live_byte_closure, now_us());
});
```

Join it on shutdown.

- [ ] **Step 4: Runtime test**

Run body + loopback + seed input. Expected observer CSV reaches:

```text
egress_frames > 0
egress_returns > 0
pairs > 0
learned_axes > 0
body.stdout = 0
body.stderr = 0
```

## Chunk 4: Verification

- [ ] Run focused tests:

```bash
cargo test --test live_byte_feedback_closure -- --nocapture
cargo test --test live_byte_carrier --test life_closure_hardening --test character_io_boundary -- --nocapture
```

- [ ] Run boundary scans:

```bash
rg -n "println!|eprintln!" src --glob '!src/bin/observer.rs' --glob '!src/bin/observer_surface.rs' --glob '!src/bin/code_replay.rs'
rg -n "semantic_readiness|SymbolizerState|concept_id|token_id|word_meaning|command_executor" src tests
```

Expected: no disallowed body matches.

- [ ] Run full crate tests:

```bash
cargo test
```

- [ ] Run bounded live experiment:

```bash
ROOT="/Users/joe/unnamed/.worktrees/architecture-contraction-tdd"
cd "$ROOT/bionic_v0" || exit 1
cargo build --bin body --bin observer --bin byte_loopback
RUN_DIR="$ROOT/runtime/byte-closure-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR/live_bytes"
./target/debug/body "$RUN_DIR" 20 >"$RUN_DIR/body.stdout" 2>"$RUN_DIR/body.stderr" &
BODY_PID=$!
./target/debug/byte_loopback "$RUN_DIR" 20 >"$RUN_DIR/loop.stdout" 2>"$RUN_DIR/loop.stderr" &
LOOP_PID=$!
sleep 1
printf 'abcabc001001\n(()())\n' >> "$RUN_DIR/live_bytes/in.frames"
wait "$BODY_PID"
wait "$LOOP_PID"
./target/debug/observer "$RUN_DIR" 5 20 session "$RUN_DIR/session.csv" >"$RUN_DIR/observer.stdout" 2>"$RUN_DIR/observer.stderr"
tail -n 5 "$RUN_DIR/session.csv"
wc -c "$RUN_DIR/body.stdout" "$RUN_DIR/body.stderr" "$RUN_DIR/loop.stdout" "$RUN_DIR/loop.stderr"
```

Expected: the session tail shows nonzero egress frames, returns, pairs, learned axes; body and loopback output files are zero bytes.
