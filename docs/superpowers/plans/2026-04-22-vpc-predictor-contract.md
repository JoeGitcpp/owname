# VPC Predictor Contract Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a unified predictor-family contract for `Ar2Predictor`, `PhotSlotPredictor`, and relation predictor so warmup, err legality, provenance, and local-update boundaries become shared substrate behavior instead of per-family convention.

**Architecture:** Reuse the existing `vpc_contract` substrate and extend it only as needed for predictor-family traces. The work is split into three chunks: freeze shared predictor-family docs, add minimal family metadata/runtime hooks, then lock the new contract through targeted tests plus regression suites for PHOT and relation prediction.

**Tech Stack:** Rust, Cargo tests, Markdown docs

---

## File Structure

- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/vpc.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/phot_predictor.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/predictor.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Reference: `/Users/joe/Documents/unnamed/docs/superpowers/specs/2026-04-22-vpc-predictor-contract-design.md`

## Chunk 1: Shared Predictor Family Contract

### Task 1: Add failing tests for common predictor-family metadata

**Files:**
- Create: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/vpc.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`

- [ ] **Step 1: Write the failing tests**
Add tests that define shared family metadata requirements:
- family code is present
- domain kind is present
- warmup/open/resolved states are distinguishable
- provenance stays numeric / engineering-only

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test vpc_predictor_contract --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
Expected: FAIL because the shared predictor-family trace shape does not exist yet.

- [ ] **Step 3: Write minimal implementation**
Extend `vpc.rs` with the smallest additional family metadata needed by all three predictor families.

- [ ] **Step 4: Run test to verify it passes**
Run the same command and confirm PASS.

- [ ] **Step 5: Commit**
```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/vpc.rs /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs
git commit -m "feat: add shared predictor-family contract metadata"
```

## Chunk 2: Hook Existing Predictor Families

### Task 2: Bring `PhotSlotPredictor` under the shared contract

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/phot_predictor.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`

- [ ] **Step 1: Write the failing tests**
Add tests asserting:
- PHOT family declares slot-local domain provenance
- warmup emits zero err/precision but still records family-level open state
- post-warmup updates resolve legally

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test phot_predictor_contract --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
Expected: FAIL until PHOT path publishes family-level traces.

- [ ] **Step 3: Write minimal implementation**
Hook `PhotSlotPredictor` / slot write path into `vpc_contract` without changing claim architecture.

- [ ] **Step 4: Run test to verify it passes**
Run the same command and confirm PASS.

- [ ] **Step 5: Commit**
```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/phot_predictor.rs /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs
git commit -m "feat: hook phot predictor into predictor contract"
```

### Task 3: Bring relation predictor under the shared contract

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`

- [ ] **Step 1: Write the failing tests**
Add tests asserting:
- relation family declares edge-local provenance
- warmup remains explicit and non-confident
- resolved relation publications carry family code and same-cycle alignment

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test relation_predictor_contract --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
Expected: FAIL until relation predictor writes family-level traces.

- [ ] **Step 3: Write minimal implementation**
Hook `refresh_global_relation_predictor` into the shared predictor-family trace substrate.

- [ ] **Step 4: Run test to verify it passes**
Run the same command and confirm PASS.

- [ ] **Step 5: Commit**
```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/viewport.rs /Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs
git commit -m "feat: hook relation predictor into predictor contract"
```

## Chunk 3: Observation, Snapshot, Hygiene

### Task 4: Expose family-level predictor traces through snapshot and observer

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs`
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`

- [ ] **Step 1: Write the failing tests**
Add tests asserting snapshot/observer show predictor-family traces as provenance only, not semantics.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test predictor_contract_snapshot --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
Expected: FAIL until snapshot/observer expose the traces.

- [ ] **Step 3: Write minimal implementation**
Reuse the current `vpc_contract` snapshot block or extend it minimally. Observer must remain a pure renderer.

- [ ] **Step 4: Run test to verify it passes**
Run the same command and confirm PASS.

- [ ] **Step 5: Commit**
```bash
git add /Users/joe/Documents/unnamed/bionic_v0/src/snapshot.rs /Users/joe/Documents/unnamed/bionic_v0/src/bin/observer.rs /Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs
git commit -m "feat: expose predictor-family traces in snapshot and observer"
```

### Task 5: Add hygiene and regression guards

**Files:**
- Modify: `/Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs`

- [ ] **Step 1: Write the failing hygiene tests**
Add source-backed tests that fail if the predictor contract path:
- reintroduces semantic or naming layers
- lets family publications overwrite observation truth
- makes warmup look like reliable confidence

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test predictor_contract_hygiene --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`
Expected: FAIL until wording and boundaries are finalized.

- [ ] **Step 3: Finalize wording and guards**
Clean up the implementation and lock the tests.

- [ ] **Step 4: Run the relevant regression suite**
Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml --test vpc_predictor_contract --test real_phot_adapter --test relation_predictor --test vpc_time_err_contract -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add /Users/joe/Documents/unnamed/bionic_v0/tests/vpc_predictor_contract.rs
git commit -m "test: add predictor contract hygiene guards"
```

---

Plan complete and saved to `docs/superpowers/plans/2026-04-22-vpc-predictor-contract.md`. Ready to execute?
