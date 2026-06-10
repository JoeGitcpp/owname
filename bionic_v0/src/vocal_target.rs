#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalTargetState {
    pub active_energy: f64,
    pub pitch_hint: f64,
    pub resonance_hint: f64,
    pub envelope_hint: f64,
    /// P3 acoustic loop: full 16-band target contour, retained for band-wise
    /// self↔target comparison. The 2 scalars above are coarse derivations of this.
    pub target_bands: [f64; crate::VIB_BANDS],
    pub last_update_t_us: u64,
}

impl VocalTargetState {
    pub fn observe_external_contour(
        &mut self,
        now_us: u64,
        energy: f64,
        pitch: f64,
        resonance: f64,
        envelope: f64,
        bands: [f64; crate::VIB_BANDS],
    ) {
        self.active_energy = energy;
        self.pitch_hint = pitch;
        self.resonance_hint = resonance;
        self.envelope_hint = envelope;
        self.target_bands = bands;
        self.last_update_t_us = now_us;
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 {
            return;
        }
        let decay_factor = 0.5_f64.powf(elapsed_us as f64 / 1_000_000.0);
        self.active_energy *= decay_factor;
    }
}
