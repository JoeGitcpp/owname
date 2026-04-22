//! Real PHOT Ingress (Phase 4A / 4B).
//!
//! This module defines the low-layer camera abstractions used by
//! `RealPhotAdapter`. Nothing here carries semantic labels, object classes,
//! tracking identities, or text. The only outputs are:
//!
//!   * a normalized grayscale grid (`CameraFrame`), and
//!   * numeric event evidence derived from it (`MotionExtract`).
//!
//! The trait `CameraFrameProvider` is the single seam between the adapter and
//! the outside world. Three implementations live here:
//!
//!   * `SyntheticCameraFrameProvider` — scriptable, deterministic; used by
//!     tests. Never wired into the product body path.
//!   * `StubRealCameraFrameProvider` — the always-faulted placeholder. Used
//!     by body.rs as the fallback when `RealCameraFrameProvider::try_new()`
//!     fails, so `phot_health = FAULT` remains observable. No silent gap.
//!   * `RealCameraFrameProvider` — Phase 4B: binds to the default macOS
//!     monocular camera via `nokhwa` + AVFoundation. `try_new()` returns
//!     `Err(CameraFault::StartupFailed)` on failure; the substrate treats
//!     startup failure as a first-class observable state, not a silent gap.
//!
//! Extraction is multi-region: per frame, the extractor segments the motion map
//! into up to `PHOT_MAX_EVENTS_PER_FRAME` independent regions (Phase 4D).

use crate::{PHOT_BANDS, PHOT_MAX_EVENTS_PER_FRAME, PHOT_REGION_MIN_PIXELS, PHOT_REGION_THRESHOLD};

/// Fixed resolution of the internal luma grid. We downsample whatever the
/// source delivers to this, so the rest of the pipeline sees one grid shape.
pub const CAM_W: usize = 64;
pub const CAM_H: usize = 48;
pub const CAM_PIX: usize = CAM_W * CAM_H;

/// A single grayscale camera frame. Values in `[0.0, 1.0]`.
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub t_us: u64,
    pub luma: Vec<f64>, // length CAM_PIX, row-major.
}

impl CameraFrame {
    pub fn zeros(t_us: u64) -> Self {
        Self {
            t_us,
            luma: vec![0.0; CAM_PIX],
        }
    }
}

/// Camera-side fault classes. Mirrors the observer-facing health states but
/// is strictly about the ingress plumbing, not about slot health.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraFault {
    /// Device could not be opened at construction time.
    StartupFailed,
    /// Device opened but a subsequent read failed.
    RuntimeError,
}

/// Trait implemented by anything that can deliver camera frames. All methods
/// must be non-blocking so the adapter's tick stays responsive.
pub trait CameraFrameProvider: Send {
    /// Non-blocking frame read. Semantics:
    ///   * `Ok(Some(frame))`  — fresh frame available.
    ///   * `Ok(None)`         — provider is healthy but no frame this tick.
    ///   * `Err(fault)`       — provider is faulted (startup or runtime).
    ///
    /// After returning `Err`, `is_healthy()` must report false until the
    /// adapter stops polling. The substrate does not try to recover a faulted
    /// provider in Phase 4A.
    fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault>;

