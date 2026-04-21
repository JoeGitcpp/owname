//! Phase 5B: Object-Level High-Layer Predictor.
//!
//! Each active ObjectSlot that has a populated fusion subtree gets one
//! `ObjectHighLayerPredictor` (ephemeral, not snapshot'd) plus one
//! `ObjectHighLayerState` output (snapshot'd, observer-facing).
//!
//! Architecture constraints:
//!   * Never on an adapter; never as a global viewport-wide predictor.
//!   * Reads only CrossModalFusionState (the fusion subtree).
//!   * Writes only ObjectHighLayerState (the hl output subtree).
//!   * Does not influence claim/scoring, spatial, or band families.
//!   * No semantic labels, object names, or tracking IDs — numeric only.
//!   * Predictor state resets with the slot (same lifecycle as phot_pred).

use crate::{OBJECT_HL_ALPHA, OBJECT_HL_ERR_ALPHA, OBJECT_HL_WARMUP_MIN};

// ---------------------------------------------------------------------------
// Output state (snapshot'd, observer-facing)
// ---------------------------------------------------------------------------

/// High-layer output subtree for one ObjectSlot.
///
/// Populated by `Viewport::refresh_slot_high_layer()`. Observers render these
/// fields; they never recompute them. All fields are numeric.
///
/// `object_err` is the instantaneous RMS prediction error across the three
/// fusion-level features (confidence, alignment, precision). It rises when the
/// fused object state is unpredictable, falls when the state is stable.
///
/// `attention_bias` is a slow-reacting stability proxy derived from an EMA of
/// `object_err²`. High (→1.0) when the object has been stable; low (→0) when
/// it is volatile.
#[derive(Clone, Copy, Debug, Default)]
pub struct ObjectHighLayerState {
    /// AR-1 prediction of cross_modal_confidence for this step.
    pub predicted_confidence: f64,
    /// AR-1 prediction of temporal_alignment_score for this step.
    pub predicted_alignment: f64,
    /// AR-1 prediction of fused_precision for this step.
    pub predicted_precision: f64,
    /// RMS prediction error across the three feature channels.
    pub object_err: f64,
    /// [0,1] stability proxy: 1 / (1 + sqrt(err_sq_ema)). Near 1.0 when stable.
    pub attention_bias: f64,
    /// Wall-clock µs when this output was last written.
    pub last_update_t_us: u64,
}

// ---------------------------------------------------------------------------
// Predictor state (ephemeral — not snapshot'd, reset with slot)
// ---------------------------------------------------------------------------

/// Slot-local AR-1 predictor for the three fusion-level features.
///
/// Tracks exponential-smoothing state for each feature independently.
/// During warm-up (`n_updates < OBJECT_HL_WARMUP_MIN`) the predictor seeds
/// its smoothers and returns a neutral output (object_err = 0, attention_bias
/// = 0.5) so callers cannot mistake a cold start for a genuinely stable object.
pub struct ObjectHighLayerPredictor {
    smoothed_confidence: f64,
    smoothed_alignment: f64,
    smoothed_precision: f64,
    /// EMA of object_err²; drives the slow-reacting attention_bias.
    err_sq_ema: f64,
    /// Total calls to `update`, including warm-up calls.
    pub n_updates: u32,
}

impl ObjectHighLayerPredictor {
    pub fn new() -> Self {
        Self {
            smoothed_confidence: 0.0,
            smoothed_alignment: 0.0,
            smoothed_precision: 0.0,
            err_sq_ema: 0.0,
            n_updates: 0,
        }
    }

