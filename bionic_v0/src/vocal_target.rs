#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalTargetState {
    pub active_energy: f64,
    pub pitch_hint: f64,
    pub resonance_hint: f64,
    pub envelope_hint: f64,
    /// P3 acoustic loop: full 16-band target contour, retained for band-wise
    /// self↔target comparison. The 2 scalars above are coarse derivations of this.
    pub target_bands: [f64; crate::VIB_BANDS],
    /// Finer-grained 64-mel target fingerprint for the vocal self-supervision
    /// loop. The 16-band `target_bands` above collapses distinct vowels/syllables
    /// together (the 64→16 downsample erases formant detail); this retains the
    /// full mel resolution so the imitation/efficacy loop can tell whether the
    /// body's own voice actually matches the target. Runtime-derived each tick
    /// from the live audio frame — not persisted (mirrors `target_bands`).
    pub target_mel64: [f64; crate::vocal_self_analysis::BANDS],
    pub last_update_t_us: u64,
}

impl Default for VocalTargetState {
    fn default() -> Self {
        VocalTargetState {
            active_energy: 0.0,
            pitch_hint: 0.0,
            resonance_hint: 0.0,
            envelope_hint: 0.0,
            target_bands: [0.0; crate::VIB_BANDS],
            target_mel64: [0.0; crate::vocal_self_analysis::BANDS],
            last_update_t_us: 0,
        }
    }
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
        mel64: [f64; crate::vocal_self_analysis::BANDS],
    ) {
        self.active_energy = energy;
        self.pitch_hint = pitch;
        self.resonance_hint = resonance;
        self.envelope_hint = envelope;
        self.target_bands = bands;
        self.target_mel64 = mel64;
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