    /// Stable health predicate. Used by the adapter to decide whether to
    /// emit a FAULT health regardless of frame presence.
    fn is_healthy(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Synthetic provider — deterministic, scriptable.
// ---------------------------------------------------------------------------

/// A single blob specification for multi-blob synthetic frames.
#[derive(Clone, Debug)]
pub struct BlobSpec {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub brightness: f64,
}

/// Per-tick script step for `SyntheticCameraFrameProvider`.
#[derive(Clone, Debug)]
pub enum SynthCamStep {
    /// No frame this tick.
    NoFrame,
    /// Flat, near-black frame (healthy but quiet).
    Idle,
    /// Localized bright blob at normalized (cx, cy) with radius r (in
    /// normalized coords). Everything else is near-black.
    Blob {
        cx: f64,
        cy: f64,
        r: f64,
        brightness: f64,
    },
    /// Multiple blobs in one frame. Each blob is rendered independently;
    /// overlapping blobs take the maximum brightness.
    MultiBlob(Vec<BlobSpec>),
    /// Signal a transient runtime error on this tick.
    RuntimeError,
}

/// Scriptable, deterministic provider. Walks through `script`, returning the
/// matching step's frame (or fault) on each `try_recv`. After the script ends
/// it keeps returning `Ok(None)` (healthy but idle).
pub struct SyntheticCameraFrameProvider {
    script: Vec<SynthCamStep>,
    idx: usize,
    healthy: bool,
    clock_us_per_tick: u64,
    t_us: u64,
}

impl SyntheticCameraFrameProvider {
    pub fn new(script: Vec<SynthCamStep>, start_t_us: u64, us_per_tick: u64) -> Self {
        Self {
            script,
            idx: 0,
            healthy: true,
            clock_us_per_tick: us_per_tick.max(1),
            t_us: start_t_us,
        }
    }

    /// Helper for tests: N `Idle` frames then M `Blob` frames then idle.
    pub fn idle_then_blob(
        n_idle: usize,
        n_blob: usize,
        cx: f64,
        cy: f64,
        r: f64,
        brightness: f64,
        start_t_us: u64,
        us_per_tick: u64,
    ) -> Self {
        let mut script = Vec::with_capacity(n_idle + n_blob);
        for _ in 0..n_idle {
            script.push(SynthCamStep::Idle);
        }
        for _ in 0..n_blob {
            script.push(SynthCamStep::Blob {
                cx,
                cy,
                r,
                brightness,
            });
        }
        Self::new(script, start_t_us, us_per_tick)
    }

    fn render_idle(t_us: u64) -> CameraFrame {
        let mut f = CameraFrame::zeros(t_us);
        // Low, uniform baseline so the extractor isn't dividing by zero.
        for v in f.luma.iter_mut() {
            *v = 0.02;
        }
        f
    }

    fn render_blob(t_us: u64, cx: f64, cy: f64, r: f64, brightness: f64) -> CameraFrame {
        let mut f = CameraFrame::zeros(t_us);
        let w = CAM_W as f64;
        let h = CAM_H as f64;
        let cpx = cx.clamp(0.0, 1.0) * (w - 1.0);
        let cpy = cy.clamp(0.0, 1.0) * (h - 1.0);
        let rpx = (r.max(1e-6)) * w.min(h);
        let rpx2 = rpx * rpx;
        let b = brightness.clamp(0.0, 1.0);
        for y in 0..CAM_H {
            for x in 0..CAM_W {
                let dx = x as f64 - cpx;
                let dy = y as f64 - cpy;
                let d2 = dx * dx + dy * dy;
                let val = if d2 <= rpx2 {
                    let t = (1.0 - d2 / rpx2).max(0.0);
                    0.02 + b * t
                } else {
                    0.02
                };
                f.luma[y * CAM_W + x] = val.clamp(0.0, 1.0);
            }
        }
        f
    }

    fn render_multi_blob(t_us: u64, blobs: &[BlobSpec]) -> CameraFrame {
        let mut f = CameraFrame::zeros(t_us);
        // Start with baseline.
        for v in f.luma.iter_mut() {
            *v = 0.02;
        }
        let w = CAM_W as f64;
        let h = CAM_H as f64;
        for blob in blobs {
            let cpx = blob.cx.clamp(0.0, 1.0) * (w - 1.0);
            let cpy = blob.cy.clamp(0.0, 1.0) * (h - 1.0);
            let rpx = blob.r.max(1e-6) * w.min(h);
            let rpx2 = rpx * rpx;
            let b = blob.brightness.clamp(0.0, 1.0);
            for y in 0..CAM_H {
                for x in 0..CAM_W {
                    let dx = x as f64 - cpx;
                    let dy = y as f64 - cpy;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= rpx2 {
                        let t = (1.0 - d2 / rpx2).max(0.0);
                        let v = 0.02 + b * t;
                        // Take the maximum so overlapping blobs don't cancel.
                        let cur = f.luma[y * CAM_W + x];
                        f.luma[y * CAM_W + x] = cur.max(v).clamp(0.0, 1.0);
                    }
                }
            }
        }
        f
    }
}

impl CameraFrameProvider for SyntheticCameraFrameProvider {
    fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
        if !self.healthy {
            return Err(CameraFault::RuntimeError);
        }
        let step = if self.idx < self.script.len() {
            self.script[self.idx].clone()
        } else {
            SynthCamStep::Idle
        };
        self.idx = self.idx.saturating_add(1);
        self.t_us = self.t_us.saturating_add(self.clock_us_per_tick);
        match step {
            SynthCamStep::NoFrame => Ok(None),
            SynthCamStep::Idle => Ok(Some(Self::render_idle(self.t_us))),
            SynthCamStep::Blob {
                cx,
                cy,
                r,
                brightness,
            } => Ok(Some(Self::render_blob(self.t_us, cx, cy, r, brightness))),
            SynthCamStep::MultiBlob(blobs) => Ok(Some(Self::render_multi_blob(self.t_us, &blobs))),
            SynthCamStep::RuntimeError => {
                self.healthy = false;
                Err(CameraFault::RuntimeError)
            }
        }
    }
    fn is_healthy(&self) -> bool {
        self.healthy
    }
}

// ---------------------------------------------------------------------------
// Stub real provider — always-startup-failed.
// ---------------------------------------------------------------------------

/// Placeholder for the real-hardware path. `try_new` attempts to bind to the
/// platform camera; in Phase 4A that binding is intentionally not wired, so
/// `try_new` always returns `Err(StartupFailed)`.
///
/// Even in the failure branch, we return a *constructed* provider that will
/// keep reporting `CameraFault::StartupFailed` forever. The adapter uses that
/// signal to set `phot_health = FAULT`, making the absence of a real camera
/// an observable substrate state rather than a silent gap.
pub struct StubRealCameraFrameProvider {
    healthy: bool,
}

impl StubRealCameraFrameProvider {
    /// Attempts to bind to the default monocular camera. Currently always
    /// returns the faulted provider.
    pub fn try_new() -> Self {
        // TODO(phase 4B): replace with nokhwa / AVFoundation init. The
        // contract above already tolerates both success and failure paths.
        Self { healthy: false }
    }
}

impl CameraFrameProvider for StubRealCameraFrameProvider {
    fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
        Err(CameraFault::StartupFailed)
    }
    fn is_healthy(&self) -> bool {
        self.healthy
    }
}

