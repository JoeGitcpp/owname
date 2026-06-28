use crate::{AXIS_COUNT, AXIS_VIS_BASE, AXIS_INTERO_BASE, AXIS_CODE_BASE, AXIS_GAUSS_BASE,
            VIB_BANDS, VIS_AXES, INTERO_AXES, CODE_AXES,
            GAUSS_SUMMARY_AXES, DEFENSE_SUMMARY_AXES};

pub const CANVAS_DIM: usize = 64;

pub const MODALITY_COUNT: usize = 5;

const MODALITY_SIZES: [usize; MODALITY_COUNT] = [
    VIB_BANDS,         // 16
    VIS_AXES,          // 128
    INTERO_AXES,       // 15
    CODE_AXES,         // 5
    GAUSS_SUMMARY_AXES + DEFENSE_SUMMARY_AXES, // 3
];

const MODALITY_BASES: [usize; MODALITY_COUNT] = [
    0,                 // VIB starts at 0
    AXIS_VIS_BASE,     // 16
    AXIS_INTERO_BASE,  // 144
    AXIS_CODE_BASE,    // 159  (VIB+VIS+INTERO = 16+128+15)
    AXIS_GAUSS_BASE,   // 164  (VIB+VIS+INTERO+CODE = 16+128+15+5)
];

const W_SELF: f64 = 1.2;
const W_INHIB: f64 = 0.15;
const THETA: f64 = 0.3;
const TAU_MS: f64 = 50.0;
const SIGMOID_GAIN: f64 = 20.0;
const PRECISION_EPSILON: f64 = 1e-6;
const IGNITION_CAPACITY: usize = 16;
const PROJ_LR: f64 = 0.001;

pub struct ModalityProjection {
    pub weights: Vec<[f64; CANVAS_DIM]>,
    pub size: usize,
    pub base: usize,
}

impl ModalityProjection {
    pub fn new_random(size: usize, base: usize, seed: u64) -> Self {
        let mut weights = Vec::with_capacity(size);
        let mut rng_state = seed;
        let scale = 1.0 / (size as f64).sqrt();
        for _ in 0..size {
            let mut row = [0.0f64; CANVAS_DIM];
            for j in 0..CANVAS_DIM {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let u = (rng_state >> 33) as f64 / (1u64 << 31) as f64;
                row[j] = (u - 0.5) * 2.0 * scale;
            }
            weights.push(row);
        }
        Self { weights, size, base }
    }

    pub fn project(&self, axes: &[f64; AXIS_COUNT], dev_mask: &[f64; AXIS_COUNT]) -> [f64; CANVAS_DIM] {
        let mut out = [0.0f64; CANVAS_DIM];
        for i in 0..self.size {
            let axis_idx = self.base + i;
            let val = axes[axis_idx] * dev_mask[axis_idx];
            if val.abs() < 1e-12 { continue; }
            for j in 0..CANVAS_DIM {
                out[j] += self.weights[i][j] * val;
            }
        }
        out
    }
}

pub struct SharedCanvas {
    pub projections: [ModalityProjection; MODALITY_COUNT],
    pub activation: [f64; CANVAS_DIM],
    pub modality_precision: [f64; MODALITY_COUNT],
    pub ignited: [bool; CANVAS_DIM],
    pub ignited_count: usize,
}

impl SharedCanvas {
    pub fn new() -> Self {
        let projections = [
            ModalityProjection::new_random(MODALITY_SIZES[0], MODALITY_BASES[0], 0xCAFE_0001),
            ModalityProjection::new_random(MODALITY_SIZES[1], MODALITY_BASES[1], 0xCAFE_0002),
            ModalityProjection::new_random(MODALITY_SIZES[2], MODALITY_BASES[2], 0xCAFE_0003),
            ModalityProjection::new_random(MODALITY_SIZES[3], MODALITY_BASES[3], 0xCAFE_0004),
            ModalityProjection::new_random(MODALITY_SIZES[4], MODALITY_BASES[4], 0xCAFE_0005),
        ];
        Self {
            projections,
            activation: [0.0; CANVAS_DIM],
            modality_precision: [1.0; MODALITY_COUNT],
            ignited: [false; CANVAS_DIM],
            ignited_count: 0,
        }
    }

    pub fn update_precision_from_surprise(&mut self, per_axis_surprise: &[f64; AXIS_COUNT]) {
        for m in 0..MODALITY_COUNT {
            let base = MODALITY_BASES[m];
            let size = MODALITY_SIZES[m];
            let mut sum_var = 0.0f64;
            let mut count = 0usize;
            for i in 0..size {
                let s = per_axis_surprise[base + i];
                if s > 0.0 {
                    sum_var += s;
                    count += 1;
                }
            }
            let mean_var = if count > 0 { sum_var / count as f64 } else { 1.0 };
            self.modality_precision[m] = 1.0 / (mean_var + PRECISION_EPSILON);
        }
    }

