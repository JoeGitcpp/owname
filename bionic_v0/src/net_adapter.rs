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
//!   [0x01]                          tag
//!   [audio_level : f32le]           RMS amplitude 0..1
//!   [audio_flux  : f32le]           spectral onset energy 0..1
//!   [bands       : 64×f32le]        mel frequency bands 0..1
//!   [motion      : f32le]           mean inter-frame luminance change 0..1
//!   [luma        : f32le]           mean brightness 0..1
//!   [aux         : 8×f32le]         spectral_centroid, spectral_flatness,
//!                                   spectral_rolloff, spectral_spread,
//!                                   zcr, hnr, onset_strength, temporal_flux
//!   [grid        : 57600×f32le]     160×120 RGB planar grid 0..1
//!   Total: 1 + (2+64+2+8)×4 + 57600×4 = 1 + 304 + 230400 = 230705 bytes
//!
//! Body → gateway (vocal output), on demand:
//!   [0x02]                       tag
//!   [n : u16le]                  sample count
//!   [samples : n×f32le]          PCM at VOCAL_SAMPLE_RATE Hz, −1..1

use std::cell::RefCell;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::adapter::ModalityAdapter;
use crate::viewport::Viewport;
use crate::{MOD_VIB, MOD_PHOT, VIB_BANDS};

/// Sample rate for vocal PCM frames sent to the gateway.
pub const VOCAL_SAMPLE_RATE: u32 = 44_100;
/// Vocal frame size in samples per push (~11.6 ms @ 44.1 kHz).
pub const VOCAL_FRAME_SAMPLES: usize = 512;

// ── Input frame constants ────────────────────────────────────────────────────
const TAG_SENSES: u8 = 0x01;
const GRID_W: usize = 160;
const GRID_H: usize = 120;
const GRID_CHANNELS: usize = 3;
const GRID_PIXELS: usize = GRID_W * GRID_H;   // 19200
const GRID_TOTAL: usize = GRID_PIXELS * GRID_CHANNELS;  // 57600
const BANDS: usize = 64;
const AUX_FEATURES: usize = 8;
// 1 tag + (audio_level + audio_flux + BANDS + motion + luma + AUX_FEATURES)×f32 + GRID_TOTAL×f32
// = 1 + (2 + 64 + 2 + 8) × 4 + 57600×4 = 1 + 304 + 230400 = 230705
const FRAME_BYTES: usize = 1 + (2 + BANDS + 2 + AUX_FEATURES) * 4 + GRID_TOTAL * 4;

// ── Output frame constants ───────────────────────────────────────────────────
const TAG_VOCAL: u8   = 0x02;
const TAG_SURFACE: u8 = 0x04;
const SURF_W: usize   = 320;
const SURF_H: usize   = 180;

// ── Shared latest-senses buffer ──────────────────────────────────────────────
#[derive(Clone)]
struct SensesFrame {
    audio_level: f32,
    audio_flux:  f32,
    bands:       [f32; BANDS],       // 64 mel bands
    motion:      f32,
    luma:        f32,
    // 8 aux time-frequency features
    spectral_centroid:  f32,
    spectral_flatness:  f32,
    spectral_rolloff:   f32,
    spectral_spread:    f32,
    zcr:                f32,
    hnr:                f32,
    onset_strength:     f32,
    temporal_flux:      f32,
    // RGB grid: [R pixels (GRID_PIXELS)][G pixels][B pixels]
    grid: Vec<f32>,  // length = GRID_TOTAL (57600)
}

