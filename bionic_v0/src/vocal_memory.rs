#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalPrototype {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub stability: f64,
    pub reuse_success: u32,
    pub last_update_t_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VocalMemoryState {
    pub prototypes: Vec<VocalPrototype>,
    pub last_update_t_us: u64,
}

impl VocalMemoryState {
    pub fn record_success(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.last_update_t_us = now_us;

        let matching_idx = self.prototypes.iter().position(|p| {
            let breath_diff = (p.breath_drive - breath).abs();
            let pitch_diff = (p.pitch_tension - pitch).abs();
            let resonance_diff = (p.resonance_position - resonance).abs();
            breath_diff < 0.1 && pitch_diff < 0.1 && resonance_diff < 0.1
        });

        if let Some(idx) = matching_idx {
            self.prototypes[idx].stability += 0.1;
            self.prototypes[idx].reuse_success += 1;
            self.prototypes[idx].last_update_t_us = now_us;
        } else {
            self.prototypes.push(VocalPrototype {
                breath_drive: breath,
                pitch_tension: pitch,
                resonance_position: resonance,
                stability: 0.1,
                reuse_success: 1,
                last_update_t_us: now_us,
            });
        }

        if self.prototypes.len() > 8 {
            self.prototypes
                .sort_by(|a, b| b.stability.partial_cmp(&a.stability).unwrap());
            self.prototypes.truncate(8);
        }
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        for p in &mut self.prototypes {
            let decay_factor = 0.5_f64.powf(elapsed_us as f64 / 10_000_000.0);
            p.stability *= decay_factor;
        }
        self.prototypes.retain(|p| p.stability > 0.05);
    }
}
