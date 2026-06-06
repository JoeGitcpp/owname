//! Modality Adapter Contract + Registry (Phase 3A).
//!
//! Every sensory modality (VIB / PHOT / MECH / future) interacts with the
//! viewport through an adapter that does exactly three things:
//!
//!   1. `write_ambient_for(modality_bit, ...)` — its own ambient channel
//!   2. nominate_or_find_slot                  — pick an object slot
//!   3. `write_slot_{vib,phot,mech,spatial}`   — write modality-specific data
//!
//! Adapters never talk to each other directly and never touch other
//! modalities' viewport fields. The registry only schedules: it owns nothing
//! semantic, and it cannot combine modality outputs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::action_consequence::tick_vib_ingress_consequence;
use crate::clock::now_us;
use crate::evolution_priors::BiasPriors;
use crate::phot::SyntheticPhotSource;
use crate::phot_camera::{CameraFault, CameraFrameProvider, MotionExtractor};
use crate::substrate_pressure::{compute_pressure, PressureSample};
use crate::synthetic_vib::SyntheticVibSource;
use crate::viewport::{EventEvidence, HealthState, SlotClaim, SpatialEvidence, Viewport};
use crate::{EVENT_NOMINATION_THRESHOLD, MOD_PHOT, MOD_VIB, PHOT_STALE_WINDOW_US, VIB_BANDS};

/// The sole write-path for one modality into the viewport. Each adapter owns
/// its own input source (mic, camera, ...) and exposes which viewport regions
/// it will touch.
///
/// No `Send` bound: adapters may hold platform handles that cannot cross
/// threads (e.g. `cpal::Stream`). The registry guarantees each adapter is
/// constructed and ticked on the same thread.
pub trait ModalityAdapter {
    fn modality_bit(&self) -> u8;
    fn writes_ambient(&self) -> bool;
    fn writes_slots(&self) -> bool;

    /// Desired tick cadence. Registry rate-limits per adapter.
    fn tick_hz(&self) -> u32 {
        200
    }

    /// Drain pending input and issue viewport writes. Must be non-blocking.
    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64);
}

#[derive(Debug)]
pub enum RegistryError {
    DuplicateModality(u8),
}

struct AdapterEntry {
    adapter: Box<dyn ModalityAdapter>,
    period_us: u64,
    last_tick_us: u64,
}

/// Owns a set of adapters and drives each at its declared cadence. No
/// semantic logic — the registry cannot observe what an adapter writes and
/// does not combine outputs.
///
/// `AdapterRegistry` is not `Send`; build it inside the driver thread via
/// [`spawn_registry_driver`].
pub struct AdapterRegistry {
    entries: Vec<AdapterEntry>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn ModalityAdapter>) -> Result<(), RegistryError> {
        let bit = adapter.modality_bit();
        if self.entries.iter().any(|e| e.adapter.modality_bit() == bit) {
            return Err(RegistryError::DuplicateModality(bit));
        }
        let hz = adapter.tick_hz().max(1);
        let period_us = 1_000_000 / hz as u64;
        self.entries.push(AdapterEntry {
            adapter,
            period_us,
            last_tick_us: 0,
        });
        Ok(())
    }

    /// Union of registered modality bits. Pure audit helper.
    pub fn modality_mask(&self) -> u8 {
        self.entries
            .iter()
            .fold(0u8, |acc, e| acc | e.adapter.modality_bit())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Fire `tick()` on every adapter whose period has elapsed since its last
    /// tick. Called in a tight loop by the driver thread.
    pub fn tick_due(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        for e in self.entries.iter_mut() {
            if now_us.saturating_sub(e.last_tick_us) >= e.period_us {
                e.adapter.tick(vp, now_us);
                e.last_tick_us = now_us;
            }
        }
    }
}

/// Spawn a driver thread. The builder runs *inside* the new thread, so the
/// registry (and every adapter it owns) never crosses thread boundaries —
/// non-Send adapter state (audio streams, GPU handles) is allowed.
///
/// The caller learns the modality_mask asynchronously via the viewport, not
/// via a return value, to preserve the "viewport is the only channel" rule.
pub fn spawn_registry_driver<F>(
    vp: Arc<Viewport>,
    stop: Arc<AtomicBool>,
    build: F,
) -> JoinHandle<()>
where
    F: FnOnce() -> AdapterRegistry + Send + 'static,
{
    std::thread::spawn(move || {
        let mut reg = build();
        // Sleep granularity fine enough for any adapter up to 500 Hz.
        let sleep = Duration::from_micros(2_000);
        while !stop.load(Ordering::Relaxed) {
            let t = now_us();
            reg.tick_due(&vp, t);
            std::thread::sleep(sleep);
        }
    })
}

