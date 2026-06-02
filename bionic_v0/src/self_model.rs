//! A2: self model — seeded by vital interoception.
//!
//! The self is NOT present at birth. It GROWS from a seed: the most persistent,
//! most self-correlated signal a body has — its own survival state
//! (interoception, from the vital core: reserve + vital_anxiety). Around that
//! seed, a coherent "self" accrues as the body accumulates evidence of agency
//! (events it itself caused) versus things that merely happened to it.
//!
//! 来自用户决断 (A2): the self slot must not be left empty; it is seeded by the
//! interoceptive vital signal. A3 guard: self-coherence has a precision CEILING
//! so the self cannot harden into 我执 (fixation deaf to correction).
//!
//! Compliance: numeric-only. No identity/owner/name. "Self" here is a coherence
//! scalar over an interoceptive seed + an agency trace — it emerges, it is not
//! a stored identity unit.

use crate::substrate::unit;

/// A3 guard: the self can never become fully certain of itself. Coherence is
/// capped below 1.0 so bottom-up correction always has a foothold (anti-我执).
const SELF_COHERENCE_CEILING: f64 = 0.92;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelfModelState {
    /// The interoceptive seed: the body's own survival state, always present
    /// (the one anchor a newborn has). Derived from vital reserve + anxiety.
    pub interoceptive_seed: f64,
    /// EMA of interoceptive stability — how steady the body-sense has been.
    /// A stable body-sense is what a coherent self can accrete around.
    pub interoceptive_stability: f64,
    /// Self-coherence [0, CEILING]: how formed the self is. ~0 at birth; grows
    /// with interoceptive stability AND accumulated agency (self-caused events).
    /// Never reaches 1.0 (A3 anti-fixation).
    pub self_coherence: f64,
    /// Smoothed agency: fraction of recent events the body itself caused.
    pub agency: f64,
    pub last_update_t_us: u64,
    pub prev_seed: f64,
}

impl SelfModelState {
    /// One self-model tick.
    /// `interoception` ∈ [0,1]: the current body-survival sense (from vital
    ///   core — e.g. reserve, or a blend of reserve and (1 - anxiety)).
    /// `agency` ∈ [0,1]: fraction of recent events the body itself caused
    ///   (from self_development's self-caused ratio / agency pressure).
    pub fn tick(&mut self, now_us: u64, interoception: f64, agency: f64) {
        let interoception = unit(interoception);
        let agency = unit(agency);

        self.prev_seed = self.interoceptive_seed;
        self.interoceptive_seed = interoception;

        // Interoceptive stability: low movement of the body-sense = steady.
        let movement = (self.interoceptive_seed - self.prev_seed).abs();
        let steadiness = unit(1.0 - movement * 4.0);
        self.interoceptive_stability =
            unit(0.9 * self.interoceptive_stability + 0.1 * steadiness);

        self.agency = unit(0.97 * self.agency + 0.03 * agency);

        // Self-coherence accrues only where there is BOTH a steady body-sense to
        // anchor on AND evidence of agency (the self is what acts and persists).
        // It grows VERY slowly: a self is built over a long lifetime, not maxed
        // in seconds. (30-min run pinned it at the 0.92 ceiling from minute 1;
        // 0.03 → 0.0006 means tens of thousands of ticks ≈ many minutes to mature.)
        let coherence_target = self.interoceptive_stability * self.agency;
        self.self_coherence = (0.9994 * self.self_coherence + 0.0006 * coherence_target)
            .clamp(0.0, SELF_COHERENCE_CEILING);

        self.last_update_t_us = now_us;
    }

    /// The self-boundary signal other substrates can read: how strongly "this is
    /// me" is currently formed. The interoceptive seed sets a floor (the body is
    /// always at least minimally itself), coherence adds the accrued self.
    pub fn self_salience(&self) -> f64 {
        unit(self.interoceptive_seed * 0.3 + self.self_coherence * 0.7)
    }
}

// ---------------------------------------------------------------------------
// A2 persistence: the formed self survives death (a body that grew a self comes
// back as itself, not a blank newborn). Independent checkpoint.
// ---------------------------------------------------------------------------

use std::path::{Path, PathBuf};

const SELF_MAGIC: u64 = 0xB10B10_0000_00A2;
const SELF_VERSION: u64 = 1;

pub struct SelfModelCheckpointPaths {
    pub final_path: PathBuf,
    pub tmp_path: PathBuf,
}

impl SelfModelCheckpointPaths {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let d = dir.as_ref().to_path_buf();
        Self {
            final_path: d.join("self_model.bin"),
            tmp_path: d.join("self_model.bin.tmp"),
        }
    }
}

