//! A1 dual-drive confluence observation tests (推论2/推论4 闭环).
//!
//! These do NOT verify an arbitration "feature"; they observe two factual
//! dynamics:
//!   1. the per-tick DualDriveReading is observer-visible — the v58 snapshot
//!      block round-trips its 5 f64 fields + sacrificing flag symmetrically;
//!   2. the region-B modulation function is a bounded continuous field that
//!      is the identity at net=0 (covered by lib unit tests; here we only
//!      observe the snapshot mirror).

use std::sync::Arc;

use bionic_v0::dual_drive::arbitrate;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn snapshot_round_trips_dual_drive_reading() {
    let vp = Arc::new(Viewport::new());

    // Put a non-default arbitrated reading on the live viewport: low survival
    // pressure + long-earned attachment → net > 0, sacrificing = true.
    let reading = arbitrate(0.2, 0.9, 0.0);
    assert!(reading.net_toward_bond > 0.0);
    assert!(reading.sacrificing);
    *vp.dual_drive.write().unwrap() = reading;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_dual_drive_snap_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let re = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(
        re.dual_drive, reading,
        "dual_drive reading must survive the round trip bit-exactly"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// principle_floor sourcing (大脑警察 ← vital-core C6 acute pain).
// Factual observations only — no behaviour targets.

#[test]
fn zero_pain_keeps_floor_zero_and_reading_unchanged() {
    // With the vital core at rest (pain = 0), the sourced floor must be 0 and
    // the arbitrated reading must be bit-identical to the previous hardcoded
    // floor = 0.0 path. This is the key regression guarantee.
    let vp = Arc::new(Viewport::new());
    assert_eq!(vp.read_vital_core().pain, 0.0);

    vp.tick_dual_drive(1_000_000);
    let dd = vp.read_dual_drive();
    assert_eq!(dd.principle_floor, 0.0, "no harm → floor must be exactly 0");

    // Recompute through the public algebra with floor = 0.0: must match.
    let sp = vp.read_vital_core().self_preservation_pressure();
    let expected = arbitrate(sp, 0.0, 0.0);
    assert_eq!(dd, expected, "zero-pain reading must equal the legacy floor=0 path");
}

#[test]
fn nonzero_pain_sources_a_continuous_floor_that_clamps_net() {
    // Inject an acute pain transient into the vital core state, then observe:
    // the floor mirrors the continuous pain value (no threshold step) and the
    // arbitration clamps net_toward_bond to <= -floor.
    let vp = Arc::new(Viewport::new());

    for &pain in &[0.05, 0.37, 0.8, 1.0] {
        vp.vital_core.write().unwrap().pain = pain;
        vp.tick_dual_drive(2_000_000);
        let dd = vp.read_dual_drive();
        assert!(
            (dd.principle_floor - pain).abs() < 1e-12,
            "floor must continuously mirror pain {pain}: got {}",
            dd.principle_floor
        );
        assert!(
            dd.net_toward_bond <= -dd.principle_floor + 1e-12,
            "harm must clamp net toward self-preservation: net={} floor={}",
            dd.net_toward_bond,
            dd.principle_floor
        );
        assert!(!dd.sacrificing, "a live floor must veto the sacrifice flag");
    }
}
