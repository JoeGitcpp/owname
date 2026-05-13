use std::path::PathBuf;

use bionic_v0::evolution_priors::BiasAxis;
use bionic_v0::learned_bias::{read_learned_bias_checkpoint, BiasLocator};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        std::process::exit(2);
    }

    let path = PathBuf::from(&args[1]);
    let Some(axis) = parse_axis(&args[2]) else {
        std::process::exit(2);
    };
    let Some(locator) = parse_locator(&args[3], &args[4]) else {
        std::process::exit(2);
    };

    let Ok(state) = read_learned_bias_checkpoint(&path) else {
        std::process::exit(1);
    };
    let Some(curve) = state
        .curves
        .iter()
        .find(|curve| curve.axis == axis && curve.locator == locator)
    else {
        std::process::exit(1);
    };

    println!("axis={}", axis_label(curve.axis));
    println!("locator={}", locator_label(curve.locator));
    println!("delta_value={:.6}", curve.delta_value);
    println!("delta_ema={:.6}", curve.delta_ema);
    println!("stability={:.6}", curve.stability);
    println!("support_count={}", curve.support_count);
    println!("deviation_count={}", curve.deviation_count);
    println!("last_update_t_us={}", curve.last_update_t_us);
    println!(
        "effective_scalar={:.6}",
        state.effective_scalar(axis, locator, 1.0, 0.25, 2.0)
    );
    println!(
        "ema_effective_scalar={:.6}",
        (1.0 + curve.delta_ema * curve.stability).clamp(0.25, 2.0)
    );
    println!("total_events={}", state.total_events);
}

fn parse_axis(raw: &str) -> Option<BiasAxis> {
    match raw {
        "ErrGain" => Some(BiasAxis::ErrGain),
        "RecurrenceGain" => Some(BiasAxis::RecurrenceGain),
        "AmbiguityGain" => Some(BiasAxis::AmbiguityGain),
        "AlignmentGain" => Some(BiasAxis::AlignmentGain),
        "SupportGain" => Some(BiasAxis::SupportGain),
        "DeviationGain" => Some(BiasAxis::DeviationGain),
        "TemporalGapGain" => Some(BiasAxis::TemporalGapGain),
        "CrossChannelGain" => Some(BiasAxis::CrossChannelGain),
        "PressureMultiplier" => Some(BiasAxis::PressureMultiplier),
        _ => None,
    }
}

fn parse_locator(kind: &str, raw_value: &str) -> Option<BiasLocator> {
    let value = raw_value.parse::<u16>().ok()?;
    match kind {
        "Global" => Some(BiasLocator::Global),
        "Band" => Some(BiasLocator::Band(value)),
        "Slot" => Some(BiasLocator::Slot(value)),
        "Edge" => Some(BiasLocator::Edge(value)),
        "Channel" => Some(BiasLocator::Channel(value)),
        _ => None,
    }
}

fn axis_label(axis: BiasAxis) -> &'static str {
    match axis {
        BiasAxis::ErrGain => "ErrGain",
        BiasAxis::RecurrenceGain => "RecurrenceGain",
        BiasAxis::AmbiguityGain => "AmbiguityGain",
        BiasAxis::AlignmentGain => "AlignmentGain",
        BiasAxis::SupportGain => "SupportGain",
        BiasAxis::DeviationGain => "DeviationGain",
        BiasAxis::TemporalGapGain => "TemporalGapGain",
        BiasAxis::CrossChannelGain => "CrossChannelGain",
        BiasAxis::PressureMultiplier => "PressureMultiplier",
    }
}

fn locator_label(locator: BiasLocator) -> String {
    match locator {
        BiasLocator::Global => String::from("Global"),
        BiasLocator::Band(value) => format!("Band({value})"),
        BiasLocator::Slot(value) => format!("Slot({value})"),
        BiasLocator::Edge(value) => format!("Edge({value})"),
        BiasLocator::Channel(value) => format!("Channel({value})"),
    }
}
