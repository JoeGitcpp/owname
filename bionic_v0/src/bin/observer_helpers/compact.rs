use bionic_v0::snapshot::SnapshotData;

pub fn compact_header() -> &'static str {
    "seq,clock_s,health,phot_h,src_w,code_frames,code_windows,egress_phase,egress_frames,egress_returns,egress_total_frames,egress_total_returns,egress_pressure,egress_deviation,pair_similarity,pairs,pair_total,pair_framed_byte_total,pair_other_total,probe_i,probe_o,evidence_p,gap_p,gap_c,gap_r,gap_cycles,gap_samples,gap_slot,gap_edge,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or,learn_fam,learn_sup"
}

pub fn compact_row(snap: &SnapshotData) -> String {
    let clock_s = (snap.clock_us.saturating_sub(snap.start_us)) as f64 * 1e-6;
    let latest_pair_similarity = snap
        .action_consequence
        .pairs
        .last()
        .map(|pair| pair.curve_similarity)
        .unwrap_or(0.0);
    format!(
        "{},{:.3},{},{},{},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.4},{:.4}",
        snap.seq,
        clock_s,
        snap.health_state,
        snap.phot_health,
        snap.source_writes,
        snap.code_ingress.recent_frames.len(),
        snap.code_ingress.sequence_windows.len(),
        snap.framed_byte_egress.phase,
        snap.framed_byte_egress.n_frames,
        snap.framed_byte_egress.n_returns,
        snap.framed_byte_egress.total_frames,
        snap.framed_byte_egress.total_returns,
        snap.framed_byte_egress.output_pressure,
        snap.framed_byte_egress.correction_deviation,
        latest_pair_similarity,
        snap.action_consequence.n_pairs,
        snap.action_consequence.total_pairs,
        snap.action_consequence.total_framed_byte_pairs,
        snap.action_consequence.total_other_pairs,
        snap.probe_law.intensity,
        snap.probe_law.output_intensity,
        snap.evidence_tension.unresolved_pressure,
        snap.active_reasoning_gap.gap_pressure,
        snap.active_reasoning_gap.closure_score,
        snap.active_reasoning_gap.reentry_score,
        snap.active_reasoning_gap.active_cycles,
        snap.active_reasoning_gap.total_samples,
        snap.active_reasoning_gap.focus_slot_id,
        snap.active_reasoning_gap.focus_edge_id,
        snap.vocal_target.active_energy,
        snap.vocal_target.pitch_hint,
        snap.vocal_target.resonance_hint,
        snap.vocal_organ.breath_drive,
        snap.vocal_organ.pitch_tension,
        snap.vocal_organ.resonance_position,
        snap.learned_bias.prediction_familiarity,
        snap.learned_bias.action_suppression,
    )
}
