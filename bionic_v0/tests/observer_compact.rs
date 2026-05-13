#[test]
fn observer_compact_source_exposes_only_short_numeric_trace_columns() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer_helpers/compact.rs"
    ))
    .expect("observer compact helper must exist");

    for required in [
        "compact_header",
        "compact_row",
        "code_frames",
        "egress_frames",
        "egress_pressure",
        "egress_deviation",
        "pair_similarity",
        "pairs",
        "probe_i",
        "evidence_p",
        "voc_tE",
    ] {
        assert!(
            src.contains(required),
            "missing compact trace field: {required}"
        );
    }

    for forbidden in ["prompt", "chat", "answer", "meaning", "token_id"] {
        assert!(
            !src.contains(forbidden),
            "compact observer must not introduce {forbidden}"
        );
    }
}
