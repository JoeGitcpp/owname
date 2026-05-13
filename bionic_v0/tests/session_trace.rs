#[test]
fn session_trace_source_defines_csv_without_chat_fields() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/observer_helpers/session.rs"
    ))
    .expect("observer session helper must exist");

    assert!(src.contains("session_header"));
    assert!(src.contains("session_row"));
    assert!(src.contains("append_session_row"));
    assert!(src.contains("code_frames"));
    assert!(src.contains("egress_frames"));
    assert!(src.contains("egress_pressure"));
    assert!(src.contains("egress_deviation"));
    assert!(src.contains("pair_similarity"));
    assert!(src.contains("latest_framed_byte_pair_similarity"));
    assert!(src.contains("pairs"));

    for forbidden in ["prompt", "answer", "role", "message", "meaning", "intent"] {
        assert!(
            !src.contains(forbidden),
            "session trace must not contain chat field: {forbidden}"
        );
    }
}
