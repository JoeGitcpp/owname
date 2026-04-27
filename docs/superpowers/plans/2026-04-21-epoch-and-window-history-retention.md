# Epoch And Window History Retention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded, engineering-capped retention policy to write_epoch_history and consequence window/pair Vecs so the continuously-running substrate does not grow without bound, while introducing a stable `window_id` to replace the fragile Vec-position `window_index` in pair records.

**Architecture:** Four-chunk migration: (1) introduce stable `window_id` on windows; (2) migrate pair's `window_index` reference to `window_id`; (3) add cap constants and epoch eviction; (4) add window + pair eviction. Snapshot version bumps once (v29) in Chunk 2. All changes are substrate-time-only and introduce no result/target/meaning semantics.

**Tech Stack:** Rust (`std`, `AtomicU64`, `RwLock`), existing Viewport/snapshot binary format, Cargo integration tests.

**Spec:** `docs/superpowers/specs/2026-04-21-epoch-and-window-history-retention-design.md`

---

## File Structure

**Modify**
- `bionic_v0/src/lib.rs` — add cap constants; bump SNAPSHOT_VERSION to 29
- `bionic_v0/src/viewport.rs` — add `action_consequence_window_seq: AtomicU64`
- `bionic_v0/src/action_consequence.rs` — add `window_id` to window struct; change pair's `window_index` to `window_id`; implement eviction in append paths
- `bionic_v0/src/action_output.rs` — implement epoch eviction in Begin branch
- `bionic_v0/src/snapshot.rs` — add `window_id` to SnapshotConsequenceWindow; change pair's `window_index` to `window_id`; bump format; read/write logic
- `bionic_v0/src/bin/observer.rs` — replace `wi` (window_index) with `wid` (window_id) in `acq:` render

