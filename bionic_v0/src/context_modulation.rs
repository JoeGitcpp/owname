use crate::feature_extraction::N_POOLS;

pub const CTX_DIM: usize = 10;
const FILM_HIDDEN: usize = 16;
const FILM_LR: f64 = 0.0001;

pub struct ContextModulator {
    pub w1: [[f64; CTX_DIM]; FILM_HIDDEN],
    pub b1: [f64; FILM_HIDDEN],
    pub w2_gamma: [[f64; FILM_HIDDEN]; N_POOLS],
    pub w2_beta: [[f64; FILM_HIDDEN]; N_POOLS],
    hidden: [f64; FILM_HIDDEN],
    pub gamma: [f64; N_POOLS],
    pub beta: [f64; N_POOLS],
}

impl ContextModulator {
    pub fn new() -> Self {
        Self {
            w1: [[0.0; CTX_DIM]; FILM_HIDDEN],
            b1: [0.0; FILM_HIDDEN],
            w2_gamma: [[0.0; FILM_HIDDEN]; N_POOLS],
            w2_beta: [[0.0; FILM_HIDDEN]; N_POOLS],
            hidden: [0.0; FILM_HIDDEN],
            gamma: [1.0; N_POOLS],
            beta: [0.0; N_POOLS],
        }
    }

    pub fn modulate(
        &mut self,
        features: &[f64; N_POOLS],
        context: &[f64; CTX_DIM],
    ) -> [f64; N_POOLS] {
        // hidden = ReLU(W1 · context + b1)
        for h in 0..FILM_HIDDEN {
            let mut sum = self.b1[h];
            for c in 0..CTX_DIM {
                sum += self.w1[h][c] * context[c];
            }
            self.hidden[h] = sum.max(0.0);
        }

        // γ = 1 + W2_γ · hidden   (centered at 1)
        // β = W2_β · hidden        (centered at 0)
        for g in 0..N_POOLS {
            let mut sum_gamma = 0.0f64;
            let mut sum_beta = 0.0f64;
            for h in 0..FILM_HIDDEN {
                sum_gamma += self.w2_gamma[g][h] * self.hidden[h];
                sum_beta += self.w2_beta[g][h] * self.hidden[h];
            }
            self.gamma[g] = 1.0 + sum_gamma;
            self.beta[g] = sum_beta;
        }

        // modulated = γ ⊙ features + β
        let mut output = [0.0f64; N_POOLS];
        for g in 0..N_POOLS {
            output[g] = self.gamma[g] * features[g] + self.beta[g];
        }
        output
    }

    pub fn update(
        &mut self,
        features: &[f64; N_POOLS],
        context: &[f64; CTX_DIM],
        grad_output: &[f64; N_POOLS],
    ) {
        // Backprop through FiLM:
        // d_loss/d_gamma_g = grad_output_g * features_g
        // d_loss/d_beta_g  = grad_output_g
        let mut grad_gamma = [0.0f64; N_POOLS];
        let mut grad_beta = [0.0f64; N_POOLS];
        for g in 0..N_POOLS {
            grad_gamma[g] = grad_output[g] * features[g];
            grad_beta[g] = grad_output[g];
        }

        // d_loss/d_hidden via W2
        let mut grad_hidden = [0.0f64; FILM_HIDDEN];
        for h in 0..FILM_HIDDEN {
            for g in 0..N_POOLS {
                grad_hidden[h] += self.w2_gamma[g][h] * grad_gamma[g];
                grad_hidden[h] += self.w2_beta[g][h] * grad_beta[g];
            }
            // ReLU derivative
            if self.hidden[h] <= 0.0 {
                grad_hidden[h] = 0.0;
            }
        }

        // Update W2_gamma, W2_beta
        for g in 0..N_POOLS {
            for h in 0..FILM_HIDDEN {
                self.w2_gamma[g][h] -= FILM_LR * grad_gamma[g] * self.hidden[h];
                self.w2_beta[g][h] -= FILM_LR * grad_beta[g] * self.hidden[h];
            }
        }

        // Update W1, b1
        for h in 0..FILM_HIDDEN {
            for c in 0..CTX_DIM {
                self.w1[h][c] -= FILM_LR * grad_hidden[h] * context[c];
            }
            self.b1[h] -= FILM_LR * grad_hidden[h];
        }
    }