// ---------------------------------------------------------------------------
// VIB adapter
// ---------------------------------------------------------------------------

/// VIB adapter — owns the mic stream and the band-energy viewport write path.
/// Non-Send: the cpal stream is bound to its construction thread.
pub struct VibAdapter {
    rx: Option<Receiver<[f64; VIB_BANDS]>>,
    _stream: Option<cpal::Stream>,
    fault: Arc<AtomicBool>,
    smoothed_energy: f64,
    smoothed_surprise: f64,
    priors: Arc<BiasPriors>,
}

impl VibAdapter {
    /// Construct inside the driver thread. If mic init fails the adapter is
    /// still returned (faulted); the viewport records the hearing fault so
    /// the registry needs no modality-specific error path.
    pub fn new(vp: &Arc<Viewport>) -> Self {
        Self::with_priors(vp, vp.birth_sensitivity())
    }

    pub fn with_priors(vp: &Arc<Viewport>, priors: Arc<BiasPriors>) -> Self {
        let fault = Arc::new(AtomicBool::new(false));
        match crate::vib::VibSource::try_new_parts(fault.clone()) {
            Ok((rx, stream)) => Self {
                rx: Some(rx),
                _stream: Some(stream),
                fault,
                smoothed_energy: 0.0,
                smoothed_surprise: 0.0,
                priors,
            },
            Err(_) => {
                vp.set_hearing_fault(true);
                Self {
                    rx: None,
                    _stream: None,
                    fault,
                    smoothed_energy: 0.0,
                    smoothed_surprise: 0.0,
                    priors,
                }
            }
        }
    }

    /// Test-only accessor for the priors reference.
    #[cfg(any(test, debug_assertions, feature = "test-support"))]
    pub fn priors(&self) -> &BiasPriors {
        &self.priors
    }

    /// Test-only entry point for process_frame that bypasses the mic stream.
    /// Exposes the same logic to integration tests.
    #[cfg(any(test, debug_assertions, feature = "test-support"))]
    pub fn process_frame_for_test(
        &mut self,
        vp: &Arc<Viewport>,
        t_us: u64,
        bands: &[f64; VIB_BANDS],
    ) {
        self.process_frame(vp, t_us, bands);
    }