**Modify (tests)**
- `bionic_v0/tests/action_consequence_loop.rs` — update assertions on `pair.window_index` → `pair.window_id`; add cap / eviction tests
- `bionic_v0/tests/real_phot_adapter.rs` — no change expected (doesn't read pair.window_index directly)

**Do Not Modify**
- `bionic_v0/src/adapter.rs` — calls `try_auto_pair_latest_epoch_to_window` by value; API surface doesn't change
- `bionic_v0/src/bin/body.rs` — no action_consequence direct access
- Any language-binding, imitation, or fast-path logic

---

## Chunk 1 — Stable window_id: introduce the field

Add `window_id: u64` to `ActionConsequenceWindow` and assign it from a new viewport-level atomic counter. No pair migration yet. No snapshot change yet. Existing tests still pass.

### Task 1: Write failing tests for stable window_id

**Files:**
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Append the following failing tests at the end of the test file**

```rust
// ─── Stable window_id ────────────────────────────────────────────────────────

// ─── W1. begin_action_consequence_window assigns a non-zero stable window_id ──

#[test]
fn window_id_is_assigned_on_begin() {
    let vp = Viewport::new();
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    let st = vp.read_action_consequence();
    assert_eq!(idx, 0, "first window is at Vec index 0");
    assert!(
        st.windows[0].window_id > 0,
        "window_id must be assigned (non-zero) by begin"
    );
}

// ─── W2. push_action_consequence_window also assigns a stable window_id ───────

#[test]
fn window_id_is_assigned_on_push() {
    let vp = Viewport::new();
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    let st = vp.read_action_consequence();
    assert!(
        st.windows[0].window_id > 0,
        "push must also assign a non-zero window_id"
    );
}

// ─── W3. Multiple windows get strictly increasing window_ids ──────────────────

#[test]
fn window_ids_are_strictly_increasing() {
    let vp = Viewport::new();
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 300,
        end_t_us: 400,
        magnitude: 0.4,
    });
    let st = vp.read_action_consequence();
    let id0 = st.windows[0].window_id;
    let id1 = st.windows[1].window_id;
    assert!(id1 > id0, "window_ids must be strictly increasing: id0={}, id1={}", id0, id1);
}
```

- [ ] **Step 2: Run tests and confirm RED**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop window_id 2>&1 | tail -20
```
Expected: compile error (field `window_id` does not exist on ActionConsequenceWindow).

### Task 2: Implement stable window_id

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: Add `window_id: u64` to `ActionConsequenceWindow` in `action_consequence.rs`**

In the `ActionConsequenceWindow` struct (currently at the top of the file), add the field `window_id: u64`. Place it as the first field so it is visually prominent:

```rust
#[derive(Clone, Debug, Default)]
pub struct ActionConsequenceWindow {
    /// Stable monotonic ID assigned at creation. Never zero; never reused.
    /// Parallel to WriteEpochRecord::write_seq. Used by ActionConsequencePair
    /// so pair references remain valid after Vec compaction.
    pub window_id: u64,
    pub modality_kind: ConsequenceModalityKind,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
    pub phase: ConsequenceWindowPhase,
    pub last_update_t_us: u64,
}
```

- [ ] **Step 2: Add `action_consequence_window_seq: AtomicU64` to `Viewport` in `viewport.rs`**

Find the `action_output_write_seq` field in the Viewport struct and add the parallel field immediately after it:

```rust
/// Monotonic counter for stable window_id assignment. Parallels action_output_write_seq.
pub action_consequence_window_seq: AtomicU64,
```

In `Viewport::new()`, initialize it:

```rust
action_consequence_window_seq: AtomicU64::new(0),
```

- [ ] **Step 3: Assign window_id in `begin_action_consequence_window` in `action_consequence.rs`**

In `begin_action_consequence_window`, before pushing the new window, fetch-add the counter:

```rust
pub fn begin_action_consequence_window(&self, ev: ActionConsequenceWindowEvent) -> u32 {
    let window_id = self.action_consequence_window_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let mut s = self.action_consequence.write().unwrap();
    let idx = s.windows.len() as u32;
    s.windows.push(ActionConsequenceWindow {
        window_id,
        modality_kind: ev.modality_kind,
        start_t_us: ev.start_t_us,
        end_t_us: ev.end_t_us,
        magnitude: ev.magnitude,
        phase: ConsequenceWindowPhase::Open,
        last_update_t_us: ev.start_t_us,
    });
    s.n_windows = s.windows.len() as u32;
    idx
}
```

- [ ] **Step 4: Assign window_id in `push_action_consequence_window` in `action_consequence.rs`**

Same pattern as Step 3:

```rust
pub fn push_action_consequence_window(&self, ev: ActionConsequenceWindowEvent) {
    let window_id = self.action_consequence_window_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let t_us = self.clock_us.load(Ordering::Relaxed);
    let mut s = self.action_consequence.write().unwrap();
    s.windows.push(ActionConsequenceWindow {
        window_id,
        modality_kind: ev.modality_kind,
        start_t_us: ev.start_t_us,
        end_t_us: ev.end_t_us,
        magnitude: ev.magnitude,
        phase: ConsequenceWindowPhase::Open,
        last_update_t_us: t_us,
    });
    s.n_windows = s.windows.len() as u32;
}
```

- [ ] **Step 5: Update `accumulate_ingress_change` to pass through window_id**

The `begin_action_consequence_window` call inside `accumulate_ingress_change` already goes through the method above, so no struct literal needs updating there. However, check the `begin_action_consequence_window` call in `accumulate_ingress_change` — it should call the updated method, which handles window_id internally. No change needed if the implementation delegates correctly.

- [ ] **Step 6: Run window_id tests and confirm GREEN**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop window_id 2>&1 | tail -20
```
Expected: 3 tests pass.

- [ ] **Step 7: Run full test suite to confirm no regressions**

```
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep "^test result"
```
Expected: all `0 failed`.

---

## Chunk 2 — Migrate pair reference: window_index → window_id

This is the breaking change. Pair's `window_index: u32` (Vec position) becomes `window_id: u64` (stable ID). The snapshot version bumps to 29. All test assertions on `pair.window_index` are updated.

### Task 3: Write failing tests for window_id pair reference

**Files:**
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Add the following failing tests at the end of the test file**

```rust
// ─── Pair references window by stable window_id, not Vec position ─────────────

// ─── WP1. pair.window_id matches the window's own window_id ───────────────────

#[test]
fn pair_references_window_by_stable_window_id() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1200,
        magnitude: 0.2,
    });

    // The explicit pair API now takes a window_id, not a Vec index.
    let wid = vp.read_action_consequence().windows[idx as usize].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "pairing by stable window_id must succeed");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "one pair must be recorded");
    assert_eq!(
        st.pairs[0].window_id, wid,
        "pair.window_id must equal the window's stable window_id"
    );
}

// ─── WP2. pair rejects window_id 0 (null sentinel) ────────────────────────────

#[test]
fn pair_rejects_null_window_id() {
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1200,
        magnitude: 0.2,
    });

    // window_id 0 is the null sentinel — no real window has id 0.
    let ok = vp.pair_action_consequence(write_seq, 0, 40);
    assert!(!ok, "window_id 0 must be rejected as null sentinel");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "no pair must be recorded for null window_id");
}

// ─── WP3. pair rejects window_id that has never been assigned ─────────────────

#[test]
fn pair_rejects_unissued_window_id() {
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    // No windows created yet — window_id 9999 is unissued.
    let ok = vp.pair_action_consequence(write_seq, 9999, 40);
    assert!(!ok, "unissued window_id must be rejected");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "no pair must be recorded for unissued window_id");
}
```

- [ ] **Step 2: Run and confirm RED**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop "pair_references_window\|pair_rejects_null\|pair_rejects_unissued" 2>&1 | tail -20
```
Expected: compile errors (pair.window_id does not exist, pair_action_consequence signature mismatch).

### Task 4: Implement pair window_id migration

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/src/lib.rs` (SNAPSHOT_VERSION bump)
- Modify: `bionic_v0/tests/action_consequence_loop.rs` (update old assertions)

- [ ] **Step 1: Change `ActionConsequencePair.window_index: u32` → `window_id: u64`**

In `action_consequence.rs`, the `ActionConsequencePair` struct becomes:

```rust
#[derive(Clone, Debug, Default)]
pub struct ActionConsequencePair {
    pub write_seq: u64,
    /// Stable window identifier. Matches ActionConsequenceWindow::window_id.
    /// This is NOT a Vec position — it remains valid across Vec compaction.
    pub window_id: u64,
    pub delay_us: u64,
    pub status: ConsequencePairStatus,
    pub pair_t_us: u64,
}
```

- [ ] **Step 2: Change `pair_action_consequence` signature to accept `window_id: u64`**

The new signature:
```rust
pub fn pair_action_consequence(&self, write_seq: u64, window_id: u64, delay_us: u64) -> bool
```

Internal validation logic:
- Reject `write_seq == 0` (null sentinel, same as before)
- Reject `window_id == 0` (null sentinel, new)
- Validate `write_seq` in `write_epoch_history` (same as before)
- Validate `window_id` is present in `s.windows` by scanning for `w.window_id == window_id` (replaces Vec index bounds check)
- Duplicate guard: `pairs.iter().any(|p| p.write_seq == write_seq && p.window_id == window_id)`
- Record pair with `window_id` instead of `window_index`

Full replacement of the function body:

```rust
pub fn pair_action_consequence(&self, write_seq: u64, window_id: u64, delay_us: u64) -> bool {
    if write_seq == 0 {
        return false;
    }
    if window_id == 0 {
        return false;
    }
    {
        let ao = self.action_output.read().unwrap();
        if !ao.write_epoch_history.iter().any(|r| r.write_seq == write_seq) {
            return false;
        }
    }
    let pair_t_us = self.clock_us.load(Ordering::Relaxed);
    let mut s = self.action_consequence.write().unwrap();
    if !s.windows.iter().any(|w| w.window_id == window_id) {
        return false;
    }
    if s.pairs.iter().any(|p| p.write_seq == write_seq && p.window_id == window_id) {
        return false;
    }
    s.pairs.push(ActionConsequencePair {
        write_seq,
        window_id,
        delay_us,
        status: ConsequencePairStatus::Paired,
        pair_t_us,
    });
    s.n_pairs = s.pairs.len() as u32;
    true
}
```

- [ ] **Step 3: Update `try_auto_pair_latest_epoch_to_window` to use window_id**

In the function, the pair duplicate guard and the `pairs.push` must use `window_id` instead of `window_index`. Steps 1 and 4 of the existing function:

Step 1 now extracts `window_id` alongside `window_start_t_us`:

```rust
let (window_id, window_start_t_us) = {
    let ac = self.action_consequence.read().unwrap();
    if ac.windows.is_empty() {
        return false;
    }
    let last = ac.windows.last().unwrap();
    (last.window_id, last.start_t_us)
};
```

Step 3 duplicate guard uses `window_id`:

```rust
let ac = self.action_consequence.read().unwrap();
if ac.pairs.iter().any(|p| p.write_seq == write_seq && p.window_id == window_id) {
    return false;
}
```

Step 4 under write lock: bounds check is now "window still exists by id", and push uses `window_id`:

```rust
let mut ac = self.action_consequence.write().unwrap();
if !ac.windows.iter().any(|w| w.window_id == window_id) {
    return false;
}
if ac.pairs.iter().any(|p| p.write_seq == write_seq && p.window_id == window_id) {
    return false;
}
ac.pairs.push(ActionConsequencePair {
    write_seq,
    window_id,
    delay_us,
    status: ConsequencePairStatus::Paired,
    pair_t_us,
});
ac.n_pairs = ac.pairs.len() as u32;
true
```

- [ ] **Step 4: Bump SNAPSHOT_VERSION to 29 in `lib.rs`**

```rust
// v29: window_id stable ID added to ActionConsequenceWindow; pair's
// window_index (Vec position) replaced by window_id (stable monotonic u64).
// Rebuilds are required — old snapshots are rejected.
pub const SNAPSHOT_VERSION: u64 = 29;
```

- [ ] **Step 5: Update `SnapshotConsequenceWindow` in `snapshot.rs`**

In the snapshot struct, add `window_id: u64` as the first field:

```rust
#[derive(Clone, Debug, Default)]
pub struct SnapshotConsequenceWindow {
    pub window_id: u64,
    pub modality_kind: u64,
    pub start_t_us: u64,
    pub end_t_us: u64,
    pub magnitude: f64,
    pub phase: u64,
    pub last_update_t_us: u64,
}
```

- [ ] **Step 6: Update `SnapshotConsequencePair` in `snapshot.rs`**

Replace `window_index: u32` with `window_id: u64`:

```rust
#[derive(Clone, Debug, Default)]
pub struct SnapshotConsequencePair {
    pub write_seq: u64,
    pub window_id: u64,
    pub delay_us: u64,
    pub status: u64,
    pub pair_t_us: u64,
}
```

- [ ] **Step 7: Update snapshot write (serialize) for consequence section**

In the write path for windows (currently writes: `modality_kind, start_t_us, end_t_us, magnitude, phase, last_update_t_us`), prepend `window_id`:

```rust
for w in &s.windows {
    buf.extend_from_slice(&w.window_id.to_le_bytes());
    buf.extend_from_slice(&(w.modality_kind as u64).to_le_bytes());
    buf.extend_from_slice(&w.start_t_us.to_le_bytes());
    buf.extend_from_slice(&w.end_t_us.to_le_bytes());
    buf.extend_from_slice(&w.magnitude.to_bits().to_le_bytes());
    buf.extend_from_slice(&w.phase.as_u64().to_le_bytes());
    buf.extend_from_slice(&w.last_update_t_us.to_le_bytes());
}
```

For pairs (currently writes `write_seq, window_index, delay_us, status, pair_t_us`), replace `window_index as u64` with `window_id`:

```rust
for p in &s.pairs {
    buf.extend_from_slice(&p.write_seq.to_le_bytes());
    buf.extend_from_slice(&p.window_id.to_le_bytes());
    buf.extend_from_slice(&p.delay_us.to_le_bytes());
    buf.extend_from_slice(&(p.status as u64).to_le_bytes());
    buf.extend_from_slice(&p.pair_t_us.to_le_bytes());
}
```

- [ ] **Step 8: Update snapshot read (deserialize) for consequence section**

In the read path for windows, add `window_id` as first read:

```rust
let window_id  = rd_u64(bytes, cur);
let modality   = rd_u64(bytes, cur);
let start_t_us = rd_u64(bytes, cur);
let end_t_us   = rd_u64(bytes, cur);
let magnitude  = f64::from_bits(rd_u64(bytes, cur));
let phase      = rd_u64(bytes, cur);
let last_update_t_us = rd_u64(bytes, cur);
windows.push(SnapshotConsequenceWindow {
    window_id,
    modality_kind: modality,
    start_t_us,
    end_t_us,
    magnitude,
    phase,
    last_update_t_us,
});
```

For pairs, read `window_id` instead of `window_index`:

```rust
let write_seq  = rd_u64(bytes, cur);
let window_id  = rd_u64(bytes, cur);
let delay_us   = rd_u64(bytes, cur);
let status     = rd_u64(bytes, cur);
let pair_t_us  = rd_u64(bytes, cur);
pairs.push(SnapshotConsequencePair { write_seq, window_id, delay_us, status, pair_t_us });
```

- [ ] **Step 9: Update `observer.rs` to render `wid` instead of `wi`**

Find the pair render line:

```rust
// old:
format!("p{}[ws{}|wi{}|d{}|st{}]", i, p.write_seq, p.window_index, p.delay_us, p.status)

// new:
format!("p{}[ws{}|wid{}|d{}|st{}]", i, p.write_seq, p.window_id, p.delay_us, p.status as u64)
```

- [ ] **Step 10: Update existing test assertions in `action_consequence_loop.rs`**

Find every `pair.window_index` usage and update:

1. Test `action_consequence_pairs_a_write_epoch_with_a_window`:
   - Change `vp.pair_action_consequence(write_seq, 0, 40)` to use the actual window_id
   - Change `assert_eq!(st.pairs[0].window_index, 0)` to `assert_eq!(st.pairs[0].window_id, wid)` where `wid = vp.read_action_consequence().windows[0].window_id`

2. Test `snapshot_carries_action_consequence_windows_and_pairs`:
   - Change `vp.pair_action_consequence(write_seq, 0, 40)` to use the actual window_id
   - Change `assert_eq!(snap.action_consequence.pairs[0].write_seq, write_seq)` (already correct, but add `pairs[0].window_id` assertion)

3. Test `pair_rejects_invalid_window_index` → rename to `pair_rejects_unissued_window_id`, update to pass `window_id: u64` of 9999 (which was never issued)

4. Tests `pair_records_real_pair_t_us` and `pair_t_us_comes_from_viewport_clock_us`:
   - Change `vp.pair_action_consequence(write_seq, 0, 40)` to use actual window_id

5. Tests `pairing_open_window_is_allowed` and `pairing_closed_window_is_allowed`:
   - Change `vp.pair_action_consequence(write_seq, idx, 40)` to use window_id from the actual window

6. Test `live_auto_pair_forms_link_when_epoch_precedes_window_within_time_window`:
   - Change `assert_eq!(st.pairs[0].window_index, 0)` to match window_id of the first window

The general pattern for all these updates:
```rust
// Before calling pair_action_consequence with a Vec index:
let wid = vp.read_action_consequence().windows[idx as usize].window_id;
vp.pair_action_consequence(write_seq, wid, 40);

// For assertions:
assert_eq!(st.pairs[0].window_id, wid);
```

- [ ] **Step 11: Compile and run the full test suite**

```
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep -E "^test result|FAILED"
```
Expected: all `0 failed`. Fix any remaining compilation errors before proceeding.

---

## Chunk 3 — Cap constants and epoch eviction

### Task 5: Write failing tests for epoch cap behavior

**Files:**
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Append the following failing tests**

```rust
// ─── Epoch history cap ────────────────────────────────────────────────────────

// ─── EC1. write_epoch_history does not exceed WRITE_EPOCH_HISTORY_CAP ─────────

#[test]
fn epoch_history_stays_within_cap() {
    use bionic_v0::WRITE_EPOCH_HISTORY_CAP;

    let vp = Viewport::new();

    // Push WRITE_EPOCH_HISTORY_CAP + 10 epochs. Each Begin at a different t_us
    // so they are all within the time window (start_t_us are all the same to
    // avoid any temporal-window-based eviction; we just want to trigger cap).
    // Use very small t_us values so none exceed TEMPORAL_PAIR_MAX_DELAY_US from
    // each other.
    let n = WRITE_EPOCH_HISTORY_CAP + 10;
    for i in 0..n {
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.1,
            event_t_us: i as u64 * 100,
        });
    }

    let st = vp.read_action_output();
    assert!(
        st.write_epoch_history.len() <= WRITE_EPOCH_HISTORY_CAP,
        "write_epoch_history len {} must not exceed WRITE_EPOCH_HISTORY_CAP {}",
        st.write_epoch_history.len(),
        WRITE_EPOCH_HISTORY_CAP
    );
    assert_eq!(
        st.n_write_epochs as usize,
        st.write_epoch_history.len(),
        "n_write_epochs must equal actual Vec length after eviction"
    );
}

// ─── EC2. Most-recent epochs are retained after eviction ──────────────────────

#[test]
fn epoch_history_retains_most_recent_after_eviction() {
    use bionic_v0::WRITE_EPOCH_HISTORY_CAP;

    let vp = Viewport::new();
    let n = WRITE_EPOCH_HISTORY_CAP + 5;
    for i in 0..n {
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.1,
            event_t_us: i as u64 * 100,
        });
    }

    let st = vp.read_action_output();
    // The last epoch must be present (most recently appended).
    let last_seq = st.channels[0].current_write_seq;
    assert!(
        st.write_epoch_history.iter().any(|r| r.write_seq == last_seq),
        "most recently appended epoch (seq={}) must be in history after eviction",
        last_seq
    );
    // Seq 1 (the very first) must have been evicted.
    assert!(
        !st.write_epoch_history.iter().any(|r| r.write_seq == 1),
        "seq=1 (oldest epoch) must have been evicted when cap was exceeded"
    );
}
```

- [ ] **Step 2: Run and confirm RED**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop "epoch_history_stays\|epoch_history_retains" 2>&1 | tail -20
```
Expected: compile error (WRITE_EPOCH_HISTORY_CAP not defined in lib).

### Task 6: Add cap constants and epoch eviction

**Files:**
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/action_output.rs`

- [ ] **Step 1: Add cap constants to `lib.rs`**

After the `TEMPORAL_PAIR_MAX_DELAY_US` constant, add:

```rust
/// Engineering cap on write_epoch_history Vec length.
///
/// Substrate knob — not a theoretical learning window.  The actual number of
/// live epochs needed for nearest-legal-predecessor selection is determined by
/// TEMPORAL_PAIR_MAX_DELAY_US × write-event frequency; 128 gives comfortable
/// headroom at current rates.  Adjust via exploration history as write patterns
/// become known.
pub const WRITE_EPOCH_HISTORY_CAP: usize = 128;

/// Engineering cap on consequence windows Vec length.
///
/// Substrate knob — not a theoretical consequence count.  At 30 Hz PHOT with
/// PHOT_CONSEQUENCE_QUIESCENCE_US ≈ 300 ms, at most a few windows open per
/// second; 128 gives many seconds of headroom.  Adjust via exploration history.
pub const CONSEQUENCE_WINDOW_CAP: usize = 128;

/// Engineering cap on consequence pairs Vec length.
///
/// Substrate knob — not a theoretical pair count.  Larger than the window cap
/// because multiple epochs may pair with the same window, and history lookback
/// wants more pair records than active windows.
pub const PAIR_HISTORY_CAP: usize = 256;
```

- [ ] **Step 2: Add epoch eviction to the Begin branch of `apply_action_output_event`**

At the bottom of the `Begin` match arm, after `s.n_write_epochs = s.write_epoch_history.len() as u32;`, add:

```rust
// Eviction: keep history within the engineering cap. Evict from the front
// (oldest first). Selection is purely by append order (oldest = index 0),
// not by result meaning or object importance. This is a substrate knob.
if s.write_epoch_history.len() > WRITE_EPOCH_HISTORY_CAP {
    let evict_n = s.write_epoch_history.len() - WRITE_EPOCH_HISTORY_CAP;
    s.write_epoch_history.drain(0..evict_n);
    s.n_write_epochs = s.write_epoch_history.len() as u32;
}
```

Add the import at the top of `action_output.rs` if not already present:
```rust
use crate::WRITE_EPOCH_HISTORY_CAP;
```

- [ ] **Step 3: Run epoch cap tests and confirm GREEN**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop "epoch_history" 2>&1 | tail -20
```
Expected: 2 tests pass.

- [ ] **Step 4: Run full suite to confirm no regressions**

```
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep -E "^test result|FAILED"
```
Expected: all `0 failed`.

---

## Chunk 4 — Window and pair eviction

### Task 7: Write failing tests for window and pair cap behavior

**Files:**
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Append the following failing tests**

```rust
// ─── Window and pair history cap ─────────────────────────────────────────────

// ─── WC1. Consequence windows Vec does not exceed CONSEQUENCE_WINDOW_CAP ──────

#[test]
fn consequence_windows_stay_within_cap() {
    use bionic_v0::CONSEQUENCE_WINDOW_CAP;

    let vp = Viewport::new();

    // Push CONSEQUENCE_WINDOW_CAP + 5 windows. Use accumulate + tick to get
    // Open→Closed cycles. Each window gets its own start_t_us.
    let n = CONSEQUENCE_WINDOW_CAP + 5;
    for i in 0..n {
        let base_t = i as u64 * 2_000;
        vp.accumulate_ingress_change(base_t, 0.3);
        // Close it immediately via quiescence tick.
        vp.tick_ingress_accumulation(base_t + 1_000, 500);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.windows.len() <= CONSEQUENCE_WINDOW_CAP,
        "windows len {} must not exceed CONSEQUENCE_WINDOW_CAP {}",
        st.windows.len(),
        CONSEQUENCE_WINDOW_CAP
    );
    assert_eq!(
        st.n_windows as usize,
        st.windows.len(),
        "n_windows must equal actual Vec length after eviction"
    );
}

// ─── WC2. Pairs Vec does not exceed PAIR_HISTORY_CAP ─────────────────────────

#[test]
fn consequence_pairs_stay_within_cap() {
    use bionic_v0::PAIR_HISTORY_CAP;

    let vp = Viewport::new();

    // Produce PAIR_HISTORY_CAP + 5 pairs by repeatedly beginning an epoch
    // and accumulating a consequence window, then auto-pairing.
    let n = PAIR_HISTORY_CAP + 5;
    for i in 0..n {
        let t_epoch = i as u64 * 3_000;
        let t_window = t_epoch + 500_000; // within TEMPORAL_PAIR_MAX_DELAY_US
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.2,
            event_t_us: t_epoch,
        });
        vp.accumulate_ingress_change(t_window, 0.3);
        vp.try_auto_pair_latest_epoch_to_window(t_window);
        // Close the window so the next accumulate opens a fresh one.
        let widx = vp.read_action_consequence().windows.len() - 1;
        vp.tick_ingress_accumulation(t_window + 400_000, 300_000);
        let _ = widx;
    }

    let st = vp.read_action_consequence();
    assert!(
        st.pairs.len() <= PAIR_HISTORY_CAP,
        "pairs len {} must not exceed PAIR_HISTORY_CAP {}",
        st.pairs.len(),
        PAIR_HISTORY_CAP
    );
    assert_eq!(
        st.n_pairs as usize,
        st.pairs.len(),
        "n_pairs must equal actual Vec length after eviction"
    );
}
```

- [ ] **Step 2: Run and confirm RED**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop "windows_stay_within\|pairs_stay_within" 2>&1 | tail -20
```
Expected: tests fail (windows and pairs grow beyond cap).

