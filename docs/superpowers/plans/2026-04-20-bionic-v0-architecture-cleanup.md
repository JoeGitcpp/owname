# bionic_v0 Architecture Cleanup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the current semantic-readiness / symbolizer / concept-id architecture from the formal body pipeline, preserve the substrate layers, and align code/docs/tests with the frozen cleanup spec.

**Architecture:** The cleanup proceeds in three passes: first stop the violating layers from participating in the live body/snapshot/observer path; then remove or quarantine their data structures and tests; finally align specs, naming, and repo entrypoints so future work reads the correct axioms first. The core substrate stays intact throughout.

**Tech Stack:** Rust, Cargo tests, Markdown docs

---

## File Structure

- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/ordered_global_refresh.rs`
- Delete or quarantine: `/Users/joe/Documents/unnamed/bionic_v0/tests/semantic_readiness.rs`
- Delete or quarantine: `/Users/joe/Documents/unnamed/bionic_v0/tests/symbolizer.rs`
- Modify: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md`
- Modify or create repo entrypoint docs near `/Users/joe/Documents/unnamed/` so future work references:
  - `/Users/joe/Documents/unnamed/docs/axioms/2026-04-20-new-axioms-v1.md`
  - `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md`

---

## Chunk 1: Stop The Violating Layers

