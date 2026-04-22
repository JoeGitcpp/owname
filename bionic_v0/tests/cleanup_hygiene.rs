//! Cleanup spec 2026-04-20 hygiene regression.
//!
//! Guards against accidental re-introduction of the rejected
//! semantic-readiness / symbolizer / concept-id architecture into the
//! active test suite, snapshot schema, or substrate APIs.

#[test]
fn active_test_suite_contains_no_semantic_contract_files() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
    assert!(
        !std::path::Path::new(&format!("{}/semantic_readiness.rs", root)).exists(),
        "tests/semantic_readiness.rs must not exist (quarantined)"
    );
    assert!(
        !std::path::Path::new(&format!("{}/symbolizer.rs", root)).exists(),
        "tests/symbolizer.rs must not exist (quarantined)"
    );
}

#[test]
fn snapshot_schema_contains_no_semantic_or_symbolizer_sections() {
    let snap_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot.rs"))
        .expect("read snapshot.rs");
    assert!(
        !snap_src.contains("SemanticReadinessState"),
        "snapshot.rs must not reference SemanticReadinessState"
    );
    assert!(
        !snap_src.contains("SymbolizerState"),
        "snapshot.rs must not reference SymbolizerState"
    );
    assert!(
        !snap_src.contains("write_semantic_readiness"),
        "snapshot.rs must not contain semantic-readiness encoder"
    );
    assert!(
        !snap_src.contains("write_symbolizer"),
        "snapshot.rs must not contain symbolizer encoder"
    );
}

#[test]
fn viewport_exposes_no_semantic_or_symbolizer_api() {
    let vp_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/viewport.rs"))
        .expect("read viewport.rs");
    for forbidden in [
        "pub fn refresh_semantic_readiness",
        "pub fn read_semantic_readiness",
        "pub fn refresh_symbolizer",
        "pub fn read_symbolizer",
        "pub struct SemanticReadinessState",
        "pub struct SymbolizerState",
        "pub struct ObjectConceptBinding",
        "pub struct RelationConceptBinding",
    ] {
        assert!(
            !vp_src.contains(forbidden),
            "viewport.rs must not declare `{}` (cleanup spec 2026-04-20)",
            forbidden
        );
    }
}
