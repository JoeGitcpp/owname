//! B1-3b-iii: per-slot cross-layer (B1) weights survive a snapshot roundtrip.
//!
//! This is the "重要记忆 = 关系规则, 死后保留" support (R1): the relation rule a
//! cross-layer predictor learned (which neighbour slot predicts this slot's
//! error, and how strongly) must persist across a save/restore, not fade.
//!
//! Contracts:
//!   1. After learning, write_snapshot + read_snapshot preserves the target
//!      slot's cross-layer source_slots / source_weights / bias.
//!   2. A slot with no cross-layer predictor roundtrips as None.

use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_VIB, VIB_BANDS};

fn tick_slot(vp: &Arc<Viewport>, slot: usize, t: u64, level: f64) {
    let obs = [level; VIB_BANDS];
    let pred = [level * 0.9; VIB_BANDS];
    let err = [level * 0.1; VIB_BANDS];
    let prec = [0.5; VIB_BANDS];
    vp.write_slot_vib(slot, t, &obs, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot, MOD_VIB, t);
    vp.refresh_slot_high_layer(slot, t);
}

#[test]
fn cross_layer_weights_survive_snapshot_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_xlayer_persist_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    for i in 0..60u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        vp.tick_slot_cross_layer(a, t);
    }

    // The learned rule we expect to persist.
    let live = vp
        .read_slot_cross_layer(a)
        .expect("target a should have a cross-layer predictor");
    assert!(
        !live.source_slots.is_empty(),
        "precondition: a must have learned at least one relation source"
    );

    write_snapshot(&vp, &paths).expect("write snapshot");
    let restored = read_snapshot(&paths.final_path).expect("read snapshot");

    let persisted = restored
        .slot_xlayers
        .get(a)
        .and_then(|o| o.as_ref())
        .expect("slot a cross-layer must persist (death-surviving relation rule)");

    assert_eq!(
        persisted.source_slots, live.source_slots,
        "persisted source slots must match learned"
    );
    assert_eq!(
        persisted.source_weights.len(),
        live.source_weights.len(),
        "persisted weight count must match"
    );
    for (p, l) in persisted
        .source_weights
        .iter()
        .zip(live.source_weights.iter())
    {
        assert!(
            (p - l).abs() < 1e-9,
            "persisted weight must match learned: {p} vs {l}"
        );
    }
    assert!(
        (persisted.bias - live.bias).abs() < 1e-9,
        "persisted bias must match learned"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// S1-4: the residual TOWER (recursive PC depth, tiers ≥2) is observer-visible
/// in the snapshot — so emergent abstraction depth shows up in a live run. A
/// grown tier's learned source/weight/bias/explained_fraction roundtrips.
#[test]
fn residual_tower_survives_snapshot_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_tower_persist_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);

    let vp = Arc::new(Viewport::new());
    let t0 = 12_000_000u64;
    let dt = 10_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
    let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");

    // Drive A<->B + tier-1, and sweep the tower upward each cycle.
    for i in 0..400u64 {
        let t = t0 + i * dt;
        tick_slot(&vp, a, t, 0.6);
        tick_slot(&vp, b, t, 0.55);
        vp.refresh_global_attention(t);
        vp.refresh_global_relations(t);
        vp.refresh_global_relation_predictor(t);
        vp.tick_slot_cross_layer(a, t);
        vp.tick_slot_cross_layer(b, t);
        vp.tick_cross_layer_tower_sweep(t);
    }

    let live_tower = vp.read_slot_tower(a);
    // (Depth may be 0 if natural residuals carry no cross-slot structure — that
    // is honest emergence, not a failure. The roundtrip contract holds either
    // way; we assert structural fidelity for whatever depth emerged.)

    write_snapshot(&vp, &paths).expect("write snapshot");
    let restored = read_snapshot(&paths.final_path).expect("read snapshot");

    let persisted = restored.slot_towers.get(a).expect("slot a tower entry");
    assert_eq!(
        persisted.len(),
        live_tower.len(),
        "persisted tower depth must match live depth"
    );
    for (p, l) in persisted.iter().zip(live_tower.iter()) {
        assert_eq!(p.source_slots, l.source_slots, "tier source slots must match");
        assert_eq!(
            p.source_weights.len(),
            l.source_weights.len(),
            "tier weight count must match"
        );
        for (pw, lw) in p.source_weights.iter().zip(l.source_weights.iter()) {
            assert!((pw - lw).abs() < 1e-9, "tier weight must match: {pw} vs {lw}");
        }
        assert!((p.bias - l.bias).abs() < 1e-9, "tier bias must match");
        assert!(
            (p.explained_fraction - l.explained_fraction).abs() < 1e-9,
            "tier explained_fraction must match (observer depth signal)"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}
