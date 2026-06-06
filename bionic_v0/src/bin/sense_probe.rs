//! sense_probe — read-only "physiological" probe of the passive perceptual
//! frontend (Part A / M1: synthetic-controlled).
//!
//! WHAT THIS IS
//!   A debug/observer-side tool, NOT a body organ. It builds the *real*
//!   perception pipeline (`Viewport` + `RealPhotAdapter`), drives it with
//!   controlled synthetic stimuli, then *reads* the internal tier-0 axes the
//!   body computes and scores whether they co-vary with the known stimulus.
//!   It never mutates body ontology — it only ticks the adapter (exactly as the
//!   live body does) and reads axes via `vp.latest_observation(...)`.
//!
//!   This turns the §6.2 acceptance table ("non-pixel / perturbation field")
//!   from code-traced into behaviorally measured. With ground-truth stimuli we
//!   can assert the co-variation must hold; the live-webcam session (M2) is the
//!   qualitative confirmation on top of this.
//!
//! WHAT IT TESTS (live-capable today, no new body wiring)
//!   1. Color        — per-region R/G/B axes track the presented hue.
//!   2. Motion       — per-region MOTION axis (brightness Δ) recovers gross
//!                     left→right and top→bottom direction.
//!   3. Looming/size — object footprint across regions grows as it approaches.
//!   4. Learning     — Gaussian recon MSE falls on a stable scene (report-only).
//!
//! WHAT IT DOES *NOT* TEST (honest scope — see §6.2)
//!   - True metric depth ρ: `depth_observer` is not ticked in the live loop and
//!     depth, by design, comes from SELF-translation + efference, not object
//!     motion. That is Part B (wire depth_observer + camera-self-motion
//!     efference into the live loop). Here we measure only the looming PROXY.
//!   - saturation / temporal_flux / edge_energy axes: not computed on the native
//!     RealPhotAdapter path (only the browser/net path), so they read ~0.
//!   - identity / recognition: vision is not yet bound cross-modally (§6 ③).
//!
//! Run:  cargo run --bin sense_probe        (no camera, no permissions needed)

use std::process::ExitCode;
use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter};
use bionic_v0::phot_camera::{
    CameraFault, CameraFrame, CameraFrameProvider, SynthCamStep, SyntheticCameraFrameProvider,
    CAM_PIX,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::{vis_axis, vis_sub, GAUSS_RECON_MSE_AXIS, VIS_GRID_COLS, VIS_REGIONS};

const DT_US: u64 = 33_333; // ~30 Hz

// --------------------------------------------------------------------------
// Read helpers (pure read-out of the shared observation axes)
// --------------------------------------------------------------------------

fn read_axis(vp: &Arc<Viewport>, axis: usize) -> f64 {
    match vp.latest_observation(axis) {
        Some(s) if s.valid => s.value,
        _ => 0.0,
    }
}

/// Mean of one visual sub-axis across all 16 regions.
fn region_axis_mean(vp: &Arc<Viewport>, sub: usize) -> f64 {
    let mut sum = 0.0;
    for r in 0..VIS_REGIONS {
        sum += read_axis(vp, vis_axis(r, sub));
    }
    sum / VIS_REGIONS as f64
}

fn region_col(region: usize) -> usize {
    region % VIS_GRID_COLS
}
fn region_row(region: usize) -> usize {
    region / VIS_GRID_COLS
}

/// Motion-weighted centroid over the 4×4 grid: (mean_col, mean_row, total).
/// Columns/rows are indexed 0..3. Returns (0,0,0) if there is no motion.
fn motion_centroid(vp: &Arc<Viewport>) -> (f64, f64, f64) {
    let (mut wc, mut wr, mut tot) = (0.0, 0.0, 0.0);
    for r in 0..VIS_REGIONS {
        let m = read_axis(vp, vis_axis(r, vis_sub::MOTION));
        wc += m * region_col(r) as f64;
        wr += m * region_row(r) as f64;
        tot += m;
    }
    if tot <= 1e-9 {
        (0.0, 0.0, 0.0)
    } else {
        (wc / tot, wr / tot, tot)
    }
}

/// How many regions carry brightness above `thresh` (object footprint), and the
/// total brightness mass.
fn bright_footprint(vp: &Arc<Viewport>, thresh: f64) -> (usize, f64) {
    let (mut n, mut mass) = (0usize, 0.0);
    for r in 0..VIS_REGIONS {
        let b = read_axis(vp, vis_axis(r, vis_sub::BRIGHTNESS));
        if b > thresh {
            n += 1;
        }
        mass += b;
    }
    (n, mass)
}

// --------------------------------------------------------------------------
// Custom provider: full-field RGB color (built-in Blob is grayscale, r=g=b).
// --------------------------------------------------------------------------

struct ColorFieldProvider {
    colors: Vec<(f64, f64, f64)>, // one entry per tick
    idx: usize,
    t_us: u64,
}
impl ColorFieldProvider {
    fn new(colors: Vec<(f64, f64, f64)>) -> Self {
        Self { colors, idx: 0, t_us: 0 }
    }
}
impl CameraFrameProvider for ColorFieldProvider {
    fn is_healthy(&self) -> bool {
        true
    }
    fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
        let (r, g, b) = self
            .colors
            .get(self.idx)
            .copied()
            .or_else(|| self.colors.last().copied())
            .unwrap_or((0.0, 0.0, 0.0));
        self.idx += 1;
        self.t_us += DT_US;
        let mut f = CameraFrame::zeros(self.t_us);
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        for i in 0..CAM_PIX {
            f.r[i] = r;
            f.g[i] = g;
            f.b[i] = b;
            f.luma[i] = luma;
        }
        Ok(Some(f))
    }
}

