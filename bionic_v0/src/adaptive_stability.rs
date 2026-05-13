// Adaptive stability substrate.
//
// This file tracks local numeric route dynamics only. Route ids and phase
// codes are engineering handles, not meanings, targets, or result classes.

use crate::viewport::Viewport;

const ROUTE_TRACE_CAP: usize = 64;
const EMA_ALPHA: f64 = 0.35;
const ROUTE_OPEN_RETAIN_US: u64 = 5_000_000;
pub const ROUTE_PHASE_OPEN: u8 = 0;
pub const ROUTE_PHASE_STEADY: u8 = 1;
pub const ROUTE_PHASE_DEGRADED: u8 = 2;
pub const ROUTE_PHASE_RECOVERING: u8 = 3;
pub const ROUTE_PHASE_COMPRESSED: u8 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdaptiveRouteSample {
    pub route_id: u64,
    pub route_code: u16,
    pub t_us: u64,
    pub err: f64,
    pub resource_cost: f64,
    pub closure_gain: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveRouteTrace {
    pub route_id: u64,
    pub route_code: u16,
    pub phase_code: u8,
    pub err_ema: f64,
    pub resource_cost_ema: f64,
    pub closure_gain_ema: f64,
    pub degradation_score: f64,
    pub bypass_weight: f64,
    pub relearn_pressure: f64,
    pub slow_path_weight: f64,
    pub fast_path_strength: f64,
    pub stable_run: u32,
    pub drift_run: u32,
    pub last_good_t_us: u64,
    pub last_update_t_us: u64,
}

impl AdaptiveRouteTrace {
    fn new(sample: AdaptiveRouteSample) -> Self {
        Self {
            route_id: sample.route_id,
            route_code: sample.route_code,
            phase_code: ROUTE_PHASE_OPEN,
            err_ema: sample.err.clamp(0.0, 1.0),
            resource_cost_ema: sample.resource_cost.clamp(0.0, 1.0),
            closure_gain_ema: sample.closure_gain.clamp(0.0, 1.0),
            degradation_score: 0.0,
            bypass_weight: 0.0,
            relearn_pressure: 0.0,
            slow_path_weight: 1.0,
            fast_path_strength: 0.0,
            stable_run: 0,
            drift_run: 0,
            last_good_t_us: 0,
            last_update_t_us: sample.t_us,
        }
    }

    fn observe(&mut self, sample: AdaptiveRouteSample) {
        let err = sample.err.clamp(0.0, 1.0);
        let cost = sample.resource_cost.clamp(0.0, 1.0);
        let closure = sample.closure_gain.clamp(0.0, 1.0);

        self.err_ema = ema(self.err_ema, err);
        self.resource_cost_ema = ema(self.resource_cost_ema, cost);
        self.closure_gain_ema = ema(self.closure_gain_ema, closure);
        self.route_code = sample.route_code;
        self.last_update_t_us = sample.t_us;

        let bad = self.err_ema * 0.55
            + self.resource_cost_ema * 0.25
            + (1.0 - self.closure_gain_ema) * 0.20;
        let good = (1.0 - self.err_ema) * 0.45
            + self.closure_gain_ema * 0.45
            + (1.0 - self.resource_cost_ema) * 0.10;

        if bad > 0.65 {
            self.stable_run = 0;
            self.drift_run = self.drift_run.saturating_add(1);
            self.degradation_score = ema(self.degradation_score, bad);
            self.bypass_weight =
                (self.bypass_weight + 0.12 + self.degradation_score * 0.08).min(1.0);
            self.relearn_pressure =
                (self.relearn_pressure + 0.10 + self.degradation_score * 0.08).min(1.0);
            self.fast_path_strength *= 0.65;
            self.slow_path_weight = self.slow_path_weight.max(0.25);
            self.phase_code = if self.fast_path_strength > 0.15 {
                ROUTE_PHASE_RECOVERING
            } else {
                ROUTE_PHASE_DEGRADED
            };
            return;
        }

        if good > 0.72 {
            self.drift_run = 0;
            self.stable_run = self.stable_run.saturating_add(1);
            self.last_good_t_us = sample.t_us;
            self.degradation_score *= 0.72;
            self.bypass_weight *= 0.55;
            self.relearn_pressure *= 0.55;
            if self.stable_run >= 4 {
                self.fast_path_strength = (self.fast_path_strength + 0.12).min(1.0);
            }
            self.phase_code = if self.fast_path_strength > 0.35 {
                ROUTE_PHASE_COMPRESSED
            } else {
                ROUTE_PHASE_STEADY
            };
            return;
        }

        self.stable_run = 0;
        self.drift_run = 0;
        self.fast_path_strength *= 0.96;
        self.degradation_score *= 0.92;
        self.bypass_weight *= 0.90;
        self.relearn_pressure *= 0.90;
        self.phase_code = if self.relearn_pressure > 0.20 {
            ROUTE_PHASE_RECOVERING
        } else {
            ROUTE_PHASE_OPEN
        };
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdaptiveStabilityState {
    pub n_routes: u32,
    pub routes: Vec<AdaptiveRouteTrace>,
    pub last_update_t_us: u64,
}

impl AdaptiveStabilityState {
    pub fn observe_sample(&mut self, sample: AdaptiveRouteSample) {
        if sample.route_id == 0 {
            return;
        }

        self.last_update_t_us = sample.t_us;
        if let Some(route) = self
            .routes
            .iter_mut()
            .find(|r| r.route_id == sample.route_id)
        {
            route.observe(sample);
            return;
        }

        self.routes.push(AdaptiveRouteTrace::new(sample));
        if self.routes.len() > ROUTE_TRACE_CAP {
            let evict_n = self.routes.len() - ROUTE_TRACE_CAP;
            self.routes.drain(0..evict_n);
        }
        self.n_routes = self.routes.len() as u32;
    }

    pub fn route(&self, route_id: u64) -> Option<&AdaptiveRouteTrace> {
        self.routes.iter().find(|r| r.route_id == route_id)
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed = now_us.saturating_sub(self.last_update_t_us);
        if elapsed == 0 {
            return;
        }
        let decay = 0.5_f64.powf(elapsed as f64 / 10_000_000.0);
        for route in &mut self.routes {
            route.degradation_score *= decay;
            route.bypass_weight *= decay;
            route.relearn_pressure *= decay;
            route.fast_path_strength *= decay.sqrt();
        }
        self.routes.retain(|r| {
            let recent = now_us.saturating_sub(r.last_update_t_us) <= ROUTE_OPEN_RETAIN_US;
            recent || r.degradation_score > 0.01 || r.fast_path_strength > 0.01
        });
        self.n_routes = self.routes.len() as u32;
        self.last_update_t_us = now_us;
    }
}

fn ema(prev: f64, next: f64) -> f64 {
    (1.0 - EMA_ALPHA) * prev + EMA_ALPHA * next
}

impl Viewport {
    pub fn read_adaptive_stability(&self) -> AdaptiveStabilityState {
        self.adaptive_stability.read().unwrap().clone()
    }

    pub fn observe_adaptive_route_sample(&self, sample: AdaptiveRouteSample) {
        let mut st = self.adaptive_stability.write().unwrap();
        st.observe_sample(sample);
    }

    pub fn tick_adaptive_stability(&self, now_us: u64) {
        let mut st = self.adaptive_stability.write().unwrap();
        st.tick_decay(now_us);
    }
}
