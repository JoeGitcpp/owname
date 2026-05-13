use std::path::PathBuf;
use std::sync::Arc;

use bionic_v0::clock::now_us;
use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
use bionic_v0::snapshot::{write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

pub fn frame_bytes(bytes: &[u8], frame_len: usize) -> Vec<Vec<u8>> {
    let len = frame_len.max(1);
    bytes.chunks(len).map(|chunk| chunk.to_vec()).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        std::process::exit(2);
    }

    let input_path = PathBuf::from(&args[1]);
    let runtime_dir = PathBuf::from(&args[2]);
    let frame_len = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);

    let bytes = match std::fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(_) => std::process::exit(1),
    };
    let frames = frame_bytes(&bytes, frame_len);
    let carrier = ScriptedCodeIngressCarrier::new(1, frames);
    let mut adapter = CodeIngressAdapter::new(carrier);
    let vp = Arc::new(Viewport::new());

    adapter.tick(&vp, now_us());
    let paths = SnapshotPaths::new(&runtime_dir);
    let _ = std::fs::create_dir_all(&runtime_dir);
    let _ = write_snapshot(&vp, &paths);
}