    fn process_frame(&mut self, vp: &Arc<Viewport>, t_us: u64, bands: &[f64; VIB_BANDS]) {
        for axis in 0..VIB_BANDS {
            vp.push_observation(axis, t_us, bands[axis]);
        }
        vp.sweep_slots(t_us);

        let inst_energy: f64 = bands.iter().map(|b| b * b).sum();

        // Collect per-band err over the shared coordinate substrate.
        let mut per_band_err = [0.0f64; VIB_BANDS];
        for axis in 0..VIB_BANDS {
            if let Some(e) = vp.latest_err(axis) {
                per_band_err[axis] = e;
            }
        }

        // Sensitivity-weighted base surprise.
        let mut inst_surprise = 0.0;
        for axis in 0..VIB_BANDS {
            inst_surprise += per_band_err[axis].abs() * self.priors.band_sensitivity_at(axis);
        }

        let disruption_score =
            crate::evolution_priors::compute_disruption_score(&per_band_err, &self.priors);
        let max_err = per_band_err.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
        let min_err = per_band_err.iter().map(|e| e.abs()).fold(1.0_f64, f64::min);
        let broadband = (disruption_score / VIB_BANDS as f64).clamp(0.0, 1.0);
        let pressure = compute_pressure(
            PressureSample {
                err: max_err,
                deviation: (max_err - min_err).abs(),
                cross_channel: broadband,
                ..Default::default()
            },
            &self.priors.pressure_weights,
        );
        inst_surprise *= 1.0 + 0.25 * pressure;

        let alpha = 0.1;
        self.smoothed_energy = (1.0 - alpha) * self.smoothed_energy + alpha * inst_energy;
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;
        // Event nomination derives from the shared substrate threshold so a
        // signal classified QUIET by viewport health cannot be flagged as an
        // event by the adapter.
        let is_event = self.smoothed_energy > EVENT_NOMINATION_THRESHOLD;

        vp.write_ambient_for(
            MOD_VIB,
            self.smoothed_energy,
            self.smoothed_surprise,
            if is_event { Some(t_us) } else { None },
            is_event,
        );
        // Wiring: record VIB ingress change as a consequence observation.
        // Extends an Open window or begins a new one; no target, result, or
        // meaning inference.
        tick_vib_ingress_consequence(vp, t_us);
        if !is_event {
            return;
        }

        let ambient = vp.read_ambient();
        // Phase 3C: adapter submits evidence; viewport decides the slot.
        // VIB carries no 2D geometry, so spatial stays None.
        let evidence = EventEvidence {
            modality_bit: MOD_VIB,
            event_t_us: t_us,
            event_energy: self.smoothed_energy,
            surprise: self.smoothed_surprise,
            spatial: None,
            nomination_seq_hint: ambient.nomination_seq,
        };
        let slot_id = match vp.claim_slot_for_event(&evidence) {
            SlotClaim::Reused(id) | SlotClaim::Allocated(id) => id,
            SlotClaim::Dropped => return,
        };

        let mut predicted = [0.0f64; VIB_BANDS];
        let mut err = [0.0f64; VIB_BANDS];
        let mut surprise = [0.0f64; VIB_BANDS];
        for axis in 0..VIB_BANDS {
            let (_a, _b, s, last_err, _mse, _u) = vp.read_predictor(axis);
            surprise[axis] = s;
            err[axis] = last_err;
            if let Some(pred) = {
                let r = vp.predicted[axis].read().unwrap();
                r.latest()
            } {
                predicted[axis] = pred.value;
            }
        }
        vp.write_slot_vib(slot_id, t_us, bands, &predicted, &err, &surprise);
        vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
        vp.refresh_slot_high_layer(slot_id, t_us);
    }
}

impl ModalityAdapter for VibAdapter {
    fn modality_bit(&self) -> u8 {
        MOD_VIB
    }
    fn writes_ambient(&self) -> bool {
        true
    }
    fn writes_slots(&self) -> bool {
        true
    }
    fn tick_hz(&self) -> u32 {
        200
    }
    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        if self.fault.swap(false, Ordering::AcqRel) {
            vp.report_audio_error();
        }
        if self.rx.is_none() {
            return;
        }
        loop {
            let bands = match self.rx.as_ref().unwrap().try_recv() {
                Ok(b) => b,
                Err(_) => break,
            };
            self.process_frame(vp, now_us, &bands);
        }
    }
}

// ---------------------------------------------------------------------------
// Synthetic PHOT adapter (Phase 3B).
// ---------------------------------------------------------------------------

/// Drives `ambient.phot`, `slot.phot`, and `slot.spatial` from a deterministic
/// synthetic source. Introduces no real camera and no visual semantics.
///
/// Rules:
///   * Only touches MOD_PHOT fields. Never writes VIB state.
///   * Never reads or writes semantic labels.
///   * Never talks to any other adapter directly.
pub struct SyntheticPhotAdapter {
    source: SyntheticPhotSource,
    smoothed_energy: f64,
    smoothed_surprise: f64,
}

impl SyntheticPhotAdapter {
    pub fn new(start_t_us: u64) -> Self {
        Self {
            source: SyntheticPhotSource::new(start_t_us),
            smoothed_energy: 0.0,
            smoothed_surprise: 0.0,
        }
    }
}

impl ModalityAdapter for SyntheticPhotAdapter {
    fn modality_bit(&self) -> u8 {
        MOD_PHOT
    }
    fn writes_ambient(&self) -> bool {
        true
    }
    fn writes_slots(&self) -> bool {
        true
    }
    fn tick_hz(&self) -> u32 {
        30
    }
    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        let frame = self.source.sample(now_us);

        // Age out stale slots using the shared substrate rule.
        vp.sweep_slots(now_us);

