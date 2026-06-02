//! C1: vital core (维生内核) — the self-survival source of the dual-drive soul.
//!
//! A predictor engine's only irreducible "food/safety" is: can it keep
//! predicting? Death = prediction stops. This core holds a `reserve` (survival
//! store) driven by real survival signals — depleted by exertion/running,
//! replenished by rest/supply. When the reserve truly bottoms out the core
//! transitions to a READABLE Standby state (待机/养老), never a NaN crash; it
//! can be revived.
//!
//! This file is C1-a: the upper layer (cognitive-scale survival prediction +
//! the reserve dynamics + the death state machine), driven by INJECTED survival
//! signals so it is pure and testable. C1-b later feeds it real hardware
//! (battery / input-stream continuity) via a low-rate sampler.
//!
//! 双层 (spec §2): the reserve here tracks survival; `vital_anxiety` is the
//! cognitive-scale signal — not "how much is left" but "how fast my predicted
//! survival trajectory is worsening" (a predictor's dread comes from prediction,
//! not the current level).
//!
//! Compliance: numeric-only. No `soul`/`need`/`death` semantic identity — just
//! a reserve scalar and a small state enum.

use crate::substrate::unit;

/// Readable life state. Death is a clean transition, never numeric divergence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VitalState {
    /// Normal life.
    #[default]
    Alive = 0,
    /// Reserve bottomed out and stayed there: low-activity 待机/养老. Revivable
    /// (supply/rest brings reserve back up → Alive). Not hardware self-destruct.
    Standby = 1,
}

/// reserve at/under this for a sustained period → Standby.
const STANDBY_FLOOR: f64 = 0.02;
/// reserve at/over this while in Standby → revive to Alive.
const REVIVE_LEVEL: f64 = 0.25;
/// consecutive low-reserve ticks before entering Standby (avoids flicker).
const STANDBY_PATIENCE: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VitalCoreState {
    /// Survival reserve [0,1]. 1 = full; 0 = depleted.
    pub reserve: f64,
    /// Previous reserve, for the trajectory signal.
    pub prev_reserve: f64,
    /// Cognitive-scale survival dread: EMA of the DOWNWARD movement of reserve
    /// (predicted trajectory worsening). High → self-preservation pressure.
    pub vital_anxiety: f64,
    /// C6: pain — an ACUTE self-damage signal, distinct from the slow anxiety.
    /// Spikes on sharp drain (heavy exertion/overload) or critically-low
    /// reserve. Innately high-gain and fast-decaying (a sharp transient, not a
    /// mood). The body's protective "this is hurting me" signal.
    pub pain: f64,
    /// Slow EMA of drain — the body's "normal exertion" baseline. Pain fires on
    /// drain ABOVE this baseline (a sudden overload), not on steady exertion:
    /// running hard but sustainably should not hurt; a sharp spike should.
    pub drain_baseline: f64,
    pub state: VitalState,
    /// Consecutive ticks with reserve below the floor.
    pub low_run: u32,
    pub last_update_t_us: u64,
    /// The supply/drain the body last metabolized this tick. Persisted and
    /// surfaced so the (often simulated) metabolic world is observable in the
    /// life trace — what is the body actually being fed and spending right now.
    pub last_supply: f64,
    pub last_drain: f64,
}

impl Default for VitalCoreState {
    fn default() -> Self {
        // Born with a healthy reserve.
        Self {
            reserve: 1.0,
            prev_reserve: 1.0,
            vital_anxiety: 0.0,
            pain: 0.0,
            drain_baseline: 0.0,
            state: VitalState::Alive,
            low_run: 0,
            last_update_t_us: 0,
            last_supply: 0.0,
            last_drain: 0.0,
        }
    }
}

