//! Deterministic synthetic VIB source (Phase 3C test utility).
//!
//! Purpose: produce VIB band frames on demand, with a configurable phase
//! offset, so tests can create VIB events that are synchronized with
//! `SyntheticPhotSource`, desynchronized with it, or silent.
//!
//! Scope:
//!   * Test/example use only — the default `bin/body` binary continues to
//!     run the real microphone adapter. Nothing in this module touches
//!     `cpal` or the system audio layer.
//!   * Low-level numeric output only. No labels, no object names.

use crate::VIB_BANDS;

/// Deterministic VIB sample frame. Shaped like a real mic frame
/// (`[f64; VIB_BANDS]`) so downstream predictor and adapter code paths are
/// identical to the live path.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticVibFrame {
    pub t_us: u64,
    pub bands: [f64; VIB_BANDS],
}

/// Deterministic synthetic VIB source.
///
/// Output: a sum of sinusoids in each band plus a periodic "event burst"
/// that raises smoothed energy above `EVENT_NOMINATION_THRESHOLD`. The burst
/// period (`burst_period_us`) and offset (`phase_offset_us`) let tests
/// produce sync / desync / silent scenarios against `SyntheticPhotSource`.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticVibSource {
    start_t_us: u64,
    phase_offset_us: u64,
    burst_period_us: u64,
    burst_width_us: u64,
    /// If `false`, `sample()` emits only quiet background noise.
    active: bool,
}

impl SyntheticVibSource {
    /// An always-active source whose bursts start at `start_t_us` and repeat
    /// every `burst_period_us`, each burst lasting `burst_width_us`.
    pub fn new(start_t_us: u64, burst_period_us: u64, burst_width_us: u64) -> Self {
        Self {
            start_t_us,
            phase_offset_us: 0,
            burst_period_us,
            burst_width_us,
            active: true,
        }
    }

    /// Add a phase offset — a positive offset delays the first burst by
    /// `us`. Useful for desync tests.
    pub fn with_phase_offset(mut self, us: u64) -> Self {
        self.phase_offset_us = us;
        self
    }

    /// Disable bursts entirely. The source still produces quiet background
    /// noise so band values are well-defined.
    pub fn silent(mut self) -> Self {
        self.active = false;
        self
    }

    /// Produce one band frame for the given absolute timestamp. Deterministic.
    pub fn sample(&self, t_us: u64) -> SyntheticVibFrame {
        let mut bands = [0.0_f64; VIB_BANDS];
        // Cheap background noise — enough to keep observations non-zero but
        // well below the event threshold.
        let t_s = (t_us.saturating_sub(self.start_t_us)) as f64 * 1e-6;
        for axis in 0..VIB_BANDS {
            let freq = 3.0 + axis as f64 * 2.0;
            let phase = 2.0 * std::f64::consts::PI * freq * t_s;
            bands[axis] = 0.0005 * phase.sin();
        }
        if !self.active {
            return SyntheticVibFrame { t_us, bands };
        }

        // Burst phase — t_us relative to start, minus phase offset.
        let rel = t_us
            .saturating_sub(self.start_t_us)
            .saturating_sub(self.phase_offset_us);
        let cycle_pos = rel % self.burst_period_us.max(1);
        if cycle_pos < self.burst_width_us {
            // Raise all bands well above threshold. Shape as a triangle pulse
            // so smoothed energy ramps rather than jumping, mimicking live
            // mic bursts.
            let frac = cycle_pos as f64 / self.burst_width_us.max(1) as f64;
            let tri = if frac < 0.5 {
                frac * 2.0
            } else {
                (1.0 - frac) * 2.0
            };
            let amp = 0.12 * tri;
            for axis in 0..VIB_BANDS {
                bands[axis] += amp * (1.0 + 0.2 * axis as f64);
            }
        }
        SyntheticVibFrame { t_us, bands }
    }
}
