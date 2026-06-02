//! C5: persistent object identity / caregiver imprinting (minimal emergent form).
//!
//! Objects live in anonymous, index-reused slots, so "is this the same object as
//! before?" is otherwise lost (and fully lost on restart). This adds a small
//! sediment of FAMILIAR OBJECTS — each a numeric fingerprint plus accumulated
//! familiarity — on top of slots (it does NOT touch the slot architecture).
//!
//! Identity is EMERGENT, not a table lookup and not a label:
//!   - A re-appearing object whose fingerprint matches a sediment raises that
//!     entry's familiarity + recognition_count ("还是上次那个").
//!   - A novel-enough fingerprint sediments a new entry.
//!   - The "responsible person / 责任人" is simply the most-familiar emergent
//!     entry — no role flag, no name, no "mother" label. Highest number wins.
//!
//! Mirrors the vocal_memory prototype-sedimentation pattern. Compliance:
//! numeric-only fingerprints (a band/spatial signature), no semantic identity.

use crate::substrate::unit;

/// Width of an object fingerprint (a compact numeric signature of how an object
/// "looks/sounds" — its band-shape + spatial tendency, all in [0,1]).
pub const OBJECT_FINGERPRINT_DIM: usize = 4;

/// Max sediment size. Like vocal_memory's cap: a life keeps only a handful of
/// deeply familiar objects; the rest fade.
pub const FAMILIAR_OBJECT_CAP: usize = 8;

/// Fingerprint match distance below which two fingerprints are "the same object".
const MATCH_DIST: f64 = 0.20;

/// Minimum real-time gap between familiarity increments for one object. Seeing
/// the same object more often than this drifts its fingerprint but does NOT
/// bond faster — familiarity is earned over real time, not tick count.
pub const FAMILIARITY_TICK_US: u64 = 1_000_000; // 1 second

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FamiliarObject {
    pub fingerprint: [f64; OBJECT_FINGERPRINT_DIM],
    /// Accumulated familiarity [0,1]: rises on each re-recognition, decays slowly.
    pub familiarity: f64,
    /// How many times this object has been re-recognized.
    pub recognition_count: u32,
    pub last_seen_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FamiliarObjectsState {
    pub objects: Vec<FamiliarObject>,
    pub last_update_t_us: u64,
}

fn dist(a: &[f64; OBJECT_FINGERPRINT_DIM], b: &[f64; OBJECT_FINGERPRINT_DIM]) -> f64 {
    let mut s = 0.0;
    for i in 0..OBJECT_FINGERPRINT_DIM {
        let d = a[i] - b[i];
        s += d * d;
    }
    s.sqrt()
}

impl FamiliarObjectsState {
    /// Observe an object's fingerprint. If it matches a known object, that
    /// object is RE-RECOGNIZED (familiarity + count rise). Otherwise a new
    /// familiar object is sedimented. Returns the index of the matched/created
    /// object and whether it was a re-recognition.
    pub fn observe(
        &mut self,
        now_us: u64,
        fingerprint: [f64; OBJECT_FINGERPRINT_DIM],
    ) -> (usize, bool) {
        self.last_update_t_us = now_us;

        // Find the closest existing object within MATCH_DIST.
        let mut best: Option<(usize, f64)> = None;
        for (i, o) in self.objects.iter().enumerate() {
            let d = dist(&o.fingerprint, &fingerprint);
            if d < MATCH_DIST && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((i, d));
            }
        }

        if let Some((idx, _)) = best {
            // Re-recognition: "还是上次那个". Familiarity accrues by REAL TIME,
            // not by tick count: seeing the same object 50 000 times in a 30-min
            // run must not max the bond. Only once per FAMILIARITY_TICK_US of
            // real presence does familiarity (and the recognition count) tick up;
            // more frequent sightings still drift the fingerprint but don't bond
            // faster. So "familiarity takes time", literally. (~0.01 per second →
            // a deep bond is minutes of presence, the responsible person earned.)
            let o = &mut self.objects[idx];
            for i in 0..OBJECT_FINGERPRINT_DIM {
                o.fingerprint[i] = 0.9 * o.fingerprint[i] + 0.1 * fingerprint[i];
            }
            let elapsed = now_us.saturating_sub(o.last_seen_us);
            if elapsed >= FAMILIARITY_TICK_US {
                o.familiarity = unit(o.familiarity + 0.01);
                o.recognition_count = o.recognition_count.saturating_add(1);
                o.last_seen_us = now_us;
            }
            return (idx, true);
        }

        // Novel object: sediment a new familiar entry.
        self.objects.push(FamiliarObject {
            fingerprint,
            familiarity: 0.01,
            recognition_count: 1,
            last_seen_us: now_us,
        });
        if self.objects.len() > FAMILIAR_OBJECT_CAP {
            // Keep the most familiar; the least-familiar fades.
            self.objects
                .sort_by(|a, b| b.familiarity.partial_cmp(&a.familiarity).unwrap());
            self.objects.truncate(FAMILIAR_OBJECT_CAP);
        }
        (self.objects.len() - 1, false)
    }

    /// The emergent "responsible person / 责任人": the most-familiar object.
    /// Not a role, not a label — just the highest accumulated familiarity.
    /// None until something has been recognized enough to stand out.
    pub fn most_familiar(&self) -> Option<&FamiliarObject> {
        self.objects
            .iter()
            .max_by(|a, b| a.familiarity.partial_cmp(&b.familiarity).unwrap())
    }
}