    /// Feed current fusion scalars; return the updated high-layer output.
    ///
    /// During warm-up: seeds smoothers, returns object_err = 0 and
    /// attention_bias = 0.5 (neutral) so the slot cannot be mistaken for a
    /// confident stable object before the predictor has any history.
    ///
    /// After warm-up: issues AR-1 prediction (previous smoothed value),
    /// computes object_err as RMS across the three channels, updates the
    /// err_sq EMA for attention_bias, then advances the smoothers.
    pub fn update(
        &mut self,
        confidence: f64,
        alignment: f64,
        precision: f64,
        now_us: u64,
    ) -> ObjectHighLayerState {
        if self.n_updates < OBJECT_HL_WARMUP_MIN {
            self.smoothed_confidence = confidence;
            self.smoothed_alignment = alignment;
            self.smoothed_precision = precision;
            self.n_updates += 1;
            return ObjectHighLayerState {
                predicted_confidence: confidence,
                predicted_alignment: alignment,
                predicted_precision: precision,
                object_err: 0.0,
                attention_bias: 0.5,
                last_update_t_us: now_us,
            };
        }

        // AR-1: prediction = previous smoothed value.
        let pred_c = self.smoothed_confidence;
        let pred_a = self.smoothed_alignment;
        let pred_p = self.smoothed_precision;

        let err_c = confidence - pred_c;
        let err_a = alignment - pred_a;
        let err_p = precision - pred_p;

        let object_err = ((err_c * err_c + err_a * err_a + err_p * err_p) / 3.0).sqrt();

        // EMA of err²: slow-reacting signal for attention_bias.
        self.err_sq_ema = OBJECT_HL_ERR_ALPHA * object_err * object_err
            + (1.0 - OBJECT_HL_ERR_ALPHA) * self.err_sq_ema;

        // Stability proxy: high when err_sq_ema is small.
        let attention_bias = 1.0 / (1.0 + self.err_sq_ema.sqrt());

        // Advance smoothers.
        self.smoothed_confidence =
            OBJECT_HL_ALPHA * confidence + (1.0 - OBJECT_HL_ALPHA) * self.smoothed_confidence;
        self.smoothed_alignment =
            OBJECT_HL_ALPHA * alignment + (1.0 - OBJECT_HL_ALPHA) * self.smoothed_alignment;
        self.smoothed_precision =
            OBJECT_HL_ALPHA * precision + (1.0 - OBJECT_HL_ALPHA) * self.smoothed_precision;

        self.n_updates += 1;

        ObjectHighLayerState {
            predicted_confidence: pred_c,
            predicted_alignment: pred_a,
            predicted_precision: pred_p,
            object_err,
            attention_bias,
            last_update_t_us: now_us,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests — no viewport required.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_does_not_fake_confidence() {
        let mut pred = ObjectHighLayerPredictor::new();
        // Both warmup calls must return object_err=0, attention_bias=0.5.
        let out0 = pred.update(0.9, 0.8, 0.5, 1_000_000);
        assert_eq!(out0.object_err, 0.0, "warmup tick 0: err must be 0");
        assert_eq!(out0.attention_bias, 0.5, "warmup tick 0: bias must be 0.5 (neutral)");
        let out1 = pred.update(0.9, 0.8, 0.5, 1_001_000);
        assert_eq!(out1.object_err, 0.0, "warmup tick 1: err must be 0");
        assert_eq!(out1.attention_bias, 0.5, "warmup tick 1: bias must be 0.5");
    }

    #[test]
    fn stable_input_gives_near_zero_err() {
        let mut pred = ObjectHighLayerPredictor::new();
        // Warmup.
        pred.update(0.8, 1.0, 0.6, 0);
        pred.update(0.8, 1.0, 0.6, 1_000);
        // 20 stable ticks.
        let mut last_err = 0.0;
        for i in 0..20 {
            let out = pred.update(0.8, 1.0, 0.6, (i + 2) * 1_000);
            last_err = out.object_err;
        }
        assert!(
            last_err < 1e-10,
            "stable input must give near-zero err; got {}",
            last_err
        );
    }

    #[test]
    fn err_spikes_on_abrupt_input_change() {
        let mut pred = ObjectHighLayerPredictor::new();
        // Warmup + stabilize on (0.8, 1.0, 0.6).
        pred.update(0.8, 1.0, 0.6, 0);
        pred.update(0.8, 1.0, 0.6, 1_000);
        for i in 0..20 {
            pred.update(0.8, 1.0, 0.6, (i + 2) * 1_000);
        }
        // Abrupt jump to (0.3, 0.0, 0.6).
        let spike = pred.update(0.3, 0.0, 0.6, 30_000);
        assert!(
            spike.object_err > 0.1,
            "err must spike on sudden change; got {}",
            spike.object_err
        );
    }

    #[test]
    fn attention_bias_rises_on_stable_signal() {
        let mut pred = ObjectHighLayerPredictor::new();
        pred.update(1.0, 1.0, 0.8, 0);
        pred.update(1.0, 1.0, 0.8, 1_000);
        let mut last_bias = 0.0;
        for i in 0..30 {
            let out = pred.update(1.0, 1.0, 0.8, (i + 2) * 1_000);
            last_bias = out.attention_bias;
        }
        assert!(
            last_bias > 0.95,
            "attention_bias must approach 1.0 on stable signal; got {}",
            last_bias
        );
    }
}
