use std::collections::VecDeque;

use crate::surface_projection::SurfaceFrame;

/// A single trail point with a normalized position and decay weight.
#[derive(Clone, Copy, Debug)]
pub struct TrailPoint {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
}

/// The trail history for a single object slot.
#[derive(Debug)]
pub struct SlotTrail {
    pub points: Vec<TrailPoint>,
}

/// An observer-side ring buffer tracking recent positions for one slot.
struct PositionRing {
    capacity: usize,
    positions: VecDeque<(f64, f64)>,
}

impl PositionRing {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            positions: VecDeque::with_capacity(capacity + 1),
        }
    }

    fn push(&mut self, x: f64, y: f64) {
        self.positions.push_back((x, y));
        while self.positions.len() > self.capacity {
            self.positions.pop_front();
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(f64, f64)> {
        self.positions.iter()
    }

    fn len(&self) -> usize {
        self.positions.len()
    }
}

/// Observer-side short history for visual surface trails.
///
/// Maintains a bounded ring of recent projected positions per object.
/// History never writes back to snapshot/body.
pub struct SurfaceHistory {
    capacity: usize,
    rings: Vec<PositionRing>,
}

impl SurfaceHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            rings: Vec::new(),
        }
    }

    /// Update history with a new surface frame.
    /// Each object in the frame appends its center to its ring.
    /// Rings are matched by position (index) in the frame.
    pub fn update(&mut self, frame: &SurfaceFrame) {
        // Ensure we have enough rings for this frame's objects
        while self.rings.len() < frame.objects.len() {
            self.rings.push(PositionRing::new(self.capacity));
        }

        // Append new positions
        for (i, obj) in frame.objects.iter().enumerate() {
            self.rings[i].push(obj.center_x, obj.center_y);
        }
    }

    /// Render trails as decaying trail segments.
    pub fn render_trails(&self) -> Vec<SlotTrail> {
        let mut result = Vec::with_capacity(self.rings.len());

        for ring in &self.rings {
            let points: Vec<TrailPoint> = ring
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| {
                    let alpha = compute_decay_alpha(i, ring.len());
                    TrailPoint { x, y, alpha }
                })
                .collect();

            result.push(SlotTrail { points });
        }

        result
    }
}

fn compute_decay_alpha(index: usize, total: usize) -> f64 {
    if total <= 1 {
        return 1.0;
    }
    let ratio = index as f64 / (total - 1) as f64;
    (1.0 - ratio).clamp(0.0, 1.0)
}