        // Smooth energy/surprise — mirrors the VIB adapter's ambient shape so
        // observers and tests can compare the two modalities directly.
        let alpha = 0.2;
        self.smoothed_energy = (1.0 - alpha) * self.smoothed_energy + alpha * frame.event_energy;
        // Surprise here is a cheap proxy (no PHOT predictor exists yet): the
        // first-derivative magnitude of event_energy.
        let inst_surprise = (frame.event_energy - self.smoothed_energy).abs();
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;

        let is_event = self.smoothed_energy > EVENT_NOMINATION_THRESHOLD;

        vp.write_ambient_for(
            MOD_PHOT,
            self.smoothed_energy,
            self.smoothed_surprise,
            if is_event { Some(now_us) } else { None },
            is_event,
        );

        if !is_event {
            return;
        }

        let ambient = vp.read_ambient();
        // Phase 3C: adapter submits evidence; viewport decides the slot. PHOT
        // carries a spatial estimate each tick, so we attach it.
        let evidence = EventEvidence {
            modality_bit: MOD_PHOT,
            event_t_us: now_us,
            event_energy: self.smoothed_energy,
            surprise: self.smoothed_surprise,
            spatial: Some(SpatialEvidence {
                pos_x: frame.pos_x,
                pos_y: frame.pos_y,
                span_x: frame.span_x,
                span_y: frame.span_y,
                precision: frame.spatial_precision,
            }),
            nomination_seq_hint: ambient.nomination_seq,
        };
        let slot_id = match vp.claim_slot_for_event(&evidence) {
            SlotClaim::Reused(id) | SlotClaim::Allocated(id) => id,
            SlotClaim::Dropped => return,
        };

        // Phase 4C: use the slot-local predictor so predicted/err/precision
        // carry real values instead of placeholder zeros.
        let observed: Vec<f64> = frame.phot_bands.to_vec();
        vp.update_slot_phot_with_predictor(slot_id, now_us, &observed);
        vp.refresh_slot_fusion(slot_id, MOD_PHOT, now_us);
        vp.refresh_slot_high_layer(slot_id, now_us);

        // Spatial target: frame carries a live estimate this tick.
        vp.write_slot_spatial(
            slot_id,
            MOD_PHOT,
            now_us,
            frame.pos_x,
            frame.pos_y,
            frame.span_x,
            frame.span_y,
            frame.spatial_precision,
        );
    }
}

// ---------------------------------------------------------------------------
// Real PHOT adapter (Phase 4A).
//
// Consumes frames from a pluggable `CameraFrameProvider`, runs a multi-region
// motion extractor (up to PHOT_MAX_EVENTS_PER_FRAME regions per frame), and
// writes numeric-only evidence into the viewport. Never emits semantic
// labels, object classes, or identity.
//
// Rules:
//   * Touches only MOD_PHOT viewport fields and `phot_health`.
//   * Slot selection is *always* delegated to viewport.claim_slot_for_event.
//   * Startup failure and runtime errors are surfaced as FAULT, not swallowed.
// ---------------------------------------------------------------------------

pub struct RealPhotAdapter {
    provider: Box<dyn CameraFrameProvider>,
    extractor: MotionExtractor,
    smoothed_energy: f64,
    smoothed_surprise: f64,
    birth_sensitivity: Arc<BiasPriors>,
    faulted: bool,
    /// Wall-clock time when the provider was first confirmed healthy (0 = not yet set).
    started_t_us: u64,
    /// Wall-clock time of the most recent frame receipt (0 = none yet).
    last_frame_t_us: u64,
    _tick_hz: u32,
    /// Stable ID of the active consequence window (0 = none).
    /// Used to explicitly close the window when motion quiesces.
    consequence_window_id: u64,
    /// Previous frame's per-region brightness, for the motion sub-axis of the
    /// unified visual perception (region_perception). [0.0; VIS_REGIONS] until
    /// the first frame.
    prev_region_brightness: [f64; crate::VIS_REGIONS],
}

impl RealPhotAdapter {
    /// Build over any provider. The adapter takes ownership.
    pub fn new(provider: Box<dyn CameraFrameProvider>) -> Self {
        Self::with_birth_sensitivity(provider, Arc::new(BiasPriors::default()))
    }