// ---------------------------------------------------------------------------
// Real hardware provider (Phase 4B) — macOS / nokhwa / AVFoundation.
// ---------------------------------------------------------------------------

/// Hardware-backed camera provider. Binds to the default monocular camera on
/// macOS via `nokhwa` + AVFoundation. Capture runs in a background thread
/// (AVFoundation calls can block); frames arrive through a bounded channel so
/// the adapter's `tick()` never blocks.
///
/// `try_new()` blocks for up to 10 s waiting for the device to open and the
/// first stream callback to confirm health. It returns `Err(StartupFailed)` on
/// any failure — it never silently falls back to synthetic data.
pub struct RealCameraFrameProvider {
    rx: std::sync::mpsc::Receiver<Result<CameraFrame, CameraFault>>,
    healthy: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Kept alive so the capture thread is not orphaned on drop.
    _thread: std::thread::JoinHandle<()>,
}

impl RealCameraFrameProvider {
    /// Attempt to bind to the default monocular camera. Returns `Err` if the
    /// device cannot be opened or the permission is denied; returns `Ok` once
    /// the capture stream is healthy.
    ///
    /// Does NOT fall back to synthetic data on failure.
    pub fn try_new() -> Result<Self, CameraFault> {
        use nokhwa::pixel_format::RgbFormat;
        use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
        use nokhwa::Camera;
        use std::sync::atomic::Ordering;
        use std::sync::mpsc;
        use std::sync::{atomic::AtomicBool, Arc};
        use std::time::Duration;

        let healthy = Arc::new(AtomicBool::new(false));
        let healthy_bg = healthy.clone();

        // Startup result channel (one-shot).
        let (startup_tx, startup_rx) = mpsc::sync_channel::<Result<(), CameraFault>>(1);
        // Frame channel — bounded: oldest frame is dropped when the adapter
        // is slower than the camera (prevents backpressure).
        let (frame_tx, frame_rx) = mpsc::sync_channel::<Result<CameraFrame, CameraFault>>(2);

        let thread = std::thread::spawn(move || {
            let index = CameraIndex::Index(0);
            let requested =
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
            let mut camera = match Camera::new(index, requested) {
                Ok(c) => c,
                Err(_) => {
                    let _ = startup_tx.send(Err(CameraFault::StartupFailed));
                    return;
                }
            };
            if camera.open_stream().is_err() {
                let _ = startup_tx.send(Err(CameraFault::StartupFailed));
                return;
            }
            healthy_bg.store(true, Ordering::SeqCst);
            let _ = startup_tx.send(Ok(()));

            loop {
                if !healthy_bg.load(Ordering::Relaxed) {
                    break;
                }
                let t = crate::clock::now_us();
                match camera.frame() {
                    Ok(buffer) => {
                        if let Some(luma) = decode_rgb_to_luma(&buffer) {
                            // try_send: drops the frame if the channel is full
                            // rather than blocking. The adapter will see Ok(None)
                            // and eventually detect staleness if this persists.
                            let _ = frame_tx.try_send(Ok(CameraFrame { t_us: t, luma }));
                        }
                    }
                    Err(_) => {
                        healthy_bg.store(false, Ordering::SeqCst);
                        let _ = frame_tx.try_send(Err(CameraFault::RuntimeError));
                        break;
                    }
                }
            }
        });

        // Wait up to 10 s for the capture thread to confirm the stream opened.
        match startup_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(Self {
                rx: frame_rx,
                healthy,
                _thread: thread,
            }),
            _ => Err(CameraFault::StartupFailed),
        }
    }
}

