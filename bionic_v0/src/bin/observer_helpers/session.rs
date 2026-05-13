use std::fs::OpenOptions;
use std::io::{Result, Write};
use std::path::Path;

use bionic_v0::snapshot::SnapshotData;

pub fn session_header() -> &'static str {
    "seq,clock_s,source_writes,code_frames,code_windows,egress_phase,egress_frames,egress_returns,egress_total_frames,egress_total_returns,egress_pressure,egress_deviation,pair_similarity,pairs,pair_total,pair_framed_byte_total,pair_other_total,probe_i,probe_o,evidence_p,learned_axes,learned_event_total,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or"
}

pub fn session_row(snap: &SnapshotData) -> String {
    let clock_s = (snap.clock_us.saturating_sub(snap.start_us)) as f64 * 1e-6;
    let latest_pair_similarity = latest_framed_byte_pair_similarity(snap);
    format!(
        "{},{:.3},{},{},{},{},{},{},{},{},{:.4},{:.4},{:.4},{},{},{},{},{:.4},{:.4},{:.4},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
        snap.seq,
        clock_s,
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
        snap.learned_bias.n_curves,
        snap.learned_bias.total_events,
        snap.vocal_target.active_energy,
        snap.vocal_target.pitch_hint,
        snap.vocal_target.resonance_hint,
        snap.vocal_organ.breath_drive,
        snap.vocal_organ.pitch_tension,
        snap.vocal_organ.resonance_position,
    )
}

fn latest_framed_byte_pair_similarity(snap: &SnapshotData) -> f64 {
    if snap.framed_byte_egress.total_returns > 0
        && snap.action_consequence.total_framed_byte_pairs > 0
    {
        (1.0 - snap.framed_byte_egress.correction_deviation).clamp(0.0, 1.0)
    } else {
        snap.action_consequence
            .pairs
            .last()
            .map(|pair| pair.curve_similarity)
            .unwrap_or(0.0)
    }
}

pub fn append_session_row(path: &Path, snap: &SnapshotData, write_header: bool) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if write_header {
        writeln!(file, "{}", session_header())?;
    }
    writeln!(file, "{}", session_row(snap))?;
    Ok(())
}
