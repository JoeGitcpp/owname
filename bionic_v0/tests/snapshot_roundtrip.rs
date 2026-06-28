use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

// ── P0: snapshot persistence invariants ─────────────────────────────
// Guards against the class of bugs: non-atomic writes, silent truncation,
// data loss across restart (persistence fix commits: 2/34).

fn write_and_read(vp: &Arc<Viewport>) -> bionic_v0::snapshot::SnapshotData {
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    write_snapshot(vp, &paths).unwrap();
    read_snapshot(&paths.final_path).unwrap()
}

#[test]
fn roundtrip_preserves_master_tick() {
    let vp = Arc::new(Viewport::new());
    vp.master_tick
        .store(42, std::sync::atomic::Ordering::Relaxed);
    let data = write_and_read(&vp);
    assert_eq!(data.master_tick, 42);
}

#[test]
fn roundtrip_preserves_clock_us() {
    let vp = Arc::new(Viewport::new());
    vp.clock_us
        .store(123_456_789, std::sync::atomic::Ordering::Relaxed);
    let data = write_and_read(&vp);
    assert_eq!(data.clock_us, 123_456_789);
}

#[test]
fn roundtrip_preserves_maturity() {
    let vp = Arc::new(Viewport::new());
    vp.restore_maturity(0.73);
    let data = write_and_read(&vp);
    assert!(
        (data.developmental_maturity - 0.73).abs() < 1e-9,
        "maturity not preserved: expected 0.73, got {}",
        data.developmental_maturity
    );
}

#[test]
fn roundtrip_preserves_predictor_after_observation() {
    let vp = Arc::new(Viewport::new());
    for i in 0..20 {
        vp.push_observation(0, 1_000_000 + i * 33_000, 0.5 + 0.1 * (i as f64).sin());
    }
    let (a_before, b_before, _, _, _, _) = vp.read_predictor(0);

    let data = write_and_read(&vp);
    let ax = &data.axes[0];
    assert!(
        (ax.a - a_before).abs() < 1e-9,
        "predictor a not preserved: expected {a_before}, got {}",
        ax.a
    );
    assert!(
        (ax.b - b_before).abs() < 1e-9,
        "predictor b not preserved: expected {b_before}, got {}",
        ax.b
    );
}

#[test]
fn roundtrip_preserves_health_state() {
    let vp = Arc::new(Viewport::new());
    vp.set_hearing_fault(true);
    vp.update_health_state(1_000_000);

    let data = write_and_read(&vp);
    assert!(data.hearing_fault, "hearing fault not preserved");
}

#[test]
fn tmp_file_cleaned_up_after_write() {
    let vp = Arc::new(Viewport::new());
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    write_snapshot(&vp, &paths).unwrap();

    assert!(
        !paths.tmp_path.exists(),
        "tmp file should be renamed away after successful write"
    );
    assert!(
        paths.final_path.exists(),
        "final snapshot file should exist"
    );
}

#[test]
fn corrupt_magic_returns_error() {
    let vp = Arc::new(Viewport::new());
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    write_snapshot(&vp, &paths).unwrap();

    let mut bytes = std::fs::read(&paths.final_path).unwrap();
    bytes[0] = 0xFF;
    bytes[1] = 0xFF;
    std::fs::write(&paths.final_path, &bytes).unwrap();

    let result = read_snapshot(&paths.final_path);
    assert!(
        result.is_err(),
        "corrupted magic should return Err, not silent data"
    );
}

#[test]
fn truncated_file_returns_error() {
    let vp = Arc::new(Viewport::new());
    let dir = tempfile::tempdir().unwrap();
    let paths = SnapshotPaths::new(dir.path());
    write_snapshot(&vp, &paths).unwrap();

    let bytes = std::fs::read(&paths.final_path).unwrap();
    std::fs::write(&paths.final_path, &bytes[..50]).unwrap();

    let result = read_snapshot(&paths.final_path);
    assert!(
        result.is_err(),
        "truncated snapshot should return Err, not panic or silent corruption"
    );
}

#[test]
fn empty_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("snapshot.bin");
    std::fs::write(&path, &[]).unwrap();

    let result = read_snapshot(&path);
    assert!(result.is_err(), "empty file should return Err");
}

#[test]
fn nonexistent_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.bin");

    let result = read_snapshot(&path);
    assert!(result.is_err(), "nonexistent file should return Err");
}
