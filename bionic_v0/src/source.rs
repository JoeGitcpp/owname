use std::f64::consts::TAU;

pub struct SyntheticSource {
    pub t0_us: u64,
}

impl SyntheticSource {
    pub fn new(t0_us: u64) -> Self {
        Self { t0_us }
    }

    pub fn sample(&self, t_us: u64) -> f64 {
        let t = (t_us.saturating_sub(self.t0_us)) as f64 * 1e-6;
        let cycle = 30.0;
        let phase = t.rem_euclid(cycle);

        let base = (TAU * 2.0 * t).sin();

        if phase < 10.0 {
            base
        } else if phase < 12.0 {
            base + 1.2
        } else if phase < 17.0 {
            (TAU * 3.5 * t).sin()
        } else if phase < 22.0 {
            base
        } else if phase < 27.0 {
            2.0 * base
        } else {
            base
        }
    }

    pub fn phase_id(&self, t_us: u64) -> u32 {
        let t = (t_us.saturating_sub(self.t0_us)) as f64 * 1e-6;
        let cycle = 30.0;
        let phase = t.rem_euclid(cycle);
        if phase < 10.0 {
            0
        } else if phase < 12.0 {
            1
        } else if phase < 17.0 {
            2
        } else if phase < 22.0 {
            3
        } else if phase < 27.0 {
            4
        } else {
            5
        }
    }
}
