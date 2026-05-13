use crate::snapshot::SnapshotData;
use crate::vocal_organ::VocalOrganPhase;

/// Observer-side live I/O evidence summary.
///
/// This is a research rendering helper over snapshot facts. It is not body
/// state and has no path back into `Viewport`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveIoEvidenceSummary {
    pub body_write_out_exists: bool,
    pub write_epoch_exists: bool,
    pub consequence_return_exists: bool,
    pub temporal_pairing_exists: bool,
    pub latest_window_id: u64,
    pub latest_pair_seq: u64,
    pub latest_write_seq: u64,
    pub latest_pair_delay_us: u64,
    pub latest_curve_similarity: f64,
}

pub fn summarize_live_io_evidence(snap: &SnapshotData) -> LiveIoEvidenceSummary {
    let vocal_active =
        snap.vocal_organ.phase == VocalOrganPhase::Active && snap.vocal_organ.breath_drive > 0.0;
    let action_active = snap
        .action_output
        .channels
        .iter()
        .any(|ch| ch.phase != 0 && ch.peak_magnitude > 0.0);
    let framed_byte_active =
        snap.framed_byte_egress.phase != 0 && snap.framed_byte_egress.n_frames > 0;

    let latest_window = snap
        .action_consequence
        .windows
        .iter()
        .max_by_key(|w| w.window_id);
    let latest_pair = snap
        .action_consequence
        .pairs
        .iter()
        .max_by_key(|p| p.pair_seq);

    LiveIoEvidenceSummary {
        body_write_out_exists: vocal_active || action_active || framed_byte_active,
        write_epoch_exists: snap.action_output.n_write_epochs > 0,
        consequence_return_exists: snap.action_consequence.n_windows > 0,
        temporal_pairing_exists: snap.action_consequence.n_pairs > 0,
        latest_window_id: latest_window.map(|w| w.window_id).unwrap_or(0),
        latest_pair_seq: latest_pair.map(|p| p.pair_seq).unwrap_or(0),
        latest_write_seq: latest_pair.map(|p| p.write_seq).unwrap_or(0),
        latest_pair_delay_us: latest_pair.map(|p| p.delay_us).unwrap_or(0),
        latest_curve_similarity: latest_pair.map(|p| p.curve_similarity).unwrap_or(0.0),
    }
}