    pub fn step(
        &mut self,
        axes: &[f64; AXIS_COUNT],
        dev_mask: &[f64; AXIS_COUNT],
        dt_ms: f64,
    ) -> [f64; CANVAS_DIM] {
        // 1. Project each modality
        let mut z_fused = [0.0f64; CANVAS_DIM];
        let mut precision_sum = 0.0f64;
        for m in 0..MODALITY_COUNT {
            let z_m = self.projections[m].project(axes, dev_mask);
            let p = self.modality_precision[m];
            precision_sum += p;
            for j in 0..CANVAS_DIM {
                z_fused[j] += p * z_m[j];
            }
        }
        if precision_sum > PRECISION_EPSILON {
            for j in 0..CANVAS_DIM {
                z_fused[j] /= precision_sum;
            }
        }

        // 2. Competitive dynamics (one-step Euler)
        let decay = dt_ms / TAU_MS;
        let mean_a: f64 = self.activation.iter().copied().sum::<f64>() / CANVAS_DIM as f64;
        for i in 0..CANVAS_DIM {
            let drive = W_SELF * self.activation[i] - W_INHIB * mean_a + z_fused[i] - THETA;
            let transfer = 1.0 / (1.0 + (-SIGMOID_GAIN * drive).exp());
            self.activation[i] = (1.0 - decay) * self.activation[i] + decay * transfer;
        }

        // 3. Ignition: top-K dimensions broadcast
        let mut indices: Vec<(usize, f64)> = self.activation.iter().copied()
            .enumerate()
            .collect();
        indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        self.ignited = [false; CANVAS_DIM];
        self.ignited_count = 0;
        for &(idx, val) in indices.iter().take(IGNITION_CAPACITY) {
            if val > 0.01 {
                self.ignited[idx] = true;
                self.ignited_count += 1;
            }
        }

        // 4. Broadcast output: ignited dimensions pass through, others zeroed
        let mut output = [0.0f64; CANVAS_DIM];
        for i in 0..CANVAS_DIM {
            if self.ignited[i] {
                output[i] = self.activation[i];
            }
        }
        output
    }

    pub fn broadcast_state(&self) -> &[f64; CANVAS_DIM] {
        &self.activation
    }