impl Default for SensesFrame {
    fn default() -> Self {
        Self {
            audio_level:       0.0,
            audio_flux:        0.0,
            bands:             [0.0; BANDS],
            motion:            0.0,
            luma:              0.0,
            spectral_centroid: 0.0,
            spectral_flatness: 0.0,
            spectral_rolloff:  0.0,
            spectral_spread:   0.0,
            zcr:               0.0,
            hnr:               0.0,
            onset_strength:    0.0,
            temporal_flux:     0.0,
            grid:              vec![0.0; GRID_TOTAL],
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
    let spectral_centroid = rf(&mut off);
    let spectral_flatness = rf(&mut off);
    let spectral_rolloff  = rf(&mut off);
    let spectral_spread   = rf(&mut off);
    let zcr               = rf(&mut off);
    let hnr               = rf(&mut off);
    let onset_strength    = rf(&mut off);
    let temporal_flux     = rf(&mut off);
    let mut grid = vec![0.0f32; GRID_TOTAL];
    for g in &mut grid { *g = rf(&mut off); }
    Some(SensesFrame {
        audio_level, audio_flux, bands, motion, luma,
        spectral_centroid, spectral_flatness, spectral_rolloff, spectral_spread,
        zcr, hnr, onset_strength, temporal_flux,
        grid,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// NetSensesAdapter: reads from a Unix socket, drives VIB + PHOT axes.
// One instance covers both VIB and PHOT (they share the same socket).
// ─────────────────────────────────────────────────────────────────────────────
pub struct NetSensesAdapter {
    latest: Arc<Mutex<SensesFrame>>,
    /// Send half for vocal output frames (written by the body's vocal ticker).
    vocal_tx: Arc<Mutex<Option<UnixStream>>>,
    /// EMA of VIB band energy — drives event nomination (matches hardware VibAdapter).
    smoothed_energy: f64,
    /// EMA of VIB surprise (energy change) — accompanies the nominated event.
    smoothed_surprise: f64,
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

        let adapter = Self {
            latest,
            vocal_tx: vocal_tx.clone(),
            smoothed_energy: 0.0,
            smoothed_surprise: 0.0,
        };
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
        // Map the 64 mel bands to VIB_BANDS (16) by averaging groups of 4.
        let per = BANDS / VIB_BANDS;  // 64/16 = 4
        let mut vib_bands = [0.0f64; VIB_BANDS];
        for b in 0..VIB_BANDS {
            let avg = frame.bands[b*per..(b+1)*per].iter().sum::<f32>() / per as f32;
            vib_bands[b] = avg as f64;
            vp.push_observation(b, now_us, avg as f64);
        }

        // ── VIB event nomination (parity with hardware VibAdapter) ──────────
        // Without this, ambient VIB candidate_flag stays false forever, so
        // tick_vocal_target_from_vib always returns early → no vocal target →
        // no imitation → the body NEVER learns to speak from network audio.
        // (This was the headless-path gap: NetSensesAdapter pushed band
        //  observations but never nominated events the way VibAdapter does.)
        let inst_energy: f64 = vib_bands.iter().map(|b| b * b).sum();
        let inst_surprise = (inst_energy - self.smoothed_energy).abs();
        let alpha = 0.1;
        self.smoothed_energy   = (1.0 - alpha) * self.smoothed_energy   + alpha * inst_energy;
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;
        let is_event = self.smoothed_energy > crate::EVENT_NOMINATION_THRESHOLD;
        vp.write_ambient_for(
            crate::MOD_VIB,
            self.smoothed_energy,
            self.smoothed_surprise,
            if is_event { Some(now_us) } else { None },
            is_event,
        );

        // ── PHOT: push visual signals as vision axes ────────────────────────
        // Browser sends a 160×120 RGB planar grid.  We map it to the 4×4
        // spatial regions the body uses (AXIS_VIS_BASE + region×VIS_AXES_PER_REGION).
        // Per region: brightness, R, G, B, saturation, motion, temporal_flux, edge_energy.
        let rw = GRID_W / 4;  // 40 pixels per region column
        let rh = GRID_H / 4;  // 30 pixels per region row
        for ry in 0..4usize {
            for rx in 0..4usize {
                let region = ry * 4 + rx;
                let mut sum_r = 0.0f32; let mut sum_g = 0.0f32; let mut sum_b = 0.0f32;
                let count = (rw * rh) as f32;
                for y in ry*rh..(ry+1)*rh {
                    for x in rx*rw..(rx+1)*rw {
                        let i = y * GRID_W + x;
                        sum_r += frame.grid[i];
                        sum_g += frame.grid[GRID_PIXELS + i];
                        sum_b += frame.grid[2 * GRID_PIXELS + i];
                    }
                }
                let r = sum_r / count;
                let g = sum_g / count;
                let b = sum_b / count;
                let brightness = (r + g + b) / 3.0;
                let max_rgb = r.max(g).max(b);
                let min_rgb = r.min(g).min(b);
                let saturation = if max_rgb > 0.01 { (max_rgb - min_rgb) / max_rgb } else { 0.0 };

                let base = crate::AXIS_VIS_BASE + region * crate::VIS_AXES_PER_REGION;
                vp.push_observation(base + crate::vis_sub::BRIGHTNESS,    now_us, brightness as f64);
                vp.push_observation(base + crate::vis_sub::R,             now_us, r as f64);
                vp.push_observation(base + crate::vis_sub::G,             now_us, g as f64);
                vp.push_observation(base + crate::vis_sub::B,             now_us, b as f64);
                vp.push_observation(base + crate::vis_sub::SATURATION,    now_us, saturation as f64);
                vp.push_observation(base + crate::vis_sub::MOTION,        now_us, frame.motion as f64);
                vp.push_observation(base + crate::vis_sub::TEMPORAL_FLUX, now_us, frame.temporal_flux as f64);
                vp.push_observation(base + crate::vis_sub::EDGE_ENERGY,   now_us, brightness as f64 * 0.5);
            }
        }

        // ── Gaussian visual field (headless) ────────────────────────────────
        // Feed the 160×120 RGB grid into the Gaussian field so the surface
        // ticker can render a soft, organic splatted reconstruction of what
        // the body currently perceives through the browser camera.
        vp.tick_gaussian_field_from_net(&frame.grid, now_us);
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
thread_local! {
    static VOCAL_SYN: RefCell<crate::vocal_synthesis::VocalSynthesizer> =
        RefCell::new(crate::vocal_synthesis::VocalSynthesizer::new());
}

pub fn sample_vocal_pcm(vp: &Arc<Viewport>) -> Vec<f32> {
    let organ = vp.read_vocal_organ();
    let reserve = vp.read_vital_core().reserve;
    VOCAL_SYN.with(|syn| {
        syn.borrow_mut().synthesize(&organ, reserve, VOCAL_FRAME_SAMPLES)
    })
}

// ── Surface pixel sender ─────────────────────────────────────────────────────
/// Maintains trail history across calls and sends rendered surface RGBA frames
/// (tag 0x04) to the gateway via the same vocal TX socket.
pub struct SurfaceSender {
    history: crate::surface_history::SurfaceHistory,
}

impl SurfaceSender {
    pub fn new() -> Self {
        Self { history: crate::surface_history::SurfaceHistory::new(16) }
    }

    pub fn send(&mut self, snap_path: &std::path::Path, vocal_tx: &Arc<Mutex<Option<UnixStream>>>) {
        let snap = match crate::snapshot::read_snapshot(snap_path) {
            Ok(s) => s,
            Err(_) => return,
        };
        let frame = crate::surface_projection::project_surface_frame(&snap);
        self.history.update(&frame);
        let trails = self.history.render_trails();
        let config = crate::surface_render::RenderConfig::new(SURF_W, SURF_H);
        let pixels = crate::surface_render::render_surface_frame(&frame, &trails, config);

        let mut guard = vocal_tx.lock().unwrap();
        let stream = match guard.as_mut() { Some(s) => s, None => return };

        let mut buf = Vec::with_capacity(5 + SURF_W * SURF_H * 4);
        buf.push(TAG_SURFACE);
        buf.extend_from_slice(&(SURF_W as u16).to_le_bytes());
        buf.extend_from_slice(&(SURF_H as u16).to_le_bytes());
        for px in &pixels { buf.extend_from_slice(px); }

        if stream.write_all(&buf).is_err() {
            *guard = None;
        }
    }

    /// Render the viewport's live Gaussian visual field as a surface frame.
    /// Used in headless mode: the field is populated by `tick_gaussian_field_from_net`
    /// via NetSensesAdapter every time a senses frame arrives from the browser.
    ///
    /// Falls back silently (no frame sent) when the field has no active Gaussians
    /// yet (i.e. no senses input received since body startup).
    pub fn send_gauss(
        &mut self,
        vp: &Arc<Viewport>,
        vocal_tx: &Arc<Mutex<Option<UnixStream>>>,
    ) {
        use crate::phot_gaussian::{GAUSS_PIX, GAUSS_REC_W, GAUSS_REC_H};

        let field = vp.read_gaussian_field();
        if field.active_count() == 0 {
            return;  // field not yet populated — wait for first senses frames
        }
        let recon = field.splat();  // 3×GAUSS_PIX planar f64 RGB in [0, 1]

        // Nearest-neighbour upscale from Gaussian rec dims → SURF_W×SURF_H (320×180).
        let frame_bytes = 5 + SURF_W * SURF_H * 4;
        let mut buf = Vec::with_capacity(frame_bytes);
        buf.push(TAG_SURFACE);
        buf.extend_from_slice(&(SURF_W as u16).to_le_bytes());
        buf.extend_from_slice(&(SURF_H as u16).to_le_bytes());
        for sy in 0..SURF_H {
            let gy = sy * GAUSS_REC_H / SURF_H;
            for sx in 0..SURF_W {
                let gx  = sx * GAUSS_REC_W / SURF_W;
                let i   = gy * GAUSS_REC_W + gx;
                let r   = (recon[i].clamp(0.0, 1.0) * 255.0) as u8;
                let g   = (recon[GAUSS_PIX + i].clamp(0.0, 1.0) * 255.0) as u8;
                let b   = (recon[2 * GAUSS_PIX + i].clamp(0.0, 1.0) * 255.0) as u8;
                buf.extend_from_slice(&[r, g, b, 255]);
            }
        }

        let mut guard = vocal_tx.lock().unwrap();
        let stream = match guard.as_mut() { Some(s) => s, None => return };
        if stream.write_all(&buf).is_err() {
            *guard = None;
        }
    }
}
