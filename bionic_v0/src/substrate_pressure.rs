fn unit(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PressureSample {
    pub err: f64,
    pub recurrence: f64,
    pub ambiguity: f64,
    pub alignment: f64,
    pub support: f64,
    pub deviation: f64,
    pub temporal_gap: f64,
    pub cross_channel: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureWeights {
    pub err: f64,
    pub recurrence: f64,
    pub ambiguity: f64,
    pub alignment: f64,
    pub support: f64,
    pub deviation: f64,
    pub temporal_gap: f64,
    pub cross_channel: f64,
    pub multiplier: f64,
}

impl Default for PressureWeights {
    fn default() -> Self {
        Self {
            err: 0.20,
            recurrence: 0.10,
            ambiguity: 0.15,
            alignment: 0.15,
            support: 0.10,
            deviation: 0.15,
            temporal_gap: 0.10,
            cross_channel: 0.05,
            multiplier: 1.0,
        }
    }
}

pub fn compute_pressure(sample: PressureSample, weights: &PressureWeights) -> f64 {
    let weighted = unit(sample.err) * weights.err
        + unit(sample.recurrence) * weights.recurrence
        + unit(sample.ambiguity) * weights.ambiguity
        + unit(sample.alignment) * weights.alignment
        + unit(sample.support) * weights.support
        + unit(sample.deviation) * weights.deviation
        + unit(sample.temporal_gap) * weights.temporal_gap
        + unit(sample.cross_channel) * weights.cross_channel;
    (weighted * weights.multiplier.max(0.0)).clamp(0.0, 2.0)
}
