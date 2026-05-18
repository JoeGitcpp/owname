# External Curriculum Training Material Registry v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an observer-side material registry and principle review for external curriculum training, so future governors can choose disturbance material without inventing body-side ontology.

**Architecture:** Create a focused `curriculum_material` module with numeric kind codes, deterministic material entries, an audit function, and a report mode. It does not run training, mutate `Viewport`, or touch body output. Existing curriculum stages remain the consumers for later work.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` observer binary, Superpowers TDD/verification workflow.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-external-curriculum-training-material-registry-v0-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-cross-channel-disturbance-coupling-v0-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-20-language-binding-layer-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no `semantic_readiness`, `symbolizer`, `concept_id`, token ids, word ids,
  object names, text-understanding fields, naming gates, meaning DB, parser,
  tokenizer, dictionary, image labels, voice labels, identity, owner, ethics,
  or personality;
- no prompt/answer training pairs;
- no source-trust or source-owner list;
- no material entry that claims what a disturbance means.

## Files

- Create: `bionic_v0/src/curriculum_material.rs`
  - Numeric material origin, carrier, and stage kind enums.
  - `CurriculumMaterialEntry`
  - `ExternalCurriculumTrainingPlan`
  - `ExternalCurriculumTrainingPlanReview`
  - `default_external_curriculum_training_plan`
  - `review_external_curriculum_training_plan`
- Modify: `bionic_v0/src/lib.rs`
  - Export `curriculum_material`.
- Create: `bionic_v0/tests/external_curriculum_material_registry.rs`
  - Acceptance and hygiene tests.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--external-curriculum-plan`.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add observer report coverage.
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-external-curriculum-training-material-registry-v0-evidence.md`

## Task 1: RED Test For Material Plan

**Files:**

- Create: `bionic_v0/tests/external_curriculum_material_registry.rs`

- [ ] **Step 1: Write failing acceptance test**

Expected API:

```rust
use bionic_v0::curriculum_material::{
    default_external_curriculum_training_plan, review_external_curriculum_training_plan,
};

#[test]
fn default_external_curriculum_plan_covers_required_material_origins_carriers_and_stages() {
    let plan = default_external_curriculum_training_plan();
    let review = review_external_curriculum_training_plan(&plan);

    assert!(review.material_groups >= 8);
    assert!(review.synthetic_groups >= 3);
    assert!(review.replay_groups >= 1);
    assert!(review.probe_return_groups >= 1);
    assert!(review.live_ingress_groups >= 1);
    assert!(review.carrier_kinds >= 5);
    assert!(review.stage_kinds >= 7);
    assert_eq!(review.body_touch_points, 0);
    assert_eq!(review.missing_required_roles, 0);
    assert_eq!(review.principle_review_pass, 1);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry default_external_curriculum_plan_covers_required_material_origins_carriers_and_stages
```

Expected: unresolved module or function.

## Task 2: Guard Test

**Files:**

- Modify: `bionic_v0/tests/external_curriculum_material_registry.rs`

- [ ] **Step 1: Add hygiene test**

Scan `src/curriculum_material.rs` for rejected ontology and body output terms.

- [ ] **Step 2: Verify guard**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry
```

Expected after implementation: pass.

## Task 3: Implement Registry

**Files:**

- Create: `bionic_v0/src/curriculum_material.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Implement minimal enums and structs**

Use `#[repr(u8)]` enums and numeric-only fields.

- [ ] **Step 2: Implement default plan**

Create at least eight material groups covering:

- synthetic mark, vib, phot, and coupled groups;
- replay snapshot/trace group;
- probe-return framed byte group;
- deferred live ingress mark/vib/phot groups.

- [ ] **Step 3: Implement review**

Count origin kinds, carrier kinds, stage kinds, reproducible groups,
observer-only groups, body touch points, and missing required roles.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry
```

Expected: all tests pass.

## Task 4: Report Mode

**Files:**

- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add failing report test**

Expect `--external-curriculum-plan` to print:

- `mode=external_curriculum_plan`
- `material_groups=`
- `synthetic_groups=`
- `replay_groups=`
- `probe_return_groups=`
- `live_ingress_groups=`
- `carrier_kinds=`
- `stage_kinds=`
- `missing_required_roles=`
- `principle_review_pass=`

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report external_curriculum_plan_report_emits_material_and_principle_review
```

Expected: fail until report mode is wired.

- [ ] **Step 3: Wire report mode**

Import the registry functions, parse the flag, and print numeric observer
evidence.

- [ ] **Step 4: Verify GREEN**

Run the same report test again.

## Task 5: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-external-curriculum-training-material-registry-v0-evidence.md`

- [ ] **Step 1: Run report**

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --external-curriculum-plan
```

- [ ] **Step 2: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test external_curriculum_material_registry --test curriculum_report
```

- [ ] **Step 3: Run full verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

- [ ] **Step 4: Record evidence**

Record RED/GREEN, report numbers, full verification, and any pre-existing
formatting issue.