    pub fn with_birth_sensitivity(
        provider: Box<dyn CameraFrameProvider>,
        birth_sensitivity: Arc<BiasPriors>,
    ) -> Self {
        let initial_fault = !provider.is_healthy();
        Self {
            provider,
            extractor: MotionExtractor::new(),
            smoothed_energy: 0.0,
            smoothed_surprise: 0.0,
            birth_sensitivity,
            faulted: initial_fault,
            started_t_us: 0,
            last_frame_t_us: 0,
            _tick_hz: 30,
            consequence_window_id: 0,
            prev_region_brightness: [0.0; crate::VIS_REGIONS],
        }
    }

    fn set_fault(&mut self, vp: &Arc<Viewport>) {
        self.faulted = true;
        self.extractor.reset();
        vp.set_phot_health(HealthState::Fault);
    }
}

impl ModalityAdapter for RealPhotAdapter {
    fn modality_bit(&self) -> u8 {
        MOD_PHOT
    }
    fn writes_ambient(&self) -> bool {
        true
    }
    fn writes_slots(&self) -> bool {
        true
    }
    fn tick_hz(&self) -> u32 {
        30
    }
    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        // Sweep regardless of frame arrival so stale slots age out on time.
        vp.sweep_slots(now_us);

        // Drain whatever the provider has this tick. We emit up to
        // PHOT_MAX_EVENTS_PER_FRAME events per processed frame; when multiple
        // frames arrive in a tick we enforce the multi-event budget per frame.
        if self.faulted {
            vp.set_phot_health(HealthState::Fault);
            // Still ping the provider: if its fault class changed it will
            // keep returning Err; we just re-affirm FAULT.
            let _ = self.provider.try_recv();
            return;
        }

        // Record startup time once on the first non-fault tick.
        if self.started_t_us == 0 {
            self.started_t_us = now_us;
            vp.set_phot_started_t_us(now_us);
        }

        let frame = match self.provider.try_recv() {
            Err(_fault) => {
                self.set_fault(vp);
                return;
            }
            Ok(None) => {
                // No frame this tick. Determine the staleness reference:
                // if we have seen at least one frame, measure from it;
                // otherwise measure from provider startup time. Either way,
                // stall > PHOT_STALE_WINDOW_US → STALE.
                let reference_t = if self.last_frame_t_us > 0 {
                    self.last_frame_t_us
                } else {
                    self.started_t_us
                };
                if now_us.saturating_sub(reference_t) > PHOT_STALE_WINDOW_US {
                    vp.set_phot_health(HealthState::Stale);
                }
                return;
            }
            Ok(Some(f)) => {
                self.last_frame_t_us = now_us;
                vp.set_phot_last_frame_t_us(now_us);
                f
            }
        };

        // Unified perception (2026-06-01 redesign): push this frame's visual
        // physical-quantity axes (per coarse region: brightness/R/G/B/motion)
        // into the SHARED observation axes — the SAME substrate hearing uses,
        // feeding the SAME abstraction engine and memory. Vision is no longer a
        // per-slot shadow. No use is presumed here; shape/identity emerge later.
        {
            let (vis_axes, new_brightness) =
                crate::phot_camera::region_perception(&frame, &self.prev_region_brightness);
            for (i, &v) in vis_axes.iter().enumerate() {
                vp.push_observation(crate::AXIS_VIS_BASE + i, now_us, v);
            }
            self.prev_region_brightness = new_brightness;
        }

        // Phase 1 (keystone ①): drive the Gaussian visual field from this same
        // frame. tick_gaussian_field runs one predictive-coding tick and mirrors
        // GAUSS_RECON_MSE / GAUSS_ACTIVE_FRAC onto the unified tier-0 axis stream.
        // Runs on every received frame, same tick as region_perception above.
        vp.tick_gaussian_field(&frame, now_us);

        // Multi-region extraction. The first frame returns an empty Vec (no
        // diff target yet).
        //
        // Distinguish two empty-Vec cases:
        //   (a) warm-up: has_prev was false before this call → first frame stored.
        //   (b) no above-threshold regions: has_prev was already true.
        // For (b) we still update smoothed_energy toward 0 so the health
        // state decays naturally.
        let had_prev = self.extractor.has_prev();
        let regions = self.extractor.extract(&frame);