// ---------------------------------------------------------------------------
// C5 persistence: familiar objects survive death (so the responsible-person
// bond is re-recognized after restart). Independent checkpoint file, like
// learned_bias.bin — its own substrate, its own store.
// ---------------------------------------------------------------------------

use std::path::{Path, PathBuf};

const FAMILIAR_MAGIC: u64 = 0xB10B10_0000_00C5;
const FAMILIAR_VERSION: u64 = 1;

pub struct FamiliarObjectsCheckpointPaths {
    pub final_path: PathBuf,
    pub tmp_path: PathBuf,
}

impl FamiliarObjectsCheckpointPaths {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let d = dir.as_ref().to_path_buf();
        Self {
            final_path: d.join("familiar_objects.bin"),
            tmp_path: d.join("familiar_objects.bin.tmp"),
        }
    }
}

pub fn write_familiar_objects_checkpoint(
    state: &FamiliarObjectsState,
    paths: &FamiliarObjectsCheckpointPaths,
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&FAMILIAR_MAGIC.to_le_bytes());
    buf.extend_from_slice(&FAMILIAR_VERSION.to_le_bytes());
    buf.extend_from_slice(&(state.objects.len() as u32).to_le_bytes());
    for o in &state.objects {
        for v in &o.fingerprint {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf.extend_from_slice(&o.familiarity.to_le_bytes());
        buf.extend_from_slice(&o.recognition_count.to_le_bytes());
        buf.extend_from_slice(&o.last_seen_us.to_le_bytes());
    }
    buf.extend_from_slice(&state.last_update_t_us.to_le_bytes());
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&paths.tmp_path)?;
        f.write_all(&buf)?;
        f.sync_data()?;
    }
    std::fs::rename(&paths.tmp_path, &paths.final_path)?;
    Ok(())
}

