use std::sync::Arc;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn snapshot_roundtrip_preserves_vocal_substrate_state() {
    let dir = std::env::temp_dir().join("bionic_v0_vocal_snap_test");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());
    vp.observe_vocal_target(1_000, 0.7, 0.3, 0.4, 0.2);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();
    assert!(snap.vocal_target.active_energy > 0.0);
}