impl VitalCoreState {
    /// One vital tick.
    /// `supply` ∈ [0,1]: incoming survival resource this tick (e.g. charging /
    ///   continuous sensory input — proto-food/safety). Replenishes reserve.
    /// `drain` ∈ [0,1]: this tick's exertion/running cost. Depletes reserve.
    /// Both are INJECTED here (C1-a); C1-b sources them from real hardware.
    pub fn tick(&mut self, now_us: u64, supply: f64, drain: f64) {
        let supply = unit(supply);
        let drain = unit(drain);
        self.last_supply = supply;
        self.last_drain = drain;
        self.prev_reserve = self.reserve;

        // Reserve dynamics: supply lifts, drain lowers. Net change scaled small
        // so reserve moves on a survival (slow) timescale, not per-tick noise.
        let net = (supply - drain) * 0.05;
        self.reserve = unit(self.reserve + net);

        // Vital anxiety = downward movement of reserve (空白 of survival opening).
        // Only worsening counts as dread; recovery doesn't.
        let worsening = (self.prev_reserve - self.reserve).max(0.0);
        self.vital_anxiety = unit(0.9 * self.vital_anxiety + 0.1 * (worsening * 20.0));

        // C6 pain: ACUTE self-damage = a SUDDEN OVERLOAD relative to the body's
        // own recent exertion baseline (not steady running, however hard), OR a
        // critically-low reserve. The baseline is a slow EMA of drain, so a
        // body that settles into sustained exertion stops hurting (the baseline
        // catches up); only a spike ABOVE the baseline hurts. Fast decay so pain
        // is a transient. (Fixes the 30-min bug where steady voicing pinned pain.)
        let overload = ((drain - self.drain_baseline - 0.25).max(0.0) / 0.4).clamp(0.0, 1.0);
        let critical_low = ((0.1 - self.reserve).max(0.0) / 0.1).clamp(0.0, 1.0);
        let pain_input = overload.max(critical_low);
        // Update the exertion baseline AFTER computing pain (slow follow).
        self.drain_baseline = unit(0.98 * self.drain_baseline + 0.02 * drain);
        self.pain = unit(0.6 * self.pain + pain_input);

        // Death state machine — clean, readable, revivable.
        match self.state {
            VitalState::Alive => {
                if self.reserve <= STANDBY_FLOOR {
                    self.low_run = self.low_run.saturating_add(1);
                    if self.low_run >= STANDBY_PATIENCE {
                        self.state = VitalState::Standby;
                    }
                } else {
                    self.low_run = 0;
                }
            }
            VitalState::Standby => {
                // Revive only when reserve has genuinely recovered.
                if self.reserve >= REVIVE_LEVEL {
                    self.state = VitalState::Alive;
                    self.low_run = 0;
                }
            }
        }

        self.last_update_t_us = now_us;
    }

    /// Self-preservation pressure this tick: rises as reserve falls and as the
    /// trajectory worsens. This is what the dual-drive arbitration weighs
    /// against emergent (caregiver/master) drive. In Standby it pins high.
    pub fn self_preservation_pressure(&self) -> f64 {
        if self.state == VitalState::Standby {
            return 1.0;
        }
        unit((1.0 - self.reserve) * 0.6 + self.vital_anxiety * 0.4)
    }
}

// ---------------------------------------------------------------------------
// C1 persistence: the survival reserve survives death (a body that was running
// low comes back still tired, not magically reset). Independent checkpoint.
// ---------------------------------------------------------------------------

use std::path::{Path, PathBuf};

const VITAL_MAGIC: u64 = 0xB10B10_0000_00C1;
/// v2 appends last_supply/last_drain after the state byte. v1 files (without
/// them) are still accepted on read — the metabolism just defaults to 0.
const VITAL_VERSION: u64 = 2;

pub struct VitalCoreCheckpointPaths {
    pub final_path: PathBuf,
    pub tmp_path: PathBuf,
}

impl VitalCoreCheckpointPaths {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let d = dir.as_ref().to_path_buf();
        Self {
            final_path: d.join("vital_core.bin"),
            tmp_path: d.join("vital_core.bin.tmp"),
        }
    }
}

