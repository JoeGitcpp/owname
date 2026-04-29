use std::sync::Arc;

use crate::viewport::{Sample, Viewport};
use crate::vpc::PredictorLifecycleState;

pub struct Ar2Predictor {
    pub axis: usize,
    pub a: f64,
    pub b: f64,
    pub base_lr: f64,
    pub clip: f64,
    pub horizon_us: u64,
    pub pending_pred: Option<PendingPred>,
}

pub struct PendingPred {
    pub predicted_value: f64,
    pub used_x0: f64,
    pub used_x1: f64,
    pub seq_at_prediction: u64,
    pub publish_t_us: u64,
    pub target_t_us: u64,
}

const VPC_FAMILY_AR2: u8 = 1;

impl Ar2Predictor {
    pub fn new(axis: usize) -> Self {
        Self {
            axis,
            a: 0.0,
            b: 0.0,
            base_lr: 2.0, // tuned for typical mic RMS (~0.005..0.5); converges in ~0.5s for loud hum
            clip: 10.0,
            horizon_us: 10_000, // 10ms @ 100 Hz predictor / 100 Hz VIB
            pending_pred: None,
        }
    }

    pub fn tick(&mut self, vp: &Arc<Viewport>) {
        let last_two: Vec<Sample> = vp.read_latest_observations(self.axis, 2);
        if last_two.len() < 2 {
            if let Some(sample) = last_two.first() {
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
        let x_now = last_two[0].value;
        let x_prev = last_two[1].value;
        let t_now = last_two[0].t_us;

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
        let mut pending_still_open = None;
        if let Some(pp) = self.pending_pred.take() {
            let observed_seq = {
                let r = vp.observed[self.axis].read().unwrap();
                r.total_written
            };
            if observed_seq > pp.seq_at_prediction && t_now >= pp.target_t_us {
                let err = x_now - pp.predicted_value;
                let eff_lr = self.base_lr * surprise;
                let da = eff_lr * err * pp.used_x0;
                let db = eff_lr * err * pp.used_x1;
                self.a = (self.a + da).clamp(-self.clip, self.clip);
                self.b = (self.b + db).clamp(-self.clip, self.clip);
                vp.push_err(self.axis, t_now, err);
                vp.resolve_vpc_axis_publication(VPC_FAMILY_AR2, self.axis, pp.target_t_us);
                last_err = err;
            } else {
                pending_still_open = Some(pp);
            }
        }

        if let Some(pp) = pending_still_open {
            self.pending_pred = Some(pp);
            return;
        }

        let pred = self.a * x_now + self.b * x_prev;
        let target_t_us = t_now + self.horizon_us;

        let obs_seq_after = {
            let r = vp.observed[self.axis].read().unwrap();
            r.total_written
        };
        vp.record_vpc_axis_publication(VPC_FAMILY_AR2, self.axis, t_now, target_t_us);
        self.pending_pred = Some(PendingPred {
            predicted_value: pred,
            used_x0: x_now,
            used_x1: x_prev,
            seq_at_prediction: obs_seq_after,
            publish_t_us: t_now,
            target_t_us,
        });

        vp.push_prediction(self.axis, target_t_us, pred);
        vp.update_predictor(self.axis, self.a, self.b, last_err);
    }
}
