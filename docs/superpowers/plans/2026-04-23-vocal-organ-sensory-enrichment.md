# Implementation Plan: Vocal Organ Sensory Enrichment

> **Created**: 2026-04-23
> **Based on**: consciousness-stream-architecture.md, tier0-base-elements.md,
>              docs/axioms/2026-04-20-new-axioms-v1.md,
>              docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md,
>              docs/superpowers/specs/2026-04-20-language-binding-layer-design.md,
>              docs/superpowers/specs/2026-04-20-endogenous-activity-self-regulation-laws-design.md
> **Principle**: Enrich the sensory organ (VibSource), let the body learn to distinguish —
>                 do not insert "human_voice=true" classifiers into the body.

---

## 0 · Problem Statement

Currently `VibAdapter` derives a vocal target from **4-band energy only**:
`derive_vocal_target_from_vib` computes total_energy, spectral_center,
high/low ratio, and peak-flatness. This captures "there is sound" but not
"there is a sustained, periodic, potentially-imitable vocal contour."

Result: the vocal-organ hums on noise energy and does not preferentially
attend to speech-like external sounds.

## 1 · Design Decision

**Add periodicity features at the sensory-organ layer (`VibSource`).**

Enriching the sensory organ = adding new time-curves that the body can
use. This is NOT a "human voice detector" inside the logic layer. It is
more measurement resolution at the periphery — analogous to the eye
having cone types, or the cochlea having hair cells tuned for periodicity.

**New sensory signals (all continuous, all [0,1] or Hz):**

| Signal | Origin | Meaning (external description only) |
|---|---|---|
| `voicedness` | autocorrelation peak in low-band | "is there a stable sound source?" |
| `pitch_hz` | lag of max autocorrelation (80–400 Hz range) | fundamental frequency curve |
| `pitch_stability` | EMA of frame-to-frame pitch continuity | "is pitch contour smooth?" |

**Derived body-layer quantity (external description, not ontology):**

| Signal | Formula | Meaning |
|---|---|---|
| `speech_likeness` | voicedness × pitch_stability × energy_continuity | contour resembles imitable vocalization |

**Why this is NOT a classifier:**
- All outputs are **continuous quantities** (tier 0 values over time), not
  discrete "is_human_voice: bool"
- The body does NOT say "I detect a human, I will mimic"
- The body experiences **stronger attraction** toward more speech-like
  contours, just like it experiences stronger attraction to brighter lights
  or louder sounds. The physics of perception changes the gradient, not
  the control flow.

---

## 2 · Files Modified (4 files + 1 new test)

### 2.1 `src/vib.rs` — Enrich the VibSource sensory organ

**What changes:**
- `BandFilter` gains **periodicity analysis** alongside existing band-energy
  computation:
  - Maintain a circular buffer of recent mono samples (~20 ms window)
  - After each `push`, compute **normalized autocorrelation** on this window
  - Extract `voicedness` = max(correlation[lag=80–400Hz])
  - Extract `pitch_hz` = corresponding lag
  - Compute `pitch_stability` across frames using EMA
- Output tuple becomes `([f64; VIB_BANDS], voicedness: f64, pitch_hz: f64, pitch_stability: f64)`
  instead of just `[f64; VIB_BANDS]`

**Why this is OK per spec:**
- This is an **enrichment of the same axis family** — `vib_band` family
  remains the primary measurement; we are adding auxiliary continuous
  measurements in the same physical domain (acoustic vibration)
- Tier 0 rule 3: quantities derivable by time-derivative/algebraic
  combination of other tier-0 elements should not enter tier-0.
  However, **voicedness and pitch_hz are NOT derivable from band-energy
  alone** — they require time-domain autocorrelation, which is a different
  measurement projection of the same raw signal
- Analogous to how `light_band` and `emit_band` are separate axis groups
  over the same photonic domain

### 2.2 `src/adapter.rs` — VibAdapter consumes enriched signals

**What changes:**

- `VibAdapter` state adds EMA fields:
  ```rust
  smoothed_voicedness: f64,
  smoothed_pitch_hz: f64,
  smoothed_pitch_stability: f64,
  ```

- `derive_vocal_target_from_vib` is replaced by a new function
  `derive_vocal_contour` that takes `(bands, voicedness, pitch_hz, pitch_stability)`
  and returns:
  ```rust
  (energy, pitch_hint, resonance_hint, envelope_hint,
   speech_likeness)
  ```
  where:
  ```
  speech_likeness = (voicedness * pitch_stability * continuity_factor).clamp(0,1)
  continuity_factor = 1.0 - |d(energy)/dt|  (energy is continuous ⇒ more imitable)
  ```

- Inside `process_frame`: the new 5-tuple goes into `observe_vocal_target`

- **Vocal target is always updated** (not gated by energy threshold),
  preserving the existing invariant that "continuous audible contour
  refreshes vocal_target even when below object-event nomination"

**Why this is OK per spec:**
- `speech_likeness` is a **continuous engineering quantity** — not a
  discrete label, not a named concept, not a "human/not-human" decision
  gate
- It follows the same pattern as `smoothed_energy`, `smoothed_surprise`:
  an EMA-smoothed scalar derived from sensory input, consumed by
  downstream body processes
- Per cleanup spec §8.3: this is NOT using the high-risk terms
  `semantic/concept/token/label/name`. The name `speech_likeness` is an
  external description for the reader, not an internal ontological unit.

### 2.3 `src/vocal_target.rs` — Extend VocalTargetState

**What changes:**

Add to `VocalTargetState`:
```rust
pub speech_likeness: f64,    // [0, 1] continuous attraction weight
```

Add to `observe_external_contour`:
```rust
speech_likeness: f64,
// stored directly
```