impl Drop for RealCameraFrameProvider {
    fn drop(&mut self) {
        // Signal the capture thread to exit gracefully.
        self.healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl CameraFrameProvider for RealCameraFrameProvider {
    fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
        use std::sync::mpsc::TryRecvError;
        match self.rx.try_recv() {
            Ok(Ok(frame)) => Ok(Some(frame)),
            Ok(Err(fault)) => {
                self.healthy
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                Err(fault)
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.healthy
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                Err(CameraFault::RuntimeError)
            }
        }
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Decode a nokhwa `Buffer` (RGB format) into a 64×48 normalised luma grid.
/// Returns `None` only if the buffer cannot be decoded or has zero dimensions.
fn decode_rgb_to_luma(buffer: &nokhwa::Buffer) -> Option<Vec<f64>> {
    use nokhwa::pixel_format::RgbFormat;

    let res = buffer.resolution();
    let src_w = res.width() as usize;
    let src_h = res.height() as usize;
    if src_w == 0 || src_h == 0 {
        return None;
    }
    let img = buffer.decode_image::<RgbFormat>().ok()?;
    let raw = img.into_raw(); // Vec<u8>, RGB interleaved
    if raw.len() != src_w * src_h * 3 {
        return None;
    }
    let mut luma = vec![0.0f64; CAM_PIX];
    for oy in 0..CAM_H {
        for ox in 0..CAM_W {
            let sx = (ox * src_w / CAM_W).min(src_w - 1);
            let sy = (oy * src_h / CAM_H).min(src_h - 1);
            let base = (sy * src_w + sx) * 3;
            let r = raw[base] as f64 / 255.0;
            let g = raw[base + 1] as f64 / 255.0;
            let b = raw[base + 2] as f64 / 255.0;
            luma[oy * CAM_W + ox] = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0);
        }
    }
    Some(luma)
}

// ---------------------------------------------------------------------------
// Motion-based multi-region extractor (Phase 4D).
// ---------------------------------------------------------------------------

/// Numeric-only event evidence extracted from one frame diff. Matches the
/// Real PHOT Ingress Contract exactly; no semantic fields.
#[derive(Clone, Copy, Debug)]
pub struct MotionExtract {
    pub phot_bands: [f64; PHOT_BANDS],
    pub pos_x: f64,
    pub pos_y: f64,
    pub span_x: f64,
    pub span_y: f64,
    pub spatial_precision: f64,
    pub event_energy: f64,
}

/// Stateful extractor: keeps the previous frame's luma so we can compute a
/// frame-difference motion map. Pure numerics.
pub struct MotionExtractor {
    prev: Option<Vec<f64>>,
}

impl MotionExtractor {
    pub fn new() -> Self {
        Self { prev: None }
    }

    /// Reset internal state — used when the provider faults so we don't diff
    /// against a stale frame on recovery.
    pub fn reset(&mut self) {
        self.prev = None;
    }

    /// Returns true once the extractor has seen at least one frame (prev is
    /// set). An empty Vec from `extract()` when `has_prev()` is true means the
    /// frame had no above-threshold motion regions; when `has_prev()` is false
    /// the empty Vec is the warm-up return for the very first frame.
    pub fn has_prev(&self) -> bool {
        self.prev.is_some()
    }

    /// Multi-region extraction. Returns an empty Vec for the very first frame
    /// (nothing to diff against). Otherwise returns 0..N regions (N capped by
    /// `PHOT_MAX_EVENTS_PER_FRAME`), sorted energy-descending.
    pub fn extract(&mut self, frame: &CameraFrame) -> Vec<MotionExtract> {
        assert_eq!(frame.luma.len(), CAM_PIX);
        let prev = match self.prev.as_ref() {
            Some(p) => p.clone(),
            None => {
                self.prev = Some(frame.luma.clone());
                return vec![];
            }
        };
        // motion[i] = |curr[i] - prev[i]|
        let mut motion = vec![0.0_f64; CAM_PIX];
        for i in 0..CAM_PIX {
            motion[i] = (frame.luma[i] - prev[i]).abs();
        }
        self.prev = Some(frame.luma.clone());

        extract_regions_from_motion(&motion, CAM_W, CAM_H)
    }
}

/// Extract connected motion regions from a pre-computed motion map.
///
/// `motion` is a row-major array of per-pixel motion magnitudes (length w*h).
/// Returns up to `PHOT_MAX_EVENTS_PER_FRAME` `MotionExtract` records, sorted
/// by `event_energy` descending (then `pos_x` ascending, then `pos_y` ascending).
pub fn extract_regions_from_motion(motion: &[f64], w: usize, h: usize) -> Vec<MotionExtract> {
    let n = w * h;
    assert_eq!(motion.len(), n);

    // --- 1. Threshold mask ---
    let above: Vec<bool> = motion.iter().map(|&v| v >= PHOT_REGION_THRESHOLD).collect();

    // --- 2. Connected components (4-connected BFS) ---
    let mut visited = vec![false; n];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for start in 0..n {
        if !above[start] || visited[start] {
            continue;
        }
        // BFS flood fill
        let mut region = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        while let Some(idx) = queue.pop_front() {
            region.push(idx);
            let px = idx % w;
            let py = idx / w;
            // 4-neighbours
            if px > 0 {
                let nb = idx - 1;
                if above[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if px + 1 < w {
                let nb = idx + 1;
                if above[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if py > 0 {
                let nb = idx - w;
                if above[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if py + 1 < h {
                let nb = idx + w;
                if above[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
        }
        if region.len() >= PHOT_REGION_MIN_PIXELS {
            components.push(region);
        }
    }

    if components.is_empty() {
        return vec![];
    }

    // --- 3. Compute MotionExtract per region ---
    let mut results: Vec<MotionExtract> = components
        .iter()
        .map(|pixels| region_to_extract(motion, w, h, pixels))
        .collect();

    // --- 4. Sort: energy descending, then pos_x ascending, then pos_y ascending ---
    results.sort_by(|a, b| {
        b.event_energy
            .partial_cmp(&a.event_energy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.pos_x
                    .partial_cmp(&b.pos_x)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(
                a.pos_y
                    .partial_cmp(&b.pos_y)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    // --- 5. Cap ---
    results.truncate(PHOT_MAX_EVENTS_PER_FRAME);
    results
}

/// Compute a `MotionExtract` for a single set of region pixel indices.
fn region_to_extract(motion: &[f64], w: usize, h: usize, pixels: &[usize]) -> MotionExtract {
    let n_px = pixels.len();

    // Sums over region
    let mut sum_m = 0.0_f64;
    let mut sum_m2 = 0.0_f64;
    let mut peak = 0.0_f64;
    for &i in pixels {
        let v = motion[i];
        sum_m += v;
        sum_m2 += v * v;
        if v > peak {
            peak = v;
        }
    }

    let event_energy = sum_m2;

    // Weighted centroid
    let (pos_x, pos_y, span_x, span_y);
    if sum_m > 1e-9 {
        let mut mx = 0.0_f64;
        let mut my = 0.0_f64;
        for &i in pixels {
            let m = motion[i];
            let px = (i % w) as f64;
            let py = (i / w) as f64;
            mx += px * m;
            my += py * m;
        }
        let cx_px = mx / sum_m;
        let cy_px = my / sum_m;
        // Second moments
        let mut vx = 0.0_f64;
        let mut vy = 0.0_f64;
        for &i in pixels {
            let m = motion[i];
            let dx = (i % w) as f64 - cx_px;
            let dy = (i / w) as f64 - cy_px;
            vx += dx * dx * m;
            vy += dy * dy * m;
        }
        vx = (vx / sum_m).max(0.0).sqrt();
        vy = (vy / sum_m).max(0.0).sqrt();
        pos_x = (cx_px / (w as f64 - 1.0)).clamp(0.0, 1.0);
        pos_y = (cy_px / (h as f64 - 1.0)).clamp(0.0, 1.0);
        span_x = (vx / w as f64).clamp(0.0, 1.0);
        span_y = (vy / h as f64).clamp(0.0, 1.0);
    } else {
        pos_x = 0.5;
        pos_y = 0.5;
        span_x = 0.0;
        span_y = 0.0;
    }

    // Spatial precision: peak / mean within region
    let mean_m = sum_m / n_px as f64;
    let spatial_precision = if mean_m > 1e-12 {
        (peak / mean_m).min(1e6)
    } else {
        0.0
    };

    // PHOT bands: build a region-only motion map (zeros outside region),
    // then call box_blur_rms on it for bands 0-2. Band 3 = sqrt(sum_m2 / n_px).
    let mut region_map = vec![0.0_f64; motion.len()];
    for &i in pixels {
        region_map[i] = motion[i];
    }
    let b0 = box_blur_rms(&region_map, w, h, 0);
    let b1 = box_blur_rms(&region_map, w, h, 1);
    let b2 = box_blur_rms(&region_map, w, h, 2);
    let b3 = (sum_m2 / n_px as f64).sqrt();

    let mut bands = [0.0_f64; PHOT_BANDS];
    bands[0] = b0;
    if PHOT_BANDS > 1 {
        bands[1] = b1;
    }
    if PHOT_BANDS > 2 {
        bands[2] = b2;
    }
    if PHOT_BANDS > 3 {
        bands[3] = b3;
    }

    MotionExtract {
        phot_bands: bands,
        pos_x,
        pos_y,
        span_x,
        span_y,
        spatial_precision,
        event_energy,
    }
}

/// Mean RMS of a box-blurred view of `grid`. `radius==0` returns the plain
/// RMS, `radius>0` blurs with a (2r+1)²-window mean filter first. Pure
/// spatial low-pass — no FFT dependency, no model.
fn box_blur_rms(grid: &[f64], w: usize, h: usize, radius: usize) -> f64 {
    if radius == 0 {
        let mut s = 0.0_f64;
        for v in grid {
            s += v * v;
        }
        return (s / grid.len() as f64).sqrt();
    }
    let r = radius as isize;
    let mut sum_sq = 0.0_f64;
    let mut n = 0_usize;
    for y in 0..h as isize {
        for x in 0..w as isize {
            let mut acc = 0.0_f64;
            let mut cnt = 0_usize;
            let x0 = (x - r).max(0);
            let x1 = (x + r).min(w as isize - 1);
            let y0 = (y - r).max(0);
            let y1 = (y + r).min(h as isize - 1);
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    acc += grid[(yy as usize) * w + (xx as usize)];
                    cnt += 1;
                }
            }
            let m = if cnt > 0 { acc / cnt as f64 } else { 0.0 };
            sum_sq += m * m;
            n += 1;
        }
    }
    (sum_sq / n.max(1) as f64).sqrt()
}