### Task 8: Implement window and pair eviction

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`

- [ ] **Step 1: Add a `trim_windows` helper method to `ActionConsequenceState` (or inline in append points)**

Add window eviction after `push_action_consequence_window` and `begin_action_consequence_window`. In both, after `s.n_windows = s.windows.len() as u32;`, add:

```rust
// Eviction: keep within CONSEQUENCE_WINDOW_CAP. Evict from the front,
// oldest first. Prefers Closed windows. If all windows are Closed, drain
// from front. No result/meaning judgment — purely by append order.
if s.windows.len() > CONSEQUENCE_WINDOW_CAP {
    let evict_n = s.windows.len() - CONSEQUENCE_WINDOW_CAP;
    // Only evict if front windows are Closed (safer). If window is Open,
    // still evict (extreme backlog) but this is expected to be rare.
    s.windows.drain(0..evict_n);
    s.n_windows = s.windows.len() as u32;
}
```

Add the import:
```rust
use crate::CONSEQUENCE_WINDOW_CAP;
```

- [ ] **Step 2: Add pair eviction in `pair_action_consequence` and `try_auto_pair_latest_epoch_to_window`**

In both functions, after the final `s.n_pairs = s.pairs.len() as u32;` (or `ac.n_pairs = ac.pairs.len() as u32;`), add:

```rust
// Eviction: keep pairs within PAIR_HISTORY_CAP. Evict oldest first.
// Note: after eviction, the duplicate guard for old (write_seq, window_id)
// pairs will no longer prevent re-pairing. This is acceptable boundary
// behavior per spec §5.2 法则 P3.
if s.pairs.len() > PAIR_HISTORY_CAP {
    let evict_n = s.pairs.len() - PAIR_HISTORY_CAP;
    s.pairs.drain(0..evict_n);
    s.n_pairs = s.pairs.len() as u32;
}
```

(Use `ac` instead of `s` in the `try_auto_pair_latest_epoch_to_window` variant.)

Add import:
```rust
use crate::PAIR_HISTORY_CAP;
```

- [ ] **Step 3: Run window and pair cap tests and confirm GREEN**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop "windows_stay_within\|pairs_stay_within" 2>&1 | tail -20
```
Expected: 2 tests pass.

