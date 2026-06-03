//! Network-backed sensory adapters for headless (server) deployment.
//!
//! In headless mode the body has no microphone, camera, or speakers.
//! Instead, a gateway process writes sensory frames to a Unix socket and
//! reads vocal synthesis frames back.  These adapters implement the same
//! `ModalityAdapter` trait as the hardware versions — the body loop is
//! unchanged.
//!
//! Wire protocol (compact binary, no field names — opaque to sniffers):
//!
//! Gateway → body (sensory input), 20 Hz:
//!   [0x01]                       tag
//!   [audio_level : f32le]        RMS amplitude 0..1
//!   [audio_flux  : f32le]        spectral onset energy 0..1
//!   [bands       : 12×f32le]     coarse frequency bands 0..1
//!   [motion      : f32le]        mean inter-frame luminance change 0..1
//!   [luma        : f32le]        mean brightness 0..1
//!   [grid        : 768×f32le]    32×24 luminance grid 0..1
//!   Total: 1 + 4+4+48 + 4+4 + 3072 = 3137 bytes
//!
//! Body → gateway (vocal output), on demand:
//!   [0x02]                       tag
//!   [n : u16le]                  sample count
//!   [samples : n×f32le]          PCM at VOCAL_SAMPLE_RATE Hz, −1..1

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::adapter::ModalityAdapter;
use crate::viewport::Viewport;
use crate::{MOD_VIB, MOD_PHOT, VIB_BANDS, AXIS_VIS_BASE, VIS_AXES_PER_REGION};

/// Sample rate for vocal PCM frames sent to the gateway.
pub const VOCAL_SAMPLE_RATE: u32 = 22_050;
/// Vocal frame size in samples per push (~23 ms @ 22 kHz).
pub const VOCAL_FRAME_SAMPLES: usize = 512;

// ── Input frame constants ────────────────────────────────────────────────────
const TAG_SENSES: u8 = 0x01;
const GRID_W: usize = 32;
const GRID_H: usize = 24;
const BANDS: usize = 12;
// 1 tag + (1+1+BANDS+1+1)×f32 + GRID_W*GRID_H×f32
const FRAME_BYTES: usize = 1 + (2 + BANDS) * 4 + GRID_W * GRID_H * 4;

// ── Output frame constants ───────────────────────────────────────────────────
const TAG_VOCAL: u8 = 0x02;

// ── Shared latest-senses buffer ──────────────────────────────────────────────
#[derive(Clone)]
struct SensesFrame {
    audio_level: f32,
    audio_flux:  f32,
    bands:       [f32; BANDS],
    motion:      f32,
    luma:        f32,
    grid:        Vec<f32>,   // GRID_W × GRID_H
}

impl Default for SensesFrame {
    fn default() -> Self {
        Self {
            audio_level: 0.0,
            audio_flux:  0.0,
            bands:       [0.0; BANDS],
            motion:      0.0,
            luma:        0.0,
            grid:        vec![0.0; GRID_W * GRID_H],
        }
    }
}

/// Parses one sensory binary frame from the wire buffer.
/// Returns `None` if the buffer is too short or the tag is wrong.
fn parse_frame(buf: &[u8]) -> Option<SensesFrame> {
    if buf.len() < FRAME_BYTES || buf[0] != TAG_SENSES {
        return None;
    }
    let mut off = 1usize;
    let mut rf = |o: &mut usize| -> f32 {
        let v = f32::from_le_bytes(buf[*o..*o+4].try_into().unwrap());
        *o += 4;
        v
    };
    let audio_level = rf(&mut off);
    let audio_flux  = rf(&mut off);
    let mut bands = [0.0f32; BANDS];
    for b in &mut bands { *b = rf(&mut off); }
    let motion = rf(&mut off);
    let luma   = rf(&mut off);
    let mut grid = vec![0.0f32; GRID_W * GRID_H];
    for g in &mut grid { *g = rf(&mut off); }
    Some(SensesFrame { audio_level, audio_flux, bands, motion, luma, grid })
}

// ─────────────────────────────────────────────────────────────────────────────
// NetSensesAdapter: reads from a Unix socket, drives VIB + PHOT axes.
// One instance covers both VIB and PHOT (they share the same socket).
// ─────────────────────────────────────────────────────────────────────────────
pub struct NetSensesAdapter {
    latest: Arc<Mutex<SensesFrame>>,
    /// Send half for vocal output frames (written by the body's vocal ticker).
    vocal_tx: Arc<Mutex<Option<UnixStream>>>,
}

