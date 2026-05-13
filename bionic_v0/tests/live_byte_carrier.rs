#[test]
fn live_byte_carrier_source_has_no_request_or_execution_surface() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/live_byte_carrier.rs"
    ))
    .expect("live_byte_carrier.rs must exist");

    for forbidden in [
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
        "command_executor",
        "std::process::Command",
        "stdin",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "live byte carrier must not contain forbidden surface: {forbidden}"
        );
    }
}

#[test]
fn live_file_carrier_reads_new_bytes_as_bounded_frames() {
    use bionic_v0::code_ingress_adapter::FramedByteCarrier;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_carrier_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");

    let mut carrier = LiveByteFileCarrier::new(path.clone(), 41, 4);
    assert!(carrier.try_recv_frame().is_none());

    std::fs::write(&path, b"abcdef").unwrap();

    let first = carrier.try_recv_frame().expect("first frame");
    let second = carrier.try_recv_frame().expect("second frame");
    assert!(carrier.try_recv_frame().is_none());

    assert_eq!(first.stream_context_id, 41);
    assert_eq!(first.bytes, b"abcd");
    assert_eq!(second.bytes, b"ef");
    assert_eq!(first.framing_validity, 1.0);
}

#[test]
fn live_file_carrier_only_reads_newly_appended_bytes() {
    use bionic_v0::code_ingress_adapter::FramedByteCarrier;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("bionic_v0_live_byte_tail_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");
    std::fs::write(&path, b"abc").unwrap();

    let mut carrier = LiveByteFileCarrier::new(path.clone(), 42, 8);
    assert_eq!(carrier.try_recv_frame().unwrap().bytes, b"abc");
    assert!(carrier.try_recv_frame().is_none());

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    file.write_all(b"def").unwrap();
    drop(file);

    assert_eq!(carrier.try_recv_frame().unwrap().bytes, b"def");
    assert!(carrier.try_recv_frame().is_none());
}

#[test]
fn live_file_carrier_feeds_code_ingress_adapter() {
    use std::sync::Arc;

    use bionic_v0::code_ingress_adapter::CodeIngressAdapter;
    use bionic_v0::live_byte_carrier::LiveByteFileCarrier;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_adapter_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");
    std::fs::write(&path, b"abcabc").unwrap();

    let vp = Arc::new(Viewport::new());
    let carrier = LiveByteFileCarrier::new(path, 43, 3);
    let mut adapter = CodeIngressAdapter::new(carrier);
    adapter.tick(&vp, 1_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 2);
    assert_eq!(st.sequence_windows.len(), 1);
    assert_eq!(st.recent_frames[0].stream_context_id, 43);
}

#[test]
fn body_runtime_wires_live_byte_carrier_without_stdio_surface() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");

    assert!(body.contains("spawn_live_byte_ingress"));
    assert!(body.contains("live_bytes"));
    assert!(body.contains("in.frames"));

    for forbidden in ["stdin", "std::process::Command", "println!", "eprintln!"] {
        assert!(
            !body.contains(forbidden),
            "body must not expose live byte carrier through {forbidden}"
        );
    }
}

#[test]
fn live_body_ingests_appended_byte_file_without_text_output() {
    use bionic_v0::snapshot::{read_snapshot, SnapshotPaths};
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let dir = std::env::temp_dir().join(format!("bionic_v0_live_body_byte_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_body"))
        .arg(&dir)
        .arg("2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn body");

    std::thread::sleep(Duration::from_millis(250));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("live_bytes").join("in.frames"))
        .unwrap();
    file.write_all(b"abcabcabc\n(()())\n001001\n").unwrap();
    drop(file);

    let output = child.wait_with_output().expect("wait body");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    let paths = SnapshotPaths::new(&dir);
    let snap = read_snapshot(&paths.final_path).expect("read body snapshot");
    assert!(
        !snap.code_ingress.recent_frames.is_empty(),
        "body snapshot should carry live byte frames"
    );
    assert!(!snap.code_ingress.sequence_windows.is_empty());
}

#[test]
fn code_ingress_windows_raise_evidence_tension_once() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::viewport::Viewport;

    let vp = Viewport::new();
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 71,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc",
        framing_validity: 1.0,
    });
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 71,
        t_start_us: 2_000,
        t_end_us: 2_000,
        bytes: b"001001",
        framing_validity: 1.0,
    });

    vp.tick_evidence_tension_from_code_ingress(3_000);
    let after_first = vp.read_evidence_tension();
    assert!(after_first.unresolved_pressure > 0.0);
    assert_eq!(after_first.history_len, 1);

    vp.tick_evidence_tension_from_code_ingress(4_000);
    let after_second = vp.read_evidence_tension();
    assert_eq!(after_second.history_len, after_first.history_len);
    assert_eq!(
        after_second.unresolved_pressure,
        after_first.unresolved_pressure
    );
}

#[test]
fn body_runtime_wires_code_ingress_pressure_before_probe_refresh() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");
    let pressure_pos = body
        .find("tick_evidence_tension_from_code_ingress")
        .expect("body must fold code ingress into evidence pressure");
    let probe_pos = body
        .find("refresh_probe_law")
        .expect("body must refresh probe law");
    assert!(pressure_pos < probe_pos);
}
