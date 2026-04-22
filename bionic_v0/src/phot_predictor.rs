//! Slot-local PHOT band predictor (Phase 4C).
//!
//! One `PhotSlotPredictor` per active ObjectSlot that carries PHOT data.
//! Each instance runs an independent AR-1 (exponential smoothing) predictor
//! per band. State is ephemeral — not snapshot'd — so the predictor cold-starts
//! when a slot is first allocated or re-allocated after expiry.
//!
//! Architecture constraints:
//!   * No semantic labels, object names, or tracking IDs.
//!   * Not shared across slots — every slot is independent.
//!   * Does not influence claim/scoring — only writes band-family output.
//!   * Precision tracks surprise (EMA of err²); it is numeric, not a label.

use crate::{
    PHOT_BANDS, PHOT_PRED_ALPHA, PHOT_PRED_PRECISION_ALPHA, PHOT_PRED_PRECISION_MAX,
    PHOT_PRED_PRECISION_MIN, PHOT_PRED_WARMUP_MIN,
};

/// Output of one predictor update step.
pub struct PhotPrediction {
    pub predicted: Vec<f64>,
    pub err: Vec<f64>,
    pub precision: Vec<f64>,
}

/// Slot-local AR-1 predictor for PHOT bands.
///
/// Each band is tracked independently with:
///   * exponential smoothing (`smoothed`) as the AR-1 state
///   * a running EMA of squared error as `precision`
///
/// During warm-up (`n_updates < PHOT_PRED_WARMUP_MIN`), the output carries
/// `err = 0` and `precision = 0` so callers can distinguish "not yet
/// predicting" from a real low-error state.
pub struct PhotSlotPredictor {
    /// Number of bands this predictor was initialized for.
    n_bands: usize,
    /// AR-1 exponential-smoothing state, one element per band.
    smoothed: [f64; PHOT_BANDS],
    /// Per-band precision (EMA of squared error). Rises with surprise.
    precision: [f64; PHOT_BANDS],
    /// Total calls to `update`, including warm-up calls.
    pub n_updates: u32,
}

impl PhotSlotPredictor {
    pub fn new(n_bands: usize) -> Self {
        let n_bands = n_bands.min(PHOT_BANDS);
        Self {
            n_bands,
            smoothed: [0.0; PHOT_BANDS],
            precision: [PHOT_PRED_PRECISION_MIN; PHOT_BANDS],
            n_updates: 0,
        }
    }

