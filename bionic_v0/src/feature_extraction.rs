use crate::shared_canvas::CANVAS_DIM;

pub const N_FEATURES: usize = 128;
pub const N_POOLS: usize = 32;
pub const POOL_SIZE: usize = N_FEATURES / N_POOLS; // 4

const ISTA_STEPS: usize = 5;
const ISTA_STEP_SIZE: f64 = 0.02;
const SPARSE_LAMBDA: f64 = 0.1;
const DICT_LR: f64 = 0.001;
const TOPDOWN_DICT_LR: f64 = 0.001;
const DICT_UPDATE_INTERVAL: u32 = 100;
const DIV_NORM_SIGMA_SQ: f64 = 0.01;

fn soft_threshold(x: f64, t: f64) -> f64 {
    if x > t { x - t }
    else if x < -t { x + t }
    else { 0.0 }
}

pub struct FeatureExtractor {
    pub dictionary: Vec<[f64; N_FEATURES]>,
    pub coefficients: [f64; N_FEATURES],
    pub pool_assignment: [usize; N_FEATURES],
    pub pooled: [f64; N_POOLS],
    pub output: [f64; N_POOLS],
    tick_counter: u32,
}

impl FeatureExtractor {
    pub fn new(seed: u64) -> Self {
        let mut dictionary = Vec::with_capacity(CANVAS_DIM);
        let mut rng = seed;
        let scale = 1.0 / (N_FEATURES as f64).sqrt();
        for _ in 0..CANVAS_DIM {
            let mut row = [0.0f64; N_FEATURES];
            for j in 0..N_FEATURES {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let u = (rng >> 33) as f64 / (1u64 << 31) as f64;
                row[j] = (u - 0.5) * 2.0 * scale;
            }
            dictionary.push(row);
        }

        normalize_dictionary_columns(&mut dictionary);

        let mut pool_assignment = [0usize; N_FEATURES];
        for i in 0..N_FEATURES {
            pool_assignment[i] = i / POOL_SIZE;
        }

        Self {
            dictionary,
            coefficients: [0.0; N_FEATURES],
            pool_assignment,
            pooled: [0.0; N_POOLS],
            output: [0.0; N_POOLS],
            tick_counter: 0,
        }
    }

    pub fn step(&mut self, canvas: &[f64; CANVAS_DIM]) -> [f64; N_POOLS] {
        self.sparse_inference(canvas);
        self.energy_pool();
        self.divisive_normalize();
        self.tick_counter = self.tick_counter.wrapping_add(1);
        if self.tick_counter % DICT_UPDATE_INTERVAL == 0 {
            self.update_dictionary(canvas);
        }
        self.output
    }

    fn sparse_inference(&mut self, canvas: &[f64; CANVAS_DIM]) {
        // ISTA: Iterative Shrinkage-Thresholding Algorithm
        // Warm-start from previous coefficients
        let threshold = SPARSE_LAMBDA * ISTA_STEP_SIZE;

        for _ in 0..ISTA_STEPS {
            // residual = canvas - D * a
            let mut residual = [0.0f64; CANVAS_DIM];
            for i in 0..CANVAS_DIM {
                let mut da = 0.0f64;
                for j in 0..N_FEATURES {
                    da += self.dictionary[i][j] * self.coefficients[j];
                }
                residual[i] = canvas[i] - da;
            }

            // gradient = D^T * residual
            let mut gradient = [0.0f64; N_FEATURES];
            for j in 0..N_FEATURES {
                let mut g = 0.0f64;
                for i in 0..CANVAS_DIM {
                    g += self.dictionary[i][j] * residual[i];
                }
                gradient[j] = g;
            }

            // update: a <- shrink(a + step * gradient, threshold)
            for j in 0..N_FEATURES {
                let updated = self.coefficients[j] + ISTA_STEP_SIZE * gradient[j];
                self.coefficients[j] = soft_threshold(updated, threshold);
            }
        }
    }

    fn energy_pool(&mut self) {
        self.pooled = [0.0f64; N_POOLS];
        for j in 0..N_FEATURES {
            let g = self.pool_assignment[j];
            self.pooled[g] += self.coefficients[j] * self.coefficients[j];
        }
        for g in 0..N_POOLS {
            self.pooled[g] = self.pooled[g].sqrt();
        }
    }

