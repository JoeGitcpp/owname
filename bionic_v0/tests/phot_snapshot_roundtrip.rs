use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::{HEALTH_OK, MOD_PHOT, QUIET_ENERGY_THRESHOLD};

#[test]
fn phot_snapshot_roundtrip_preserves_health_ambient_slot_and_spatial() {
    let vp = Arc::new(Viewport::new());

    // --- health: arrange substrate so OK is produced ---
    let t = 2_000_000u64;
    vp.push_observation(0, t, QUIET_ENERGY_THRESHOLD * 10.0);
    vp.update_health_state(t + 1_000);
    assert_eq!(vp.read_health_state().as_u8(), HEALTH_OK);

    // --- ambient.phot: non-zero + rising-edge candidate ---
    vp.write_ambient_for(MOD_PHOT, 0.75, 0.12, Some(t), true);

    // --- slot.phot + slot.spatial on the same slot ---
    let slot = vp.allocate_slot(MOD_PHOT, t, 42).expect("alloc");
    let phot_obs = vec![0.1, 0.2, 0.3, 0.4];
    let phot_pred = vec![0.05, 0.15, 0.25, 0.35];
    let phot_err = vec![0.01, 0.02, 0.03, 0.04];
    let phot_prec = vec![0.9, 0.8, 0.7, 0.6];
    vp.write_slot_phot(slot, t, &phot_obs, &phot_pred, &phot_err, &phot_prec);
    vp.write_slot_spatial(slot, MOD_PHOT, t, 0.3, 0.4, 0.1, 0.15, 0.55);

    // --- write & re-read ---
    let dir = std::env::temp_dir().join("bionic_v0_phase3b_snap");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let re = read_snapshot(&paths.final_path).unwrap();

    // --- health state survives ---
    assert_eq!(re.health_state, HEALTH_OK);

    // --- ambient phot fields survive ---
    assert!((re.ambient_phot.event_energy - 0.75).abs() < 1e-12);
    assert!((re.ambient_phot.surprise - 0.12).abs() < 1e-12);
    assert!(re.ambient_phot.candidate_flag);
    assert_eq!(re.ambient_phot.last_event_t_us, t);
    assert_eq!(re.ambient_nomination_seq, 1);

    // --- slot phot family survives ---
    let rs = &re.slots[slot];
    assert!(rs.active);
    assert_eq!(rs.source_modality & MOD_PHOT, MOD_PHOT);
    let fam = rs.phot.as_ref().expect("phot family present");
    assert_eq!(fam.n_bands as usize, phot_obs.len());
    assert_eq!(fam.observed, phot_obs);
    assert_eq!(fam.predicted, phot_pred);
    assert_eq!(fam.err, phot_err);
    assert_eq!(fam.surprise, phot_prec);

    // --- spatial survives ---
    let sp = rs.spatial.expect("spatial present");
    assert!((sp.pos_x - 0.3).abs() < 1e-12);
    assert!((sp.pos_y - 0.4).abs() < 1e-12);
    assert!((sp.span_x - 0.1).abs() < 1e-12);
    assert!((sp.span_y - 0.15).abs() < 1e-12);
    assert!((sp.precision - 0.55).abs() < 1e-12);
    assert_eq!(sp.last_update_t_us, t);
    assert!(sp.valid);
}
