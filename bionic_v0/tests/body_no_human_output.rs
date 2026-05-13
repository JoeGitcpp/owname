//! Cleanup spec 2026-04-20 §4.5: the body process must not emit
//! human-readable text. All visible output must come from the observer
//! reading snapshot facts. Direct prints inside the body cross the
//! research/body separation red line.

#[test]
fn body_owned_sources_contain_no_human_readable_diagnostics() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let sources = [
        "src/bin/body.rs",
        "src/vib.rs",
        "src/adapter.rs",
        "src/vocal_adapter.rs",
    ];
    let forbidden = [
        "println!",
        "eprintln!",
        "dbg!",
        "panic!(\"",
        "expect(",
        "format!(\"",
        "let _ = reg.register",
    ];

    let mut violations = Vec::new();
    for source in sources {
        let path = std::path::Path::new(manifest_dir).join(source);
        let contents = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", path.display());
        });

        for (line_idx, line) in contents.lines().enumerate() {
            for marker in forbidden {
                if line.contains(marker) {
                    violations.push(format!("{source}:{} contains {marker}", line_idx + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "body-owned sources must not contain human-readable diagnostics:\n{}",
        violations.join("\n")
    );
}
