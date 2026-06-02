//! observe_tower — read-only observer for the recursive predictive-coding DEPTH
//! (the per-slot residual tower) out of a body snapshot.
//!
//! This is the ONLY way to SEE emergent abstraction depth in a live run: the
//! tower is persisted in snapshot v51 at a variable offset, so it must be read
//! through the real Rust deserializer (not by fixed-offset byte poking). It is
//! an OBSERVER tool — it reads the snapshot the body writes; it never touches
//! the body or claims to be its voice (Prime Directive 推论2).
//!
//! Usage:
//!   observe_tower <path>      where <path> is a runtime dir OR a snapshot.bin
//!
//! Output (per slot that grew structure):
//!   tier 1 = the slot's object_err predictor (mse, #sources)
//!   tier ≥2 = residual tower: each tier's explained_fraction + #sources
//!   total_depth = 1 (tier 1) + tower height
//! plus a summary line: max depth, how many slots grew a tower.

use std::env;
use std::path::PathBuf;

use bionic_v0::snapshot::read_snapshot;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: observe_tower <runtime_dir | snapshot.bin>");
        std::process::exit(2);
    }
    let arg = PathBuf::from(&args[1]);
    let snapshot_path = if arg.is_dir() {
        arg.join("snapshot.bin")
    } else {
        arg
    };

    let snap = match read_snapshot(&snapshot_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not read snapshot {}: {e}", snapshot_path.display());
            std::process::exit(1);
        }
    };

    println!("=== residual tower (recursive PC depth) @ {} ===", snapshot_path.display());
    println!(
        "(tier 1 = object_err predictor; tier ≥2 = residual-of-residual; depth emerges from learnable structure, self-limits on noise)"
    );

    let n_slots = snap.slot_towers.len().max(snap.slot_xlayers.len());
    let mut max_depth = 0usize;
    let mut slots_with_tower = 0usize;
    let mut slots_with_tier1 = 0usize;

    for slot in 0..n_slots {
        let tier1 = snap.slot_xlayers.get(slot).and_then(|o| o.as_ref());
        let tower = snap.slot_towers.get(slot).map(|t| t.as_slice()).unwrap_or(&[]);

        // Skip slots with nothing at all.
        if tier1.is_none() && tower.is_empty() {
            continue;
        }
        if tier1.is_some() {
            slots_with_tier1 += 1;
        }
        if !tower.is_empty() {
            slots_with_tower += 1;
        }

        let depth = (tier1.is_some() as usize) + tower.len();
        max_depth = max_depth.max(depth);

        let mut line = format!("slot {slot:>2}  depth={depth}");
        if let Some(t1) = tier1 {
            line.push_str(&format!("  | T1(mse={:.5}, nsrc={})", t1.err_pred_mse, t1.source_slots.len()));
        }
        for (i, tier) in tower.iter().enumerate() {
            line.push_str(&format!(
                "  T{}(expl={:.2}, mse={:.5}, nsrc={})",
                i + 2,
                tier.explained_fraction,
                tier.err_pred_mse,
                tier.source_slots.len()
            ));
        }
        println!("{line}");
    }

    println!(
        "--- summary: max_depth={max_depth}  slots_with_tier1={slots_with_tier1}  slots_with_tower(≥tier2)={slots_with_tower} ---"
    );
    if slots_with_tower == 0 {
        println!(
            "(no slot grew a tier ≥2 yet — either too early, or the substrate's residuals carry no learnable cross-slot structure. This is honest emergence, not a bug.)"
        );
    }
}
