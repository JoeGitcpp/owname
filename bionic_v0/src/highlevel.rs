use std::sync::Arc;

use crate::viewport::{HighLevelState, Viewport};
use crate::VIB_BANDS;

const SURPRISE_ALPHA: f64 = 0.1;

/// Per-band surprise tracker using EMA of err².
/// Reads each band's MSE from viewport, computes per-band surprise (EMA of MSE),
/// and writes it so callers downstream can use a unified [0,1] surprise scale.
pub struct HighLevelPredictor {
    per_band_a: [f64; VIB_BANDS],
    per_band_last_mse: [f64; VIB_BANDS],
    per_band_surprise: [f64; VIB_BANDS],
    base_lr: f64,
    clip: f64,
}

impl HighLevelPredictor {
    pub fn new() -> Self {
        Self {
            per_band_a: [0.0; VIB_BANDS],
            per_band_last_mse: [0.0; VIB_BANDS],
            per_band_surprise: [0.0; VIB_BANDS],
            base_lr: 0.5,
            clip: 5.0,
        }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>) {
        let mut total_mse = 0.0f64;
        let mut total_pred_mse = 0.0f64;
        let mut total_meta_err = 0.0f64;
        let mut total_surprise = 0.0f64;

        for axis in 0..VIB_BANDS {
            let (_, _, _, _, mse, _) = vp.read_predictor(axis);

            // Per-band AR-1 on this band's MSE (tracks error pattern over time)
            let pred_mse = self.per_band_a[axis] * self.per_band_last_mse[axis];
            let meta_err = mse - pred_mse;
            let da = self.base_lr * meta_err * self.per_band_last_mse[axis];
            self.per_band_a[axis] = (self.per_band_a[axis] + da).clamp(-self.clip, self.clip);

            // Per-band surprise: EMA of MSE, clamped to [0,1].
            // High MSE on this band → high surprise.
            // A quiet band stays at low surprise; an active band earns high surprise.
            self.per_band_surprise[axis] =
                SURPRISE_ALPHA * mse + (1.0 - SURPRISE_ALPHA) * self.per_band_surprise[axis];
            self.per_band_surprise[axis] = self.per_band_surprise[axis].clamp(0.0, 1.0);

            vp.set_surprise(axis, self.per_band_surprise[axis]);

            total_mse += mse;
            total_pred_mse += pred_mse;
            total_meta_err += meta_err;
            total_surprise += self.per_band_surprise[axis];

            self.per_band_last_mse[axis] = mse;
        }

        let n = VIB_BANDS as f64;
        let avg_a = self.per_band_a.iter().sum::<f64>() / n;

        // HighLevelState stores averages for snapshot observability.
        // Per-band surprise truth is stored in viewport's per-axis PredictorState.surprise.
        vp.update_hl_state(HighLevelState {
            a: avg_a,
            last_avg_mse: total_mse / n,
            pred_mse: total_pred_mse / n,
            meta_err: total_meta_err / n,
            modulation: total_surprise / n,
        });
    }
}
