// External research tool — NOT part of AI body.
// Strings / logs permitted here per architecture doc (研发工具).

#[path = "observer_helpers/compact.rs"]
mod observer_compact;
#[path = "observer_helpers/session.rs"]
mod observer_session;

use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use bionic_v0::language_binding::{BindingStatus, BindingTargetRef, PatternKind};
use bionic_v0::snapshot::{read_snapshot, SnapshotData};
use bionic_v0::vpc_cross_layer::AxisCrossLayerState;
use bionic_v0::vpc::{ObservationDomainKind, PredictorLifecycleState, PublicationStatus};
use bionic_v0::{
    CLAIM_KIND_ALLOCATED, CLAIM_KIND_DROPPED, CLAIM_KIND_REUSED, HEALTH_FAULT, HEALTH_OK,
    HEALTH_QUIET, HEALTH_STALE,
};

const OBSERVER_VPC_RECENT_LIMIT: usize = 8;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runtime_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "runtime".to_string());
    let poll_hz: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let max_polls: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let compact = args.get(4).is_some_and(|mode| mode == "compact");
    let session_csv = args.get(5).map(PathBuf::from);
    let mut session_header_written = false;

    let snapshot_path = PathBuf::from(&runtime_dir).join("snapshot.bin");
    let period = Duration::from_millis(1000 / poll_hz.max(1));

    println!(
        "# observer start dir={} poll_hz={} max_polls={}",
        runtime_dir, poll_hz, max_polls
    );
    // Header: global + per-band (obs|pred|err|mse|prec) + HL avg err + snap_fails
    if compact {
        println!("{}", observer_compact::compact_header());
    } else {
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
          bind:n|traces[...]|vpc:n|pub[...]|open[...]\t\
         ep:n|epochs[...]\t\
         voc:tE|tp|tr|te|ob|op|or|oph|ip|ig|ic|if|mn\t\
         prb:i|s|ph|o\t\
         etn:t|p|h|pc\t\
         tf:fault|count\t\
         dd:sp|at|CF(conflict)|NB(net_toward_bond)|pf|sac\t\
         mat:m\t\
         axl:n|act|upd|expl(n|avg|max)|top[ax:e|n|w[src:w,...]]"
        );
    }

    let mut last_seq: u64 = 0;
    let mut polls: u64 = 0;

    loop {
        match read_snapshot(&snapshot_path) {
            Ok(snap) => {
                if snap.seq != last_seq {
                    last_seq = snap.seq;
                    if compact {
                        println!("{}", observer_compact::compact_row(&snap));
                    } else {
                        print_row(&snap);
                    }
                    if let Some(path) = session_csv.as_ref() {
                        let _ = observer_session::append_session_row(
                            path,
                            &snap,
                            !session_header_written,
                        );
                        session_header_written = true;
                    }
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

fn binding_pattern_label(kind: PatternKind) -> &'static str {
    match kind {
        PatternKind::PhotModality => "ph",
        PatternKind::VibModality => "vb",
        PatternKind::MechModality => "mc",
        PatternKind::SymbolSeq => "ss",
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

fn vpc_domain_label(kind: ObservationDomainKind, primary: u16) -> String {
    match kind {
        ObservationDomainKind::Axis => format!("ax{}", primary),
        ObservationDomainKind::SlotFamily => format!("sf{}", primary),
        ObservationDomainKind::RelationEdge => format!("re{}", primary),
        ObservationDomainKind::ReasoningFragment => format!("rf{}", primary),
    }
}

fn vpc_status_label(status: PublicationStatus) -> &'static str {
    match status {
        PublicationStatus::Open => "op",
        PublicationStatus::Resolved => "rs",
        PublicationStatus::Expired => "ex",
    }
}

fn predictor_lifecycle_label(state: PredictorLifecycleState) -> &'static str {
    match state {
        PredictorLifecycleState::InsufficientContext => "ins",
        PredictorLifecycleState::Warmup => "warm",
        PredictorLifecycleState::OpenPublication => "opn",
        PredictorLifecycleState::ResolvedPublication => "res",
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
            obs, pred, err, ax.mse, ax.surprise
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
                format!("{:.4},{:.4},{:.4},{:.4}", get(0), get(1), get(2), get(3),)
            }
        };
        let phot_block = match s.phot.as_ref() {
            None => "-".to_string(),
            Some(f) => {
                // Phase 4C: show obs>pred±err[surp] per band.
                let parts: Vec<String> = (0..f.observed.len())
                    .map(|i| {
                        let obs = f.observed.get(i).copied().unwrap_or(f64::NAN);
                        let pred = f.predicted.get(i).copied().unwrap_or(f64::NAN);
                        let err = f.err.get(i).copied().unwrap_or(f64::NAN);
                        let surp = f.surprise.get(i).copied().unwrap_or(f64::NAN);
                        format!("{:.4}>{:.4}{:+.4}[{:.3}]", obs, pred, err, surp)
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
                let hl_age_ms = snap.clock_us.saturating_sub(hl.last_update_t_us) as f64 / 1000.0;
                format!(
                    "hl:c{:.3}|a{:.3}|s{:.3}|err{:.4}|bias{:.3}|{:.0}ms",
                    hl.predicted_confidence,
                    hl.predicted_alignment,
                    hl.predicted_surprise,
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
        let attn_block = format!("attn:{:.3}|pb:{:.2}", s.attention_share, s.precision_bias,);
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
    let top_str: String = ga
        .top_slot_ids
        .iter()
        .filter(|&&id| id >= 0)
        .map(|&id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    row.push_str(&format!(
        "\tga:f{}|fs{:.3}|N{}|H{:.3}|top=[{}]",
        ga.focused_slot_id, ga.focused_score, ga.n_active, ga.attention_entropy, top_str,
    ));

    // Relation column — observer renders snapshot truth; never recomputes scores.
    // pred sub-block fields come from RelationEdgePredictionState written by
    // refresh_global_relation_predictor; observer reads them verbatim.
    let gr = &snap.global_relations;
    let mut rel_parts: Vec<String> = Vec::new();
    for edge in gr.edges.iter() {
        if !edge.active {
            continue;
        }
        let pred_age_ms = snap.clock_us.saturating_sub(edge.pred.last_predict_t_us) as f64 / 1000.0;
        rel_parts.push(format!(
            "s{}->d{}:co{:.3}|ll{:+.3}|pr{:.3}|cf{:.2}|pc{:.3}|pl{:+.3}|pp{:.3}|pf{:.2}|err{:.4}|rs{:.4}|{:.0}ms",
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
            edge.pred.relation_surprise,
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
    let slot_str: String = rs
        .routing_focus_slot_ids
        .iter()
        .filter(|&&id| id >= 0)
        .map(|&id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let edge_str: String = rs
        .routing_focus_edge_ids
        .iter()
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
        if slot_str.is_empty() {
            "-".to_string()
        } else {
            slot_str
        },
        if edge_str.is_empty() {
            "-".to_string()
        } else {
            edge_str
        },
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

    let code = &snap.code_ingress;
    let code_latest = code
        .recent_frames
        .last()
        .map(|frame| {
            format!(
                "f{}|s{}|len{}|h{:x}|p{}|w{}",
                frame.frame_seq,
                frame.stream_context_id,
                frame.byte_len,
                frame.byte_hash,
                code.projection_candidates.len(),
                code.sequence_windows.len()
            )
        })
        .unwrap_or_else(|| "-".to_string());
    row.push_str(&format!(
        "\tcode:n{}|{}",
        code.recent_frames.len(),
        code_latest
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

    let vpc = &snap.vpc_contract;
    let trace_str = if vpc.predictor_traces.is_empty() {
        "-".to_string()
    } else {
        vpc.predictor_traces
            .iter()
            .skip(
                vpc.predictor_traces
                    .len()
                    .saturating_sub(OBSERVER_VPC_RECENT_LIMIT),
            )
            .map(|trace| {
                format!(
                    "pf{}:{}|{}->{}|{}",
                    trace.predictor_family,
                    vpc_domain_label(trace.domain.kind, trace.domain.primary),
                    trace.horizon.publish_t_us,
                    trace.horizon.target_end_t_us,
                    predictor_lifecycle_label(trace.lifecycle),
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    let pub_str = if vpc.publications.is_empty() {
        "-".to_string()
    } else {
        vpc.publications
            .iter()
            .skip(
                vpc.publications
                    .len()
                    .saturating_sub(OBSERVER_VPC_RECENT_LIMIT),
            )
            .map(|trace| {
                format!(
                    "pf{}:{}|{}->{}|{}",
                    trace.predictor_family,
                    vpc_domain_label(trace.domain.kind, trace.domain.primary),
                    trace.horizon.publish_t_us,
                    trace.horizon.target_end_t_us,
                    vpc_status_label(trace.status),
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    let open_str = if vpc.open_predictions.is_empty() {
        "-".to_string()
    } else {
        vpc.open_predictions
            .iter()
            .skip(
                vpc.open_predictions
                    .len()
                    .saturating_sub(OBSERVER_VPC_RECENT_LIMIT),
            )
            .map(|trace| {
                format!(
                    "pf{}:{}|{}->{}|pi{}",
                    trace.predictor_family,
                    vpc_domain_label(trace.domain.kind, trace.domain.primary),
                    trace.horizon.publish_t_us,
                    trace.horizon.target_end_t_us,
                    trace.publication_index,
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    row.push_str(&format!(
        "\tvpc:n{}|tr{}|tr_recent[{}]|pub{}|pub_recent[{}]|open{}|open_recent[{}]",
        vpc.publications.len(),
        vpc.predictor_traces.len(),
        trace_str,
        vpc.publications.len(),
        pub_str,
        vpc.open_predictions.len(),
        open_str
    ));

    let ao = &snap.action_output;
    let ch_str: String = ao
        .channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            format!(
                "ch{}[k{}|ph{}|m{:.3}|pk{:.3}|s{}|u{}|ws{}]",
                i,
                ch.channel_kind,
                ch.phase,
                ch.current_magnitude,
                ch.peak_magnitude,
                ch.start_t_us,
                ch.last_update_t_us,
                ch.current_write_seq
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    row.push_str(&format!(
        "\tout:n{}|{}",
        ao.n_channels,
        if ch_str.is_empty() {
            "-".to_string()
        } else {
            ch_str
        }
    ));

    // v27: write epoch history — numeric/time/phase only; no target or result labels.
    let ep_str: String = ao
        .write_epoch_history
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            format!(
                "e{}[ws{}|k{}|m{:.3}|s{}|u{}|ph{}]",
                i,
                ep.write_seq,
                ep.channel_kind,
                ep.start_magnitude,
                ep.start_t_us,
                ep.last_update_t_us,
                ep.last_phase
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    row.push_str(&format!(
        "\tep:n{}|{}",
        ao.n_write_epochs,
        if ep_str.is_empty() {
            "-".to_string()
        } else {
            ep_str
        }
    ));

    let acq = &snap.action_consequence;
    let w_str: String = acq
        .windows
        .iter()
        .enumerate()
        .map(|(i, w)| {
            // phase: 0=Open 1=Closed — substrate lifecycle field, no result semantics
            format!(
                "w{}[id{}|m{}|s{}|e{}|mg{:.3}|ph{}|lu{}]",
                i,
                w.window_id,
                w.modality_kind,
                w.start_t_us,
                w.end_t_us,
                w.magnitude,
                w.phase,
                w.last_update_t_us
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let p_str: String = acq
        .pairs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "p{}[ws{}|wid{}|d{}|st{}|cs{:.3}]",
                i, p.write_seq, p.window_id, p.delay_us, p.status, p.curve_similarity
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    row.push_str(&format!(
        "\tacq:w{}|{}|p{}|{}",
        acq.n_windows,
        if w_str.is_empty() {
            "-".to_string()
        } else {
            w_str
        },
        acq.n_pairs,
        if p_str.is_empty() {
            "-".to_string()
        } else {
            p_str
        }
    ));

    let fbe = &snap.framed_byte_egress;
    row.push_str(&format!(
        "\tfbe:ph{}|sid{}|ws{}|nf{}|nr{}|lf{}|rw{}|p{:.3}|pc{:.3}|dev{:.3}",
        fbe.phase,
        fbe.active_stream_id,
        fbe.current_write_seq,
        fbe.n_frames,
        fbe.n_returns,
        fbe.last_frame_seq,
        fbe.last_return_window_id,
        fbe.output_pressure,
        fbe.projection_confidence,
        fbe.correction_deviation,
    ));

    // v36: vocal substrate — observer renders snapshot values only.
    let vt = &snap.vocal_target;
    let vo = &snap.vocal_organ;
    let vm = &snap.vocal_memory;
    let (loop_pressure, last_gap, conv_count, fail_count) = snap.vocal_gap_trace();
    row.push_str(&format!(
        "\tvoc:tE{:.3}|tp{:.3}|tr{:.3}|te{:.3}|ob{:.3}|op{:.3}|or{:.3}|oph{}|ip{:.3}|ig{:.3}|ic{}|if{}|mn{}",
        vt.active_energy,
        vt.pitch_hint,
        vt.resonance_hint,
        vt.envelope_hint,
        vo.breath_drive,
        vo.pitch_tension,
        vo.resonance_position,
        vo.phase.as_u8(),
        loop_pressure,
        last_gap,
        conv_count,
        fail_count,
        vm.n_prototypes,
    ));

    // v30: probe-law — numeric values + phase code only; no semantic labels.
    let prb = &snap.probe_law;
    row.push_str(&format!(
        "\tprb:i{:.4}|s{:.4}|ph{}|o{:.4}",
        prb.intensity, prb.suppression, prb.phase, prb.output_intensity,
    ));

    let et = &snap.evidence_tension;
    row.push_str(&format!(
        "\tetn:t{:.4}|p{:.4}|h{}|pc{}",
        et.current_tension, et.unresolved_pressure, et.history_len, et.last_pair_cursor,
    ));

    // v57: thread-fault axis (B1) — current flag + cumulative count, so a
    // half-dead body (a crashed thread) is visible at a glance.
    row.push_str(&format!(
        "\ttf:{}|{}",
        if snap.thread_fault { 1 } else { 0 },
        snap.thread_fault_count,
    ));

    // v58: dual-drive confluence — observer renders snapshot truth only.
    // CF (conflict) and NB (net_toward_bond) are the core soul readouts,
    // hence the uppercase markers.
    let dd = &snap.dual_drive;
    row.push_str(&format!(
        "\tdd:sp{:.3}|at{:.3}|CF{:.3}|NB{:+.3}|pf{:.3}|sac{}",
        dd.self_preservation,
        dd.attachment,
        dd.conflict,
        dd.net_toward_bond,
        dd.principle_floor,
        if dd.sacrificing { 1 } else { 0 },
    ));

    // v60: developmental maturity clock m∈[0,1].
    row.push_str(&format!("\tmat:{:.4}", snap.developmental_maturity));

    // v59: axis-level cross-layer bank — summary, never a full dump.
    row.push_str(&format!("\taxl:{}", axis_xlayer_summary(&snap.axis_xlayers)));

    println!("{}", row);
}

/// Explained fraction of the target error's variance: 1 - mse/var, clamped to
/// [0,1]. Derived for display only — the observer never feeds this back.
fn axis_xlayer_explained(s: &AxisCrossLayerState) -> Option<f64> {
    if s.n_updates == 0 || s.target_err_var <= 1e-12 {
        return None;
    }
    Some((1.0 - s.err_pred_mse / s.target_err_var).clamp(0.0, 1.0))
}

/// v59 summary line: bank size, active predictors (n_updates>0), total
/// updates, how many predictors explain >5% of their target err variance
/// (avg/max explained over those with a defined ratio), and the top-3
/// predictors by explained fraction with their strongest learned weights.
fn axis_xlayer_summary(states: &[AxisCrossLayerState]) -> String {
    if states.is_empty() {
        return "-".to_string();
    }
    let n = states.len();
    let active = states.iter().filter(|s| s.n_updates > 0).count();
    let upd_total: u64 = states.iter().map(|s| s.n_updates as u64).sum();

    let mut explained: Vec<(usize, f64)> = states
        .iter()
        .enumerate()
        .filter_map(|(i, s)| axis_xlayer_explained(s).map(|e| (i, e)))
        .collect();
    let n_expl = explained.iter().filter(|(_, e)| *e > 0.05).count();
    let avg_expl = if explained.is_empty() {
        0.0
    } else {
        explained.iter().map(|(_, e)| e).sum::<f64>() / explained.len() as f64
    };
    let max_expl = explained
        .iter()
        .map(|(_, e)| *e)
        .fold(0.0f64, f64::max);

    explained.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_str: String = explained
        .iter()
        .take(3)
        .map(|&(i, e)| {
            let s = &states[i];
            // strongest learned weights (by |w|), up to 3 sources
            let mut ws: Vec<(usize, f64)> = s
                .source_axes
                .iter()
                .copied()
                .zip(s.source_weights.iter().copied())
                .collect();
            ws.sort_by(|a, b| {
                b.1.abs()
                    .partial_cmp(&a.1.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let w_str: String = ws
                .iter()
                .take(3)
                .map(|(src, w)| format!("{}:{:+.3}", src, w))
                .collect::<Vec<_>>()
                .join(",");
            format!("ax{}:e{:.2}|n{}|w[{}]", s.target_axis, e, s.n_updates, w_str)
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "n{}|act{}|upd{}|expl(n{}|avg{:.2}|max{:.2})|top[{}]",
        n,
        active,
        upd_total,
        n_expl,
        avg_expl,
        max_expl,
        if top_str.is_empty() { "-" } else { &top_str },
    )
}
