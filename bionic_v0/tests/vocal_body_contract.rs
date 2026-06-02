#[test]
fn body_runtime_wires_live_vocal_ticks_and_output_adapter() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    // 改造6: the three former vocal drivers were unified into ONE outlet
    // (tick_vocal_drive_field). External sound is still observed into the
    // target first; the unified outlet then combines all drive sources and
    // voices once. So body must bridge VIB→target, then run the unified outlet.
    let target_tick = body
        .find("tick_vocal_target_from_vib")
        .expect("body.rs must bridge live VIB ingress into vocal_target");
    let outlet_tick = body
        .find("tick_vocal_drive_field")
        .expect("body.rs must wire the unified vocal drive outlet into runtime");
    assert!(
        target_tick < outlet_tick,
        "vocal target refresh must run before the unified outlet consumes the target"
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
