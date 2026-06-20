# bionic_v0 Global Architecture Map

> Status: updated control review baseline on 2026-05-06 after integrating the
> completed worktrees into `replace-by-filament`.
>
> 2026-06-06: appended §6 (Representation Path & Load-Bearing Acceptance — Route A)
> recording the non-pixel / perturbation-field charter and the ①/②/③ acceptance
> rubric, with current attainment from the 2026-06-06 code trace.

## 0. Boundary Notes

This document is an external engineering map. File names such as
`language_binding.rs`, `code_ingress.rs`, `framed_byte_egress.rs`,
`evidence_tension.rs`, and `adaptive_stability.rs` are engineering indices, not
internal ontology units.

High-risk words such as `semantic`, `concept`, `token`, `label`, `name`,
`identity`, `owner`, and `personality` appear here only to describe forbidden
drift or external review categories. They are not admissible body-side facts.
The body remains numeric, temporal, curve-based, and observer-separated.

## 1. Integrated Flow

```text
ordinary ingress
  -> ambient / slot claim / object slots / fusion / spatial
  -> predictor family / relation predictor / reasoning / reasoning history
  -> probe law / evidence tension / adaptive stability
  -> action output
       -> vocal organ / vocal adapter
       -> framed byte egress
  -> ordinary ingress consequence windows
  -> action-consequence pairs
  -> language binding events only when pair evidence is strong enough
  -> snapshot v64
  -> observer

computer-native byte ingress
  -> code ingress extraction buffer (CodeIngressState — derived, not independent)
  -> tier-0 CODE axes (CODE_POS / CODE_SPAN / CODE_STRUCT_ORDER_*) via push_observation
       ← single authoritative representation of digital objects
  -> evidence tension (reads CODE axes, not buffer)
  -> snapshot / observer (display mirror of buffer + axes; no decision path)

birth sensitivity and learned bias
  -> numeric update-pressure curves
  -> learned delta summary
  -> snapshot / observer
```

Observer output is downstream only. It must not request body fields that would
turn observations into body ontology.

## 2. Current Completion Matrix

| Area | Status | Engineering landing | Evidence |
|---|---|---|---|
| Cleanup boundary | Complete | `body.rs`, `viewport.rs`, `snapshot.rs`, `observer.rs`, cleanup tests | rejected semantic-readiness / symbolizer / concept-id path remains absent |
| Perception ingress | Complete for current v0 | `adapter.rs`, `vib.rs`, `phot_camera.rs`, `phot_predictor.rs` | VIB/PHOT adapter, health, stale, multi-event, and predictor tests pass |
| Object / relation / reasoning substrate | Complete for current v0 | `viewport.rs`, `object_high_layer.rs`, `surface_*`, relation and reasoning state | object, relation, surface, reasoning, and history tests pass |
| VPC timing and error contract | Complete | `vpc.rs`, `snapshot.rs`, observer trace | VPC contract tests pass |
| Action output | Complete as substrate | `action_output.rs`, `probe_law.rs`, `viewport.rs` | action-output, probe, and live preflight tests pass |
| Action consequence | Complete as substrate | `action_consequence.rs`, snapshot / observer | 45 action-consequence tests pass |
| Vocal runtime bridge | Complete as body-side execution path | `vocal_target.rs`, `vocal_organ.rs`, `vocal_adapter.rs`, `vocal_imitation.rs`, `vocal_memory.rs`, `body.rs` | vocal runtime and body contract tests pass |
| Live I/O preflight | Complete as deterministic substrate proof | action output + vocal + ingress + consequence + observer | live preflight tests pass; physical hardware availability remains an environment finding |
| Code ingress | Complete — single representation enforced | `code_ingress.rs` (extraction buffer, declared non-independent), `code_ingress_adapter.rs`, `viewport.rs::tick_evidence_tension_from_code_ingress` (reads CODE axes), snapshot / observer | code ingress tests pass; digital_object_predictable_repr_lands_on_code_axes guard added |
| Framed byte egress | Complete as minimal digital write-out substrate | `framed_byte_egress.rs`, action output, consequence, snapshot / observer | framed-byte egress tests pass |
| Evidence pressure | Contracted to generic pressure substrate | `substrate_pressure.rs`, `evidence_tension.rs`, `probe_law.rs` | contraction and evidence-tension tests pass |
| Learned bias persistence | Contracted to generic pressure-axis sediment only | `learned_bias.rs`, snapshot summary | learned-bias and contraction tests pass |
| Adaptive route dynamics | Contracted to numeric route traces only | `adaptive_stability.rs`, snapshot / observer | adaptive-stability and contraction tests pass |
| Pattern binding events | Complete as pair-evidence binding event path | `language_binding.rs`, action-consequence pair evidence | language binding runtime tests pass |
| Active gap observer evidence | Complete as numeric observer trace | `active_reasoning.rs`, snapshot / observer session rows | active-gap observer tests pass |
| Self-development loop | Complete as numeric predictive life substrate | `self_development.rs`, `viewport.rs`, `body.rs`, snapshot / observer | self-development and pre-language growth tests pass |
| Snapshot / observer | Complete for integrated v48 layout | `snapshot.rs`, `observer.rs` | full cargo test passes |