fn pass_str(ok: bool) -> &'static str {
    if ok {
        "PASS"
    } else {
        "FAIL"
    }
}

// --------------------------------------------------------------------------
// Test 1 — Color discrimination
// --------------------------------------------------------------------------

fn test_color() -> bool {
    println!("\n── Test 1: COLOR — do per-region R/G/B axes track the hue? ──");
    let n = 5usize; // frames per color (R/G/B axes are direct per-frame means)
    let phases: [(&str, (f64, f64, f64)); 4] = [
        ("RED  ", (0.90, 0.05, 0.05)),
        ("GREEN", (0.05, 0.90, 0.05)),
        ("BLUE ", (0.05, 0.05, 0.90)),
        ("WHITE", (0.85, 0.85, 0.85)),
    ];
    let mut colors = Vec::new();
    for (_, c) in &phases {
        for _ in 0..n {
            colors.push(*c);
        }
    }
    let mut adapter = RealPhotAdapter::new(Box::new(ColorFieldProvider::new(colors)));
    let vp = Arc::new(Viewport::new());

    let mut all_ok = true;
    let mut tick = 0u64;
    for (pi, (name, _)) in phases.iter().enumerate() {
        // Drive this color's frames; read after the last one.
        for _ in 0..n {
            adapter.tick(&vp, 1_000_000 + tick * DT_US);
            tick += 1;
        }
        let r = region_axis_mean(&vp, vis_sub::R);
        let g = region_axis_mean(&vp, vis_sub::G);
        let b = region_axis_mean(&vp, vis_sub::B);
        // Expected dominant channel per phase index: 0=R,1=G,2=B,3=all-high.
        let margin = 0.30;
        let ok = match pi {
            0 => r > g + margin && r > b + margin,
            1 => g > r + margin && g > b + margin,
            2 => b > r + margin && b > g + margin,
            3 => r > 0.4 && g > 0.4 && b > 0.4, // white: all channels high
            _ => false,
        };
        all_ok &= ok;
        println!(
            "  {}  R={:.2} G={:.2} B={:.2}   [{}]",
            name,
            r,
            g,
            b,
            pass_str(ok)
        );
    }
    println!("  → color discrimination: {}", pass_str(all_ok));
    all_ok
}

// --------------------------------------------------------------------------
// Test 2 — Motion direction (gross, over the 4×4 grid)
// --------------------------------------------------------------------------