        if regions.is_empty() {
            if !had_prev {
                // Warm-up frame: no diff target yet. Publish ambient QUIET and bail.
                vp.write_ambient_for(MOD_PHOT, 0.0, 0.0, None, false);
                vp.set_phot_health(HealthState::Quiet);
            } else {
                // Post-warmup frame with no above-threshold motion. Decay ambient.
                let alpha = 0.2_f64;
                self.smoothed_energy *= 1.0 - alpha;
                self.smoothed_surprise *= 1.0 - alpha;
                let is_event = self.smoothed_energy > EVENT_NOMINATION_THRESHOLD;
                vp.write_ambient_for(
                    MOD_PHOT,
                    self.smoothed_energy,
                    self.smoothed_surprise,
                    if is_event { Some(now_us) } else { None },
                    is_event,
                );
                vp.set_phot_health(if is_event {
                    HealthState::Ok
                } else {
                    HealthState::Quiet
                });
                // Motion quiescence: close the active consequence window explicitly.
                // If the window was already closed by quiescence accumulation, this
                // is a safe no-op (idempotent on phase).
                if self.consequence_window_id != 0 {
                    vp.close_action_consequence_window(self.consequence_window_id, now_us);
                    self.consequence_window_id = 0;
                }
            }
            return;
        }