### Task 1: Remove semantic/symbolizer stages from the live body cycle

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/ordered_global_refresh.rs`

- [ ] **Step 1: Write the failing test**

Add or update a test that asserts the ordered refresh contract ends at reasoning history and snapshot, not semantic-readiness or symbolizer.

```rust
#[test]
fn ordered_refresh_cycle_stops_before_semantic_layers() {
    let vp = Arc::new(Viewport::new());
    let t = 1_000_000;

    vp.refresh_global_attention(t);
    vp.refresh_global_relations(t);
    vp.refresh_global_relation_predictor(t);
    vp.refresh_global_reasoning(t);
    vp.refresh_global_reasoning_history(t);

    let snap = vp.read_global_reasoning_memory_summary();
    assert_eq!(snap.last_update_t_us, t);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ordered_refresh --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because the existing tests/comments/contracts still expect semantic-readiness and symbolizer in the cycle.

- [ ] **Step 3: Write minimal implementation**

Update `body.rs` so:

- `run_global_refresh_cycle` no longer calls `refresh_semantic_readiness`
- `run_global_refresh_cycle` no longer calls `refresh_symbolizer`
- comments describe the new terminal stage accurately

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ordered_refresh --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs /Users/joe/Documents/unnamed/bionic_v0/tests/ordered_global_refresh.rs
git commit -m "refactor: remove semantic layers from body refresh cycle"
```

### Task 2: Remove body-process human-readable prints

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs`
- Test: add a small regression test or lint-style grep check under `/Users/joe/Documents/unnamed/bionic_v0/tests`

- [ ] **Step 1: Write the failing test**

Add a grep-based or unit-style regression that fails if `body.rs` contains `println!` or `eprintln!`.

```rust
#[test]
fn body_contains_no_human_readable_prints() {
    let body = std::fs::read_to_string("src/bin/body.rs").unwrap();
    assert!(!body.contains("println!"));
    assert!(!body.contains("eprintln!"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test body_contains_no_human_readable_prints --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because `body.rs` currently prints camera startup/fault text.

- [ ] **Step 3: Write minimal implementation**

Replace the `eprintln!` diagnostics with body-safe numeric state handling only.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test body_contains_no_human_readable_prints --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/bin/body.rs
git commit -m "refactor: remove body process print output"
```

---

## Chunk 2: Remove Or Quarantine The Violating Substrate

### Task 3: Remove semantic-readiness and symbolizer state from viewport/lib contracts

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Test: targeted viewport tests plus compile checks

- [ ] **Step 1: Write the failing test**

Add or update tests to assert:

- `Viewport` no longer exposes `read_semantic_readiness`
- `Viewport` no longer exposes `read_symbolizer`
- constants such as `SEMANTIC_*` and `CONCEPT_*` are no longer part of the public contract

```rust
// compile-fail style expectation or plain test comments in the plan:
// code referencing semantic_readiness/symbolizer should no longer compile.
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test semantic_readiness --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: Existing semantic tests still compile/run against the old API.

- [ ] **Step 3: Write minimal implementation**

In `viewport.rs` and `lib.rs`:

- remove `SemanticReadinessState`
- remove `SymbolizerState`
- remove `refresh_semantic_readiness`
- remove `refresh_symbolizer`
- remove `read_semantic_readiness`
- remove `read_symbolizer`
- remove related constants and comments

If deletion is too large for one pass, move them into an explicitly quarantined internal module not referenced by `body`, `snapshot`, or `observer`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`

Expected: semantic/symbolizer tests are gone or quarantined; remaining substrate tests pass.

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/src/lib.rs
git commit -m "refactor: remove semantic readiness and symbolizer contracts"
```

### Task 4: Remove violating fields from snapshot and observer

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Test: snapshot roundtrip tests and observer output tests

- [ ] **Step 1: Write the failing test**

Add/update snapshot regression coverage so the snapshot schema no longer expects `semantic_readiness` or `symbolizer`.

```rust
#[test]
fn snapshot_schema_contains_no_semantic_or_symbolizer_sections() {
    let snap_src = std::fs::read_to_string("src/snapshot.rs").unwrap();
    assert!(!snap_src.contains("semantic_readiness"));
    assert!(!snap_src.contains("symbolizer"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test snapshot_schema_contains_no_semantic_or_symbolizer_sections --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because the snapshot and observer still carry those sections.

- [ ] **Step 3: Write minimal implementation**

In `snapshot.rs`:

- drop v21/v22 semantic/symbolizer fields from the current live schema
- remove read/write helpers for those structures
- bump or simplify the snapshot versioning accordingly

In `observer.rs`:

- remove semantic-readiness display block
- remove symbolizer display block
- keep observer limited to remaining substrate facts

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test snapshot --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS for the retained snapshot tests

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs /Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs
git commit -m "refactor: remove semantic layers from snapshot and observer"
```

### Task 5: Delete or quarantine the semantic/symbolizer test contracts

**Files:**
- Delete or quarantine: `/Users/joe/Documents/unnamed/bionic_v0/tests/semantic_readiness.rs`
- Delete or quarantine: `/Users/joe/Documents/unnamed/bionic_v0/tests/symbolizer.rs`
- Modify: any test modules that import those APIs

- [ ] **Step 1: Write the failing test**

Add a repository hygiene test that fails if semantic/symbolizer contract tests still exist under the active test suite.

```rust
#[test]
fn active_test_suite_contains_no_semantic_contract_files() {
    assert!(!std::path::Path::new("tests/semantic_readiness.rs").exists());
    assert!(!std::path::Path::new("tests/symbolizer.rs").exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test active_test_suite_contains_no_semantic_contract_files --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

Either:

- delete the test files outright, or
- move them under an `experimental/` or `quarantine/` path outside the active Cargo test suite with a clear header that they protect a rejected architecture

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`

Expected: PASS without semantic/symbolizer contract coverage

- [ ] **Step 5: Commit**

```bash
git add -A /Users/joe/Documents/unnamed/bionic_v0/tests
git commit -m "test: remove semantic and symbolizer contract suite"
```

---

## Chunk 3: Align Docs And Future Entry Points

### Task 6: Update the old v0 spec so it no longer seeds future drift

**Files:**
- Modify: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md`
- Reference: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md`
- Reference: `/Users/joe/Documents/unnamed/docs/axioms/2026-04-20-new-axioms-v1.md`

- [ ] **Step 1: Write the failing test**

Add a doc-grep check that fails if the old v0 spec still presents the known drift terms without qualification.

```bash
rg -n "大脑警察|主人|ethics_predictor|可视化输出" /Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md
```

- [ ] **Step 2: Run test to verify it fails**

Run the grep above.

Expected: matches present in the current document.

- [ ] **Step 3: Write minimal implementation**

Revise the 2026-04-19 spec so that:

- external observation stays outside the body boundary
- risky names are replaced or explicitly marked non-ontological
- the document links forward to the cleanup spec and language axioms
- implementation scope matches current reality or is clearly marked stale

- [ ] **Step 4: Run test to verify it passes**

Run: `rg -n "大脑警察|主人|ethics_predictor|可视化输出" /Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md`

Expected: no unqualified matches, or only matches inside explicit deprecation notes

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-19-bionic-consciousness-stream-v0-design.md
git commit -m "docs: align v0 spec with cleanup architecture"
```

### Task 7: Add a repository entrypoint so future work reads the axioms first

**Files:**
- Create or modify a root entrypoint doc near `/Users/joe/Documents/unnamed/`
- Reference:
  - `/Users/joe/Documents/unnamed/docs/axioms/2026-04-20-new-axioms-v1.md`
  - `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md`

- [ ] **Step 1: Write the failing test**

Create a simple repo hygiene check that fails when no root entrypoint doc points developers to the axioms/specs.

```bash
test -f /Users/joe/Documents/unnamed/README.md || test -f /Users/joe/Documents/unnamed/AGENTS.md
```

- [ ] **Step 2: Run test to verify it fails**

Run the shell check above.

Expected: FAIL because no root entrypoint currently exists.

- [ ] **Step 3: Write minimal implementation**

Create a root `README.md` or `AGENTS.md` that says, at minimum:

- read the language axioms first
- read the architecture cleanup spec before changing `bionic_v0`
- semantic-readiness / symbolizer / concept-id are rejected architecture

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
test -f /Users/joe/Documents/unnamed/README.md || test -f /Users/joe/Documents/unnamed/AGENTS.md
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/README.md /Users/joe/Documents/unnamed/AGENTS.md
git commit -m "docs: add repository entrypoint for cleanup axioms"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-20-bionic-v0-architecture-cleanup.md`. Ready to execute?
