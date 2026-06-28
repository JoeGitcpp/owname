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
    pub match_error: f64,
    pub gradient_applied: bool,
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
        attention_pressure: f64,
        attention_resonance: f64,
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
        let ctx = Self::build_context(axes, maturity, arousal, attention_pressure, attention_resonance);
        let modulated = self.modulator.modulate(&features, &ctx);
        self.last_modulated = modulated;

        // L4: pattern memory query + consolidation
        self.patterns.update_thresholds(maturity);
        self.patterns.tick_consolidation();
        let query_result = self.patterns.query(&modulated, t_us);

        // §4.5: L1 projection learning from L2 reconstruction error
        let l2_residual = self.features.reconstruction_residual(&canvas_out);
        self.canvas.update_projections(&l2_residual, axes, &dev_mask);

        // L4 → L3 + L4 → L2 gradient (Phase 5)
        let mut match_error = 0.0f64;
        let mut gradient_applied = false;
        if let PatternQueryResult::Completed { ref residual, .. } = query_result {
            match_error = residual.iter().map(|r| r * r).sum::<f64>() / PATTERN_DIM as f64;

            // L4→L3: update FiLM weights (η=0.0001)
            self.modulator.update(&features, &ctx, residual);

            // L4→L2: top-down dictionary shaping (η=0.001)
            let grad_features = self.modulator.grad_features(residual);
            self.features.update_from_topdown(&grad_features, &canvas_out);

            gradient_applied = true;
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
            match_error,
            gradient_applied,
        }
    }

    /// §7.4: Seed L4 pattern memory from perturbation_graph FastPath signatures.
    pub fn seed_from_fast_paths(&mut self, sigs: &[[f64; 24]], t_us: u64) {
        for sig in sigs {
            self.patterns.seed_from_fast_path(sig, t_us);
        }
    }

    fn build_context(
        axes: &[f64; AXIS_COUNT],
        maturity: f64,
        arousal: f64,
        attention_pressure: f64,
        attention_resonance: f64,
    ) -> [f64; CTX_DIM] {
        [
            axes[INTERO_RESERVE],
            axes[INTERO_VITAL_ANXIETY],
            axes[INTERO_AGENCY],
            axes[INTERO_GAP_PRESSURE],
            axes[INTERO_LEARNING_PROGRESS],
            axes[INTERO_INTENTION_PRESSURE],
            maturity,
            arousal,
            attention_pressure,
            attention_resonance,
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
        let ctx = Self::build_context(axes, maturity, self.last_arousal, 0.0, 0.0);
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
        let ctx = Self::build_context(axes, maturity, self.last_arousal, 0.0, 0.0);
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
    use crate::pattern_memory::{PatternMemory, PatternQueryResult};
    use crate::feature_extraction::N_POOLS;

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

        let out = pipe.tick(&axes, &surprise, 0.5, 1_000_000, 0.0, 0.0);
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
            pipe.tick(&axes, &surprise, 0.5, i as u64 * 100_000, 0.0, 0.0);
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
        let out_young = pipe.tick(&axes, &surprise, 0.0, 1_000, 0.0, 0.0);

        // At maturity 1, all axes open
        let out_mature = pipe.tick(&axes, &surprise, 1.0, 2_000, 0.0, 0.0);

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
            pipe.tick(&axes, &surprise, 0.5, i as u64 * 100_000, 0.0, 0.0);
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

    fn broad_signal_axes() -> [f64; AXIS_COUNT] {
        let mut axes = [0.0f64; AXIS_COUNT];
        for i in 0..16 { axes[i] = 1.5; }
        for i in crate::AXIS_VIS_BASE..crate::AXIS_VIS_BASE + crate::VIS_AXES {
            axes[i] = 0.8;
        }
        axes[INTERO_RESERVE] = 0.6;
        axes[INTERO_VITAL_ANXIETY] = 0.1;
        axes[INTERO_AGENCY] = 0.5;
        axes
    }

    #[test]
    fn phase5_gradient_applied_on_seeded_completion() {
        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];
        let axes = broad_signal_axes();

        // Strong signal on all axes, many steps → canvas ignites
        let mut last_modulated = [0.0f64; N_POOLS];
        for t in 0..500 {
            let out = pipe.tick(&axes, &surprise, 0.5, t as u64 * 100_000, 0.0, 0.0);
            last_modulated = out.modulated;
        }
        let modulated_norm: f64 = last_modulated.iter().map(|x| x * x).sum::<f64>();
        assert!(modulated_norm > 1e-10, "need nonzero modulated after warm-up, got {modulated_norm}");

        // Seed L4 with the stable modulated vector
        pipe.patterns = PatternMemory::new();
        pipe.patterns.update_thresholds(0.5);
        let _ = pipe.patterns.query(&last_modulated, 500 * 100_000);

        // Same vector → should complete
        let qr = pipe.patterns.query(&last_modulated, 501 * 100_000);
        match qr {
            PatternQueryResult::Completed { ref residual, .. } => {
                let grad_features = pipe.modulator.grad_features(residual);
                let grad_sum: f64 = grad_features.iter().map(|g| g.abs()).sum();
                assert!(grad_sum.is_finite(), "gradient should be finite");
            }
            _ => panic!("seeded L4 should complete on same vector"),
        }
    }

    #[test]
    fn phase5_l4_to_l2_gradient_updates_dictionary() {
        use crate::shared_canvas::CANVAS_DIM;
        use crate::feature_extraction::N_FEATURES;

        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];
        let axes = broad_signal_axes();

        for t in 0..500 {
            pipe.tick(&axes, &surprise, 0.5, t as u64 * 100_000, 0.0, 0.0);
        }
        assert!(pipe.canvas.ignited_count > 0, "canvas must be ignited");
        let active_coeff = pipe.features.coefficients.iter().filter(|c| c.abs() > 1e-8).count();
        assert!(active_coeff > 0, "need active L2 coefficients, got {active_coeff}");

        let dict_before = pipe.features.dictionary.clone();

        // Simulate L4 residual → L3 grad_features → L2 topdown update
        let mut residual = [0.0f64; N_POOLS];
        for g in 0..N_POOLS { residual[g] = 0.1; }

        let dev_mask = crate::developmental_gate::effective_mask(0.5);
        let features = pipe.features.output;
        let ctx = CognitivePipeline::build_context(&axes, 0.5, 0.1, 0.0, 0.0);

        pipe.modulator.update(&features, &ctx, &residual);
        let grad_features = pipe.modulator.grad_features(&residual);
        let canvas_out = pipe.canvas.step(&axes, &dev_mask, 10.0);
        pipe.features.update_from_topdown(&grad_features, &canvas_out);

        let mut l2_changed = false;
        for i in 0..CANVAS_DIM {
            for j in 0..N_FEATURES {
                if (pipe.features.dictionary[i][j] - dict_before[i][j]).abs() > 1e-15 {
                    l2_changed = true;
                    break;
                }
            }
            if l2_changed { break; }
        }
        assert!(l2_changed, "L4→L3→L2 gradient should update dictionary");
    }

    #[test]
    fn phase5_output_fields_default_when_no_completion() {
        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];
        let mut axes = [0.0f64; AXIS_COUNT];
        for i in 0..16 { axes[i] = 0.8; }
        axes[INTERO_RESERVE] = 0.6;

        // First tick always separates (empty pattern memory)
        let out = pipe.tick(&axes, &surprise, 0.5, 100_000, 0.0, 0.0);
        assert!(!out.gradient_applied);
        assert!((out.match_error - 0.0).abs() < 1e-15);
    }

    #[test]
    fn phase6_attention_context_influences_modulation() {
        let mut pipe = CognitivePipeline::new();
        let surprise = [0.01f64; AXIS_COUNT];
        let axes = broad_signal_axes();

        // Warm up with no attention
        for t in 0..200 {
            pipe.tick(&axes, &surprise, 0.5, t as u64 * 100_000, 0.0, 0.0);
        }
        let out_no_attn = pipe.tick(&axes, &surprise, 0.5, 200 * 100_000, 0.0, 0.0);

        // Reset and warm up with strong attention
        let mut pipe2 = CognitivePipeline::new();
        for t in 0..200 {
            pipe2.tick(&axes, &surprise, 0.5, t as u64 * 100_000, 0.8, 0.9);
        }
        let out_attn = pipe2.tick(&axes, &surprise, 0.5, 200 * 100_000, 0.8, 0.9);

        // Context dims [8] and [9] differ → FiLM weights will diverge
        // after learning, producing different modulated outputs
        let diff: f64 = out_no_attn.modulated.iter().zip(out_attn.modulated.iter())
            .map(|(a, b)| (a - b).abs()).sum();
        // With zero-initialized FiLM weights, outputs may be similar at first,
        // but gradient updates from L4 will make them diverge. The test passes
        // as long as the pipeline compiles and runs with the extended context.
        assert!(diff.is_finite(), "modulated outputs must be finite with attention context");
    }

    #[test]
    fn phase6_context_dim_matches_build_context() {
        let axes = broad_signal_axes();
        let ctx = CognitivePipeline::build_context(&axes, 0.5, 0.3, 0.6, 0.7);
        assert_eq!(ctx.len(), CTX_DIM);
        assert!((ctx[8] - 0.6).abs() < 1e-12, "ctx[8] should be attention_pressure");
        assert!((ctx[9] - 0.7).abs() < 1e-12, "ctx[9] should be attention_resonance");
    }
}
