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
use bionic_v0::vocal_sequence::VocalSequence;
use bionic_v0::perturbation_graph::PerturbationGraph;

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
        if let Err(e) = bionic_v0::atomic_write(&master_path, &master.serialize()) {
            eprintln!("merge_artifacts: write {master_path:?} failed: {e}");
        }
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
        if let Err(e) = bionic_v0::atomic_write(&master_path, &master.serialize()) {
            eprintln!("merge_artifacts: write {master_path:?} failed: {e}");
        }
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
        if let Err(e) = bionic_v0::atomic_write(&master_path, &master.serialize()) {
            eprintln!("merge_artifacts: write {master_path:?} failed: {e}");
        }
    }
    n
}

// #8: the sequence layer's transition edges. Same path as recall (its nodes):
// the mainline harvests vocal_sequence.bin to the pool, and here it is MERGED
// (union edges, keep the stronger count per from→to — VocalSequence::merge) into
// master_vocal_sequence.bin, so learned vocal ORDER accumulates across restarts
// and branches instead of living only in /tmp (built-then-lost).
fn merge_sequence(master_dir: &Path, pool_dir: &Path) -> usize {
    let master_path = master_dir.join("master_vocal_sequence.bin");
    let mut master = fs::read(&master_path)
        .ok()
        .and_then(|b| VocalSequence::deserialize(&b))
        .unwrap_or_default();
    let mut n = 0;
    for f in pool_files(pool_dir, "__vocal_sequence.bin") {
        if let Some(branch) = fs::read(&f).ok().and_then(|b| VocalSequence::deserialize(&b)) {
            master.merge(&branch);
            n += 1;
        }
        let _ = fs::remove_file(&f);
    }
    if n > 0 {
        if let Err(e) = bionic_v0::atomic_write(&master_path, &master.serialize()) {
            eprintln!("merge_artifacts: write {master_path:?} failed: {e}");
        }
    }
    n
}

// Phase 3: the tier-0 perturbation graph. Same pool→merge path as the vocal
// sequence layer: the mainline harvests `perturbation_graph.bin` to the pool,
// and here it is MERGED (union nodes by signature distance, blend matched
// stats, remap edges across lineages — PerturbationGraph::merge) into
// `master_perturbation_graph.bin`. Visitor branches' patches accumulate into
// the master graph instead of being clobbered each harvest.
//
// merge_dist 0.25 is slightly tighter than vision_tier0's TIER0_SIG_MATCH_DIST
// (0.30): cross-lineage merge should err on the side of treating two close
// signatures as the SAME node (since they came from the same world), but tight
// enough to keep visually-distinct patches separate.
fn merge_perturbation_graph(master_dir: &Path, pool_dir: &Path) -> usize {
    let master_path = master_dir.join("master_perturbation_graph.bin");
    let mut master = fs::read(&master_path)
        .ok()
        .and_then(|b| PerturbationGraph::deserialize(&b))
        .unwrap_or_default();
    let mut n = 0;
    for f in pool_files(pool_dir, "__perturbation_graph.bin") {
        if let Some(branch) = fs::read(&f).ok().and_then(|b| PerturbationGraph::deserialize(&b)) {
            master.merge(&branch, 0.25);
            n += 1;
        }
        let _ = fs::remove_file(&f);
    }
    if n > 0 {
        if let Err(e) = bionic_v0::atomic_write(&master_path, &master.serialize()) {
            eprintln!("merge_artifacts: write {master_path:?} failed: {e}");
        }
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
    let seq = merge_sequence(master_dir, pool_dir);
    let pg = merge_perturbation_graph(master_dir, pool_dir);
    println!(
        "merge_artifacts: recall={recall} vocal_memory={vmem} phoneme={phon} sequence={seq} perturbation_graph={pg} (pooled branch artifacts merged into mainline)"
    );
}
