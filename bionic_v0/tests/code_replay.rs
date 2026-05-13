#[test]
fn code_replay_source_uses_framed_carrier_not_parser() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bin/code_replay.rs"
    ))
    .expect("code_replay.rs must exist");

    assert!(src.contains("ScriptedCodeIngressCarrier"));
    assert!(src.contains("CodeIngressAdapter"));
    assert!(src.contains("frame_bytes"));
    assert!(src.contains("chunks(len)"));

    for forbidden in [
        "prompt",
        "chat",
        "answer",
        "token",
        "meaning",
        "parse_sentence",
    ] {
        assert!(
            !src.contains(forbidden),
            "code replay must not introduce {forbidden}"
        );
    }
}

#[test]
fn scripted_code_ingress_carrier_already_records_byte_frames() {
    use std::sync::Arc;

    use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
    use bionic_v0::viewport::Viewport;

    let vp = Arc::new(Viewport::new());
    let carrier = ScriptedCodeIngressCarrier::new(7, vec![b"abc".to_vec(), b"def".to_vec()]);
    let mut adapter = CodeIngressAdapter::new(carrier);

    adapter.tick(&vp, 1_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 2);
    assert_eq!(st.recent_frames[0].stream_context_id, 7);
    assert_eq!(st.recent_frames[0].byte_len, 3);
    assert_eq!(st.sequence_windows.len(), 1);
}

#[test]
fn code_replay_binary_writes_snapshot_with_code_frames() {
    use bionic_v0::snapshot::{read_snapshot, SnapshotPaths};
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!("bionic_v0_code_replay_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.bin");
    std::fs::write(&input, b"abcabcabc\n").unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_code_replay"))
        .arg(&input)
        .arg(&dir)
        .arg("4")
        .status()
        .expect("run code_replay");

    assert!(status.success());
    let paths = SnapshotPaths::new(&dir);
    let snap = read_snapshot(&paths.final_path).expect("read replay snapshot");
    assert_eq!(snap.code_ingress.recent_frames.len(), 3);
    assert_eq!(snap.code_ingress.sequence_windows.len(), 1);
}

#[test]
fn code_replay_binary_rejects_missing_input_without_text_output() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_code_replay_missing_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_code_replay"))
        .arg(dir.join("missing.bin"))
        .arg(&dir)
        .arg("4")
        .output()
        .expect("run code_replay");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
