use bionic_v0::adapter::{AdapterRegistry, RegistryError, SyntheticPhotAdapter};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_MECH, MOD_PHOT, MOD_VIB, VIB_BANDS};
use std::sync::Arc;

fn main() {
    let vp = Arc::new(Viewport::new());

    // --- Registry contract ---
    // Phase 3B: synthetic PHOT adapter replaces the no-op stub. The duplicate-
    // modality rejection and modality_mask checks remain meaningful.
    let mut reg = AdapterRegistry::new();
    reg.register(Box::new(SyntheticPhotAdapter::new(0)))
        .expect("register phot");
    assert_eq!(reg.len(), 1);
    assert_eq!(reg.modality_mask(), MOD_PHOT);
    match reg.register(Box::new(SyntheticPhotAdapter::new(0))) {
        Err(RegistryError::DuplicateModality(b)) => assert_eq!(b, MOD_PHOT),
        other => panic!("expected DuplicateModality, got {:?}", other),
    }
    let now = 1_000_000u64;

    // --- Slot round-trip ---
    let slot = vp.allocate_slot(MOD_VIB, now, 7).expect("alloc");
    // Tile 4-band patterns across the 16 VIB bands (M1.5 widened VIB_BANDS 4→16).
    let obs: [f64; VIB_BANDS] = std::array::from_fn(|i| [0.1, 0.2, 0.3, 0.4][i % 4]);
    let pred: [f64; VIB_BANDS] = std::array::from_fn(|i| [0.05, 0.15, 0.25, 0.35][i % 4]);
    let err: [f64; VIB_BANDS] = std::array::from_fn(|i| [0.01, 0.02, 0.03, 0.04][i % 4]);
    let prec: [f64; VIB_BANDS] = std::array::from_fn(|i| [1.0, 1.1, 1.2, 1.3][i % 4]);
    assert_eq!(VIB_BANDS, 16);
    vp.write_slot_vib(slot, now, &obs, &pred, &err, &prec);
    let phot_obs = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    vp.write_slot_phot(
        slot,
        now,
        &phot_obs,
        &vec![0.0; 7],
        &vec![0.0; 7],
        &vec![0.0; 7],
    );

    // Spatial: a future-PHOT-style 2D estimate (numbers only). Out-of-range
    // components should clamp into [0,1].
    vp.write_slot_spatial(slot, MOD_PHOT, now, 0.42, 0.37, 0.20, 0.15, 0.8);
    // Overshoot to prove the soft clamp.
    let slot2 = vp.allocate_slot(MOD_PHOT, now, 9).expect("alloc slot2");
    vp.write_slot_spatial(slot2, MOD_PHOT, now, 1.5, -0.2, 0.5, 0.5, 0.4);
    let snap_slot2 = vp.read_slot(slot2).unwrap();
    let sp2 = snap_slot2.spatial.unwrap();
    assert_eq!(sp2.pos_x, 1.0);
    assert_eq!(sp2.pos_y, 0.0);
    assert!(sp2.valid);

    // Invalidation path: spatial present but not fresh.
    vp.invalidate_slot_spatial(slot2, now + 10);
    let snap_slot2b = vp.read_slot(slot2).unwrap();
    assert!(snap_slot2b.spatial.as_ref().unwrap().valid == false);

    // --- Ambient isolation ---
    vp.write_ambient_for(MOD_VIB, 0.25, 0.10, Some(now), true);
    let amb_after_vib = vp.read_ambient();
    assert_eq!(amb_after_vib.vib.event_energy, 0.25);
    assert_eq!(amb_after_vib.vib.candidate_flag, true);
    assert_eq!(amb_after_vib.phot.event_energy, 0.0);
    assert_eq!(amb_after_vib.phot.candidate_flag, false);
    assert_eq!(amb_after_vib.mech.event_energy, 0.0);
    assert_eq!(amb_after_vib.nomination_seq, 1);

    vp.write_ambient_for(MOD_PHOT, 9.0, 2.0, Some(now + 1), true);
    let amb = vp.read_ambient();
    assert_eq!(
        amb.vib.event_energy, 0.25,
        "VIB scalar untouched by PHOT write"
    );
    assert_eq!(amb.vib.candidate_flag, true);
    assert_eq!(amb.phot.event_energy, 9.0);
    assert_eq!(amb.phot.candidate_flag, true);
    assert_eq!(amb.mech.event_energy, 0.0);
    assert_eq!(amb.nomination_seq, 2);

    assert_eq!(amb.mech.surprise, 0.0);
    assert_eq!(amb.mech.candidate_flag, false);

    // --- Snapshot round-trip ---
    let dir = std::env::temp_dir().join("bionic_v0_phase3a");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let re = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(re.ambient_nomination_seq, 2);
    assert_eq!(re.ambient_vib.event_energy, 0.25);
    assert!(re.ambient_vib.candidate_flag);
    assert_eq!(re.ambient_phot.event_energy, 9.0);
    assert!(re.ambient_phot.candidate_flag);
    assert_eq!(re.ambient_mech.event_energy, 0.0);
    assert!(!re.ambient_mech.candidate_flag);

    let rs = &re.slots[slot];
    assert!(rs.active);
    assert_eq!(rs.source_modality, MOD_VIB | MOD_PHOT);
    assert_eq!(rs.vib.as_ref().unwrap().n_bands, 16);
    assert_eq!(rs.phot.as_ref().unwrap().n_bands, 7);
    assert!(rs.mech.is_none());
    let sp = rs.spatial.as_ref().expect("spatial present");
    assert!((sp.pos_x - 0.42).abs() < 1e-9);
    assert!((sp.pos_y - 0.37).abs() < 1e-9);
    assert!((sp.span_x - 0.20).abs() < 1e-9);
    assert!((sp.span_y - 0.15).abs() < 1e-9);
    assert!((sp.precision - 0.8).abs() < 1e-9);
    assert!(sp.valid);

    // slot2 survives round-trip as invalid-spatial
    let rs2 = &re.slots[slot2];
    let sp2r = rs2.spatial.as_ref().unwrap();
    assert_eq!(sp2r.pos_x, 1.0);
    assert_eq!(sp2r.pos_y, 0.0);
    assert!(!sp2r.valid);
    let _ = MOD_MECH;

    println!(
        "OK phase3a ambient-vectorized roundtrip: nom_seq={} vib_E={:.3} phot_E={:.3} mech_E={:.3}",
        re.ambient_nomination_seq,
        re.ambient_vib.event_energy,
        re.ambient_phot.event_energy,
        re.ambient_mech.event_energy,
    );
}
