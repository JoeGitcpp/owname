// External research tool — NOT part of AI body.
// Strings / logs permitted here per architecture doc (研发工具).

use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use bionic_v0::language_binding::{BindingStatus, BindingTargetRef, ExternalPatternKind};
use bionic_v0::snapshot::{read_snapshot, SnapshotData};
use bionic_v0::{
    CLAIM_KIND_ALLOCATED, CLAIM_KIND_DROPPED, CLAIM_KIND_REUSED, HEALTH_FAULT, HEALTH_OK,
    HEALTH_QUIET, HEALTH_STALE,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runtime_dir = args.get(1).cloned().unwrap_or_else(|| "runtime".to_string());
    let poll_hz: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let max_polls: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    let snapshot_path = PathBuf::from(&runtime_dir).join("snapshot.bin");
    let period = Duration::from_millis(1000 / poll_hz.max(1));

    println!(
        "# observer start dir={} poll_hz={} max_polls={}",
        runtime_dir, poll_hz, max_polls
    );
    // Header: global + per-band (obs|pred|err|mse|prec) + HL avg err + snap_fails
    println!(
        "seq\tclock_s\thealth\tphot_h\tphot_started_age_ms\tphot_last_frame_age_ms\tsrc_w\tpred_t\t\
         b0:obs|pred|err|mse|prec\tb1:obs|pred|err|mse|prec\t\
         b2:obs|pred|err|mse|prec\tb3:obs|pred|err|mse|prec\t\
         hl_avg_err\tsnap_fails\t\
         amb_nom\tamb_vib:E|S|c\tamb_phot:E|S|c\tamb_mech:E|S|c\t\
         slots(id:mod|kind|nom|age_ms|vib[b0..b3]|phot[b0..]|sp=x,y,wx,wy,p,v|hl:c|a|p|err|bias|age|attn:share|pb)\t\
         claim:seq|kind|slot|score(total|t|sp|m|e|su)|N|min\t\
         ga:f|fs|N|H|top\t\
         rel:s->d:co|ll|pr|cf|pc|pl|pp|pf|err|rp|age_ms;...\t\
         rsn:sl|ce|ni|slots[...]|edges[...]:w...\t\
         mem:nov|per|rst|rec|fill|dom\t\
         bind:n|traces[...]"
    );

    let mut last_seq: u64 = 0;
    let mut polls: u64 = 0;

    loop {
        match read_snapshot(&snapshot_path) {
            Ok(snap) => {
                if snap.seq != last_seq {
                    last_seq = snap.seq;
                    print_row(&snap);
                }
            }
            Err(e) => {
                eprintln!("# snapshot read err: {}", e);
            }
        }
        polls += 1;
        if max_polls > 0 && polls >= max_polls {
            break;
        }
        sleep(period);
    }
}

/// Pure rendering — the observer never recomputes thresholds. Body writes
/// `health_state` into the viewport; we display the number.
fn health_label(snap: &SnapshotData) -> &'static str {
    label_health(snap.health_state)
}

fn label_health(h: u8) -> &'static str {
    match h {
        HEALTH_FAULT => "FAULT",
        HEALTH_OK => "OK",
        HEALTH_QUIET => "QUIET",
        HEALTH_STALE => "STALE",
        _ => "?",
    }
}

fn binding_pattern_label(kind: ExternalPatternKind) -> &'static str {
    match kind {
        ExternalPatternKind::VisualGlyph => "vg",
        ExternalPatternKind::AudiblePattern => "ap",
        ExternalPatternKind::GesturePattern => "gp",
        ExternalPatternKind::SymbolSequence => "ss",
    }
}

fn binding_status_label(status: BindingStatus) -> &'static str {
    match status {
        BindingStatus::Candidate => "cand",
        BindingStatus::Reinforced => "rein",
        BindingStatus::Weakened => "weak",
    }
}

fn binding_target_label(target: BindingTargetRef) -> String {
    match target {
        BindingTargetRef::ObjectSlot { slot_id } => format!("obj{}", slot_id),
        BindingTargetRef::RelationEdge { edge_index } => format!("rel{}", edge_index),
        BindingTargetRef::TimeWindow {
            start_t_us,
            end_t_us,
        } => format!("t{}-{}", start_t_us, end_t_us),
    }
}