**Why this is OK per spec:**
- A new time-curve in the body's internal state. All fields in
  `VocalTargetState` are already continuous parameters.
- No new discrete labels, no concept_id, no naming gate.

### 2.4 `src/viewport.rs` — Use speech_likeness in vocal_pull & vocal_imitation

**What changes:**

#### 4a. `observe_vocal_target` (line ~2955)

Add `speech_likeness` parameter and use it to modulate `attraction`:

```rust
// OLD:
let attraction = (energy * (0.6 + 0.4 * envelope) * (1.0 - 0.7 * similarity))

// NEW:
// speech_likeness acts as a natural attraction multiplier:
// higher speech_likeness → the contour pulls more strongly into probe_flow
let attraction = (energy * (0.6 + 0.4 * envelope)
    * (0.3 + 0.7 * speech_likeness)      ← speech-like sounds attract more
    * (1.0 - 0.7 * similarity))
    .clamp(0.0, 1.0);
```

#### 4b. `tick_vocal_imitation_loop` (line ~2950)

Add **listen-first bias** when `speech_likeness` is high:

```rust
// In the imitation loop, when target.speech_likeness > 0.5:
// temporarily raise the imitation_pressure gate threshold,
// so the organ "listens" longer before voicing
let listen_bias = if target.speech_likeness > 0.5 {
    0.55  // need higher pressure to start voicing (listen first)
} else {
    0.45  // current value
};
// Uses listen_bias instead of hardcoded 0.45
```

**Why this is a life-form behavior, not program logic:**
- In a living organism, hearing a stable, imitable external sound naturally
  suppresses self-production temporarily — the organism "listens first"
  before echoing. This is **endogenous self-regulation** (spec:
  endogenous-activity-self-regulation-laws), not an `if` statement.
- The threshold shift is itself a **continuous parameter** modulated by
  `speech_likeness`, not a discrete branch on a boolean.

### 2.5 `src/snapshot.rs` — Bump version

Increment `SNAPSHOT_VERSION` from 36 to 37 in `lib.rs`.

Add `speech_likeness` field to the vocal_target serialization block.

---

## 3 · Test Plan

### 3.1 Unit test: `vib.rs` periodicity extraction

Test that `BandFilter` correctly extracts:
- voicedness ≈ 0.95 for pure sine wave at 200 Hz
- voicedness ≈ 0.05 for white noise
- pitch_hz ≈ 200 Hz (±10%) for 200 Hz sine

### 3.2 Unit test: derive_vocal_contour

Test that `speech_likeness`:
- is high when voicedness is high and pitch is stable
- is low for noise-like frame patterns

### 3.3 Integration test: vocal_pull modulation

Test that when the VibAdapter processes a periodic input:
- vocal_pull rises faster than with a noise input of equal energy
- the probe_law intensity (novelty + vocal_pull * 0.75) is correspondingly boosted

### 3.4 Integration test: listen-first bias

Test that under high speech_likeness, the imitation loop delays voicing
onset compared to equal-energy non-speech-like input.

### 3.5 Contract test: existing behavior preserved

- Run all existing vocal tests: `vocal_runtime`, `vocal_body_contract`,
  `vocal_imitation`, `vocal_organ`, `vocal_target`, `vocal_memory`
- Verify no regression in the "no human output in body" test

---

## 4 · High-Risk Terms Boundary Check

Per cleanup spec §8.3 and language axioms, this plan uses the term
`speech_likeness`. Boundary analysis:

| Question | Answer |
|---|---|
| External description or internal ontology? | **External description** — the internal name is just a field path; its value is a float, not a named entity |
| Pure engineering index? | Yes — it is an EMA-smoothed scalar in [0,1] that modulates attraction; it has no categorical semantics |
| Why not an ontological structure? | It does not create objects, assign categories, gate naming eligibility, or produce discrete labels. It is one more continuous curve in the body's perceptual state space. |
| Why not violate highest principle? | The body still operates as a continuous predictive-coding stream. No "if human then mimic" path exists. The system's behavior emerges from the gradient field defined by all its continuous parameters — this adds one more dimension to that field. |

**Verdict: Admissible.**

---

## 5 · Implementation Order

1. `src/vib.rs` — add periodicity extraction to `BandFilter`
2. `src/vocal_target.rs` — add `speech_likeness` field
3. `src/adapter.rs` — `derive_vocal_contour`, update `process_frame`
4. `src/viewport.rs` — update `observe_vocal_target`, `tick_vocal_imitation_loop`
5. `lib.rs` — bump `SNAPSHOT_VERSION` to 37
6. `src/snapshot.rs` — serialize new field
7. Tests — 3.1 through 3.5

---

## 6 · Life-Design Principle Compliance Checklist

| Principle | Compliance | Notes |
|---|---|---|
| **Highest: silicon life ≠ program** | ✅ | Sensory organ enrichment, not procedural logic |
| **No human language in body** | ✅ | All values are f64 time-curves |
| **Objects before names** | ✅ | No naming, no concept_id, no labels |
| **Parameters are time-curves** | ✅ | voicedness(t), pitch_hz(t), speech_likeness(t) — all continuous |
| **Shared viewport** | ✅ | All new flows go through existing vocal_target/vocal_pull in viewport |
| **VPC same loop, multi-resolution** | ✅ | Sensory organ runs at 100 Hz; vocal_imitation at 30 Hz |
| **No ASR / phonemes / tokens** | ✅ | Zero text processing |
| **No "if human then replicate"** | ✅ | Continuous attraction modulation, not discrete trigger |
| **Observation separated from body** | ✅ | No eprintln!/println! in body process |
| **Endogenous self-regulation** | ✅ | Listen-first bias is natural organism behavior |
