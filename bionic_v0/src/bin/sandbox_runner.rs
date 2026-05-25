//! Sandbox Runner — observer-side bytecode execution environment.
//!
//! Reads body's framed byte output from `live_bytes/out.frames`,
//! tries to execute it with a minimal stack-based bytecode VM,
//! and writes a 32-byte numeric result frame to `live_bytes/in.frames`.
//!
//! This is observer-side infrastructure. No bionic_v0 dependency.
//! Usage: sandbox_runner <runtime_dir> [--once]

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const RESULT_FRAME_LEN: usize = 32;
const MAX_INSTRUCTIONS: usize = 1000;
const MAX_STACK: usize = 256;
const MEM_SIZE: usize = 256;
const REG_COUNT: usize = 8;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut runtime_dir: Option<PathBuf> = None;
    let mut once = false;

    for arg in args.iter().skip(1) {
        if arg == "--once" {
            once = true;
        } else if runtime_dir.is_none() {
            runtime_dir = Some(PathBuf::from(arg));
        }
    }

    let runtime_dir = runtime_dir.unwrap_or_else(|| PathBuf::from("runtime_life"));
    let out_path = runtime_dir.join("live_bytes").join("out.frames");
    let in_path = runtime_dir.join("live_bytes").join("in.frames");
    fs::create_dir_all(runtime_dir.join("live_bytes")).ok();

    eprintln!(
        "sandbox_runner start dir={} once={}",
        runtime_dir.display(),
        once
    );

    loop {
        match fs::read(&out_path) {
            Ok(bytes) if !bytes.is_empty() => {
                let result = run_vm(&bytes);
                let _ = fs::write(&in_path, &result);
                if once {
                    break;
                }
                let _ = fs::write(&out_path, &[]);
            }
            Ok(_) => {
                if once {
                    break;
                }
            }
            Err(_) => {}
        }

        if once {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn run_vm(bytecode: &[u8]) -> [u8; RESULT_FRAME_LEN] {
    let mut regs = [0i64; REG_COUNT];
    let mut stack: Vec<i64> = Vec::with_capacity(MAX_STACK);
    let mut mem = [0u8; MEM_SIZE];
    let mut pc: usize = 0;
    let mut instructions: usize = 0;

    loop {
        if instructions >= MAX_INSTRUCTIONS {
            return invalid_frame(3);
        }
        if pc >= bytecode.len() {
            return ok_frame(regs[0], instructions as u64);
        }

        let op = bytecode[pc];
        pc += 1;
        instructions += 1;

        match op {
            0x00 => {} // NOP
            0x0F => {
                // HALT
                return ok_frame(regs[0], instructions as u64);
            }
            0x01 => {
                // PUSH imm16
                if pc + 2 > bytecode.len() {
                    return invalid_frame(1);
                }
                let val = u16::from_le_bytes([bytecode[pc], bytecode[pc + 1]]) as i64;
                pc += 2;
                if stack.len() >= MAX_STACK {
                    return invalid_frame(2);
                }
                stack.push(val);
            }
            0x02 => {
                // POP
                stack.pop();
            }
            0x03 => {
                // ADD
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_add(b));
            }
            0x04 => {
                // SUB
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_sub(b));
            }
            0x05 => {
                // MUL
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(a.wrapping_mul(b));
            }
            0x06 => {
                // DIV
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                if b == 0 {
                    stack.push(0);
                } else {
                    stack.push(a / b);
                }
            }
            0x07 => {
                // LOAD reg
                if pc >= bytecode.len() {
                    return invalid_frame(1);
                }
                let r = (bytecode[pc] as usize) % REG_COUNT;
                pc += 1;
                stack.push(regs[r]);
            }
            0x08 => {
                // STORE reg
                if pc >= bytecode.len() {
                    return invalid_frame(1);
                }
                let r = (bytecode[pc] as usize) % REG_COUNT;
                pc += 1;
                regs[r] = stack.pop().unwrap_or(0);
            }
            0x09 => {
                // CMP: push 1 if equal, 0 otherwise
                let b = stack.pop().unwrap_or(0);
                let a = stack.pop().unwrap_or(0);
                stack.push(if a == b { 1 } else { 0 });
            }
            0x0A => {
                // JG: pop addr, cond; jump if cond > 0
                let cond = stack.pop().unwrap_or(0);
                let addr = stack.pop().unwrap_or(0);
                if cond > 0 {
                    pc = (addr as usize).min(bytecode.len());
                }
            }
            0x0B => {
                // JL: pop addr, cond; jump if cond < 0
                let cond = stack.pop().unwrap_or(0);
                let addr = stack.pop().unwrap_or(0);
                if cond < 0 {
                    pc = (addr as usize).min(bytecode.len());
                }
            }
            0x0C => {
                // JE: pop addr, cond; jump if cond == 0
                let cond = stack.pop().unwrap_or(0);
                let addr = stack.pop().unwrap_or(0);
                if cond == 0 {
                    pc = (addr as usize).min(bytecode.len());
                }
            }
            0x0D => {
                // STOREM: pop addr, val; mem[addr] = val as u8
                let val = stack.pop().unwrap_or(0);
                let addr = stack.pop().unwrap_or(0) as usize % MEM_SIZE;
                mem[addr] = (val & 0xFF) as u8;
            }
            0x0E => {
                // LOADM: pop addr; push mem[addr] as i64
                let addr = stack.pop().unwrap_or(0) as usize % MEM_SIZE;
                stack.push(mem[addr] as i64);
            }
            _ => {
                return invalid_frame(0); // unknown opcode
            }
        }
    }
}

fn ok_frame(result: i64, instructions: u64) -> [u8; RESULT_FRAME_LEN] {
    let mut frame = [0u8; RESULT_FRAME_LEN];
    frame[0] = 0;
    frame[1] = 0;
    frame[2..10].copy_from_slice(&result.to_le_bytes());
    frame[10..18].copy_from_slice(&instructions.to_le_bytes());
    frame
}

fn invalid_frame(reason: u8) -> [u8; RESULT_FRAME_LEN] {
    let mut frame = [0u8; RESULT_FRAME_LEN];
    frame[0] = 0xFF;
    frame[1] = reason;
    frame
}