fn print_row(snap: &SnapshotData) {
    let clock_s = (snap.clock_us.saturating_sub(snap.start_us)) as f64 * 1e-6;

    // Age of PHOT startup and last-frame times. Observer renders snapshot
    // truth — it never recomputes phot_health from these values.
    let phot_started_age_ms = if snap.phot_started_t_us > 0 {
        format!(
            "{:.0}",
            snap.clock_us.saturating_sub(snap.phot_started_t_us) as f64 / 1000.0
        )
    } else {
        "-".to_string()
    };
    let phot_last_frame_age_ms = if snap.phot_last_frame_t_us > 0 {
        format!(
            "{:.0}",
            snap.clock_us.saturating_sub(snap.phot_last_frame_t_us) as f64 / 1000.0
        )
    } else {
        "-".to_string()
    };

    let mut row = format!(
        "{}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{}",
        snap.seq,
        clock_s,
        health_label(snap),
        label_health(snap.phot_health),
        phot_started_age_ms,
        phot_last_frame_age_ms,
        snap.source_writes,
        snap.predictor_ticks,
    );

    for ax in snap.axes.iter() {
        let obs = ax.observed.last().map(|s| s.1).unwrap_or(f64::NAN);
        let pred = ax.predicted.last().map(|s| s.1).unwrap_or(f64::NAN);
        let err = ax.err.last().map(|s| s.1).unwrap_or(f64::NAN);
        row.push_str(&format!(
            "\t{:.5}|{:.5}|{:+.5}|{:.5}|{:.3}",
            obs, pred, err, ax.mse, ax.precision
        ));
    }

    row.push_str(&format!(
        "\t{:+.5}\t{}",
        snap.hl_meta_err, snap.snapshot_fail_count,
    ));

    // v7: ambient (shared nomination_seq + per-modality triples)
    row.push_str(&format!("\t{}", snap.ambient_nomination_seq));
    for sub in [&snap.ambient_vib, &snap.ambient_phot, &snap.ambient_mech] {
        row.push_str(&format!(
            "\t{:.5}|{:.5}|{}",
            sub.event_energy,
            sub.surprise,
            if sub.candidate_flag { 1 } else { 0 },
        ));
    }

    // v5: slots — one block per active slot
    row.push('\t');
    let mut first = true;
    for (i, s) in snap.slots.iter().enumerate() {
        if !s.active {
            continue;
        }
        let age_ms = snap.clock_us.saturating_sub(s.last_update_t_us) as f64 / 1000.0;
        if !first {
            row.push(';');
        }
        first = false;
        let vib_block = match s.vib.as_ref() {
            None => "-".to_string(),
            Some(f) => {
                let get = |idx: usize| f.observed.get(idx).copied().unwrap_or(f64::NAN);
                format!(
                    "{:.4},{:.4},{:.4},{:.4}",
                    get(0),
                    get(1),
                    get(2),
                    get(3),
                )
            }
        };
        let phot_block = match s.phot.as_ref() {
            None => "-".to_string(),
            Some(f) => {
                // Phase 4C: show obs>pred±err[prec] per band.
                let parts: Vec<String> = (0..f.observed.len())
                    .map(|i| {
                        let obs = f.observed.get(i).copied().unwrap_or(f64::NAN);
                        let pred = f.predicted.get(i).copied().unwrap_or(f64::NAN);
                        let err = f.err.get(i).copied().unwrap_or(f64::NAN);
                        let prec = f.precision.get(i).copied().unwrap_or(f64::NAN);
                        format!("{:.4}>{:.4}{:+.4}[{:.3}]", obs, pred, err, prec)
                    })
                    .collect();
                parts.join(",")
            }
        };
        let sp_block = match s.spatial.as_ref() {
            None => "-".to_string(),
            Some(sp) => format!(
                "{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                sp.pos_x,
                sp.pos_y,
                sp.span_x,
                sp.span_y,
                sp.precision,
                if sp.valid { 1 } else { 0 },
            ),
        };
        // High-layer block: snapshot truth only; observer never recomputes.
        let hl_block = match s.high_layer.as_ref() {
            None => "hl=-".to_string(),
            Some(hl) => {
                let hl_age_ms = snap.clock_us
                    .saturating_sub(hl.last_update_t_us) as f64 / 1000.0;
                format!(
                    "hl:c{:.3}|a{:.3}|p{:.3}|err{:.4}|bias{:.3}|{:.0}ms",
                    hl.predicted_confidence,
                    hl.predicted_alignment,
                    hl.predicted_precision,
                    hl.object_err,
                    hl.attention_bias,
                    hl_age_ms,
                )
            }
        };
        // Joint marker is pure rendering — the observer never binds; it
        // reports what the viewport already decided.
        let kind = match (s.vib.is_some(), s.phot.is_some()) {
            (true, true) => "JOINT",
            (true, false) => "VIB",
            (false, true) => "PHOT",
            (false, false) => "-",
        };
        let attn_block = format!(
            "attn:{:.3}|pb:{:.2}",
            s.attention_share,
            s.precision_bias,
        );
        row.push_str(&format!(
            "{}:m{:02x}|{}|nom{}|{:.0}ms|vib={}|phot={}|sp={}|{}|{}",
            i,
            s.source_modality,
            kind,
            s.nomination_seq,
            age_ms,
            vib_block,
            phot_block,
            sp_block,
            hl_block,
            attn_block,
        ));
    }
    if first {
        row.push('-');
    }

    // v10: last claim debug — observer renders the numeric record written by
    // the viewport. Does NOT recompute scores.
    let lc = &snap.last_claim;
    let kind = match lc.kind {
        CLAIM_KIND_DROPPED => "DROP",
        CLAIM_KIND_REUSED => "REUSE",
        CLAIM_KIND_ALLOCATED => "ALLOC",
        _ => "?",
    };
    row.push_str(&format!(
        "\t{}|{}|s{}|{:.3}(t{:.2},sp{:.2},m{:.2},e{:.2},su{:.2})|N{}|min{:.2}",
        lc.seq,
        kind,
        lc.chosen_slot,
        lc.best_score,
        lc.best_time_score,
        lc.best_spatial_score,
        lc.best_modality_score,
        lc.best_energy_score,
        lc.best_surprise_score,
        lc.n_candidates_considered,
        lc.min_reuse_score,
    ));

    // Global attention column — observer renders; never recomputes scores.
    let ga = &snap.global_attention;
    let top_str: String = ga.top_slot_ids.iter()
        .filter(|&&id| id >= 0)
        .map(|&id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    row.push_str(&format!(
        "\tga:f{}|fs{:.3}|N{}|H{:.3}|top=[{}]",
        ga.focused_slot_id,
        ga.focused_score,
        ga.n_active,
        ga.attention_entropy,
        top_str,
    ));

    // Relation column — observer renders snapshot truth; never recomputes scores.
    // pred sub-block fields come from RelationEdgePredictionState written by
    // refresh_global_relation_predictor; observer reads them verbatim.
    let gr = &snap.global_relations;
    let mut rel_parts: Vec<String> = Vec::new();
    for edge in gr.edges.iter() {
        if !edge.active { continue; }
        let pred_age_ms = snap.clock_us
            .saturating_sub(edge.pred.last_predict_t_us) as f64 / 1000.0;
        rel_parts.push(format!(
            "s{}->d{}:co{:.3}|ll{:+.3}|pr{:.3}|cf{:.2}|pc{:.3}|pl{:+.3}|pp{:.3}|pf{:.2}|err{:.4}|rp{:.4}|{:.0}ms",
            edge.src_slot_id,
            edge.dst_slot_id,
            edge.co_change_score,
            edge.lead_lag_score,
            edge.predictive_score,
            edge.relation_confidence,
            edge.pred.predicted_co_change,
            edge.pred.predicted_lead_lag,
            edge.pred.predicted_predictive,
            edge.pred.predicted_confidence,
            edge.pred.relation_err,
            edge.pred.relation_precision,
            pred_age_ms,
        ));
    }
    let rel_str = if rel_parts.is_empty() {
        "-".to_string()
    } else {
        rel_parts.join(";")
    };
    row.push_str(&format!("\trel:{}", rel_str));

    // Reasoning column — observer renders snapshot truth; never recomputes.
    // rsn:sl<load>|ce<coord>|ni<instab>|slots[s0,s1,...]|edges[idx:w,...]
    let rs = &snap.global_reasoning;
    let slot_str: String = rs.routing_focus_slot_ids.iter()
        .filter(|&&id| id >= 0)
        .map(|&id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let edge_str: String = rs.routing_focus_edge_ids.iter()
        .zip(rs.routing_weights.iter())
        .take_while(|(&id, _)| id >= 0)
        .map(|(&id, &w)| format!("{}:{:.3}", id, w))
        .collect::<Vec<_>>()
        .join(",");
    row.push_str(&format!(
        "\trsn:sl{:.4}|ce{:.4}|ni{:.4}|slots[{}]|edges[{}]",
        rs.global_surprise_load,
        rs.coordinated_event_score,
        rs.network_instability_score,
        if slot_str.is_empty() { "-".to_string() } else { slot_str },
        if edge_str.is_empty() { "-".to_string() } else { edge_str },
    ));

    // Phase 5G.1: memory summary column — observer renders snapshot truth;
    // never recomputes novelty / persistence / recurrence / stability.
    // Values come from Viewport::refresh_global_reasoning_history (v20 field).
    let mem = &snap.global_reasoning_memory;
    row.push_str(&format!(
        "\tmem:nov{:.2}|per{:.2}|rst{:.2}|rec{:.2}|fill{:.2}|dom{:.2}",
        mem.novelty_score,
        mem.persistence_score,
        mem.routing_stability_score,
        mem.pattern_recurrence_score,
        mem.history_fill_ratio,
        mem.dominant_focus_persistence,
    ));

    let bind = &snap.language_binding;
    let trace_str = if bind.traces.is_empty() {
        "-".to_string()
    } else {
        bind.traces
            .iter()
            .map(|trace| {
                format!(
                    "{}:{}->{}|s{:.2}|{}|ev{}|r{}|w{}",
                    binding_pattern_label(trace.pattern_kind),
                    trace.pattern_hash,
                    binding_target_label(trace.target),
                    trace.strength.value,
                    binding_status_label(trace.status),
                    trace.evidence_refs.len(),
                    trace.reinforce_count,
                    trace.weaken_count,
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    row.push_str(&format!("\tbind:n{}|{}", bind.n_bindings, trace_str));

    println!("{}", row);
}