## 3. Relationship Notes

`viewport` is still the shared state substrate. New systems were integrated as
subtrees under this shared substrate rather than point-to-point pipelines.

`action_output` is the common write epoch ledger. Vocal output and framed byte
egress both create or adjust write epochs before their consequences are allowed
to pair back through ordinary ingress.

`action_consequence` is the common return structure. It records windows and
pairs, not goals, success labels, failure labels, source ownership, or identity.

`probe_law` now reads both reasoning novelty and unresolved evidence pressure.
It remains a numeric pressure/phase substrate, not a curiosity module and not a
text reasoning layer.

`evidence_tension` is downstream of generic pressure samples and upstream of
probe pressure. It records unresolved pressure and closure support only.

`language_binding` is downstream of pair evidence. It can form, reinforce, or
weaken binding traces, but it does not decide object existence and does not
introduce a naming readiness gate.

`learned_bias` changes only generic pressure-axis deltas and summary traces.
Base birth sensitivity remains separate from learned sediment.

`adaptive_stability` is route-local. It can raise numeric degradation,
recovery, and compression pressures for a route without rewriting neighboring
routes or restoring an artificial template.

`snapshot` v48 appends the newer substrates after earlier sections. The current
layout keeps the already-integrated v37 code ingress, v38 learned-bias summary,
v39 adaptive stability, v40 evidence tension, v41 framed-byte egress, v42
action-consequence pair sequence order, and v43 long-run cumulative closure
counters. v44 records write epoch start magnitude for closed action-consequence
evidence. v45 splits cumulative pair counts by consequence-return modality so
long-run byte-closure training can be read without conflating unrelated return
paths. v46 appends active-gap pressure, closure, re-entry, cycle count, and
trace samples for observer-side long-run curves. v47 appends
self-development predictive-loop evidence. v48 appends pre-language solitary
growth evidence so observer-side reports can distinguish useful recurrence
from mechanical repetition.

## 4. Remaining Hardening Items

1. Live hardware feedback is still environment-dependent. Deterministic tests
   prove the substrate path; actual microphone/camera/audio device evidence must
   be reported as observer-side capability, not body-side semantic failure.
2. Framed byte egress has a body-side framed-byte trace and return loop, but no
   external carrier policy beyond the minimal substrate contract.
3. Snapshot compatibility is intentionally current-version only. If long-lived
   snapshot archives matter, add explicit migration readers rather than
   weakening the body schema.

## 5. Verification Baseline

Fresh verification on 2026-05-06:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Result: all unit tests, integration tests, and doc tests passed.

## 6. Representation Path & Load-Bearing Acceptance — Route A (非像素 / 扰动场)

> Appended 2026-06-06. Theory charter for "Route A pure emergence" plus the
> acceptance rubric used to reconcile design-vs-reality each build. Sources:
> `2026-06-01-unified-perception-substrate-redesign.md`,
> `2026-06-01-life-state-honest-assessment.md`, `2026-06-06` code trace.
>
> Per §0: the criteria below are an **external acceptance/review rubric**. Terms
> such as `identity`, `recognition`, `object`, `word↔object` name review
> categories and forbidden-drift checks, not admissible body-side ontology. The
> body stays numeric, temporal, curve-based, observer-separated.

