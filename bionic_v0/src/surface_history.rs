use std::collections::{BTreeMap, VecDeque};

use crate::surface_projection::SurfaceFrame;

#[derive(Clone, Copy, Debug)]
pub struct TrailPoint {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
}

#[derive(Debug)]
pub struct SlotTrail {
    pub slot_id: usize,
    pub points: Vec<TrailPoint>,
}

#[derive(Debug)]
struct PositionRing {
    capacity: usize,
    positions: VecDeque<(f64, f64)>,
    missing_frames: usize,
}

impl PositionRing {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            positions: VecDeque::with_capacity(capacity + 1),
            missing_frames: 0,
        }
    }

    fn push(&mut self, x: f64, y: f64) {
        self.positions.push_back((x, y));
        self.missing_frames = 0;
        while self.positions.len() > self.capacity {
            self.positions.pop_front();
        }
    }

    fn mark_missing(&mut self) {
        self.missing_frames = self.missing_frames.saturating_add(1);
    }

    fn decay_factor(&self, stale_after: usize) -> f64 {
        if self.missing_frames == 0 {
            return 1.0;
        }
        if stale_after == 0 {
            return 0.0;
        }
        (1.0 - self.missing_frames as f64 / stale_after as f64).clamp(0.0, 1.0)
    }
}

/// Observer-side short history for visual surface trails.
///
/// Maintains a bounded ring of recent projected positions keyed by stable slot id.
/// History never writes back to snapshot/body.
pub struct SurfaceHistory {
    capacity: usize,
    stale_after: usize,
    rings: BTreeMap<usize, PositionRing>,
}

impl SurfaceHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            stale_after: capacity.max(1),
            rings: BTreeMap::new(),
        }
    }

    pub fn update(&mut self, frame: &SurfaceFrame) {
        for ring in self.rings.values_mut() {
            ring.mark_missing();
        }

        for obj in &frame.objects {
            self.rings
                .entry(obj.slot_id)
                .or_insert_with(|| PositionRing::new(self.capacity))
                .push(obj.center_x, obj.center_y);
        }

        let stale_after = self.stale_after;
        self.rings
            .retain(|_, ring| !ring.positions.is_empty() && ring.missing_frames < stale_after);
    }

    pub fn render_trails(&self) -> Vec<SlotTrail> {
        let mut result = Vec::with_capacity(self.rings.len());

        for (&slot_id, ring) in &self.rings {
            let decay = ring.decay_factor(self.stale_after);
            if decay <= 0.0 {
                continue;
            }

            let points: Vec<TrailPoint> = ring
                .positions
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| TrailPoint {
                    x,
                    y,
                    alpha: compute_decay_alpha(i, ring.positions.len()) * decay,
                })
                .collect();

            if !points.is_empty() {
                result.push(SlotTrail { slot_id, points });
            }
        }

        result
    }
}

fn compute_decay_alpha(index: usize, total: usize) -> f64 {
    if total <= 1 {
        return 1.0;
    }
    let ratio = index as f64 / (total - 1) as f64;
    ratio.clamp(0.0, 1.0)
}
