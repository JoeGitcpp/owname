use std::process::Command;

fn sandbox_bin() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("target/release/sandbox_runner")
}

fn setup_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sandbox_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();
    dir
}

fn make_push_halt() -> Vec<u8> {
    vec![0x01, 0x2A, 0x00, 0x0F] // PUSH 42, HALT
}

fn make_add_halt() -> Vec<u8> {
    vec![
        0x01, 0x03, 0x00, // PUSH 3
        0x01, 0x04, 0x00, // PUSH 4
        0x03,             // ADD
        0x08, 0x00,       // STORE r0
        0x0F,             // HALT
    ]
}

fn make_loop_program() -> Vec<u8> {
    // Infinite loop: PUSH 0 (addr), PUSH 1 (cond), JG back to pc=0, HALT
    vec![0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x0A, 0x0F]
}

#[test]
fn sandbox_vm_executes_push_halt() {
    let dir = setup_dir("pushhalt");
    std::fs::write(dir.join("live_bytes").join("out.frames"), &make_push_halt()).unwrap();

    let output = Command::new(sandbox_bin())
        .arg(&dir)
        .arg("--once")
        .output()
        .expect("sandbox_runner should run");

    assert!(output.status.success());

    let result = std::fs::read(dir.join("live_bytes").join("in.frames")).unwrap();
    assert_eq!(result.len(), 32);
    assert_eq!(result[0], 0, "status should be OK");
    assert!(result[10..18] != [0u8; 8], "instruction count should be non-zero");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sandbox_vm_adds_correctly() {
    let dir = setup_dir("add");
    std::fs::write(dir.join("live_bytes").join("out.frames"), &make_add_halt()).unwrap();

    Command::new(sandbox_bin())
        .arg(&dir)
        .arg("--once")
        .output()
        .unwrap();

    let result = std::fs::read(dir.join("live_bytes").join("in.frames")).unwrap();
    assert_eq!(result[0], 0);
    let result_val = i64::from_le_bytes(result[2..10].try_into().unwrap());
    assert_eq!(result_val, 7);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sandbox_vm_unknown_opcode_returns_invalid() {
    let dir = setup_dir("badop");
    let bad = vec![0x01, 0x01, 0x00, 0xFE]; // PUSH 1, then unknown opcode 0xFE
    std::fs::write(dir.join("live_bytes").join("out.frames"), &bad).unwrap();

    Command::new(sandbox_bin())
        .arg(&dir)
        .arg("--once")
        .output()
        .unwrap();

    let result = std::fs::read(dir.join("live_bytes").join("in.frames")).unwrap();
    assert_eq!(result[0], 0xFF, "unknown opcode should be INVALID");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sandbox_vm_infinite_loop_hits_instruction_limit() {
    let dir = setup_dir("loop");
    std::fs::write(dir.join("live_bytes").join("out.frames"), &make_loop_program()).unwrap();

    let output = Command::new(sandbox_bin())
        .arg(&dir)
        .arg("--once")
        .output()
        .unwrap();

    assert!(output.status.success(), "should not crash on infinite loop");

    let result = std::fs::read(dir.join("live_bytes").join("in.frames")).unwrap();
    assert_eq!(result.len(), 32);
    assert_eq!(result[0], 0xFF, "infinite loop should return INVALID");

    let _ = std::fs::remove_dir_all(&dir);
}