pub fn read_familiar_objects_checkpoint(
    path: &Path,
) -> std::io::Result<FamiliarObjectsState> {
    use std::io::{Error, ErrorKind};
    let bytes = std::fs::read(path)?;
    let mut c = 0usize;
    let rd_u64 = |b: &[u8], c: &mut usize| -> std::io::Result<u64> {
        if *c + 8 > b.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        Ok(u64::from_le_bytes(a))
    };
    let rd_u32 = |b: &[u8], c: &mut usize| -> std::io::Result<u32> {
        if *c + 4 > b.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        let mut a = [0u8; 4];
        a.copy_from_slice(&b[*c..*c + 4]);
        *c += 4;
        Ok(u32::from_le_bytes(a))
    };
    let rd_f64 = |b: &[u8], c: &mut usize| -> std::io::Result<f64> {
        if *c + 8 > b.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[*c..*c + 8]);
        *c += 8;
        Ok(f64::from_le_bytes(a))
    };
    if rd_u64(&bytes, &mut c)? != FAMILIAR_MAGIC {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    if rd_u64(&bytes, &mut c)? != FAMILIAR_VERSION {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let n = rd_u32(&bytes, &mut c)? as usize;
    if n > FAMILIAR_OBJECT_CAP {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let mut objects = Vec::with_capacity(n);
    for _ in 0..n {
        let mut fingerprint = [0.0; OBJECT_FINGERPRINT_DIM];
        for v in fingerprint.iter_mut() {
            *v = rd_f64(&bytes, &mut c)?;
        }
        let familiarity = rd_f64(&bytes, &mut c)?;
        let recognition_count = rd_u32(&bytes, &mut c)?;
        let last_seen_us = rd_u64(&bytes, &mut c)?;
        objects.push(FamiliarObject {
            fingerprint,
            familiarity,
            recognition_count,
            last_seen_us,
        });
    }
    let last_update_t_us = rd_u64(&bytes, &mut c)?;
    Ok(FamiliarObjectsState {
        objects,
        last_update_t_us,
    })
}

/// Extract an object fingerprint from a slot's VIB band-shape. The fingerprint
/// is the band ENERGY SHAPE normalized to sum 1 (so it captures "what this
/// object sounds like" — its spectral contour — independent of loudness, which
/// is transient). Returns None when there's no usable band energy.
pub fn fingerprint_from_slot(
    vib: Option<&crate::viewport::BandFamilyState>,
) -> Option<[f64; OBJECT_FINGERPRINT_DIM]> {
    let vib = vib?;
    let n = vib.observed.len().min(OBJECT_FINGERPRINT_DIM);
    if n == 0 {
        return None;
    }
    let mut fp = [0.0; OBJECT_FINGERPRINT_DIM];
    let mut sum = 0.0;
    for i in 0..n {
        let e = vib.observed[i].abs();
        fp[i] = e;
        sum += e;
    }
    if sum < 1e-9 {
        return None; // silent slot has no spectral identity
    }
    for v in fp.iter_mut() {
        *v = unit(*v / sum);
    }
    Some(fp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(a: f64, b: f64, c: f64, d: f64) -> [f64; OBJECT_FINGERPRINT_DIM] {
        [a, b, c, d]
    }

    #[test]
    fn same_fingerprint_is_re_recognized_not_duplicated() {
        let mut s = FamiliarObjectsState::default();
        let face = fp(0.2, 0.5, 0.3, 0.8);
        let (_, new1) = s.observe(1_000, face);
        assert!(!new1, "first sighting is novel");
        let (_, new2) = s.observe(1_000 + FAMILIARITY_TICK_US, fp(0.21, 0.49, 0.31, 0.79)); // 1s later
        assert!(new2, "near-identical fingerprint must be RE-RECOGNIZED");
        assert_eq!(s.objects.len(), 1, "must not duplicate the same object");
        assert_eq!(s.objects[0].recognition_count, 2);
        assert!(
            s.objects[0].familiarity > 0.01,
            "familiarity must grow on re-recognition: {}",
            s.objects[0].familiarity
        );
    }

    #[test]
    fn different_fingerprint_sediments_new_object() {
        let mut s = FamiliarObjectsState::default();
        s.observe(1_000, fp(0.2, 0.5, 0.3, 0.8));
        s.observe(2_000, fp(0.9, 0.1, 0.7, 0.2)); // clearly different
        assert_eq!(s.objects.len(), 2, "distinct fingerprints are distinct objects");
    }

    #[test]
    fn familiarity_grows_slowly_responsible_person_must_be_earned() {
        // After only ~20 seconds of presence the bond is still shallow — the
        // responsible person is EARNED over long real time, not maxed fast.
        // (30-min fix: every object hit 1.0 within a minute before; now
        // familiarity ticks ~0.01 per real second, gated by FAMILIARITY_TICK_US.)
        let mut s = FamiliarObjectsState::default();
        let face = fp(0.3, 0.4, 0.5, 0.6);
        for k in 0..20u64 {
            s.observe(1_000 + k * FAMILIARITY_TICK_US, face);
        }
        assert!(
            s.objects[0].familiarity < 0.3,
            "~20s presence should still be a shallow bond, not maxed: {}",
            s.objects[0].familiarity
        );
        // But long sustained presence (minutes) does eventually bond deeply.
        for k in 0..600u64 {
            s.observe(100_000 + k * FAMILIARITY_TICK_US, face);
        }
        assert!(
            s.objects[0].familiarity > 0.8,
            "long sustained presence earns a deep bond: {}",
            s.objects[0].familiarity
        );
    }

    #[test]
    fn responsible_person_emerges_from_most_repeated() {
        let mut s = FamiliarObjectsState::default();
        let caregiver = fp(0.3, 0.4, 0.5, 0.6);
        let stranger = fp(0.8, 0.2, 0.1, 0.9);
        // caregiver appears many times; stranger once.
        for k in 0..10 {
            s.observe(1_000 + k * FAMILIARITY_TICK_US, caregiver);
        }
        s.observe(9_999, stranger);

        let top = s.most_familiar().expect("someone is most familiar");
        assert!(
            dist(&top.fingerprint, &caregiver) < MATCH_DIST,
            "the most-repeated object emerges as the responsible person (no label)"
        );
        assert!(top.recognition_count >= 10);
    }
}