/// Sweep a blob and return the motion-weighted (col, row) centroid trajectory
/// across the moving frames.
fn sweep_trajectory(positions: &[(f64, f64)]) -> Vec<(f64, f64)> {
    // 3 establishing frames at the first position (motion decays to ~0), then
    // step through `positions`; read centroid on each moving frame.
    let (x0, y0) = positions[0];
    let mut script = vec![
        SynthCamStep::Blob { cx: x0, cy: y0, r: 0.12, brightness: 0.95 };
        3
    ];
    for &(cx, cy) in positions {
        script.push(SynthCamStep::Blob { cx, cy, r: 0.12, brightness: 0.95 });
    }
    let mut adapter =
        RealPhotAdapter::new(Box::new(SyntheticCameraFrameProvider::new(script, 0, DT_US)));
    let vp = Arc::new(Viewport::new());

    let mut traj = Vec::new();
    let mut tick = 0u64;
    // establishing frames
    for _ in 0..3 {
        adapter.tick(&vp, 1_000_000 + tick * DT_US);
        tick += 1;
    }
    // moving frames
    for _ in positions {
        adapter.tick(&vp, 1_000_000 + tick * DT_US);
        tick += 1;
        let (c, r, tot) = motion_centroid(&vp);
        if tot > 1e-6 {
            traj.push((c, r));
        }
    }
    traj
}

fn test_motion() -> bool {
    println!("\n── Test 2: MOTION — is gross direction recoverable from the MOTION axis? ──");
    let steps = 11usize;

    // Left→right sweep: cx 0.15→0.85, cy fixed.
    let lr: Vec<(f64, f64)> = (0..steps)
        .map(|i| (0.15 + 0.70 * i as f64 / (steps - 1) as f64, 0.5))
        .collect();
    let lr_traj = sweep_trajectory(&lr);
    let lr_ok = if lr_traj.len() >= 2 {
        let dc = lr_traj.last().unwrap().0 - lr_traj[0].0;
        let dr = (lr_traj.last().unwrap().1 - lr_traj[0].1).abs();
        println!(
            "  L→R sweep: col {:.2}→{:.2} (Δcol={:+.2}), row drift={:.2}",
            lr_traj[0].0,
            lr_traj.last().unwrap().0,
            dc,
            dr
        );
        dc > 1.0 && dr < 1.0 // column advances ≥1 cell, row stays put
    } else {
        println!("  L→R sweep: insufficient motion samples");
        false
    };

    // Top→bottom sweep: cy 0.15→0.85, cx fixed.
    let tb: Vec<(f64, f64)> = (0..steps)
        .map(|i| (0.5, 0.15 + 0.70 * i as f64 / (steps - 1) as f64))
        .collect();
    let tb_traj = sweep_trajectory(&tb);
    let tb_ok = if tb_traj.len() >= 2 {
        let dr = tb_traj.last().unwrap().1 - tb_traj[0].1;
        let dc = (tb_traj.last().unwrap().0 - tb_traj[0].0).abs();
        println!(
            "  T→B sweep: row {:.2}→{:.2} (Δrow={:+.2}), col drift={:.2}",
            tb_traj[0].1,
            tb_traj.last().unwrap().1,
            dr,
            dc
        );
        dr > 1.0 && dc < 1.0
    } else {
        println!("  T→B sweep: insufficient motion samples");
        false
    };

    let ok = lr_ok && tb_ok;
    println!("  → motion direction: {}", pass_str(ok));
    ok
}

// --------------------------------------------------------------------------
// Test 3 — Looming / size proxy
// --------------------------------------------------------------------------

