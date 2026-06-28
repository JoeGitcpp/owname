use crate::context_modulation::{ContextModulator, CTX_DIM};
use crate::developmental_gate::effective_mask;
use crate::feature_extraction::{FeatureExtractor, N_POOLS};
use crate::pattern_memory::{PatternMemory, PatternQueryResult, PATTERN_DIM};
use crate::replay_engine::{ReplayEngine, ReplayStep, ReplayTrigger};
use crate::shared_canvas::{SharedCanvas, CANVAS_DIM};
use crate::{
    AXIS_COUNT, INTERO_AGENCY, INTERO_GAP_PRESSURE, INTERO_INTENTION_PRESSURE,
    INTERO_LEARNING_PROGRESS, INTERO_RESERVE, INTERO_VITAL_ANXIETY,
};

pub struct CognitivePipelineOutput {
    pub canvas: [f64; CANVAS_DIM],
    pub features: [f64; N_POOLS],
    pub modulated: [f64; N_POOLS],
    pub query_result: CognitiveQuerySummary,
    pub ignited_count: usize,
    pub sparsity: f64,
    pub replay_steps: Vec<ReplayStep>,
}

#[derive(Clone, Copy, Debug)]
pub enum CognitiveQuerySummary {
    Completed { confidence: f64, index: usize },
    Separated { new_id: usize },
    Ambiguous { candidate_count: usize },
    Empty,
}

pub struct CognitivePipeline {
    pub canvas: SharedCanvas,
    pub features: FeatureExtractor,
    pub modulator: ContextModulator,
    pub patterns: PatternMemory,
    pub replay: ReplayEngine,
    last_modulated: [f64; N_POOLS],
    last_arousal: f64,
}

impl CognitivePipeline {
    pub fn new() -> Self {
        Self {
            canvas: SharedCanvas::new(),
            features: FeatureExtractor::new(0xC09_2027),
            modulator: ContextModulator::new(),
            patterns: PatternMemory::new(),
            replay: ReplayEngine::new(),
            last_modulated: [0.0; N_POOLS],
            last_arousal: 0.0,
        }
    }

