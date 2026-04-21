//! Cleanup spec 2026-04-20 §4.5: the body process must not emit
//! human-readable text. All visible output must come from the observer
//! reading snapshot facts. Direct prints inside the body cross the
//! research/body separation red line.

#[test]
fn body_contains_no_human_readable_prints() {
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/body.rs"
    ))
    .expect("read body.rs");
    assert!(
        !body.contains("println!"),
        "body.rs must not contain println!"
    );
    assert!(
        !body.contains("eprintln!"),
        "body.rs must not contain eprintln!"
    );
}
