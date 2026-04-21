# Language Binding Layer Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first minimal language-binding foundation on top of the cleaned substrate without reintroducing naming gates, concept ids, or token-centric ontology.

**Architecture:** This first phase does not build full language understanding. It adds a bounded binding substrate that can observe external pattern events, reference existing object/relation/time structures, store reversible binding traces, and expose those traces only through observation-safe state. Learning units are binding events, not tokens or concept allocators.

**Tech Stack:** Rust, Cargo tests, Markdown docs

---

## File Structure

- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/language_binding.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`
- Modify: `/Users/joe/Documents/unnamed/AGENTS.md`
- Reference: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-20-language-binding-layer-design.md`

---

## Chunk 1: Define The Binding Substrate

### Task 1: Add the binding-state data model

**Files:**
- Create: `/Users/joe/Documents/unnamed/bionic_v0/src/language_binding.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/lib.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add tests that define the minimum allowed state:

- external pattern kind
- target references to object/relation/time structures
- binding strength
- candidate / reinforced / weakened status
- reversible evidence pointers

```rust
#[test]
fn binding_state_tracks_traces_not_concept_ids() {
    let st = LanguageBindingState::default();
    assert_eq!(st.n_bindings, 0);
    assert!(!format!("{:?}", st).contains("concept"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because the module and types do not exist yet.

- [ ] **Step 3: Write minimal implementation**

In `language_binding.rs`, define only bounded, reversible binding types such as:

- `ExternalPatternKind`
- `BindingTargetRef`
- `BindingStrength`
- `BindingStatus`
- `BindingTrace`
- `LanguageBindingState`

Do not introduce:

- `concept_id`
- semantic gates
- token allocators
- naming-complete flags

Export the module from `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/language_binding.rs /Users/joe/Documents/unnamed/bionic_v0/src/lib.rs /Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs
git commit -m "feat: add language binding substrate types"
```

### Task 2: Mount the binding state on the viewport without giving it ontological authority

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add tests asserting:

- `Viewport` can hold language-binding state
- the binding state is read-only with respect to object/relation substrate
- the binding state has no readiness or concept APIs

```rust
#[test]
fn viewport_exposes_binding_state_without_readiness_gates() {
    let vp = Viewport::new();
    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test viewport_exposes_binding_state_without_readiness_gates --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because the viewport does not expose the binding state yet.

- [ ] **Step 3: Write minimal implementation**

Add a `language_binding` subtree to `Viewport` with:

- initializer
- read method
- narrow write/update methods for binding traces only

Do not wire it into `body` as a gate or promotion stage.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs
git commit -m "feat: mount language binding state on viewport"
```

---

## Chunk 2: Add Minimal Binding Event Learning

### Task 3: Define the first binding-event update path

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/language_binding.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add tests for the minimum reversible event cycle:

- propose a candidate binding
- reinforce a matching binding
- weaken or demote a failed binding
- preserve evidence references

```rust
#[test]
fn binding_event_learning_is_reversible() {
    let mut st = LanguageBindingState::default();
    let ev = sample_binding_event();
    st.apply_binding_event(ev);
    assert_eq!(st.n_bindings, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test binding_event_learning_is_reversible --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL because there is no binding-event update path yet.

- [ ] **Step 3: Write minimal implementation**

Implement a minimal `BindingEvent` and `apply_binding_event` path that:

- creates a candidate binding when a new external pattern references an active target
- reinforces the same binding on repeated support
- weakens or demotes it on contradictory feedback
- never creates irreversible names or ids

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/language_binding.rs /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs
git commit -m "feat: add reversible binding event updates"
```

### Task 4: Add hygiene tests that block rejected architecture from reappearing in the binding layer

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add grep- or source-based tests that fail if the new binding layer contains:

- `semantic_readiness`
- `symbolizer`
- `concept_id`
- `token_id`
- `naming gate`

```rust
#[test]
fn binding_layer_contains_no_rejected_architecture_terms() {
    let src = std::fs::read_to_string("src/language_binding.rs").unwrap();
    assert!(!src.contains("concept_id"));
    assert!(!src.contains("semantic_readiness"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Intentionally add or observe at least one failing term while drafting the module, then run:

`cargo test binding_layer_contains_no_rejected_architecture_terms --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL before cleanup, PASS after cleanup.

- [ ] **Step 3: Write minimal implementation**

Clean the module and finalize the hygiene tests.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs
git commit -m "test: add language binding hygiene guards"
```

---

## Chunk 3: Observation And Repo Entry Alignment

### Task 5: Expose binding traces through snapshot and observer as traces, not concepts

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Test: `/Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add tests that assert snapshot/observer expose:

- binding count
- target references
- strengths
- statuses

while not exposing:

- names
- concept ids
- semantic-ready flags

```rust
#[test]
fn snapshot_carries_binding_traces_not_concepts() {
    let snap_src = std::fs::read_to_string("src/snapshot.rs").unwrap();
    assert!(!snap_src.contains("concept_id"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test snapshot_carries_binding_traces_not_concepts --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: FAIL until snapshot/observer support is added cleanly.

- [ ] **Step 3: Write minimal implementation**

Add a bounded snapshot representation for language-binding traces and a matching observer block.

Observer output must stay descriptive of trace state only; it must not attach names or infer ontological identities.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test language_binding --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs /Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs /Users/joe/Documents/unnamed/bionic_v0/tests/language_binding.rs
git commit -m "feat: expose language binding traces in snapshot and observer"
```

### Task 6: Update the repository entrypoint to include the new binding spec

**Files:**
- Modify: `/Users/joe/Documents/unnamed/AGENTS.md`

- [ ] **Step 1: Write the failing test**

Add a simple doc-grep check:

```bash
rg -n "language-binding-layer-design" /Users/joe/Documents/unnamed/AGENTS.md
```

- [ ] **Step 2: Run test to verify it fails**

Run the grep above.

Expected: FAIL because `AGENTS.md` does not yet mention the new spec.

- [ ] **Step 3: Write minimal implementation**

Update `AGENTS.md` so future work reads:

- the language axioms
- the cleanup spec
- the language binding spec

in that order.

- [ ] **Step 4: Run test to verify it passes**

Run: `rg -n "language-binding-layer-design" /Users/joe/Documents/unnamed/AGENTS.md`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/Documents/unnamed/AGENTS.md
git commit -m "docs: add language binding spec to repository entrypoint"
```

---

## Chunk 4: Verification Gate

### Task 7: Run full verification and document the non-git limitation

**Files:**
- Modify if needed: `/Users/joe/Documents/unnamed/docs/superpowers/plans/2026-04-20-language-binding-layer-foundation.md`

- [ ] **Step 1: Write the failing test**

Define the verification command set:

```bash
cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml
```

Expected before implementation: FAIL because the feature does not yet exist.

- [ ] **Step 2: Run test to verify it fails**

Run the command above before implementation starts.

- [ ] **Step 3: Write minimal implementation**

Complete the preceding tasks.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml
```

Expected: PASS across the existing substrate tests and the new language-binding tests.

Also record explicitly:

- this workspace is not a git repository
- commit steps in this plan must be translated into “stage logically / report unable to commit”

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: add language binding foundation"
```

If this fails because the workspace is not a git repository, report that explicitly and do not fabricate a commit.

---

Plan complete and saved to `docs/superpowers/plans/2026-04-20-language-binding-layer-foundation.md`. Ready to execute?