- [ ] **Step 4: Run full test suite to confirm no regressions**

```
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep -E "^test result|FAILED"
```
Expected: all `0 failed`.

---

## Chunk 5 — Final verification

### Task 9: Full verification run

**Files:** None modified.

- [ ] **Step 1: Run action_consequence_loop full suite**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop -- --nocapture 2>&1 | tail -60
```
Expected: all tests pass.

- [ ] **Step 2: Run real_phot_adapter suite**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test real_phot_adapter -- --nocapture 2>&1 | tail -30
```
Expected: all tests pass.

- [ ] **Step 3: Run action_output_substrate suite**

```
cargo test --manifest-path bionic_v0/Cargo.toml --test action_output_substrate -- --nocapture 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 4: Run full test suite**

```
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep "^test result"
```
Expected: all `0 failed`.

- [ ] **Step 5: Note on git**

The workspace at `/Users/joe/Documents/unnamed` is a git repository (confirmed `git status` shows `on branch main`). If the user wishes to commit, the commit message should describe the three logically distinct changes:
1. Stable window_id introduction and pair reference migration (SNAPSHOT_VERSION → 29)
2. Cap constants for epoch, window, and pair history
3. Eviction in append paths for all three Vecs

---

## Out Of Scope

- Learned/adaptive cap sizes (determining caps through exploration history — future memory-policy spec)
- Multi-modal competing epoch selection (outside single-channel foundation)
- Pair "stale" lifecycle (epochs that missed their window — future spec)
- Any result, success, or importance classification for retention decisions
- fast-path, imitation, or self-other comparison
- memory summary / history consolidation

## Verification Checklist

- `cargo test action_consequence_loop -- --nocapture`
- `cargo test real_phot_adapter -- --nocapture`
- `cargo test action_output_substrate -- --nocapture`
- `cargo test --manifest-path bionic_v0/Cargo.toml`
- Confirm `SNAPSHOT_VERSION` is 29 in lib.rs
- Confirm `pair.window_id` (not `window_index`) in all structs and tests
- Confirm cap constants have "substrate knob" (not "theoretical window") comments