impl NetSensesAdapter {
    /// Bind a Unix socket at `socket_path` and spawn a reader thread.
    /// The gateway is expected to connect shortly after the body starts.
    /// `vocal_tx` is shared with `NetVocalSink` so the vocal ticker can
    /// write PCM frames back to the gateway on the same connection.
    pub fn new(socket_path: PathBuf) -> (Self, Arc<Mutex<Option<UnixStream>>>) {
        let latest: Arc<Mutex<SensesFrame>> = Arc::new(Mutex::new(SensesFrame::default()));
        let vocal_tx: Arc<Mutex<Option<UnixStream>>> = Arc::new(Mutex::new(None));

        let latest2   = latest.clone();
        let vocal_tx2 = vocal_tx.clone();

        // Accept a single gateway connection, read sensory frames, hand the
        // write-end to the vocal sink.
        std::thread::Builder::new()
            .name("net-senses-reader".into())
            .spawn(move || {
                // Remove stale socket file if it exists.
                let _ = std::fs::remove_file(&socket_path);
                let listener = UnixListener::bind(&socket_path)
                    .expect("net_adapter: bind Unix socket");
                // Block until the gateway connects.
                let (mut stream, _) = listener.accept()
                    .expect("net_adapter: accept gateway connection");
                // Hand a cloned write-end to the vocal sink.
                let write_end = stream.try_clone()
                    .expect("net_adapter: clone socket for vocal sink");
                *vocal_tx2.lock().unwrap() = Some(write_end);

                // Read sensory frames from the gateway indefinitely.
                let mut buf = vec![0u8; FRAME_BYTES];
                loop {
                    match stream.read_exact(&mut buf) {
                        Ok(()) => {
                            if let Some(frame) = parse_frame(&buf) {
                                *latest2.lock().unwrap() = frame;
                            }
                        }
                        Err(_) => break, // gateway disconnected
                    }
                }
                // Gateway disconnected: clear the vocal sink so the ticker stops.
                *vocal_tx2.lock().unwrap() = None;
            })
            .expect("net_adapter: spawn reader thread");

        let adapter = Self { latest, vocal_tx: vocal_tx.clone() };
        (adapter, vocal_tx)
    }

    fn read_latest(&self) -> SensesFrame {
        self.latest.lock().unwrap().clone()
    }
}

impl ModalityAdapter for NetSensesAdapter {
    fn modality_bit(&self) -> u8 { MOD_VIB | MOD_PHOT }
    fn writes_ambient(&self) -> bool { true }
    fn writes_slots(&self)   -> bool { false }
    fn tick_hz(&self) -> u32 { 20 }

    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        let frame = self.read_latest();

        // ── VIB: push audio bands as hearing axes ───────────────────────────
        // Map the 12 browser bands to VIB_BANDS (4) by averaging groups.
        let per = BANDS / VIB_BANDS;
        for b in 0..VIB_BANDS {
            let avg = frame.bands[b*per..(b+1)*per].iter().sum::<f32>() / per as f32;
            vp.push_observation(b, now_us, avg as f64);
        }
        // Ambient VIB energy: use the RMS level to nudge the last-observation
        // timestamp (vital supply depends on sensory continuity).
        // We already pushed per-axis observations above; that's sufficient.

        // ── PHOT: push visual signals as vision axes ────────────────────────
        // Browser sends a 32×24 luma grid.  We map it to the 2×2 spatial
        // regions the body uses (AXIS_VIS_BASE + region×VIS_AXES_PER_REGION).
        // Per region: brightness, R(≈luma), G(≈luma), B(≈luma), motion.
        // (Without colour from browser, R=G=B=luma; colour is a future upgrade.)
        let rw = GRID_W / 2;  // 16 columns per region
        let rh = GRID_H / 2;  // 12 rows per region
        for ry in 0..2usize {
            for rx in 0..2usize {
                let region = ry * 2 + rx;
                let mut sum_luma = 0.0f32;
                let count = (rw * rh) as f32;
                for y in ry*rh..(ry+1)*rh {
                    for x in rx*rw..(rx+1)*rw {
                        sum_luma += frame.grid[y * GRID_W + x];
                    }
                }
                let brightness = sum_luma / count;
                let base = AXIS_VIS_BASE + region * VIS_AXES_PER_REGION;
                vp.push_observation(base,     now_us, brightness as f64); // brightness
                vp.push_observation(base + 1, now_us, brightness as f64); // R ≈ luma
                vp.push_observation(base + 2, now_us, brightness as f64); // G ≈ luma
                vp.push_observation(base + 3, now_us, brightness as f64); // B ≈ luma
                vp.push_observation(base + 4, now_us, frame.motion as f64); // motion
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NetVocalSink: samples the vocal organ and writes PCM frames to the gateway.
// Runs in the body's vocal ticker thread via `spawn_net_vocal_ticker`.
// ─────────────────────────────────────────────────────────────────────────────

/// Write one vocal PCM frame to the gateway.  Non-blocking best-effort:
/// if the socket is not yet connected or breaks, silently drops the frame.
pub fn send_vocal_frame(
    vocal_tx: &Arc<Mutex<Option<UnixStream>>>,
    samples: &[f32],
) {
    let mut guard = vocal_tx.lock().unwrap();
    let stream = match guard.as_mut() {
        Some(s) => s,
        None => return,
    };
    let n = samples.len() as u16;
    let mut buf = Vec::with_capacity(3 + samples.len() * 4);
    buf.push(TAG_VOCAL);
    buf.extend_from_slice(&n.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    if stream.write_all(&buf).is_err() {
        // Gateway disconnected; clear the slot so we stop trying.
        *guard = None;
    }
}

/// Sample the vocal organ and return a PCM frame (headless equivalent of
/// `vocal_adapter::sample_vocal_waveform`).
pub fn sample_vocal_pcm(vp: &Arc<Viewport>) -> Vec<f32> {
    let organ = vp.read_vocal_organ();
    let n = VOCAL_FRAME_SAMPLES;
    if organ.breath_drive < 0.01 {
        return vec![0.0f32; n];
    }
    let pitch = 100.0 + organ.pitch_tension as f64 * 400.0;
    let resonance = organ.resonance_position as f64 * 3.0;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / n as f64;
        let f = (t * pitch * 2.0 * std::f64::consts::PI).sin()
              + (t * pitch * 4.0 * std::f64::consts::PI).sin() * 0.3
              + (t * pitch * 6.0 * std::f64::consts::PI).sin() * 0.1 * resonance;
        out.push((organ.breath_drive as f64 * f).clamp(-1.0, 1.0) as f32);
    }
    out
}