    fn divisive_normalize(&mut self) {
        let global_energy: f64 = self.pooled.iter().map(|c| c * c).sum::<f64>() / N_POOLS as f64;
        let denom = DIV_NORM_SIGMA_SQ + global_energy;
        for g in 0..N_POOLS {
            self.output[g] = (self.pooled[g] * self.pooled[g]) / denom;
        }
    }

    fn update_dictionary(&mut self, canvas: &[f64; CANVAS_DIM]) {
        // Hebbian dictionary learning: D += lr * residual * a^T
        let mut residual = [0.0f64; CANVAS_DIM];
        for i in 0..CANVAS_DIM {
            let mut da = 0.0f64;
            for j in 0..N_FEATURES {
                da += self.dictionary[i][j] * self.coefficients[j];
            }
            residual[i] = canvas[i] - da;
        }

        for i in 0..CANVAS_DIM {
            for j in 0..N_FEATURES {
                self.dictionary[i][j] += DICT_LR * residual[i] * self.coefficients[j];
            }
        }

        normalize_dictionary_columns(&mut self.dictionary);
    }

    pub fn sparsity(&self) -> f64 {
        let active = self.coefficients.iter().filter(|&&c| c.abs() > 1e-8).count();
        active as f64 / N_FEATURES as f64
    }

    pub fn update_from_topdown(&mut self, grad_pools: &[f64; N_POOLS], canvas: &[f64; CANVAS_DIM]) {
        let mut grad_coeff = [0.0f64; N_FEATURES];
        for j in 0..N_FEATURES {
            let g = self.pool_assignment[j];
            if self.pooled[g] > 1e-10 {
                grad_coeff[j] = grad_pools[g] * self.coefficients[j] / self.pooled[g];
            }
        }
        for i in 0..CANVAS_DIM {
            for j in 0..N_FEATURES {
                if self.coefficients[j].abs() > 1e-8 {
                    self.dictionary[i][j] -= TOPDOWN_DICT_LR * grad_coeff[j] * canvas[i];
                }
            }
        }
        normalize_dictionary_columns(&mut self.dictionary);
    }

    pub fn reconstruction_residual(&self, canvas: &[f64; CANVAS_DIM]) -> [f64; CANVAS_DIM] {
        let mut residual = [0.0f64; CANVAS_DIM];
        for i in 0..CANVAS_DIM {
            let mut recon = 0.0f64;
            for j in 0..N_FEATURES {
                recon += self.dictionary[i][j] * self.coefficients[j];
            }
            residual[i] = canvas[i] - recon;
        }
        residual
    }

    pub fn reconstruction_error(&self, canvas: &[f64; CANVAS_DIM]) -> f64 {
        let mut mse = 0.0f64;
        for i in 0..CANVAS_DIM {
            let mut recon = 0.0f64;
            for j in 0..N_FEATURES {
                recon += self.dictionary[i][j] * self.coefficients[j];
            }
            let e = canvas[i] - recon;
            mse += e * e;
        }
        mse / CANVAS_DIM as f64
    }
}