### 6.1 Principle — understanding as homomorphism (同态), not pixel isomorphism (同构)

The body does not reconstruct a pixel picture to "understand," and is not required
to. Understanding requires a **homomorphism (同态)** to the world — preservation of
the task-relevant relations/invariants needed for prediction and action — NOT an
**isomorphism (同构)** that copies the world pixel-for-pixel. A picture is a human
*output format*, not the substrate of understanding (brains carry edges,
orientations, motion, prediction and prediction-error — never a pixel buffer). For a
substrate whose native ontology is perturbation, the world is natively a *structure
of surprises*, not an image; forcing it into pixels is a lossy foreign re-encoding.

This is the charter for **Route A (pure emergence)**: slice perception by physical
quantity (like audio slices frequency bands) into one shared axis space, feed one
abstraction engine, and let shape / depth / identity **emerge from inter-region
relations** — build no recognizers at this layer. This deliberately avoids forced
**convergence** onto a human-shaped pixel representation (the listed risk).

Representation status: **on-path and load-bearing in code.** Camera frames collapse at
ingress into region × physical-quantity axes (`phot_camera::region_perception`), share
the auditory axis space (`AXIS_VIS_BASE == VIB_BANDS`), and only error / surprise /
disturbance flow downstream (`motion = inter-frame Δ`; Gaussian field learns residual;
`PhotPrediction.err/surprise`; prediction pressure field = field of unresolved errors).
No pixel grid is stored or reconstructed.

### 6.2 Three load-bearing acceptance criteria

"Non-pixel" alone is not sufficient. For the alien representation to actually
*understand and align*, three conditions must hold **together**. "Internally coherent
logic" is the *result* of these three, never a substitute for them.

| # | Criterion (canonical) | What it requires | Current status | Evidence |
|---|---|---|---|---|
| ① | **同态够 · Structural sufficiency** | Homomorphism preserves enough world-structure (edges, surfaces, depth, occlusion, boundaries, individuation) for the task. Alien *form* is fine; thin *structure* is not. | **未达 / NOT MET** — coarse 2×2–4×4 region grid; fingerprint is 4-D vib only, ignores in-slot phot/spatial. Mechanism in place, content/thickness ~empty. Path correct (coarse→fine emergence + recursive depth design); threshold not reached. | `phot_camera.rs` region axes; `familiar_objects.rs:252`; `2026-06-02-recursive-predictive-coding-depth-emergence-design.md` |
| ② | **接地 · Grounding loop** | Representation pays rent via a closed predict→act→observe-error→update loop, not a free-floating self-consistent fantasy. | **部分 / PARTIAL** — perceptual predictive coding learns from sensory error and runs in body, not stubbed. Action→consequence loop closes onto **pressure/exploration + binding strength**, but behavioral error (`curve_similarity`) does **not** retrain predictor weights. | `predictor.rs` (AR-2 from `err`); `vpc_cross_layer.rs` (Widrow-Hoff from `meta_err`); `phot_gaussian.rs` (residual); `action_consequence.rs:355-471`; `evidence_tension.rs:42-68`; `body.rs:123` |
| ③ | **可质询接口 · Queryable interface** | Alien representation is bindable to / queryable through shared human referents, so alignment is *testable*, not assumed. | **机制在·referent缺 / MECHANISM PRESENT, REFERENT MISSING** — `language_binding` is real, evidence-driven (pattern↔target strength from pair `curve_similarity`), not templated, with developmental precision. But it is post-hoc/action-gated, and the key referent — cross-modal **joint attention** (seen-object + heard-sound → one slot) — is **not wired**; recognition is sound-only, so word↔object cannot yet bind. | `language_binding.rs:98-232`; self-assessment §4–5 |

### 6.3 Nearest bottleneck

①∩③ intersect at one organ: **cross-modal joint attention + true individuation**
(fingerprint reads slot phot/spatial, not vib-only; same-moment seen-object + heard-sound
bind to one slot). Landing it simultaneously thickens structure [fills ①] and gives the
interface a real referent to bind [fills ③]. Until then the path is "form correct,
alignment not yet startable." Asking "how long to train communication" is premature until
these organs connect.
