#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalImitationState {
    pub imitation_pressure: f64,
    pub suppression: f64,
    pub last_gap: f64,
    pub converge_count: u32,
    pub fail_count: u32,
    pub last_update_t_us: u64,
}

impl VocalImitationState {
    pub fn observe_gap(&mut self, now_us: u64, gap: f64, suppression: f64) {
        self.last_gap = gap;
        self.suppression = suppression;
        self.last_update_t_us = now_us;

        if gap > 0.5 {
            self.imitation_pressure += gap * 0.3;
            self.fail_count += 1;
            self.converge_count = 0;
        } else {
            self.imitation_pressure *= 0.7;
            self.converge_count += 1;
            if gap < 0.2 && self.converge_count >= 3 {
                self.imitation_pressure = 0.0;
            }
        }
        self.imitation_pressure = self.imitation_pressure.clamp(0.0, 1.0);
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        let decay_factor = 0.5_f64.powf(elapsed_us as f64 / 3_000_000.0);
        self.imitation_pressure *= decay_factor;
        self.suppression *= decay_factor;
    }
}