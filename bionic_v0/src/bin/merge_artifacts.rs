//! merge_artifacts — distributed-training harvest: fold pooled BRANCH vocal
//! artifacts into the MAINLINE master, by structure-aware merge (not overwrite).
//!
//! Usage:  merge_artifacts <master_dir> <pool_dir>
//!
//! For each prototype artifact type it loads `master_<name>.bin`, merges every
//! pooled `*__<name>.bin` into it (via the type's own `merge`, which sums
//! evidence / keeps the best per target — see vocal_target_recall / vocal_memory
//! / phoneme_memory), writes `master_<name>.bin` back, and deletes the consumed
//! pool files. Run this SINGLE-THREADED (e.g. the gateway's periodic merge tick)
//! so concurrent branches — which only ever WRITE into the pool on session end —
//! never race the master.
//!
//! learned_bias is merged separately (EMA, gateway-side training_pool.mjs).
//! snapshot is intentionally seed-only (mainline grows its own; a short branch's
//! 12 MB cross-layer drift is not worth merging back).

use std::fs;
use std::path::{Path, PathBuf};

use bionic_v0::phoneme_memory::PhonemeMemory;
use bionic_v0::vocal_memory::VocalMemoryState;
use bionic_v0::vocal_target_recall::VocalTargetRecall;

fn pool_files(pool_dir: &Path, suffix: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = fs::read_dir(pool_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with(suffix))
            {
                v.push(p);
            }
        }
    }
    v.sort(); // deterministic ingest order
    v
}

fn merge_recall(master_dir: &Path, pool_dir: &Path) -> usize {
    let master_path = master_dir.join("master_vocal_target_recall.bin");
    let mut master = fs::read(&master_path)
        .ok()
        .and_then(|b| VocalTargetRecall::deserialize(&b))
        .unwrap_or_default();
    let mut n = 0;
    for f in pool_files(pool_dir, "__vocal_target_recall.bin") {
        if let Some(branch) = fs::read(&f).ok().and_then(|b| VocalTargetRecall::deserialize(&b)) {
            master.merge(&branch);
            n += 1;
        }
        let _ = fs::remove_file(&f);
    }
    if n > 0 {
        let _ = fs::write(&master_path, master.serialize());
    }
    n
}

fn merge_vocal_memory(master_dir: &Path, pool_dir: &Path) -> usize {
    let master_path = master_dir.join("master_vocal_memory.bin");
    let mut master = fs::read(&master_path)
        .ok()
        .and_then(|b| VocalMemoryState::deserialize(&b))
        .unwrap_or_default();
    let mut n = 0;
    for f in pool_files(pool_dir, "__vocal_memory.bin") {
        if let Some(branch) = fs::read(&f).ok().and_then(|b| VocalMemoryState::deserialize(&b)) {
            master.merge(&branch);
            n += 1;
        }
        let _ = fs::remove_file(&f);
    }
    if n > 0 {
        let _ = fs::write(&master_path, master.serialize());
    }
    n
}

fn merge_phoneme(master_dir: &Path, pool_dir: &Path) -> usize {
    let master_path = master_dir.join("master_phoneme_memory.bin");
    let mut master = fs::read(&master_path)
        .ok()
        .and_then(|b| PhonemeMemory::deserialize(&b))
        .unwrap_or_default();
    let mut n = 0;
    for f in pool_files(pool_dir, "__phoneme_memory.bin") {
        if let Some(branch) = fs::read(&f).ok().and_then(|b| PhonemeMemory::deserialize(&b)) {
            master.merge(&branch);
            n += 1;
        }
        let _ = fs::remove_file(&f);
    }
    if n > 0 {
        let _ = fs::write(&master_path, master.serialize());
    }
    n
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: merge_artifacts <master_dir> <pool_dir>");
        std::process::exit(2);
    }
    let master_dir = Path::new(&args[1]);
    let pool_dir = Path::new(&args[2]);
    if let Err(e) = fs::create_dir_all(master_dir) {
        eprintln!("merge_artifacts: cannot create master dir: {e}");
        std::process::exit(1);
    }

    let recall = merge_recall(master_dir, pool_dir);
    let vmem = merge_vocal_memory(master_dir, pool_dir);
    let phon = merge_phoneme(master_dir, pool_dir);
    println!(
        "merge_artifacts: recall={recall} vocal_memory={vmem} phoneme={phon} (pooled branch artifacts merged into mainline)"
    );
}