fn test_looming() -> bool {
    println!("\n── Test 3: LOOMING/SIZE — does the footprint grow as the object approaches? ──");
    let steps = 9usize;
    let r_min = 0.05;
    let r_max = 0.45;
    let mut script = vec![SynthCamStep::Idle; 2]; // seed
    for i in 0..steps {
        let r = r_min + (r_max - r_min) * i as f64 / (steps - 1) as f64;
        script.push(SynthCamStep::Blob { cx: 0.5, cy: 0.5, r, brightness: 0.95 });
    }
    let mut adapter =
        RealPhotAdapter::new(Box::new(SyntheticCameraFrameProvider::new(script, 0, DT_US)));
    let vp = Arc::new(Viewport::new());

    let mut tick = 0u64;
    for _ in 0..2 {
        adapter.tick(&vp, 1_000_000 + tick * DT_US);
        tick += 1;
    }
    let mut footprints = Vec::new();
    for i in 0..steps {
        adapter.tick(&vp, 1_000_000 + tick * DT_US);
        tick += 1;
        let (n, mass) = bright_footprint(&vp, 0.10);
        let r = r_min + (r_max - r_min) * i as f64 / (steps - 1) as f64;
        footprints.push((r, n, mass));
    }
    for (r, n, mass) in &footprints {
        println!("  r={:.2}  footprint={:>2} regions  brightness_mass={:.2}", r, n, mass);
    }
    let first_n = footprints.first().map(|t| t.1).unwrap_or(0);
    let last_n = footprints.last().map(|t| t.1).unwrap_or(0);
    let first_mass = footprints.first().map(|t| t.2).unwrap_or(0.0);
    let last_mass = footprints.last().map(|t| t.2).unwrap_or(0.0);
    // Footprint or brightness mass must grow clearly as the object nears.
    let ok = last_n > first_n && last_mass > first_mass + 0.10;
    println!(
        "  → looming proxy (footprint {}→{}, mass {:.2}→{:.2}): {}",
        first_n,
        last_n,
        first_mass,
        last_mass,
        pass_str(ok)
    );
    ok
}

// --------------------------------------------------------------------------
// Test 4 — Predictive learning (report-only): Gaussian recon MSE on a stable scene
// --------------------------------------------------------------------------

fn report_learning() {
    println!("\n── Info: PREDICTIVE LEARNING — Gaussian recon MSE on a stable scene (report-only) ──");
    let mut script = vec![SynthCamStep::Idle; 2];
    script.extend(vec![
        SynthCamStep::Blob { cx: 0.5, cy: 0.5, r: 0.2, brightness: 0.9 };
        180
    ]);
    let mut adapter =
        RealPhotAdapter::new(Box::new(SyntheticCameraFrameProvider::new(script, 0, DT_US)));
    let vp = Arc::new(Viewport::new());

    let mut early = f64::NAN;
    for step in 0..182u64 {
        adapter.tick(&vp, 1_000_000 + step * DT_US);
        if step == 10 {
            early = read_axis(&vp, GAUSS_RECON_MSE_AXIS);
        }
    }
    let late = read_axis(&vp, GAUSS_RECON_MSE_AXIS);
    println!(
        "  recon MSE: early(step10)={:.5}  late(step181)={:.5}  ({})",
        early,
        late,
        if late < early {
            "falling — field is learning the scene"
        } else {
            "not falling on this scene"
        }
    );
}

fn main() -> ExitCode {
    println!("================================================================");
    println!(" sense_probe — read-only physiological probe (Part A / M1 synthetic)");
    println!(" Drives the REAL pipeline (Viewport + RealPhotAdapter) with");
    println!(" ground-truth stimuli and reads the tier-0 axes it computes.");
    println!(" Grid: {} regions ({}×{}). Pixel pictures are never reconstructed.", VIS_REGIONS, bionic_v0::VIS_GRID_ROWS, VIS_GRID_COLS);
    println!("================================================================");

    let c = test_color();
    let m = test_motion();
    let l = test_looming();
    report_learning();

    println!("\n================================ SUMMARY ================================");
    println!("  Color discrimination : {}", pass_str(c));
    println!("  Motion direction     : {}", pass_str(m));
    println!("  Looming / size proxy : {}", pass_str(l));
    println!("  NOT tested here: true metric depth (Part B), saturation/edge/flux");
    println!("                  axes (native path n/a), identity (cross-modal gap).");
    println!("========================================================================");

    if c && m && l {
        println!("ALL PASS — passive inference co-varies with color/motion/size.");
        ExitCode::SUCCESS
    } else {
        println!("SOME FAILED — see per-test lines above.");
        ExitCode::FAILURE
    }
}
