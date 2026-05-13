#[test]
fn body_runtime_wires_live_vocal_ticks_and_output_adapter() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    let target_tick = body
        .find("tick_vocal_target_from_vib")
        .expect("body.rs must bridge live VIB ingress into vocal_target");
    let imitation_tick = body
        .find("tick_vocal_imitation_loop")
        .expect("body.rs must wire the live vocal imitation loop into runtime");
    assert!(
        target_tick < imitation_tick,
        "vocal target refresh must run before imitation loop consumes the target"
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
