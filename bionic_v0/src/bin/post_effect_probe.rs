use std::env;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use bionic_v0::learned_bias::LearnedBiasCheckpointPaths;
use bionic_v0::post_effect::{analyze_sessions, summarize_repeats};

const SEED_BYTES: &[u8] = b"\x11\x29\x41\x59\x11\x29\x41\x59\n\x03\x08\x13\x21\x03\x08\x13\x21\n";
const ALT_SEED_BYTES: &[u8] =
    b"\x07\x1f\x37\x53\x07\x1f\x37\x53\n\x02\x05\x0d\x22\x02\x05\x0d\x22\n";

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let result = match args.get(1).map(String::as_str) {
        Some("analyze") => analyze_mode(&args[2..]),
        Some("analyze-repeat") => analyze_repeat_mode(&args[2..]),
        Some("run") => run_mode(&args[2..]),
        Some("run-hold") => run_hold_mode(&args[2..]),
        Some("dose") => dose_mode(&args[2..]),
        Some("generalize") => generalize_mode(&args[2..]),
        _ => Err(Error::from(ErrorKind::InvalidInput)),
    };

    if result.is_err() {
        std::process::exit(1);
    }
}

fn analyze_mode(args: &[String]) -> std::io::Result<()> {
    if args.len() != 3 {
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    let report = analyze_sessions(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
    )?;
    for line in report.lines() {
        println!("{line}");
    }
    Ok(())
}

fn analyze_repeat_mode(args: &[String]) -> std::io::Result<()> {
    if args.is_empty() {
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let reports = args
        .iter()
        .map(|raw| {
            let root = Path::new(raw);
            analyze_sessions(
                &root.join("cold").join("session.csv"),
                &root.join("train").join("session.csv"),
                &root.join("after").join("session.csv"),
            )
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let repeat = summarize_repeats(&reports);
    for line in repeat.lines() {
        println!("{line}");
    }
    Ok(())
}

fn run_mode(args: &[String]) -> std::io::Result<()> {
    if args.len() < 4 {
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let root = PathBuf::from(&args[0]);
    let cold_secs = parse_secs(&args[1])?;
    let train_secs = parse_secs(&args[2])?;
    let after_secs = parse_secs(&args[3])?;
    let poll_hz = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);

    let bins = RuntimeBins::discover()?;
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;

    let cold = root.join("cold");
    let train = root.join("train");
    let after = root.join("after");

    run_live_stage(&bins, &cold, cold_secs, poll_hz)?;
    run_live_stage(&bins, &train, train_secs, poll_hz)?;
    copy_checkpoint(&train, &after)?;
    run_live_stage(&bins, &after, after_secs, poll_hz)?;

    let report = analyze_sessions(
        &cold.join("session.csv"),
        &train.join("session.csv"),
        &after.join("session.csv"),
    )?;

    println!("root={}", root.display());
    println!("cold_dir={}", cold.display());
    println!("train_dir={}", train.display());
    println!("after_dir={}", after.display());
    println!("body_output_status=silent");
    for line in report.lines() {
        println!("{line}");
    }

    Ok(())
}

fn run_hold_mode(args: &[String]) -> std::io::Result<()> {
    if args.len() < 5 {
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let root = PathBuf::from(&args[0]);
    let cold_secs = parse_secs(&args[1])?;
    let train_secs = parse_secs(&args[2])?;
    let hold_secs = parse_secs(&args[3])?;
    let after_secs = parse_secs(&args[4])?;
    let poll_hz = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(5);

    let bins = RuntimeBins::discover()?;
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;

    let cold = root.join("cold");
    let train = root.join("train");
    let hold = root.join("hold");
    let after = root.join("after");

    run_live_stage(&bins, &cold, cold_secs, poll_hz)?;
    run_live_stage(&bins, &train, train_secs, poll_hz)?;
    copy_checkpoint(&train, &hold)?;
    run_idle_stage(&bins, &hold, hold_secs, poll_hz)?;
    copy_checkpoint(&hold, &after)?;
    run_live_stage(&bins, &after, after_secs, poll_hz)?;

    let report = analyze_sessions(
        &cold.join("session.csv"),
        &train.join("session.csv"),
        &after.join("session.csv"),
    )?;
    let hold_persistence_visible =
        hold_checkpoint_events(&hold)? >= report.train_learned_event_last;

    println!("root={}", root.display());
    println!("cold_dir={}", cold.display());
    println!("train_dir={}", train.display());
    println!("hold_dir={}", hold.display());
    println!("after_dir={}", after.display());
    println!("body_output_status=silent");
    println!("hold_output_status=silent");
    println!("hold_persistence_visible={hold_persistence_visible}");
    for line in report.lines() {
        println!("{line}");
    }

    Ok(())
}

fn dose_mode(args: &[String]) -> std::io::Result<()> {
    if args.len() < 5 {
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let root = PathBuf::from(&args[0]);
    let cold_secs = parse_secs(&args[1])?;
    let after_secs = parse_secs(&args[2])?;
    let poll_hz = args[3].parse().unwrap_or(5);
    let train_secs = args[4..]
        .iter()
        .map(|raw| parse_secs(raw))
        .collect::<std::io::Result<Vec<_>>>()?;

    let bins = RuntimeBins::discover()?;
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;

    println!("dose_root={}", root.display());
    for train_sec in train_secs {
        let stage_root = root.join(format!("train-{train_sec}"));
        run_closed_loop(
            &bins,
            &stage_root,
            cold_secs,
            train_sec,
            after_secs,
            poll_hz,
        )?;
        let report = analyze_sessions(
            &stage_root.join("cold").join("session.csv"),
            &stage_root.join("train").join("session.csv"),
            &stage_root.join("after").join("session.csv"),
        )?;
        println!("dose_train_secs={train_sec}");
        for line in report.lines() {
            if line.starts_with("effect_status=")
                || line.starts_with("after_vs_cold_pair_rate_delta_pct=")
                || line.starts_with("after_vs_cold_egress_deviation_avg_delta=")
                || line.starts_with("after_vs_cold_pair_similarity_avg_delta=")
            {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn generalize_mode(args: &[String]) -> std::io::Result<()> {
    if args.len() < 4 {
        return Err(Error::from(ErrorKind::InvalidInput));
    }

    let root = PathBuf::from(&args[0]);
    let cold_secs = parse_secs(&args[1])?;
    let train_secs = parse_secs(&args[2])?;
    let after_secs = parse_secs(&args[3])?;
    let poll_hz = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);

    let bins = RuntimeBins::discover()?;
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;

    let cold = root.join("cold");
    let train = root.join("train");
    let after = root.join("after");

    run_live_stage_with_seed(&bins, &cold, cold_secs, poll_hz, SEED_BYTES)?;
    run_live_stage_with_seed(&bins, &train, train_secs, poll_hz, SEED_BYTES)?;
    copy_checkpoint(&train, &after)?;
    run_live_stage_with_seed(&bins, &after, after_secs, poll_hz, ALT_SEED_BYTES)?;

    let report = analyze_sessions(
        &cold.join("session.csv"),
        &train.join("session.csv"),
        &after.join("session.csv"),
    )?;

    println!("root={}", root.display());
    println!("generalize_after_disturbance=alternate");
    for line in report.lines() {
        println!("{line}");
    }

    Ok(())
}

fn run_closed_loop(
    bins: &RuntimeBins,
    root: &Path,
    cold_secs: u64,
    train_secs: u64,
    after_secs: u64,
    poll_hz: u64,
) -> std::io::Result<()> {
    let cold = root.join("cold");
    let train = root.join("train");
    let after = root.join("after");

    run_live_stage(bins, &cold, cold_secs, poll_hz)?;
    run_live_stage(bins, &train, train_secs, poll_hz)?;
    copy_checkpoint(&train, &after)?;
    run_live_stage(bins, &after, after_secs, poll_hz)
}

fn run_live_stage(
    bins: &RuntimeBins,
    stage_dir: &Path,
    duration_secs: u64,
    poll_hz: u64,
) -> std::io::Result<()> {
    run_live_stage_with_seed(bins, stage_dir, duration_secs, poll_hz, SEED_BYTES)
}

fn run_live_stage_with_seed(
    bins: &RuntimeBins,
    stage_dir: &Path,
    duration_secs: u64,
    poll_hz: u64,
    seed_bytes: &[u8],
) -> std::io::Result<()> {
    std::fs::create_dir_all(stage_dir.join("live_bytes"))?;

    let body = Command::new(&bins.body)
        .arg(stage_dir)
        .arg(duration_secs.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    std::thread::sleep(Duration::from_millis(200));
    append_seed(stage_dir, seed_bytes)?;

    let loopback = Command::new(&bins.loopback)
        .arg(stage_dir)
        .arg(duration_secs.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let max_polls = duration_secs.saturating_mul(poll_hz.max(1)) + poll_hz.max(1);
    let observer = Command::new(&bins.observer)
        .arg(stage_dir)
        .arg(poll_hz.max(1).to_string())
        .arg(max_polls.to_string())
        .arg("compact")
        .arg(stage_dir.join("session.csv"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let body_out = body.wait_with_output()?;
    let loopback_out = loopback.wait_with_output()?;

    if !body_out.status.success()
        || !body_out.stdout.is_empty()
        || !body_out.stderr.is_empty()
        || !loopback_out.status.success()
        || !loopback_out.stdout.is_empty()
        || !loopback_out.stderr.is_empty()
        || !observer.status.success()
    {
        return Err(Error::from(ErrorKind::Other));
    }

    Ok(())
}

fn run_idle_stage(
    bins: &RuntimeBins,
    stage_dir: &Path,
    duration_secs: u64,
    poll_hz: u64,
) -> std::io::Result<()> {
    std::fs::create_dir_all(stage_dir.join("live_bytes"))?;

    let body = Command::new(&bins.body)
        .arg(stage_dir)
        .arg(duration_secs.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    std::thread::sleep(Duration::from_millis(200));

    let max_polls = duration_secs.saturating_mul(poll_hz.max(1)) + poll_hz.max(1);
    let observer = Command::new(&bins.observer)
        .arg(stage_dir)
        .arg(poll_hz.max(1).to_string())
        .arg(max_polls.to_string())
        .arg("compact")
        .arg(stage_dir.join("session.csv"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let body_out = body.wait_with_output()?;

    if !body_out.status.success()
        || !body_out.stdout.is_empty()
        || !body_out.stderr.is_empty()
        || !observer.status.success()
    {
        return Err(Error::from(ErrorKind::Other));
    }

    Ok(())
}

fn append_seed(stage_dir: &Path, seed_bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(stage_dir.join("live_bytes").join("in.frames"))?;
    file.write_all(seed_bytes)
}

fn copy_checkpoint(src_dir: &Path, dst_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst_dir)?;
    let src_paths = LearnedBiasCheckpointPaths::new(src_dir);
    let dst_paths = LearnedBiasCheckpointPaths::new(dst_dir);
    if src_paths.final_path.exists() {
        std::fs::copy(&src_paths.final_path, &dst_paths.final_path)?;
    }
    Ok(())
}

fn hold_checkpoint_events(hold_dir: &Path) -> std::io::Result<u64> {
    let paths = LearnedBiasCheckpointPaths::new(hold_dir);
    bionic_v0::learned_bias::read_learned_bias_checkpoint(&paths.final_path)
        .map(|state| state.total_events)
}

fn parse_secs(raw: &str) -> std::io::Result<u64> {
    raw.parse::<u64>()
        .map(|v| v.max(1))
        .map_err(|_| Error::from(ErrorKind::InvalidInput))
}

struct RuntimeBins {
    body: PathBuf,
    loopback: PathBuf,
    observer: PathBuf,
}

impl RuntimeBins {
    fn discover() -> std::io::Result<Self> {
        let current = env::current_exe()?;
        let dir = current
            .parent()
            .ok_or_else(|| Error::from(ErrorKind::NotFound))?;
        Ok(Self {
            body: env_path("BIONIC_BODY_BIN").unwrap_or_else(|| dir.join("body")),
            loopback: env_path("BIONIC_LOOPBACK_BIN").unwrap_or_else(|| dir.join("byte_loopback")),
            observer: env_path("BIONIC_OBSERVER_BIN").unwrap_or_else(|| dir.join("observer")),
        })
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}