pub fn write_vital_core_checkpoint(
    state: &VitalCoreState,
    paths: &VitalCoreCheckpointPaths,
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&VITAL_MAGIC.to_le_bytes());
    buf.extend_from_slice(&VITAL_VERSION.to_le_bytes());
    buf.extend_from_slice(&state.reserve.to_le_bytes());
    buf.extend_from_slice(&state.vital_anxiety.to_le_bytes());
    buf.extend_from_slice(&state.pain.to_le_bytes());
    buf.extend_from_slice(&(state.state as u8).to_le_bytes());
    buf.extend_from_slice(&state.last_supply.to_le_bytes());
    buf.extend_from_slice(&state.last_drain.to_le_bytes());
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&paths.tmp_path)?;
        f.write_all(&buf)?;
        f.sync_data()?;
    }
    std::fs::rename(&paths.tmp_path, &paths.final_path)?;
    Ok(())
}

pub fn read_vital_core_checkpoint(path: &Path) -> std::io::Result<VitalCoreState> {
    use std::io::{Error, ErrorKind};
    let bytes = std::fs::read(path)?;
    if bytes.len() < 8 + 8 + 8 + 8 + 8 + 1 {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }
    let rd_u64 = |b: &[u8], c: &mut usize| {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        u64::from_le_bytes(a)
    };
    let rd_f64 = |b: &[u8], c: &mut usize| {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        f64::from_le_bytes(a)
    };
    let mut c = 0usize;
    if rd_u64(&bytes, &mut c) != VITAL_MAGIC {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let version = rd_u64(&bytes, &mut c);
    if version != 1 && version != 2 {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let reserve = rd_f64(&bytes, &mut c);
    let vital_anxiety = rd_f64(&bytes, &mut c);
    let pain = rd_f64(&bytes, &mut c);
    let state = if bytes[c] == 1 {
        VitalState::Standby
    } else {
        VitalState::Alive
    };
    c += 1; // consume the state byte
    // v2 appends last_supply/last_drain; v1 files default them to 0.
    let (last_supply, last_drain) = if version >= 2 && bytes.len() >= c + 16 {
        (rd_f64(&bytes, &mut c), rd_f64(&bytes, &mut c))
    } else {
        (0.0, 0.0)
    };
    Ok(VitalCoreState {
        reserve,
        prev_reserve: reserve,
        vital_anxiety,
        pain,
        drain_baseline: 0.0,
        state,
        low_run: 0,
        last_update_t_us: 0,
        last_supply,
        last_drain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_roundtrip_preserves_reserve_and_state() {
        let dir = std::env::temp_dir().join(format!("bionic_vital_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = VitalCoreCheckpointPaths::new(&dir);

        let mut v = VitalCoreState::default();
        for k in 0..50 {
            v.tick(k * 1000, 0.0, 1.0); // run it down a bit
        }
        let before = v;
        write_vital_core_checkpoint(&before, &paths).unwrap();
        let after = read_vital_core_checkpoint(&paths.final_path).unwrap();
        assert!((after.reserve - before.reserve).abs() < 1e-9, "reserve persists");
        assert_eq!(after.state, before.state, "life state persists");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn heavy_exertion_spikes_pain_then_it_fades() {
        let mut v = VitalCoreState::default();
        // Heavy drain → acute pain spike.
        v.tick(1000, 0.0, 1.0);
        let hurt = v.pain;
        assert!(hurt > 0.3, "heavy exertion must spike pain: {hurt}");
        // Stop the harm → pain fades fast (it's a transient, not a mood).
        for k in 0..10 {
            v.tick(2000 + k * 1000, 0.5, 0.1); // gentle
        }
        assert!(v.pain < hurt * 0.3, "pain must fade quickly when harm stops: {}", v.pain);
    }

    #[test]
    fn sustained_voicing_does_not_pin_pain() {
        // The 30-min bug: steady voicing (drain 0.65 every tick) pinned pain at
        // 0.313 forever. Now a sustained load must let the baseline catch up so
        // pain fades — only a SPIKE above the settled baseline should hurt.
        let mut v = VitalCoreState::default();
        let mut t = 0u64;
        // Hold a constant heavy-ish drain, but keep reserve up (supply matches)
        // so pain can only come from drain, not depletion.
        for _ in 0..600 {
            v.tick(t, 0.65, 0.65);
            t += 1000;
        }
        assert!(
            v.pain < 0.05,
            "steady sustained exertion must NOT pin pain (baseline catches up): {}",
            v.pain
        );
        // A sudden spike ABOVE the settled baseline still hurts.
        v.tick(t, 0.65, 1.0);
        assert!(v.pain > 0.1, "a spike above the settled baseline must still hurt: {}", v.pain);
    }

    #[test]
    fn gentle_running_does_not_hurt() {
        let mut v = VitalCoreState::default();
        for k in 0..20 {
            v.tick(k * 1000, 0.5, 0.2); // light, sustainable load
        }
        assert!(v.pain < 0.1, "gentle running must not hurt: {}", v.pain);
    }

    #[test]
    fn critically_low_reserve_hurts() {
        let mut v = VitalCoreState::default();
        // Deplete to critical, then hold with neutral drain (so pain comes from
        // the low reserve itself, not from drain).
        let mut t = 0u64;
        for _ in 0..400 {
            v.tick(t, 0.0, 0.5);
            t += 1000;
        }
        assert!(v.reserve < 0.1, "precondition: reserve critical");
        assert!(v.pain > 0.3, "critically-low reserve must hurt: {}", v.pain);
    }

    #[test]
    fn drain_depletes_reserve_supply_restores_it() {
        let mut v = VitalCoreState::default();
        assert_eq!(v.reserve, 1.0);
        // pure drain → reserve falls (会累/会饿).
        for k in 0..20 {
            v.tick(k * 1000, 0.0, 1.0);
        }
        let drained = v.reserve;
        assert!(drained < 1.0, "drain must deplete reserve: {drained}");
        // pure supply → reserve climbs back.
        for k in 0..20 {
            v.tick(100_000 + k * 1000, 1.0, 0.0);
        }
        assert!(v.reserve > drained, "supply must restore reserve");
    }

    #[test]
    fn falling_reserve_raises_vital_anxiety() {
        let mut v = VitalCoreState::default();
        for k in 0..10 {
            v.tick(k * 1000, 0.0, 1.0);
        }
        assert!(
            v.vital_anxiety > 0.0,
            "a worsening survival trajectory must raise vital_anxiety"
        );
    }

    #[test]
    fn sustained_depletion_enters_standby_then_revives() {
        let mut v = VitalCoreState::default();
        // Drive reserve to the floor and hold it there.
        let mut t = 0u64;
        for _ in 0..400 {
            v.tick(t, 0.0, 1.0);
            t += 1000;
        }
        assert_eq!(
            v.state,
            VitalState::Standby,
            "sustained depletion must enter Standby (待机), reserve={}",
            v.reserve
        );
        // Standby pins self-preservation high.
        assert_eq!(v.self_preservation_pressure(), 1.0);

        // Sustained supply revives it.
        for _ in 0..400 {
            v.tick(t, 1.0, 0.0);
            t += 1000;
        }
        assert_eq!(
            v.state,
            VitalState::Alive,
            "sustained supply must revive from Standby (可复苏)"
        );
    }

    #[test]
    fn reserve_never_diverges() {
        let mut v = VitalCoreState::default();
        // Extreme alternating inputs must keep reserve in [0,1], never NaN.
        let mut t = 0u64;
        for k in 0..1000 {
            v.tick(t, (k % 2) as f64, ((k + 1) % 2) as f64);
            t += 1;
            assert!(v.reserve.is_finite() && (0.0..=1.0).contains(&v.reserve));
        }
    }
}
