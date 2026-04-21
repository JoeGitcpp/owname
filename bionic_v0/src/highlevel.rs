use std::sync::Arc;

use crate::viewport::{HighLevelState, Viewport};
use crate::VIB_BANDS;

const REF_MSE: f64 = 1e-5;
const PRECISION_MIN: f64 = 0.2;
const PRECISION_MAX: f64 = 8.0;

/// Per-band AR-1 predictor on each band's MSE.
/// Reads each band's MSE from viewport, writes each band's precision independently.
/// Forbids global gain: all bands get their own precision based on their own error pattern.
pub struct HighLevelPredictor {
    per_band_a: [f64; VIB_BANDS],
    per_band_last_mse: [f64; VIB_BANDS],
    base_lr: f64,
    clip: f64,
}

impl HighLevelPredictor {
    pub fn new() -> Self {
        Self {
            per_band_a: [0.0; VIB_BANDS],
            per_band_last_mse: [0.0; VIB_BANDS],
            base_lr: 0.5,
            clip: 5.0,
        }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>) {
        let mut total_mse = 0.0f64;
        let mut total_pred_mse = 0.0f64;
        let mut total_meta_err = 0.0f64;
        let mut total_precision = 0.0f64;

        for axis in 0..VIB_BANDS {
            let (_, _, _, _, mse, _) = vp.read_predictor(axis);

            // Per-band AR-1 on this band's MSE (tracks error pattern over time)
            let pred_mse = self.per_band_a[axis] * self.per_band_last_mse[axis];
            let meta_err = mse - pred_mse;
            let da = self.base_lr * meta_err * self.per_band_last_mse[axis];
            self.per_band_a[axis] = (self.per_band_a[axis] + da).clamp(-self.clip, self.clip);

            // Per-band precision: proportional to this band's own MSE vs baseline.
            // High MSE on this band → high precision for this band → faster adaptation here.
            // A quiet band stays at low precision; an active band earns high precision.
            let band_precision = (mse / REF_MSE.max(1e-12)).clamp(PRECISION_MIN, PRECISION_MAX);

            // Write per-band precision — intentionally different values per band.
            vp.set_precision(axis, band_precision);

            total_mse += mse;
            total_pred_mse += pred_mse;
            total_meta_err += meta_err;
            total_precision += band_precision;

            self.per_band_last_mse[axis] = mse;
        }

        let n = VIB_BANDS as f64;
        let avg_a = self.per_band_a.iter().sum::<f64>() / n;

        // HighLevelState stores averages for snapshot observability.
        // Per-band precision truth is in viewport's per-axis PredictorState.precision.
        vp.update_hl_state(HighLevelState {
            a: avg_a,
            last_avg_mse: total_mse / n,
            pred_mse: total_pred_mse / n,
            meta_err: total_meta_err / n,
            modulation: total_precision / n,
        });
    }
}
