# Parallel Focus Field + Active Aperture Echo v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the pressure field from many waves into parallel retained focus traces with a single active aperture and bounded secondary echo pressure.

**Architecture:** Add `active_surface.rs` as a numeric substrate over `prediction_pressure_field`. Store `ActiveSurfaceState` in `Viewport`, refresh it in body after pressure-field hydration and before probe law, and let probe law fold aperture + bounded echo pressure into drive.

**Tech Stack:** Rust crate `bionic_v0`, cargo tests, observer-side `curriculum_report`, existing `Viewport`, `prediction_pressure_field`, `probe_law`, and `action_output` substrates.

---

## Files

- Create: `docs/superpowers/specs/2026-05-18-parallel-focus-field-active-aperture-echo-v0-design.md`
- Create: `docs/superpowers/plans/2026-05-18-parallel-focus-field-active-aperture-echo-v0.md`
- Create: `bionic_v0/src/active_surface.rs`
- Create: `bionic_v0/tests/active_surface_aperture_echo.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/probe_law.rs`
- Modify: `bionic_v0/src/curriculum.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

## Task 1: Failing Active Surface Tests

- [ ] **Step 1: Write tests first**

Add `bionic_v0/tests/active_surface_aperture_echo.rs` covering:

- three or more focus traces coexist,
- aperture count is one,
- echo exists and is bounded below aperture pressure,
- dominant focus enters aperture,
- non-aperture traces are retained,
- suppressed trace can reenter after quiet rehydration,
- closure reduces aperture trace without deleting all traces,
- event pressure can displace aperture and displaced trace retains residue,
- probe drive uses aperture plus echo,
- aperture switches stay bounded under stress,
- body source order is pressure field -> active surface -> probe law,
- source guard rejects forbidden architecture terms.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml active_surface_aperture_echo --test active_surface_aperture_echo
```

Expected: FAIL because `active_surface` does not exist yet.

## Task 2: Implement Active Surface Substrate

- [ ] **Step 1: Add module and state**

Create `bionic_v0/src/active_surface.rs` with:

- `ActiveFocusTrace`
- `ActiveSurfaceState`
- `refresh_active_surface`
- `apply_active_surface_closure`
- `read_active_surface`
- helper methods for aperture count, echo bound, and effective probe pressure.

- [ ] **Step 2: Add `Viewport` storage**

Modify `bionic_v0/src/viewport.rs`:

- import `ActiveSurfaceState`,
- add `pub active_surface: RwLock<ActiveSurfaceState>`,
- initialize it in `Viewport::new()`.

- [ ] **Step 3: Expose module**

Modify `bionic_v0/src/lib.rs`:

```rust
pub mod active_surface;
```

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml active_surface_aperture_echo --test active_surface_aperture_echo
```

Expected: active surface unit tests pass except report wiring if not yet added.

## Task 3: Body and Probe Coupling

- [ ] **Step 1: Wire body order**

Modify `bionic_v0/src/bin/body.rs`:

```rust
vp.tick_prediction_pressure_field_from_substrate(t);
vp.refresh_active_surface(t);
vp.refresh_probe_law(t);
```

- [ ] **Step 2: Fold surface pressure into probe law**

Modify `bionic_v0/src/probe_law.rs`:

- read `active_surface`,
- compute `aperture_pressure + echo_pressure * 0.35`,
- use it as the preferred field drive when traces exist,
- keep raw pressure field as fallback when active surface has no traces.

- [ ] **Step 3: Re-run focused tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml active_surface_aperture_echo --test active_surface_aperture_echo
```

## Task 4: Numeric Report

- [ ] **Step 1: Write report test first**

Add `active_surface_aperture_echo_report_emits_numeric_evidence` to
`bionic_v0/tests/curriculum_report.rs`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml active_surface_aperture_echo_report_emits_numeric_evidence --test curriculum_report
```

Expected: FAIL until the report flag exists.

- [ ] **Step 3: Implement evidence and report flag**

Add `ActiveSurfaceApertureEchoEvidence` and
`run_active_surface_aperture_echo_curriculum` in `bionic_v0/src/curriculum.rs`.

Update `bionic_v0/src/bin/curriculum_report.rs` to parse and print:

```text
--active-surface-aperture-echo
```

- [ ] **Step 4: Verify report**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml active_surface_aperture_echo_report_emits_numeric_evidence --test curriculum_report
cargo run --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --active-surface-aperture-echo
```

## Task 5: Full Verification

- [ ] **Step 1: Format and diff checks**

Run:

```bash
cargo fmt --manifest-path bionic_v0/Cargo.toml --check
git diff --check
```

- [ ] **Step 2: Full suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: all tests pass. Existing unrelated warnings may remain warnings only.
