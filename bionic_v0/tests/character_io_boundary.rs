fn read(path: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|_| String::new())
}

#[test]
fn character_io_sources_do_not_introduce_chat_or_token_ontology() {
    let paths = [
        "src/bin/observer.rs",
        "src/code_ingress.rs",
        "src/code_ingress_adapter.rs",
        "src/framed_byte_egress.rs",
        "src/framed_byte_carrier.rs",
        "src/live_byte_closure.rs",
        "src/live_byte_loopback.rs",
        "src/bin/byte_loopback.rs",
        "src/viewport.rs",
    ];
    let forbidden = [
        "prompt",
        "chat",
        "assistant_message",
        "user_message",
        "system_message",
        "token_id",
        "token_embedding",
        "word_meaning",
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "intent",
        "personality",
        "owner",
        "command_executor",
        "shell",
    ];

    for path in paths {
        let src = read(path);
        for term in forbidden {
            assert!(
                !src.contains(term),
                "{path} must not contain forbidden character-I/O drift term: {term}"
            );
        }
    }
}

#[test]
fn body_owned_sources_still_do_not_print_human_readable_output() {
    let paths = [
        "src/bin/body.rs",
        "src/vocal_adapter.rs",
        "src/code_ingress.rs",
        "src/code_ingress_adapter.rs",
        "src/framed_byte_egress.rs",
        "src/framed_byte_carrier.rs",
        "src/live_byte_closure.rs",
        "src/viewport.rs",
    ];
    for path in paths {
        let src = read(path);
        assert!(!src.contains("println!"), "{path} must not call println!");
        assert!(!src.contains("eprintln!"), "{path} must not call eprintln!");
    }
}
