// Per-region inverse-depth (ρ) observer.
//
// Maintains a per-spatial-region estimate of scene depth via an incremental
// local update rule driven by efference copy (known self-motion) and observed
// image displacement.  No backprop; no global optimization; fully local.
//
// Update rule:
//   ρ ← ρ + lr · (d_obs − tt · ρ) · tt
//
// Where:
//   ρ       = inverse depth (1/z); higher = closer
//   d_obs   = observed pixel displacement proxy (VIS_SUB_MOTION axis value)
//   tt      = lateral translation term from efference copy
//
// When tt = None (still camera or pure rotation), the region is marked
// unobservable and ρ is NOT updated — depth information is undefined.
//
// This is phase A: depth from known self-motion.  Phase B will connect the
// per-region ρ to the visual prediction substrate as an internal signal.

use std::sync::Arc;

use crate::viewport::Viewport;
use crate::VIS_REGIONS;

/// Minimum |tt| below which depth observability is skipped even inside the
/// update function.  Prevents division-by-near-zero in the gradient step.
const DEPTH_MIN_TT: f64 = 1e-6;

/// Hard bounds on inverse-depth to prevent unbounded drift.
/// ρ ∈ [DEPTH_RHO_MIN, DEPTH_RHO_MAX] covers depths roughly 0.01..100 metres
/// for a scene measured in normalised units.
pub const DEPTH_RHO_MIN: f64 = 0.01;
pub const DEPTH_RHO_MAX: f64 = 100.0;

/// Default learning rate for the depth update step.
pub const DEPTH_DEFAULT_LR: f64 = 0.1;

/// Per-region inverse-depth state.
#[derive(Clone, Debug)]
pub struct DepthObserver {
    /// Per-region inverse depth estimate ρ = 1/z.  Initialised to 1.0
    /// (unit depth) — a neutral prior with no bias toward near or far.
    pub inv_depth: [f64; VIS_REGIONS],
    /// Whether depth was observable on the most-recent tick.
    /// False when tt = None (still or rotating).
    pub observable: [bool; VIS_REGIONS],
    pub lr: f64,
}

impl Default for DepthObserver {
    fn default() -> Self {
        Self {
            inv_depth: [1.0; VIS_REGIONS],
            observable: [false; VIS_REGIONS],
            lr: DEPTH_DEFAULT_LR,
        }
    }
}

impl DepthObserver {
    /// Update the depth estimate for `region` given an observed image
    /// displacement proxy `obs_disp` and the efference translation term `tt`.
    ///
    /// * `obs_disp` — brightness-change proxy for pixel displacement, as
    ///   produced by the `VIS_SUB_MOTION` axis (in `[0,1]`).
    /// * `tt` — lateral translation from `EfferenceCopy::translation_term()`.
    ///   `None` marks the region unobservable and skips the update.
    pub fn update(&mut self, region: usize, obs_disp: f64, tt: Option<f64>) {
        if region >= VIS_REGIONS {
            return;
        }
        match tt {
            None => {
                self.observable[region] = false;
            }
            Some(tt) if tt.abs() < DEPTH_MIN_TT => {
                self.observable[region] = false;
            }
            Some(tt) => {
                self.observable[region] = true;
                let rho = self.inv_depth[region];
                let err = obs_disp - tt * rho;
                let new_rho = rho + self.lr * err * tt;
                self.inv_depth[region] = new_rho.clamp(DEPTH_RHO_MIN, DEPTH_RHO_MAX);
            }
        }
    }
}

impl Viewport {
    /// Update all per-region depth estimates from the current efference copy
    /// and observed per-region motion values.
    ///
    /// `obs_motion` must have length `VIS_REGIONS`; each element is the
    /// `VIS_SUB_MOTION` axis value for that region (brightness-change proxy
    /// for pixel displacement).
    pub fn update_depth_from_efference(&self, obs_motion: &[f64; VIS_REGIONS]) {
        let ec = self.read_efference_copy();
        let tt = ec.translation_term();
        let mut depth = self.depth_observer.write().unwrap();
        for region in 0..VIS_REGIONS {
            depth.update(region, obs_motion[region], tt);
        }
    }

    /// Read a snapshot of the current depth observer state.
    pub fn read_depth_observer(&self) -> DepthObserver {
        self.depth_observer.read().unwrap().clone()
    }
}

/// Per-tick depth update entry point.
///
/// Pulls the current efference copy from the viewport, reads per-region motion
/// observations, and runs one depth-observer update step.
///
/// `obs_motion[r]` = current VIS_SUB_MOTION axis value for region `r`.
pub fn tick_depth_observer(vp: &Arc<Viewport>, obs_motion: &[f64; VIS_REGIONS]) {
    vp.update_depth_from_efference(obs_motion);
}
