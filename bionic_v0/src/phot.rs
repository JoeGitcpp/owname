//! Synthetic PHOT substrate (Phase 3B).
//!
//! Purpose: exercise the `ambient.phot`, `slot.phot`, and `slot.spatial`
//! write paths without introducing a real camera or visual semantics. Outputs
//! are deterministic, low-level numeric fields only — no labels, no object
//! names, no tracking IDs.
//!
//! A single `SyntheticPhotSource` composes three simple patterns:
//!   * a bright blob that slides left-to-right across the normalized screen
//!   * a vertical pulse bar whose brightness swells with phase
//!   * a periodic full-field brightness swell that modulates all bands
//!
//! The three patterns are combined into four PHOT bands and a normalized 2D
//! spatial tuple. Downstream, the `SyntheticPhotAdapter` in `adapter.rs`
//! reads one frame per tick and writes it into the viewport.

use crate::PHOT_BANDS;

/// Deterministic PHOT sample. Layout is intentionally flat — the substrate
/// never exposes semantic labels or object hints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntheticPhotFrame {
    pub t_us: u64,
    pub phot_bands: [f64; PHOT_BANDS],
    pub pos_x: f64,
    pub pos_y: f64,
    pub span_x: f64,
    pub span_y: f64,
    pub spatial_precision: f64,
    pub event_energy: f64,
}

/// Deterministic synthetic PHOT source. Given the same `t_us`, `sample`
/// returns the same frame on repeated calls (pure function of inputs).
#[derive(Clone, Copy, Debug)]
pub struct SyntheticPhotSource {
    start_t_us: u64,
}

impl SyntheticPhotSource {
    pub fn new(start_t_us: u64) -> Self {
        Self { start_t_us }
    }

    /// Produce one frame for the given absolute timestamp. Deterministic.
    pub fn sample(&self, t_us: u64) -> SyntheticPhotFrame {
        // Treat pre-start as start so negative phase never appears.
        let delta_us = t_us.saturating_sub(self.start_t_us);
        let t_s = delta_us as f64 * 1e-6;

        // Phase progressions (seconds -> [0,1) cyclic via fract()).
        let blob_period_s = 4.0;
        let bar_period_s = 1.0;
        let swell_period_s = 6.0;

        let blob_phase = ((t_s / blob_period_s) % 1.0 + 1.0) % 1.0;
        let bar_phase = ((t_s / bar_period_s) % 1.0 + 1.0) % 1.0;
        let swell_phase = ((t_s / swell_period_s) % 1.0 + 1.0) % 1.0;

        // Moving blob: ping-pong across [0.1, 0.9].
        let tri = if blob_phase < 0.5 {
            blob_phase * 2.0
        } else {
            (1.0 - blob_phase) * 2.0
        };
        let pos_x = 0.1 + tri * 0.8;
        // Blob drifts slightly vertically so tests see non-trivial pos_y too.
        let pos_y = 0.5 + 0.1 * (swell_phase - 0.5);
        let span_x = 0.1;
        let span_y = 0.1;

        // Vertical pulse bar brightness (scalar in [0,1]).
        let bar_bright = 0.5 * (1.0 + cos_approx(2.0 * PI * bar_phase)).max(0.0);

        // Full-field swell (adds a DC-like component across all bands).
        let swell = 0.25 * (1.0 + cos_approx(2.0 * PI * swell_phase)).max(0.0);

        // Compose PHOT_BANDS. Band 0 tracks the blob's x position (spatial
        // cue), band 1 tracks the bar brightness, band 2 tracks the swell,
        // band 3 is a weighted mix to guarantee non-zero cross-term.
        let mut phot_bands = [0.0_f64; PHOT_BANDS];
        phot_bands[0] = 0.1 + 0.6 * pos_x + 0.2 * swell;
        if PHOT_BANDS > 1 {
            phot_bands[1] = 0.1 + 0.7 * bar_bright + 0.1 * swell;
        }
        if PHOT_BANDS > 2 {
            phot_bands[2] = 0.05 + 0.8 * swell;
        }
        if PHOT_BANDS > 3 {
            phot_bands[3] = 0.05 + 0.4 * bar_bright + 0.4 * swell + 0.1 * pos_x;
        }

        let event_energy: f64 = phot_bands.iter().map(|b| b * b).sum();
        // Precision rises with swell (adapter is most confident during peak
        // brightness). Keep strictly positive so callers can use the value
        // as an inverse-variance weight.
        let spatial_precision = 0.5 + 0.5 * swell;

        SyntheticPhotFrame {
            t_us,
            phot_bands,
            pos_x,
            pos_y,
            span_x,
            span_y,
            spatial_precision,
            event_energy,
        }
    }
}

// Tiny trig helpers — we avoid `f64::cos` only to stay independent of
// platform libm determinism across hosts; `f64::cos` is already fine, so we
// use it directly here. (Function kept behind a name so we can swap later.)
const PI: f64 = std::f64::consts::PI;

fn cos_approx(x: f64) -> f64 {
    x.cos()
}
