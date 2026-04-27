#[test]
fn body_runtime_wires_live_vocal_ticks_and_output_adapter() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    assert!(
        body.contains("tick_vocal_imitation_loop"),
        "body.rs must wire the live vocal imitation loop into runtime"
    );
    assert!(
        body.contains("tick_vocal_memory"),
        "body.rs must wire live vocal memory maintenance into runtime"
    );
    assert!(
        body.contains("spawn_vocal_output"),
        "body.rs must start the speaker-facing vocal adapter"
    );
}