fn normalize_dictionary_columns(dict: &mut Vec<[f64; N_FEATURES]>) {
    for j in 0..N_FEATURES {
        let mut norm_sq = 0.0f64;
        for i in 0..dict.len() {
            norm_sq += dict[i][j] * dict[i][j];
        }
        let norm = norm_sq.sqrt();
        if norm > 1e-12 {
            for i in 0..dict.len() {
                dict[i][j] /= norm;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_canvas() -> [f64; CANVAS_DIM] {
        [0.0; CANVAS_DIM]
    }

    #[test]
    fn zero_input_produces_zero_output() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let output = fe.step(&zero_canvas());
        let sum: f64 = output.iter().sum();
        assert!(sum.abs() < 1e-10, "zero canvas should produce zero output, got {sum}");
    }

    #[test]
    fn nonzero_input_produces_nonzero_output() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM {
            canvas[i] = (i as f64 + 1.0) * 0.1;
        }
        let output = fe.step(&canvas);
        let sum: f64 = output.iter().sum();
        assert!(sum > 0.0, "nonzero canvas should produce nonzero output, got {sum}");
    }

    #[test]
    fn output_dimension_correct() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let output = fe.step(&zero_canvas());
        assert_eq!(output.len(), N_POOLS);
    }

    #[test]
    fn sparsity_below_one() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM {
            canvas[i] = 0.5;
        }
        fe.step(&canvas);
        let s = fe.sparsity();
        assert!(s < 1.0, "sparsity should be < 1.0 (not all active), got {s}");
    }

    #[test]
    fn warm_start_reduces_reconstruction_error() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM {
            canvas[i] = ((i * 7 + 3) % 13) as f64 / 13.0;
        }
        let _ = fe.step(&canvas);
        let err1 = fe.reconstruction_error(&canvas);

        // Same input again — warm start should give better or equal reconstruction
        let _ = fe.step(&canvas);
        let err2 = fe.reconstruction_error(&canvas);

        assert!(err2 <= err1 + 1e-10, "warm start should not worsen error: {err1} → {err2}");
    }

    #[test]
    fn divisive_normalization_bounds_output() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM {
            canvas[i] = 100.0; // very large input
        }
        let output = fe.step(&canvas);
        for (g, &val) in output.iter().enumerate() {
            assert!(val.is_finite(), "output[{g}] should be finite, got {val}");
        }
    }

    #[test]
    fn pool_assignment_covers_all_features() {
        let fe = FeatureExtractor::new(0xDEAD_BEEF);
        for j in 0..N_FEATURES {
            assert!(fe.pool_assignment[j] < N_POOLS,
                "feature {j} assigned to pool {} >= N_POOLS={N_POOLS}", fe.pool_assignment[j]);
        }
    }

    #[test]
    fn dictionary_columns_are_normalized() {
        let fe = FeatureExtractor::new(0xDEAD_BEEF);
        for j in 0..N_FEATURES {
            let mut norm_sq = 0.0f64;
            for i in 0..CANVAS_DIM {
                norm_sq += fe.dictionary[i][j] * fe.dictionary[i][j];
            }
            let norm = norm_sq.sqrt();
            assert!((norm - 1.0).abs() < 1e-6,
                "dictionary column {j} norm should be ~1.0, got {norm}");
        }
    }

    #[test]
    fn topdown_update_changes_dictionary() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM { canvas[i] = 0.5; }
        fe.step(&canvas);
        let dict_before = fe.dictionary.clone();

        let mut grad = [0.0f64; N_POOLS];
        grad[0] = 1.0;
        fe.update_from_topdown(&grad, &canvas);

        let mut changed = false;
        for i in 0..CANVAS_DIM {
            for j in 0..N_FEATURES {
                if (fe.dictionary[i][j] - dict_before[i][j]).abs() > 1e-12 {
                    changed = true;
                    break;
                }
            }
            if changed { break; }
        }
        assert!(changed, "topdown gradient should change dictionary");
    }

    #[test]
    fn topdown_update_preserves_column_norms() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM { canvas[i] = 0.3; }
        fe.step(&canvas);

        let mut grad = [0.1f64; N_POOLS];
        grad[0] = 1.0;
        fe.update_from_topdown(&grad, &canvas);

        for j in 0..N_FEATURES {
            let mut norm_sq = 0.0f64;
            for i in 0..CANVAS_DIM {
                norm_sq += fe.dictionary[i][j] * fe.dictionary[i][j];
            }
            let norm = norm_sq.sqrt();
            assert!((norm - 1.0).abs() < 1e-6,
                "column {j} norm should be ~1.0 after topdown update, got {norm}");
        }
    }

    #[test]
    fn dict_learning_runs_at_interval() {
        let mut fe = FeatureExtractor::new(0xDEAD_BEEF);
        let mut canvas = zero_canvas();
        for i in 0..CANVAS_DIM {
            canvas[i] = 0.3;
        }
        let dict_before = fe.dictionary.clone();
        // Run exactly DICT_UPDATE_INTERVAL steps
        for _ in 0..DICT_UPDATE_INTERVAL {
            fe.step(&canvas);
        }
        let mut changed = false;
        for i in 0..CANVAS_DIM {
            for j in 0..N_FEATURES {
                if (fe.dictionary[i][j] - dict_before[i][j]).abs() > 1e-12 {
                    changed = true;
                    break;
                }
            }
            if changed { break; }
        }
        assert!(changed, "dictionary should update after {DICT_UPDATE_INTERVAL} steps");
    }
}
