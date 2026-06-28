use std::sync::Arc;

use crate::viewport::{Sample, Viewport};
use crate::vpc::PredictorLifecycleState;

pub const N_CTX: usize = 4;
pub const N_TAPS: usize = 4;

pub struct ContextGatedPredictor {
    pub axis: usize,
    pub coeffs: [[f64; N_TAPS]; N_CTX],
    pub base_lr: f64,
    pub clip: f64,
    pub horizon_us: u64,
    pub pending_pred: Option<PendingPredCg>,
}

pub struct PendingPredCg {
    pub predicted_value: f64,
    pub used_x: [f64; N_TAPS],
    pub used_ctx: usize,
    pub seq_at_prediction: u64,
    pub publish_t_us: u64,
    pub target_t_us: u64,
}

const VPC_FAMILY_AR2: u8 = 1;

fn quantize_ctx(x: &[f64; N_TAPS]) -> usize {
    let bit_0 = (x[0] > x[1]) as usize;
    let bit_1 = (x[1] > x[2]) as usize;
    bit_0 * 2 + bit_1
}

impl ContextGatedPredictor {
    pub fn new(axis: usize) -> Self {
        Self {
            axis,
            coeffs: [[0.0; N_TAPS]; N_CTX],
            base_lr: 2.0,
            clip: 10.0,
            horizon_us: 10_000,
            pending_pred: None,
        }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>) {
        let last_four: Vec<Sample> = vp.read_latest_observations(self.axis, N_TAPS);
        if last_four.len() < 2 {
            if let Some(sample) = last_four.first() {
                vp.record_vpc_axis_predictor_trace(
                    VPC_FAMILY_AR2,
                    self.axis,
                    sample.t_us,
                    sample.t_us + self.horizon_us,
                    PredictorLifecycleState::InsufficientContext,
                );
            }
            return;
        }

        let n = last_four.len();
        let mut x = [0.0f64; N_TAPS];
        for i in 0..N_TAPS {
            x[i] = if i < n { last_four[i].value } else { last_four[n - 1].value };
        }
        let t_now = last_four[0].t_us;

        let (surprise, _a_vp, _b_vp, _e, _mse, _u) = {
            let ps = vp.predictors[self.axis].read().unwrap();
            (
                ps.surprise,
                ps.a,
                ps.b,
                ps.last_err,
                ps.mean_sq_err,
                ps.total_updates,
            )
        };

        let mut last_err = 0.0f64;
        let mut last_ctx = 0usize;
        let mut pending_still_open = None;
        if let Some(pp) = self.pending_pred.take() {
            let observed_seq = {
                let r = vp.observed[self.axis].read().unwrap();
                r.total_written
            };
            if observed_seq > pp.seq_at_prediction && t_now >= pp.target_t_us {
                let err = x[0] - pp.predicted_value;
                let m = vp.developmental_maturity();
                let eff_lr = self.base_lr * surprise * crate::ar2_lr_scale(m);
                let ctx = pp.used_ctx;
                for tap in 0..N_TAPS {
                    let d = eff_lr * err * pp.used_x[tap];
                    self.coeffs[ctx][tap] =
                        (self.coeffs[ctx][tap] + d).clamp(-self.clip, self.clip);
                }
                vp.push_err(self.axis, t_now, err);
                vp.resolve_vpc_axis_publication(VPC_FAMILY_AR2, self.axis, pp.target_t_us);
                last_err = err;
                last_ctx = ctx;
            } else {
                pending_still_open = Some(pp);
            }
        }

        if let Some(pp) = pending_still_open {
            self.pending_pred = Some(pp);
            return;
        }

        let external_prior = vp.read_prior(self.axis);
        let ctx = quantize_ctx(&x);
        let c = &self.coeffs[ctx];
        let pred = c[0] * x[0] + c[1] * x[1] + c[2] * x[2] + c[3] * x[3] + external_prior;
        let target_t_us = t_now + self.horizon_us;

        let obs_seq_after = {
            let r = vp.observed[self.axis].read().unwrap();
            r.total_written
        };
        vp.record_vpc_axis_publication(VPC_FAMILY_AR2, self.axis, t_now, target_t_us);
        self.pending_pred = Some(PendingPredCg {
            predicted_value: pred,
            used_x: x,
            used_ctx: ctx,
            seq_at_prediction: obs_seq_after,
            publish_t_us: t_now,
            target_t_us,
        });

        vp.push_prediction(self.axis, target_t_us, pred);
        vp.update_predictor_ctx(self.axis, self.coeffs[ctx][0], self.coeffs[ctx][1], last_err, last_ctx);
    }
}