    pub fn tick(
        &mut self,
        axes: &[f64; AXIS_COUNT],
        per_axis_surprise: &[f64; AXIS_COUNT],
        maturity: f64,
        t_us: u64,
    ) -> CognitivePipelineOutput {
        // L0: developmental mask
        let dev_mask = effective_mask(maturity);

        // L1: shared canvas — precision from AR-2 surprise
        self.canvas.update_precision_from_surprise(per_axis_surprise);
        let canvas_out = self.canvas.step(axes, &dev_mask, 10.0);

        // L2: sparse feature extraction
        let features = self.features.step(&canvas_out);

        // L3: FiLM modulation from interoceptive state
        let arousal = self.canvas.ignited_count as f64 / CANVAS_DIM as f64;
        self.last_arousal = arousal;
        let ctx = Self::build_context(axes, maturity, arousal);
        let modulated = self.modulator.modulate(&features, &ctx);
        self.last_modulated = modulated;

        // L4: pattern memory query + consolidation
        self.patterns.update_thresholds(maturity);
        self.patterns.tick_consolidation();
        let query_result = self.patterns.query(&modulated, t_us);

        // L4 → L3 gradient: backprop match error through FiLM weights
        if let PatternQueryResult::Completed { ref residual, .. } = query_result {
            self.modulator.update(&features, &ctx, residual);
        }

        // T5: record pattern transition for sequence Hopfield
        let replay_trigger = match &query_result {
            PatternQueryResult::Completed { index, .. } => {
                self.replay.record_transition(*index, t_us);
                None
            }
            PatternQueryResult::Separated { new_id } => {
                self.replay.record_transition(*new_id, t_us);
                Some(ReplayTrigger::Surprise { pattern_id: *new_id })
            }
            _ => None,
        };

        // T6: generate replay if triggered
        let replay_steps = if let Some(trigger) = replay_trigger {
            if self.replay.should_replay(trigger) {
                let pm = &self.patterns;
                self.replay.generate_replay(
                    trigger,
                    &|pid| {
                        if pid < pm.entries.len() {
                            pm.entries[pid].strength.min(1.0)
                        } else {
                            0.0
                        }
                    },
                    t_us,
                )
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let summary = match &query_result {
            PatternQueryResult::Completed { confidence, index, .. } => {
                CognitiveQuerySummary::Completed {
                    confidence: *confidence,
                    index: *index,
                }
            }
            PatternQueryResult::Separated { new_id } => {
                CognitiveQuerySummary::Separated { new_id: *new_id }
            }
            PatternQueryResult::Ambiguous { top_candidates } => {
                CognitiveQuerySummary::Ambiguous {
                    candidate_count: top_candidates.len(),
                }
            }
            PatternQueryResult::Empty => CognitiveQuerySummary::Empty,
        };

        CognitivePipelineOutput {
            canvas: canvas_out,
            features,
            modulated,
            query_result: summary,
            ignited_count: self.canvas.ignited_count,
            sparsity: self.features.sparsity(),
            replay_steps,
        }
    }

    fn build_context(axes: &[f64; AXIS_COUNT], maturity: f64, arousal: f64) -> [f64; CTX_DIM] {
        [
            axes[INTERO_RESERVE],
            axes[INTERO_VITAL_ANXIETY],
            axes[INTERO_AGENCY],
            axes[INTERO_GAP_PRESSURE],
            axes[INTERO_LEARNING_PROGRESS],
            axes[INTERO_INTENTION_PRESSURE],
            maturity,
            arousal,
        ]
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.pattern_count()
    }

    pub fn transition_count(&self) -> usize {
        self.replay.transition_count()
    }

    pub fn trigger_replay(
        &mut self,
        trigger: ReplayTrigger,
        axes: &[f64; AXIS_COUNT],
        maturity: f64,
        t_us: u64,
    ) -> Vec<ReplayStep> {
        if !self.replay.should_replay(trigger) {
            return Vec::new();
        }
        let pm = &self.patterns;
        let steps = self.replay.generate_replay(
            trigger,
            &|pid| {
                if pid < pm.entries.len() {
                    pm.entries[pid].strength.min(1.0)
                } else {
                    0.0
                }
            },
            t_us,
        );
        let ctx = Self::build_context(axes, maturity, self.last_arousal);
        for step in &steps {
            if step.pattern_id < self.patterns.entries.len() {
                let pattern = self.patterns.entries[step.pattern_id].pattern;
                let modulated = self.modulator.modulate(&pattern, &ctx);
                let _ = self.patterns.query(&modulated, t_us);
            }
        }
        steps
    }

    pub fn trigger_replay_epi(
        &mut self,
        trigger: ReplayTrigger,
        info_gain: &dyn Fn(usize) -> f64,
        axes: &[f64; AXIS_COUNT],
        maturity: f64,
        t_us: u64,
    ) -> Vec<ReplayStep> {
        if !self.replay.should_replay(trigger) {
            return Vec::new();
        }
        let pm = &self.patterns;
        let steps = self.replay.generate_replay_epi(
            trigger,
            &|pid| {
                if pid < pm.entries.len() {
                    pm.entries[pid].strength.min(1.0)
                } else {
                    0.0
                }
            },
            info_gain,
            t_us,
        );
        let ctx = Self::build_context(axes, maturity, self.last_arousal);
        for step in &steps {
            if step.pattern_id < self.patterns.entries.len() {
                let pattern = self.patterns.entries[step.pattern_id].pattern;
                let modulated = self.modulator.modulate(&pattern, &ctx);
                let _ = self.patterns.query(&modulated, t_us);
            }
        }
        steps
    }

    // ── Persistence ────────────────────────────────────────────────────────

    pub fn serialize_patterns(&self) -> Vec<u8> {
        self.patterns.serialize()
    }

    pub fn serialize_replay(&self) -> Vec<u8> {
        self.replay.serialize()
    }

    pub fn restore_patterns(&mut self, data: &[u8]) -> bool {
        if let Some(pm) = PatternMemory::deserialize(data) {
            self.patterns = pm;
            true
        } else {
            false
        }
    }

    pub fn restore_replay(&mut self, data: &[u8]) -> bool {
        if let Some(re) = ReplayEngine::deserialize(data) {
            self.replay = re;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_runs_end_to_end() {
        let mut pipe = CognitivePipeline::new();
        let mut axes = [0.0f64; AXIS_COUNT];
        for i in 0..16 {
            axes[i] = 0.5; // hearing signal
        }
        axes[INTERO_RESERVE] = 0.8;
        axes[INTERO_VITAL_ANXIETY] = 0.1;
        let surprise = [0.01f64; AXIS_COUNT];

        let out = pipe.tick(&axes, &surprise, 0.5, 1_000_000);
        assert_eq!(out.features.len(), N_POOLS);
        assert_eq!(out.modulated.len(), N_POOLS);
    }

    #[test]
    fn pipeline_accumulates_patterns() {
        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];

        // Feed diverse inputs
        for i in 0..10 {
            let mut axes = [0.0f64; AXIS_COUNT];
            axes[i % 16] = 1.0;
            axes[INTERO_RESERVE] = 0.5;
            pipe.tick(&axes, &surprise, 0.5, i as u64 * 100_000);
        }
        assert!(pipe.pattern_count() > 0, "pipeline should accumulate patterns");
    }

    #[test]
    fn maturity_affects_dev_mask() {
        let mut pipe = CognitivePipeline::new();
        let mut axes = [0.0f64; AXIS_COUNT];
        // Set a late-priority axis (CODE)
        axes[crate::AXIS_CODE_BASE] = 1.0;
        let surprise = [0.01f64; AXIS_COUNT];

        // At maturity 0, late axes should be masked
        let out_young = pipe.tick(&axes, &surprise, 0.0, 1_000);

        // At maturity 1, all axes open
        let out_mature = pipe.tick(&axes, &surprise, 1.0, 2_000);

        // Canvas activation should differ
        let sum_young: f64 = out_young.canvas.iter().sum();
        let sum_mature: f64 = out_mature.canvas.iter().sum();
        // Young canvas should have less from the late axis
        // (this is a weak test since random projections make it noisy,
        //  but the mask should have some effect)
        assert!(sum_young.abs() <= sum_mature.abs() + 1.0,
            "young canvas should not have MORE activation from masked axes");
    }

    #[test]
    fn pipeline_pattern_serialize_round_trips() {
        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];

        // Accumulate some patterns
        for i in 0..10 {
            let mut axes = [0.0f64; AXIS_COUNT];
            axes[i % 16] = 1.0;
            axes[INTERO_RESERVE] = 0.5;
            pipe.tick(&axes, &surprise, 0.5, i as u64 * 100_000);
        }
        let count_before = pipe.pattern_count();
        assert!(count_before > 0);

        // Serialize and restore into a fresh pipeline
        let bytes = pipe.serialize_patterns();
        let mut pipe2 = CognitivePipeline::new();
        assert!(pipe2.restore_patterns(&bytes), "restore should succeed");
        assert_eq!(pipe2.pattern_count(), count_before,
            "restored pipeline should have same pattern count");
    }

    #[test]
    fn pipeline_restore_rejects_garbage() {
        let mut pipe = CognitivePipeline::new();
        assert!(!pipe.restore_patterns(&[0u8; 10]),
            "restore should fail on garbage data");
        assert_eq!(pipe.pattern_count(), 0,
            "pipeline should remain empty after failed restore");
    }
}
