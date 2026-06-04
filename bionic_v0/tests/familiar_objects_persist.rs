//! C5 persistence: the familiar-objects sediment (and thus the emergent
//! responsible-person bond) survives a checkpoint roundtrip — so the body
//! re-recognizes "还是上次那个" after restart, not just within one lifetime.

use bionic_v0::familiar_objects::{
    read_familiar_objects_checkpoint, write_familiar_objects_checkpoint,
    FamiliarObjectsCheckpointPaths, FamiliarObjectsState,
};

#[test]
fn familiar_objects_survive_checkpoint_roundtrip() {
    let dir = std::env::temp_dir().join(format!("bionic_familiar_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = FamiliarObjectsCheckpointPaths::new(&dir);

    let mut s = FamiliarObjectsState::default();
    // OBJECT_FINGERPRINT_DIM = 8 (4 VIB + 4 PHOT halves).
    let caregiver = [0.5, 0.3, 0.15, 0.05, 0.0, 0.0, 0.0, 0.0];
    for k in 0..12 {
        s.observe(1_000 + k * 100, caregiver);
    }
    s.observe(99_999, [0.05, 0.15, 0.3, 0.5, 0.0, 0.0, 0.0, 0.0]); // a stranger, once
    let before_top = s.most_familiar().cloned().expect("most familiar");

    write_familiar_objects_checkpoint(&s, &paths).expect("write");
    let restored = read_familiar_objects_checkpoint(&paths.final_path).expect("read");

    assert_eq!(
        restored.objects.len(),
        s.objects.len(),
        "object count must persist"
    );
    let after_top = restored.most_familiar().expect("most familiar after restore");
    assert_eq!(
        after_top.recognition_count, before_top.recognition_count,
        "the responsible person's recognition history must survive death"
    );
    for i in 0..before_top.fingerprint.len() {
        assert!(
            (after_top.fingerprint[i] - before_top.fingerprint[i]).abs() < 1e-9,
            "fingerprint must persist exactly"
        );
    }
    assert!(
        (after_top.familiarity - before_top.familiarity).abs() < 1e-9,
        "familiarity must persist"
    );

    std::fs::remove_dir_all(&dir).ok();
}
