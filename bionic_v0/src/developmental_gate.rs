use crate::AXIS_COUNT;

pub const DEV_GATE_N_FLOOR: usize = 32;
const DEV_GATE_SIGMOID_K: f64 = 20.0;
const DEV_GATE_SIGMOID_M0: f64 = 0.3;

const PRIORITY_1_MAX: usize = 16 + 16 + 5; // VIB(16) + VIS brightness+motion per region(16) + INTERO core 5 = 37, clamped by N_FLOOR logic
const PRIORITY_2_THRESH: f64 = 0.15;
const PRIORITY_3_THRESH: f64 = 0.45;
const PRIORITY_4_THRESH: f64 = 0.70;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AxisPriority {
    Neonatal = 1,
    Early    = 2,
    Mid      = 3,
    Late     = 4,
}

pub fn axis_priority(axis: usize) -> AxisPriority {
    use crate::*;
    if axis < VIB_BANDS {
        return AxisPriority::Neonatal;
    }
    if axis >= AXIS_VIS_BASE && axis < AXIS_VIS_BASE + VIS_AXES {
        let region = (axis - AXIS_VIS_BASE) / VIS_AXES_PER_REGION;
        let sub = (axis - AXIS_VIS_BASE) % VIS_AXES_PER_REGION;
        return match sub {
            vis_sub::BRIGHTNESS | vis_sub::MOTION => AxisPriority::Neonatal,
            vis_sub::EDGE_ENERGY | vis_sub::TEMPORAL_FLUX => AxisPriority::Early,
            vis_sub::R | vis_sub::G | vis_sub::B | vis_sub::SATURATION => AxisPriority::Mid,
            _ => AxisPriority::Mid,
        };
    }
    if axis >= AXIS_INTERO_BASE && axis < AXIS_INTERO_BASE + INTERO_AXES {
        let offset = axis - AXIS_INTERO_BASE;
        return match offset {
            0..=4 => AxisPriority::Neonatal, // reserve, anxiety, pain, coherence, agency
            5..=9 => AxisPriority::Early,    // attachment, gap, closure, learning, presence
            _ => AxisPriority::Mid,          // graph density, fast path, intention, post-hoc
        };
    }
    AxisPriority::Late
}

fn priority_maturity_threshold(p: AxisPriority) -> f64 {
    match p {
        AxisPriority::Neonatal => 0.0,
        AxisPriority::Early    => PRIORITY_2_THRESH,
        AxisPriority::Mid      => PRIORITY_3_THRESH,
        AxisPriority::Late     => PRIORITY_4_THRESH,
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

pub fn axis_gain(maturity: f64, axis: usize) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    let p = axis_priority(axis);
    let m_thresh = priority_maturity_threshold(p);
    if m < m_thresh {
        return 0.0;
    }
    let x = DEV_GATE_SIGMOID_K * (m - m_thresh - 0.05);
    sigmoid(x).clamp(0.0, 1.0)
}

pub fn effective_mask(maturity: f64) -> [f64; AXIS_COUNT] {
    let mut mask = [0.0f64; AXIS_COUNT];
    for i in 0..AXIS_COUNT {
        mask[i] = axis_gain(maturity, i);
    }
    mask
}

pub fn effective_axis_count(maturity: f64) -> usize {
    let mut count = 0usize;
    for i in 0..AXIS_COUNT {
        if axis_gain(maturity, i) > 0.01 {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neonatal_opens_floor_axes() {
        let mask = effective_mask(0.0);
        let open = mask.iter().filter(|&&g| g > 0.01).count();
        assert!(open >= DEV_GATE_N_FLOOR, "neonatal open={open}, expected >= {DEV_GATE_N_FLOOR}");
    }

    #[test]
    fn mature_opens_all() {
        let mask = effective_mask(1.0);
        let open = mask.iter().filter(|&&g| g > 0.99).count();
        assert_eq!(open, AXIS_COUNT, "mature should open all {AXIS_COUNT} axes, got {open}");
    }

    #[test]
    fn monotonic_opening() {
        let steps = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let counts: Vec<usize> = steps.iter().map(|&m| effective_axis_count(m)).collect();
        for w in counts.windows(2) {
            assert!(w[1] >= w[0], "axis count should be monotonically non-decreasing: {counts:?}");
        }
    }

    #[test]
    fn vib_always_neonatal() {
        for axis in 0..crate::VIB_BANDS {
            assert_eq!(axis_priority(axis), AxisPriority::Neonatal);
        }
    }

    #[test]
    fn brightness_motion_neonatal() {
        for region in 0..crate::VIS_REGIONS {
            assert_eq!(axis_priority(crate::vis_axis(region, crate::vis_sub::BRIGHTNESS)), AxisPriority::Neonatal);
            assert_eq!(axis_priority(crate::vis_axis(region, crate::vis_sub::MOTION)), AxisPriority::Neonatal);
        }
    }

    #[test]
    fn color_axes_mid_priority() {
        for region in 0..crate::VIS_REGIONS {
            assert_eq!(axis_priority(crate::vis_axis(region, crate::vis_sub::R)), AxisPriority::Mid);
            assert_eq!(axis_priority(crate::vis_axis(region, crate::vis_sub::G)), AxisPriority::Mid);
            assert_eq!(axis_priority(crate::vis_axis(region, crate::vis_sub::B)), AxisPriority::Mid);
        }
    }

    #[test]
    fn code_axes_late() {
        for axis in crate::AXIS_CODE_BASE..crate::AXIS_CODE_BASE + crate::CODE_AXES {
            assert_eq!(axis_priority(axis), AxisPriority::Late);
        }
    }
}
