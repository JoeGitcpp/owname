// Efference copy substrate — body's self-motion awareness.
//
// An efference copy is the corollary discharge of a motor command: the body's
// internal copy of "what movement I just commanded."  This module stores the
// most-recently-commanded camera translation so that the visual prediction
// substrate can condition on it (B1 external_prior pattern: input only, no
// coefficient update).
//
// Only LATERAL translation (tx) generates parallax and makes depth observable.
// Pure rotation, forward/backward motion (tz), and stillness all yield
// translation_term = None → depth marked unobservable for that tick.

use std::sync::Arc;

use crate::viewport::Viewport;
use crate::{vis_axis, VIS_REGIONS, VIS_SUB_MOTION};

/// Minimum lateral displacement |tx| (in normalised frame-width units) for
/// depth to be considered observable.
///
/// The value is deliberately tiny — its purpose is only to guard against
/// floating-point near-zero in the depth update denominator.  The sine-wave
/// curriculum starts at amplitude 0.002 with period T=60 ticks, giving a
/// peak Δx ≈ 0.002 × 2π/60 ≈ 0.00021.  EFFERENCE_MIN_TT must be smaller
/// than this so that even the very first curriculum phase produces observable
/// efference copies.
pub const EFFERENCE_MIN_TT: f64 = 0.0001;

/// The commanded camera translation for one tick — the "efference copy."
///
/// All coordinates are normalized: tx = 1.0 means one full frame-width of
/// lateral displacement per tick.  Only tx drives parallax; ty and tz are
/// carried for completeness but currently yield translation_term = None.
#[derive(Clone, Copy, Debug, Default)]
pub struct EfferenceCopy {
    /// Horizontal (X) component — drives parallax and depth observability.
    pub tx: f64,
    /// Vertical (Y) component — carried but not used for depth this phase.
    pub ty: f64,
    /// Forward (Z) component — no parallax; does not make depth observable.
    pub tz: f64,
    pub t_us: u64,
}

impl EfferenceCopy {
    /// The "translation term" (tt) used in the depth-update equation:
    ///   ρ ← ρ + lr · (d_obs − tt · ρ) · tt
    ///
    /// Returns `Some(|tx|)` when lateral motion is above `EFFERENCE_MIN_TT`.
    /// Returns `None` when the camera is still or performing pure rotation —
    /// the depth observer must mark the region as unobservable and skip the
    /// update.
    #[inline]
    pub fn translation_term(&self) -> Option<f64> {
        let tt = self.tx.abs();
        if tt >= EFFERENCE_MIN_TT {
            Some(tt)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EfferenceCopyState {
    pub current: EfferenceCopy,
}

impl Viewport {
    /// Write the latest commanded translation into the efference substrate.
    pub fn push_efference_copy(&self, ec: EfferenceCopy) {
        *self.efference_copy.write().unwrap() = EfferenceCopyState { current: ec };
    }

    /// Read the most-recently-pushed efference copy.
    pub fn read_efference_copy(&self) -> EfferenceCopy {
        self.efference_copy.read().unwrap().current
    }

    /// Inject efference-based motion priors for all visual-region motion axes.
    ///
    /// For each region `r`:
    ///   expected_motion = tt · ρ_r
    /// where `tt` is the lateral translation term and `ρ_r` is the region's
    /// current inverse-depth estimate.
    ///
    /// Injects via `push_prior` so the AR-2 predictor on each VIS_SUB_MOTION
    /// axis sees the expectation as an INPUT prior (B1 pattern: prior is added
    /// to the prediction but is deliberately excluded from the a/b coefficient
    /// update — no cross-layer backprop).
    ///
    /// When translation_term is None (still camera), all motion priors are set
    /// to 0.0 so predictors do not wrongly expect motion.
    pub fn inject_efference_motion_priors(&self) {
        let ec = self.read_efference_copy();
        match ec.translation_term() {
            None => {
                for region in 0..VIS_REGIONS {
                    self.push_prior(vis_axis(region, VIS_SUB_MOTION), 0.0);
                }
            }
            Some(tt) => {
                let depth = self.depth_observer.read().unwrap();
                for region in 0..VIS_REGIONS {
                    let rho = depth.inv_depth[region];
                    let expected = (tt * rho).clamp(0.0, 1.0);
                    self.push_prior(vis_axis(region, VIS_SUB_MOTION), expected);
                }
            }
        }
    }
}

/// Convenience: drive the full efference→depth→prior pipeline in one call.
///
/// Call order per tick:
///   1. `push_efference_copy` (already done by caller before this)
///   2. `inject_efference_motion_priors` — priors set before predictor ticks
///   3. After observations are pushed: `update_depth_from_efference`
///
/// This helper covers step 2 and is the preferred entry point for the body
/// or test harness.
pub fn tick_efference_priors(vp: &Arc<Viewport>) {
    vp.inject_efference_motion_priors();
}