    pub fn grad_features(&self, grad_output: &[f64; N_POOLS]) -> [f64; N_POOLS] {
        let mut grad = [0.0f64; N_POOLS];
        for g in 0..N_POOLS {
            grad[g] = self.gamma[g] * grad_output[g];
        }
        grad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_ctx() -> [f64; CTX_DIM] { [0.0f64; CTX_DIM] }
    fn zero_features() -> [f64; N_POOLS] { [0.0; N_POOLS] }

    #[test]
    fn identity_at_init() {
        let mut cm = ContextModulator::new();
        let features = [0.5f64; N_POOLS];
        let output = cm.modulate(&features, &zero_ctx());
        for g in 0..N_POOLS {
            assert!((output[g] - features[g]).abs() < 1e-10,
                "initial modulation should be identity: feature={}, output={}", features[g], output[g]);
        }
    }

    #[test]
    fn gamma_centered_at_one() {
        let cm = ContextModulator::new();
        for g in 0..N_POOLS {
            assert!((cm.gamma[g] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn beta_centered_at_zero() {
        let cm = ContextModulator::new();
        for g in 0..N_POOLS {
            assert!(cm.beta[g].abs() < 1e-10);
        }
    }

    #[test]
    fn zero_features_give_beta_only() {
        let mut cm = ContextModulator::new();
        // Manually set w2_beta to nonzero
        cm.w2_beta[0][0] = 1.0;
        cm.w1[0][0] = 1.0;
        cm.b1[0] = 0.1;

        let mut ctx = [0.0f64; CTX_DIM];
        ctx[0] = 1.0;
        let output = cm.modulate(&zero_features(), &ctx);
        assert!(output[0].abs() > 0.0, "nonzero beta should produce nonzero output even with zero features");
    }

    #[test]
    fn different_contexts_produce_different_outputs() {
        let mut cm = ContextModulator::new();
        // Set some weights so context matters
        for h in 0..FILM_HIDDEN {
            cm.w1[h][0] = 0.5;
            cm.b1[h] = 0.1;
        }
        for g in 0..N_POOLS {
            cm.w2_gamma[g][0] = 0.1;
        }

        let features = [1.0f64; N_POOLS];
        let ctx_a = [0.0f64; CTX_DIM];
        let mut ctx_b = [0.0f64; CTX_DIM];
        ctx_b[0] = 5.0;

        let out_a = cm.modulate(&features, &ctx_a);
        let out_b = cm.modulate(&features, &ctx_b);

        let mut diff = 0.0f64;
        for g in 0..N_POOLS {
            diff += (out_a[g] - out_b[g]).abs();
        }
        assert!(diff > 0.01, "different contexts should produce different outputs, diff={diff}");
    }

    #[test]
    fn update_changes_weights() {
        let mut cm = ContextModulator::new();
        let features = [1.0f64; N_POOLS];
        let ctx = [1.0f64; CTX_DIM]; // all ones context for update test
        let grad = [1.0f64; N_POOLS];

        // Need nonzero hidden for gradient to flow
        cm.b1 = [0.1; FILM_HIDDEN];
        cm.modulate(&features, &ctx);
        let w2g_before = cm.w2_gamma[0][0];

        cm.update(&features, &ctx, &grad);
        assert!((cm.w2_gamma[0][0] - w2g_before).abs() > 1e-12,
            "update should change weights");
    }

    #[test]
    fn modulation_is_multiplicative() {
        let mut cm = ContextModulator::new();
        // Set gamma to 2.0 for all pools
        for g in 0..N_POOLS {
            cm.w2_gamma[g][0] = 1.0; // will produce gamma = 1 + hidden[0]
        }
        cm.w1[0][0] = 1.0;
        cm.b1[0] = 0.0;

        let features = [3.0f64; N_POOLS];
        let mut ctx = [0.0f64; CTX_DIM];
        ctx[0] = 1.0;
        let output = cm.modulate(&features, &ctx);
        // gamma = 1 + 1.0 * (1.0*1.0+0.0) = 2.0, beta = 0
        // output = 2.0 * 3.0 = 6.0
        assert!((output[0] - 6.0).abs() < 1e-6, "gamma=2 * feature=3 should = 6, got {}", output[0]);
    }

    #[test]
    fn grad_features_scales_by_gamma() {
        let mut cm = ContextModulator::new();
        cm.w2_gamma[0][0] = 1.0;
        cm.w1[0][0] = 1.0;
        let mut ctx = [0.0f64; CTX_DIM];
        ctx[0] = 1.0;
        let features = [1.0f64; N_POOLS];
        cm.modulate(&features, &ctx);
        // gamma[0] = 1 + hidden[0] = 1 + 1.0 = 2.0

        let grad_output = [1.0f64; N_POOLS];
        let grad = cm.grad_features(&grad_output);
        assert!((grad[0] - 2.0).abs() < 1e-6,
            "grad_features[0] should = gamma[0] * grad_output[0] = 2.0, got {}", grad[0]);
    }

    #[test]
    fn grad_features_identity_at_init() {
        let cm = ContextModulator::new();
        let grad_output = [0.5f64; N_POOLS];
        let grad = cm.grad_features(&grad_output);
        for g in 0..N_POOLS {
            assert!((grad[g] - 0.5).abs() < 1e-10,
                "at init gamma=1, grad should pass through");
        }
    }
}
