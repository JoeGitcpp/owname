use std::sync::atomic::Ordering;

use crate::adaptive_stability::AdaptiveRouteSample;
use crate::evolution_priors::BiasAxis;
use crate::learned_bias::{BiasLocator, LearnedBiasEventInput};
use crate::viewport::Viewport;
use crate::TEMPORAL_PAIR_MAX_DELAY_US;

impl Viewport {
    pub fn tick_life_closure_producers(&self, now_us: u64) {
        self.produce_from_action_pairs(now_us);
        self.produce_from_framed_byte_returns(now_us);
    }

    fn produce_from_action_pairs(&self, now_us: u64) {
        let last_pair_seq = self
            .life_closure_pair_producer_cursor
            .load(Ordering::Relaxed);
        let pairs = {
            let ac = self.action_consequence.read().unwrap();
            ac.pairs
                .iter()
                .filter(|pair| pair.pair_seq > last_pair_seq)
                .cloned()
                .collect::<Vec<_>>()
        };
        if pairs.is_empty() {
            return;
        }

        let mut max_pair_seq = last_pair_seq;
        for pair in pairs {
            max_pair_seq = max_pair_seq.max(pair.pair_seq);
            let support = pair.curve_similarity.clamp(0.0, 1.0);
            let deviation = 1.0 - support;
            let delay_cost =
                (pair.delay_us as f64 / TEMPORAL_PAIR_MAX_DELAY_US.max(1) as f64).clamp(0.0, 1.0);

            self.observe_adaptive_route_sample(AdaptiveRouteSample {
                route_id: pair.write_seq,
                // Engineering locator only; not a world category.
                route_code: 1,
                t_us: now_us,
                err: deviation,
                resource_cost: delay_cost,
                closure_gain: support,
            });
            self.apply_learned_bias_event(LearnedBiasEventInput {
                axis: BiasAxis::AlignmentGain,
                locator: BiasLocator::Global,
                requested_delta: (support - deviation) * 0.05,
                support,
                deviation,
                evidence_ref: pair.pair_seq,
                event_t_us: now_us,
            });
        }
        self.life_closure_pair_producer_cursor
            .store(max_pair_seq, Ordering::Relaxed);
    }

    fn produce_from_framed_byte_returns(&self, now_us: u64) {
        let last_return_seq = self
            .life_closure_framed_return_producer_cursor
            .load(Ordering::Relaxed);
        let returns = {
            let fbe = self.framed_byte_egress.read().unwrap();
            fbe.return_traces
                .iter()
                .filter(|trace| trace.return_seq > last_return_seq)
                .copied()
                .collect::<Vec<_>>()
        };
        if returns.is_empty() {
            return;
        }

        let mut max_return_seq = last_return_seq;
        for ret in returns {
            max_return_seq = max_return_seq.max(ret.return_seq);
            let deviation = ret.correction_deviation.clamp(0.0, 1.0);
            let support = (1.0 - deviation).clamp(0.0, 1.0);
            let direction = (ret.magnitude - ret.write_pressure).clamp(-1.0, 1.0);
            self.apply_learned_bias_event(LearnedBiasEventInput {
                axis: BiasAxis::CrossChannelGain,
                locator: BiasLocator::Channel(ret.stream_id.min(u16::MAX as u64) as u16),
                requested_delta: direction * support * 0.05,
                support,
                deviation,
                evidence_ref: ret.return_seq,
                event_t_us: now_us,
            });
        }
        self.life_closure_framed_return_producer_cursor
            .store(max_return_seq, Ordering::Relaxed);
    }
}