        // Compute frame-level ambient from aggregate of all regions.
        // Use the sum of all region energies as the representative signal.
        let total_energy: f64 = regions.iter().map(|r| r.event_energy).sum();
        let frame_energy = total_energy;
        let alpha = 0.2_f64;
        self.smoothed_energy = (1.0 - alpha) * self.smoothed_energy + alpha * frame_energy;
        let base_inst_surprise = (frame_energy - self.smoothed_energy).abs();
        let max_span = regions
            .iter()
            .map(|r| r.span_x.max(r.span_y))
            .fold(0.0_f64, f64::max);
        let spread = if regions.len() > 1 {
            (regions.len() as f64 / crate::PHOT_MAX_EVENTS_PER_FRAME as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let pressure = compute_pressure(
            PressureSample {
                err: base_inst_surprise,
                deviation: spread,
                temporal_gap: max_span,
                cross_channel: if regions.len() > 1 { spread } else { 0.0 },
                ..Default::default()
            },
            &self.birth_sensitivity.pressure_weights,
        );
        let inst_surprise = base_inst_surprise * (1.0 + 0.25 * pressure);
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;

        let is_event = self.smoothed_energy > EVENT_NOMINATION_THRESHOLD;

        vp.write_ambient_for(
            MOD_PHOT,
            self.smoothed_energy,
            self.smoothed_surprise,
            if is_event { Some(now_us) } else { None },
            is_event,
        );
        vp.set_phot_health(if is_event {
            HealthState::Ok
        } else {
            HealthState::Quiet
        });

        if !is_event {
            return;
        }

        // Submit one claim per region, in deterministic order (already sorted by extractor).
        // Write spatial immediately after each claim so subsequent claims in this same
        // frame see the correct position and score low for distant regions.
        for region in &regions {
            let ambient = vp.read_ambient(); // fresh per region
            let ev = EventEvidence {
                modality_bit: MOD_PHOT,
                event_t_us: now_us,
                event_energy: region.event_energy,
                surprise: self.smoothed_surprise,
                spatial: Some(SpatialEvidence {
                    pos_x: region.pos_x,
                    pos_y: region.pos_y,
                    span_x: region.span_x,
                    span_y: region.span_y,
                    precision: region.spatial_precision,
                }),
                nomination_seq_hint: ambient.nomination_seq,
            };
            let slot_id = match vp.claim_slot_for_event(&ev) {
                SlotClaim::Reused(id) | SlotClaim::Allocated(id) => id,
                SlotClaim::Dropped => continue,
            };

            // Write spatial before the next claim so the scorer can distinguish
            // this slot from ones for different spatial regions in the same frame.
            vp.write_slot_spatial(
                slot_id,
                MOD_PHOT,
                now_us,
                region.pos_x,
                region.pos_y,
                region.span_x,
                region.span_y,
                region.spatial_precision,
            );

            let observed: Vec<f64> = region.phot_bands.to_vec();
            vp.update_slot_phot_with_predictor(slot_id, now_us, &observed);
            vp.refresh_slot_fusion(slot_id, MOD_PHOT, now_us);
            vp.refresh_slot_high_layer(slot_id, now_us);
        }

        // Wiring: record the PHOT ingress change as a consequence observation.
        // total_energy is already computed above. Extends an Open window or
        // begins a new one; no target, result, or meaning inference.
        // Track the stable window_id for explicit close on motion quiescence.
        self.consequence_window_id = vp.accumulate_ingress_change(now_us, total_energy);

        // Live temporal bridge: if a recent write epoch exists in the shared
        // substrate, try to auto-pair it with the just-formed/extended
        // consequence window. Engineering approximation only; no target,
        // result, or meaning inference. TEMPORAL_PAIR_MAX_DELAY_US is a
        // substrate knob (defined in lib.rs), not a fixed theoretical window.
        vp.try_auto_pair_latest_epoch_to_window(now_us);

        let _ = CameraFault::StartupFailed; // keep enum reachable for readers
    }
}

// ---------------------------------------------------------------------------
// Synthetic VIB adapter (Phase 3C — test and example use only).
// ---------------------------------------------------------------------------

/// Drives `ambient.vib` and `slot.vib` from a deterministic `SyntheticVibSource`.
/// Never wired into `bin/body`; the live microphone `VibAdapter` remains the
/// default. Tests and examples use this to create controlled sync/desync
/// scenarios against `SyntheticPhotAdapter`.
pub struct SyntheticVibAdapter {
    source: SyntheticVibSource,
    smoothed_energy: f64,
    smoothed_surprise: f64,
}

impl SyntheticVibAdapter {
    pub fn new(source: SyntheticVibSource) -> Self {
        Self {
            source,
            smoothed_energy: 0.0,
            smoothed_surprise: 0.0,
        }
    }
}

impl ModalityAdapter for SyntheticVibAdapter {
    fn modality_bit(&self) -> u8 {
        MOD_VIB
    }
    fn writes_ambient(&self) -> bool {
        true
    }
    fn writes_slots(&self) -> bool {
        true
    }
    fn tick_hz(&self) -> u32 {
        200
    }
    fn tick(&mut self, vp: &Arc<Viewport>, now_us: u64) {
        let frame = self.source.sample(now_us);

        for axis in 0..VIB_BANDS {
            vp.push_observation(axis, now_us, frame.bands[axis]);
        }
        vp.sweep_slots(now_us);

        let inst_energy: f64 = frame.bands.iter().map(|b| b * b).sum();
        let mut inst_surprise = 0.0;
        for axis in 0..VIB_BANDS {
            if let Some(e) = vp.latest_err(axis) {
                inst_surprise += e.abs();
            }
        }
        let alpha = 0.1;
        self.smoothed_energy = (1.0 - alpha) * self.smoothed_energy + alpha * inst_energy;
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;

        let is_event = self.smoothed_energy > EVENT_NOMINATION_THRESHOLD;

        vp.write_ambient_for(
            MOD_VIB,
            self.smoothed_energy,
            self.smoothed_surprise,
            if is_event { Some(now_us) } else { None },
            is_event,
        );
        if !is_event {
            return;
        }

        let ambient = vp.read_ambient();
        let evidence = EventEvidence {
            modality_bit: MOD_VIB,
            event_t_us: now_us,
            event_energy: self.smoothed_energy,
            surprise: self.smoothed_surprise,
            spatial: None,
            nomination_seq_hint: ambient.nomination_seq,
        };
        let slot_id = match vp.claim_slot_for_event(&evidence) {
            SlotClaim::Reused(id) | SlotClaim::Allocated(id) => id,
            SlotClaim::Dropped => return,
        };

        let predicted = [0.0f64; VIB_BANDS];
        let mut err = [0.0f64; VIB_BANDS];
        let mut surprise = [0.0f64; VIB_BANDS];
        for axis in 0..VIB_BANDS {
            let (_a, _b, s, last_err, _mse, _u) = vp.read_predictor(axis);
            surprise[axis] = s;
            err[axis] = last_err;
        }
        vp.write_slot_vib(slot_id, now_us, &frame.bands, &predicted, &err, &surprise);
        vp.refresh_slot_fusion(slot_id, MOD_VIB, now_us);
        vp.refresh_slot_high_layer(slot_id, now_us);
    }
}
