use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::Path;

#[derive(Clone, Debug, Default)]
struct Row {
    clock_s: f64,
    egress_total_frames: f64,
    egress_total_returns: f64,
    pair_framed_byte_total: f64,
    pair_other_total: f64,
    evidence_p: f64,
    learned_event_total: f64,
}

pub fn summarize_path(path: &Path) -> std::io::Result<Vec<String>> {
    summarize_paths(&[path.to_path_buf()])
}

pub fn summarize_paths<P: AsRef<Path>>(paths: &[P]) -> std::io::Result<Vec<String>> {
    let mut rows = Vec::new();
    for path in paths {
        rows.extend(read_rows(path.as_ref())?);
    }
    rows.sort_by(|a, b| {
        a.clock_s
            .partial_cmp(&b.clock_s)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    summarize_rows(&rows)
}

fn read_rows(path: &Path) -> std::io::Result<Vec<Row>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let columns = header
        .split(',')
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect::<HashMap<_, _>>();

    lines
        .map(|line| parse_row(line, &columns))
        .collect::<std::io::Result<Vec<_>>>()
}

fn summarize_rows(rows: &[Row]) -> std::io::Result<Vec<String>> {
    let first = rows
        .first()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let last = rows
        .last()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let duration_s = (last.clock_s - first.clock_s).max(0.0);
    let minutes = duration_s / 60.0;
    let evidence_min = rows
        .iter()
        .map(|row| row.evidence_p)
        .fold(f64::INFINITY, f64::min);
    let evidence_max = rows
        .iter()
        .map(|row| row.evidence_p)
        .fold(f64::NEG_INFINITY, f64::max);
    let pair_other_delta = delta(last.pair_other_total, first.pair_other_total);

    Ok(vec![
        format!("rows={}", rows.len()),
        format!("duration_s={duration_s:.3}"),
        format!(
            "egress_total_frames_delta={}",
            delta(last.egress_total_frames, first.egress_total_frames) as u64
        ),
        format!(
            "egress_total_frames_per_min={:.3}",
            rate(last.egress_total_frames, first.egress_total_frames, minutes)
        ),
        format!(
            "egress_total_returns_delta={}",
            delta(last.egress_total_returns, first.egress_total_returns) as u64
        ),
        format!(
            "egress_total_returns_per_min={:.3}",
            rate(
                last.egress_total_returns,
                first.egress_total_returns,
                minutes
            )
        ),
        format!(
            "pair_framed_byte_total_delta={}",
            delta(last.pair_framed_byte_total, first.pair_framed_byte_total) as u64
        ),
        format!(
            "pair_framed_byte_total_per_min={:.3}",
            rate(
                last.pair_framed_byte_total,
                first.pair_framed_byte_total,
                minutes
            )
        ),
        format!("pair_other_total_delta={}", pair_other_delta as u64),
        format!(
            "learned_event_total_delta={}",
            delta(last.learned_event_total, first.learned_event_total) as u64
        ),
        format!(
            "learned_event_total_per_min={:.3}",
            rate(last.learned_event_total, first.learned_event_total, minutes)
        ),
        format!("evidence_p_min={evidence_min:.4}"),
        format!("evidence_p_max={evidence_max:.4}"),
        format!(
            "closure_status={}",
            if pair_other_delta > 0.0 {
                "contaminated"
            } else {
                "clean"
            }
        ),
    ])
}

fn parse_row(line: &str, columns: &HashMap<&str, usize>) -> std::io::Result<Row> {
    let fields = line.split(',').collect::<Vec<_>>();
    Ok(Row {
        clock_s: read_f64(&fields, columns, "clock_s")?,
        egress_total_frames: read_f64(&fields, columns, "egress_total_frames")?,
        egress_total_returns: read_f64(&fields, columns, "egress_total_returns")?,
        pair_framed_byte_total: read_f64(&fields, columns, "pair_framed_byte_total")?,
        pair_other_total: read_f64(&fields, columns, "pair_other_total")?,
        evidence_p: read_f64(&fields, columns, "evidence_p")?,
        learned_event_total: read_f64(&fields, columns, "learned_event_total")?,
    })
}

fn read_f64(fields: &[&str], columns: &HashMap<&str, usize>, name: &str) -> std::io::Result<f64> {
    let idx = *columns
        .get(name)
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    fields
        .get(idx)
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?
        .parse::<f64>()
        .map_err(|_| Error::from(ErrorKind::InvalidData))
}

fn delta(last: f64, first: f64) -> f64 {
    (last - first).max(0.0)
}

fn rate(last: f64, first: f64, minutes: f64) -> f64 {
    if minutes <= 0.0 {
        0.0
    } else {
        delta(last, first) / minutes
    }
}