pub fn write_self_model_checkpoint(
    state: &SelfModelState,
    paths: &SelfModelCheckpointPaths,
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&SELF_MAGIC.to_le_bytes());
    buf.extend_from_slice(&SELF_VERSION.to_le_bytes());
    buf.extend_from_slice(&state.interoceptive_seed.to_le_bytes());
    buf.extend_from_slice(&state.interoceptive_stability.to_le_bytes());
    buf.extend_from_slice(&state.self_coherence.to_le_bytes());
    buf.extend_from_slice(&state.agency.to_le_bytes());
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&paths.tmp_path)?;
        f.write_all(&buf)?;
        f.sync_data()?;
    }
    std::fs::rename(&paths.tmp_path, &paths.final_path)?;
    Ok(())
}

pub fn read_self_model_checkpoint(path: &Path) -> std::io::Result<SelfModelState> {
    use std::io::{Error, ErrorKind};
    let bytes = std::fs::read(path)?;
    if bytes.len() < 8 * 2 + 8 * 4 {
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
    if rd_u64(&bytes, &mut c) != SELF_MAGIC {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    if rd_u64(&bytes, &mut c) != SELF_VERSION {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let interoceptive_seed = rd_f64(&bytes, &mut c);
    let interoceptive_stability = rd_f64(&bytes, &mut c);
    let self_coherence = rd_f64(&bytes, &mut c);
    let agency = rd_f64(&bytes, &mut c);
    Ok(SelfModelState {
        interoceptive_seed,
        interoceptive_stability,
        self_coherence,
        agency,
        last_update_t_us: 0,
        prev_seed: interoceptive_seed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_roundtrip_preserves_self() {
        let dir = std::env::temp_dir().join(format!("bionic_self_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = SelfModelCheckpointPaths::new(&dir);
        let mut s = SelfModelState::default();
        let mut t = 0u64;
        for _ in 0..300 {
            s.tick(t, 0.7, 0.8);
            t += 1000;
        }
        let before = s;
        write_self_model_checkpoint(&before, &paths).unwrap();
        let after = read_self_model_checkpoint(&paths.final_path).unwrap();
        assert!((after.self_coherence - before.self_coherence).abs() < 1e-9);
        assert!((after.interoceptive_seed - before.interoceptive_seed).abs() < 1e-9);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn newborn_self_is_near_zero_but_has_a_seed() {
        let mut s = SelfModelState::default();
        // First tick: a body-sense exists (seed), but no self has formed yet.
        s.tick(1_000, 0.8, 0.0);
        assert!(s.interoceptive_seed > 0.0, "the interoceptive seed is present from the start");
        assert!(s.self_coherence < 0.05, "newborn self-coherence ~0 (not born with a self)");
    }

    #[test]
    fn self_grows_slowly_over_a_long_life_not_in_seconds() {
        let mut s = SelfModelState::default();
        let mut t = 0u64;
        // After only a SHORT while, the self is still barely formed (not maxed).
        for _ in 0..100 {
            s.tick(t, 0.7, 0.8);
            t += 1000;
        }
        assert!(
            s.self_coherence < 0.1,
            "short exposure → self still nascent, not maxed: {}",
            s.self_coherence
        );
        // Over a LONG life (tens of thousands of ticks) the self matures.
        for _ in 0..30_000 {
            s.tick(t, 0.7, 0.8);
            t += 1000;
        }
        assert!(
            s.self_coherence > 0.2,
            "a long life of steady body-sense + agency matures the self: {}",
            s.self_coherence
        );
    }

    #[test]
    fn agency_absent_keeps_self_unformed() {
        let mut s = SelfModelState::default();
        // Steady body but NO agency (everything just happens to it) → no self.
        let mut t = 0u64;
        for _ in 0..300 {
            s.tick(t, 0.7, 0.0);
            t += 1000;
        }
        assert!(
            s.self_coherence < 0.05,
            "without agency, the self does not form (just a steady body): {}",
            s.self_coherence
        );
    }

    #[test]
    fn self_coherence_never_hits_one_anti_fixation() {
        let mut s = SelfModelState::default();
        let mut t = 0u64;
        // Maximal steady + maximal agency for a long time.
        for _ in 0..5000 {
            s.tick(t, 1.0, 1.0);
            t += 1000;
        }
        assert!(
            s.self_coherence <= SELF_COHERENCE_CEILING,
            "self-coherence must stay under the ceiling (anti-我执): {}",
            s.self_coherence
        );
        assert!(s.self_coherence < 1.0);
    }
}