    /// §4.5: Update projection matrices from L2 reconstruction error.
    /// dW_m/dt = -α · ∂E_sparse / ∂W_m
    pub fn update_projections(
        &mut self,
        l2_residual: &[f64; CANVAS_DIM],
        axes: &[f64; AXIS_COUNT],
        dev_mask: &[f64; AXIS_COUNT],
    ) {
        let precision_sum: f64 = self.modality_precision.iter().sum();
        if precision_sum < PRECISION_EPSILON { return; }

        for m in 0..MODALITY_COUNT {
            let p_scale = self.modality_precision[m] / precision_sum;
            let proj = &mut self.projections[m];
            for k in 0..proj.size {
                let axis_idx = proj.base + k;
                let x = axes[axis_idx] * dev_mask[axis_idx];
                if x.abs() < 1e-12 { continue; }
                for d in 0..CANVAS_DIM {
                    proj.weights[k][d] -= PROJ_LR * p_scale * l2_residual[d] * x;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_axes() -> [f64; AXIS_COUNT] {
        [0.0; AXIS_COUNT]
    }

    fn ones_mask() -> [f64; AXIS_COUNT] {
        [1.0; AXIS_COUNT]
    }

    #[test]
    fn canvas_initializes_to_zero() {
        let canvas = SharedCanvas::new();
        for &a in canvas.activation.iter() {
            assert!((a - 0.0).abs() < 1e-12);
        }
    }

    #[test]
    fn zero_input_no_ignition() {
        let mut canvas = SharedCanvas::new();
        for _ in 0..50 {
            canvas.step(&zero_axes(), &ones_mask(), 10.0);
        }
        assert_eq!(canvas.ignited_count, 0,
            "zero input should not ignite any dimension");
    }

    #[test]
    fn nonzero_input_produces_activation() {
        let mut canvas = SharedCanvas::new();
        let mut axes = zero_axes();
        // Strong signal across multiple modalities
        for i in 0..crate::VIB_BANDS {
            axes[i] = 2.0;
        }
        for i in crate::AXIS_VIS_BASE..crate::AXIS_VIS_BASE + crate::VIS_AXES {
            axes[i] = 1.0;
        }
        let mask = ones_mask();
        // Many steps: dynamics need time to build up with random projections
        let mut output = [0.0f64; CANVAS_DIM];
        for _ in 0..200 {
            output = canvas.step(&axes, &mask, 10.0);
        }
        let max_act = canvas.activation.iter().cloned().fold(0.0f64, f64::max);
        assert!(max_act > 0.001, "strong input should produce nonzero activation after warmup, max={max_act}");
    }

    #[test]
    fn ignition_respects_capacity() {
        let mut canvas = SharedCanvas::new();
        let mut axes = zero_axes();
        for i in 0..AXIS_COUNT {
            axes[i] = 1.0;
        }
        let mask = ones_mask();
        for _ in 0..50 {
            canvas.step(&axes, &mask, 10.0);
        }
        assert!(canvas.ignited_count <= IGNITION_CAPACITY,
            "ignited count {} should <= {IGNITION_CAPACITY}", canvas.ignited_count);
    }

    #[test]
    fn dev_mask_zeros_suppress_modality() {
        let mut canvas = SharedCanvas::new();
        let mut axes = zero_axes();
        for i in 0..crate::VIB_BANDS {
            axes[i] = 1.0;
        }
        let mut mask = ones_mask();
        for i in 0..crate::VIB_BANDS {
            mask[i] = 0.0;
        }
        for _ in 0..20 {
            canvas.step(&axes, &mask, 10.0);
        }
        assert_eq!(canvas.ignited_count, 0,
            "masked-out modality should not ignite canvas");
    }

    #[test]
    fn precision_updates_from_surprise() {
        let mut canvas = SharedCanvas::new();
        let mut surprise = [0.0f64; AXIS_COUNT];
        // High surprise on VIB → low precision
        for i in 0..crate::VIB_BANDS {
            surprise[i] = 10.0;
        }
        // Low surprise on VIS → high precision
        for i in crate::AXIS_VIS_BASE..crate::AXIS_VIS_BASE + crate::VIS_AXES {
            surprise[i] = 0.01;
        }
        canvas.update_precision_from_surprise(&surprise);
        const MODALITY_VIB: usize = 0;
        const MODALITY_VIS: usize = 1;
        assert!(canvas.modality_precision[MODALITY_VIS] > canvas.modality_precision[MODALITY_VIB],
            "low surprise modality should have higher precision");
    }

    #[test]
    fn modality_projection_sizes_match_axis_layout() {
        let total: usize = MODALITY_SIZES.iter().sum();
        assert_eq!(total, AXIS_COUNT, "modality sizes should sum to AXIS_COUNT={AXIS_COUNT}, got {total}");
    }

    #[test]
    fn modality_bases_are_contiguous() {
        for m in 1..MODALITY_COUNT {
            assert_eq!(MODALITY_BASES[m], MODALITY_BASES[m-1] + MODALITY_SIZES[m-1],
                "modality {m} base should be {}, got {}",
                MODALITY_BASES[m-1] + MODALITY_SIZES[m-1], MODALITY_BASES[m]);
        }
    }

    #[test]
    fn activation_bounded_under_strong_input() {
        let mut canvas = SharedCanvas::new();
        let mut axes = [2.0f64; AXIS_COUNT];
        let mask = ones_mask();
        for _ in 0..500 {
            canvas.step(&axes, &mask, 10.0);
        }
        for (i, &a) in canvas.activation.iter().enumerate() {
            assert!(a.is_finite(), "activation[{i}] must be finite, got {a}");
            assert!(a <= 1.0 + 1e-6, "activation[{i}] must be <= 1.0 (sigmoid bound), got {a}");
        }
    }

    #[test]
    fn ignition_stabilizes_with_consistent_input() {
        let mut canvas = SharedCanvas::new();
        let mut axes = [0.0f64; AXIS_COUNT];
        for i in 0..crate::VIB_BANDS { axes[i] = 1.5; }
        for i in crate::AXIS_VIS_BASE..crate::AXIS_VIS_BASE + crate::VIS_AXES {
            axes[i] = 0.8;
        }
        let mask = ones_mask();
        let mut counts = Vec::new();
        for t in 0..200 {
            canvas.step(&axes, &mask, 10.0);
            if t >= 150 {
                counts.push(canvas.ignited_count);
            }
        }
        let min_c = *counts.iter().min().unwrap();
        let max_c = *counts.iter().max().unwrap();
        assert!(max_c - min_c <= 2,
            "ignited count should stabilize (range {min_c}..{max_c} over last 50 ticks)");
    }

    #[test]
    fn projection_learning_changes_weights() {
        let mut canvas = SharedCanvas::new();
        let axes = ones_mask();
        let mask = ones_mask();
        for _ in 0..50 {
            canvas.step(&axes, &mask, 10.0);
        }
        let w_before = canvas.projections[0].weights[0].clone();

        let mut residual = [0.0f64; CANVAS_DIM];
        residual[0] = 0.5;
        canvas.update_projections(&residual, &axes, &mask);

        let mut changed = false;
        for d in 0..CANVAS_DIM {
            if (canvas.projections[0].weights[0][d] - w_before[d]).abs() > 1e-15 {
                changed = true;
                break;
            }
        }
        assert!(changed, "L2 reconstruction residual should update L1 projection weights");
    }
}