    /// Feed one new observation vector, advance the predictor, and return the
    /// prediction output for this step.
    ///
    /// Warm-up (n_updates < PHOT_PRED_WARMUP_MIN): seeds the smoother from
    /// the observed values and emits zero err / zero precision so callers
    /// cannot mistake a warm-up tick for a confident low-error prediction.
    ///
    /// After warm-up: issues an AR-1 prediction (previous smoothed value),
    /// computes err = observed - predicted, updates precision via EMA of err²,
    /// then updates the AR-1 state.
    pub fn update(&mut self, observed: &[f64]) -> PhotPrediction {
        let n = self.n_bands.min(observed.len());

        if self.n_updates < PHOT_PRED_WARMUP_MIN {
            // Seed the smoother; output no real prediction yet.
            for i in 0..n {
                self.smoothed[i] = observed[i];
            }
            self.n_updates += 1;
            return PhotPrediction {
                predicted: observed[..n].to_vec(),
                err: vec![0.0; n],
                precision: vec![PHOT_PRED_PRECISION_MIN; n],
            };
        }

        // AR-1: prediction is the previous smoothed value.
        let predicted: Vec<f64> = self.smoothed[..n].to_vec();
        let err: Vec<f64> = observed[..n]
            .iter()
            .zip(predicted.iter())
            .map(|(o, p)| o - p)
            .collect();

        // Precision tracks surprise: rises with err², falls when err is small.
        for i in 0..n {
            let e2 = err[i] * err[i];
            self.precision[i] = PHOT_PRED_PRECISION_ALPHA * e2
                + (1.0 - PHOT_PRED_PRECISION_ALPHA) * self.precision[i];
            self.precision[i] =
                self.precision[i].clamp(PHOT_PRED_PRECISION_MIN, PHOT_PRED_PRECISION_MAX);
        }

        // Update the AR-1 smoother state for next step.
        for i in 0..n {
            self.smoothed[i] =
                PHOT_PRED_ALPHA * observed[i] + (1.0 - PHOT_PRED_ALPHA) * self.smoothed[i];
        }
        self.n_updates += 1;

        PhotPrediction {
            predicted,
            err,
            precision: self.precision[..n].to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests — no camera or viewport required.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// New slot / warmup: predictor must not emit fake confidence.
    #[test]
    fn phot_predictor_warmup_does_not_fake_confidence() {
        let mut pred = PhotSlotPredictor::new(4);
        let obs = [0.3, 0.5, 0.1, 0.2];
        // First warm-up call.
        let out = pred.update(&obs);
        assert_eq!(
            out.precision,
            vec![0.0; 4],
            "warmup call 1: precision must be zero"
        );
        assert_eq!(out.err, vec![0.0; 4], "warmup call 1: err must be zero");
        // Second warm-up call (n_updates=1, still < WARMUP_MIN=2).
        let out2 = pred.update(&obs);
        assert_eq!(
            out2.precision,
            vec![0.0; 4],
            "warmup call 2: precision must be zero"
        );
        assert_eq!(out2.err, vec![0.0; 4], "warmup call 2: err must be zero");
        // Third call: now predicting. With constant obs, err should still be ~0.
        let out3 = pred.update(&obs);
        // predicted was seeded to obs during warmup so err ≈ 0.
        for i in 0..4 {
            assert!(
                out3.err[i].abs() < 1e-12,
                "band {}: err={} but expected ≈0 after seeded warmup",
                i,
                out3.err[i]
            );
        }
    }

    /// Stable signal: err must converge toward zero.
    #[test]
    fn phot_predictor_converges_on_stable_signal() {
        let mut pred = PhotSlotPredictor::new(4);
        let obs = [0.6_f64, 0.3, 0.1, 0.8];
        // Warmup.
        pred.update(&obs);
        pred.update(&obs);
        // Run 20 more ticks with the same signal.
        let mut last_err = [f64::INFINITY; 4];
        for _ in 0..20 {
            let out = pred.update(&obs);
            for i in 0..4 {
                last_err[i] = out.err[i].abs();
            }
        }
        for i in 0..4 {
            assert!(
                last_err[i] < 1e-10,
                "band {}: err={} must converge to near-zero on constant signal",
                i,
                last_err[i]
            );
        }
    }

    /// Sudden value change: err must spike immediately, then reconverge.
    #[test]
    fn phot_predictor_err_spikes_on_motion_change() {
        let mut pred = PhotSlotPredictor::new(4);
        let obs_a = [0.2_f64, 0.2, 0.2, 0.2];
        let obs_b = [0.9_f64, 0.9, 0.9, 0.9];

        // Warmup + stabilize on obs_a.
        pred.update(&obs_a);
        pred.update(&obs_a);
        for _ in 0..15 {
            pred.update(&obs_a);
        }

        // Sudden jump to obs_b.
        let spike = pred.update(&obs_b);
        let spike_err: Vec<f64> = spike.err.iter().map(|e| e.abs()).collect();

        // Reconverge on obs_b.
        for _ in 0..20 {
            pred.update(&obs_b);
        }
        let converged = pred.update(&obs_b);
        let converged_err: Vec<f64> = converged.err.iter().map(|e| e.abs()).collect();

        for i in 0..4 {
            assert!(
                spike_err[i] > 0.1,
                "band {}: err={} must spike on sudden change (expected >0.1)",
                i,
                spike_err[i]
            );
            assert!(
                converged_err[i] < spike_err[i] * 0.1,
                "band {}: err must reconverge; spike={} converged={}",
                i,
                spike_err[i],
                converged_err[i]
            );
        }
    }

    /// Precision tracks surprise: low err → precision stays low,
    /// high err → precision rises.
    #[test]
    fn phot_precision_tracks_prediction_quality() {
        let mut pred = PhotSlotPredictor::new(1);
        let stable = [0.5_f64];

        // Warmup + stabilize.
        pred.update(&stable);
        pred.update(&stable);
        for _ in 0..30 {
            pred.update(&stable);
        }
        let stable_out = pred.update(&stable);
        let precision_after_stable = stable_out.precision[0];

        // Spike: large sudden change.
        let spike_obs = [5.0_f64];
        pred.update(&spike_obs);
        let spike_out = pred.update(&spike_obs);
        let precision_after_spike = spike_out.precision[0];

        assert!(
            precision_after_spike > precision_after_stable,
            "precision must rise after high-err spike: stable={} spike={}",
            precision_after_stable,
            precision_after_spike
        );
    }
}
