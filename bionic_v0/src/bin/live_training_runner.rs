use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Args {
    runtime_dir: PathBuf,
    solitary_secs: u64,
    live_secs: u64,
    sample_interval_secs: u64,
}

#[derive(Debug)]
struct Bins {
    body: PathBuf,
    report: PathBuf,
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("live_training_runner_error={err}");
            std::process::exit(2);
        }
    }
}

fn run() -> std::io::Result<i32> {
    let args = parse_args(std::env::args().collect());
    let bins = resolve_bins()?;
    std::fs::create_dir_all(&args.runtime_dir)?;
    std::fs::create_dir_all(args.runtime_dir.join("runner_samples"))?;

    let fixture_fire = run_report(&bins.report, "--fire-live-closure", None)?;
    let fixture_fire_gate = field_u8(&fixture_fire, "fire_live_closure_gate_pass");
    let fixture_deterministic = field_u8(&fixture_fire, "deterministic_fixture");
    let fixture_runtime_read = field_u8(&fixture_fire, "runtime_snapshot_read");

    let total_secs = args.solitary_secs.saturating_add(args.live_secs);
    if total_secs > 0 {
        run_body_and_sample(&args, &bins, total_secs)?;
    }

    let body_stdout = args.runtime_dir.join("body.stdout");
    let body_stderr = args.runtime_dir.join("body.stderr");
    let body_silent = file_is_empty(&body_stdout) && file_is_empty(&body_stderr);
    let snapshot_exists = args.runtime_dir.join("snapshot.bin").exists();

    println!("mode=live_training_runner");
    println!("root={}", args.runtime_dir.display());
    println!("solitary_secs={}", args.solitary_secs);
    println!("live_secs={}", args.live_secs);
    println!("sample_interval_secs={}", args.sample_interval_secs);
    println!("snapshot_exists={}", u8::from(snapshot_exists));
    println!(
        "body_output_status={}",
        if body_silent { "silent" } else { "not_silent" }
    );
    println!("fixture_deterministic_fixture={fixture_deterministic}");
    println!("fixture_runtime_snapshot_read={fixture_runtime_read}");
    println!("fixture_fire_live_closure_gate_pass={fixture_fire_gate}");

    if !body_silent {
        return Ok(1);
    }

    let prelingual = run_report(
        &bins.report,
        "--pre-language-solitary-growth",
        Some(&args.runtime_dir),
    )?;
    let nursery = run_report(
        &bins.report,
        "--live-human-nursery",
        Some(&args.runtime_dir),
    )?;
    let fire = run_report(&bins.report, "--fire-live-closure", Some(&args.runtime_dir))?;

    print_final_summary(&prelingual, &nursery, &fire);

    let prelingual_pass = field_u8(&prelingual, "prelingual_growth_gate_pass") == 1;
    let live_io_pass = field_u8(&nursery, "phot_structure_observed") == 1
        && field_u32(&nursery, "vocal_write_epochs") >= 1
        && field_u32(&nursery, "return_windows") >= 1
        && field_u8(&nursery, "temporal_pairing_observed") == 1;
    let nursery_turn_pass = field_u8(&nursery, "closure_or_reentry_observed") == 1
        && field_u8(&fire, "fire_live_closure_gate_pass") == 1;
    let mark_text_allowed = prelingual_pass && live_io_pass && nursery_turn_pass;

    println!("solitary_gate_pass={}", u8::from(prelingual_pass));
    println!("live_io_gate_pass={}", u8::from(live_io_pass));
    println!("nursery_turn_gate_pass={}", u8::from(nursery_turn_pass));
    println!(
        "mark_text_grounding_allowed={}",
        u8::from(mark_text_allowed)
    );
    println!("overall_gate_pass={}", u8::from(mark_text_allowed));

    Ok(0)
}

fn parse_args(args: Vec<String>) -> Args {
    let mut runtime_dir: Option<PathBuf> = None;
    let mut solitary_secs = 1_800;
    let mut live_secs = 600;
    let mut sample_interval_secs = 30;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--solitary-secs" => {
                if let Some(value) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    solitary_secs = value;
                }
                i += 2;
            }
            "--live-secs" => {
                if let Some(value) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    live_secs = value;
                }
                i += 2;
            }
            "--sample-interval-secs" => {
                if let Some(value) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    sample_interval_secs = value;
                }
                i += 2;
            }
            value => {
                if runtime_dir.is_none() {
                    runtime_dir = Some(PathBuf::from(value));
                }
                i += 1;
            }
        }
    }

    Args {
        runtime_dir: runtime_dir.unwrap_or_else(|| PathBuf::from("runtime")),
        solitary_secs,
        live_secs,
        sample_interval_secs: sample_interval_secs.max(1),
    }
}

fn resolve_bins() -> std::io::Result<Bins> {
    Ok(Bins {
        body: env_bin("BIONIC_BODY_BIN", "body")?,
        report: env_bin("BIONIC_CURRICULUM_REPORT_BIN", "curriculum_report")?,
    })
}

fn env_bin(env_key: &str, sibling_name: &str) -> std::io::Result<PathBuf> {
    if let Ok(path) = std::env::var(env_key) {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe()?;
    Ok(exe
        .parent()
        .map(|dir| dir.join(sibling_name))
        .unwrap_or_else(|| PathBuf::from(sibling_name)))
}

fn run_body_and_sample(args: &Args, bins: &Bins, total_secs: u64) -> std::io::Result<()> {
    let stdout = File::create(args.runtime_dir.join("body.stdout"))?;
    let stderr = File::create(args.runtime_dir.join("body.stderr"))?;
    let mut body = Command::new(&bins.body)
        .arg(&args.runtime_dir)
        .arg(total_secs.to_string())
        .arg("--solitary-low-power")
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;

    let started = Instant::now();
    let mut sample_idx = 0_u32;
    loop {
        if body.try_wait()?.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_secs(args.sample_interval_secs));
        if args.runtime_dir.join("snapshot.bin").exists() {
            sample_idx = sample_idx.saturating_add(1);
            write_sample_reports(args, bins, sample_idx)?;
        }
        if started.elapsed() > Duration::from_secs(total_secs.saturating_add(5)) {
            break;
        }
    }

    let _ = body.wait()?;
    if args.runtime_dir.join("snapshot.bin").exists() {
        write_sample_reports(args, bins, sample_idx.saturating_add(1))?;
    }
    Ok(())
}

fn write_sample_reports(args: &Args, bins: &Bins, idx: u32) -> std::io::Result<()> {
    let sample_dir = args.runtime_dir.join("runner_samples");
    for (name, flag) in [
        (
            "pre_language_solitary_growth",
            "--pre-language-solitary-growth",
        ),
        ("live_human_nursery", "--live-human-nursery"),
        ("fire_live_closure", "--fire-live-closure"),
    ] {
        let output = Command::new(&bins.report)
            .arg(flag)
            .arg(&args.runtime_dir)
            .output()?;
        std::fs::write(
            sample_dir.join(format!("{idx:04}_{name}.txt")),
            output.stdout,
        )?;
    }
    Ok(())
}

fn run_report(
    report_bin: &Path,
    flag: &str,
    runtime_dir: Option<&Path>,
) -> std::io::Result<BTreeMap<String, String>> {
    let mut cmd = Command::new(report_bin);
    cmd.arg(flag);
    if let Some(runtime_dir) = runtime_dir {
        cmd.arg(runtime_dir);
    }
    let output = cmd.output()?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("report_failed flag={flag}"),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_key_values(&stdout))
}

fn parse_key_values(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn print_final_summary(
    prelingual: &BTreeMap<String, String>,
    nursery: &BTreeMap<String, String>,
    fire: &BTreeMap<String, String>,
) {
    println!(
        "prelingual_runtime_snapshot_read={}",
        field_u8(prelingual, "runtime_snapshot_read")
    );
    println!(
        "prelingual_growth_gate_pass={}",
        field_u8(prelingual, "prelingual_growth_gate_pass")
    );
    println!(
        "stable_recurrence={:.6}",
        field_f64(prelingual, "stable_recurrence")
    );
    println!(
        "room_familiarity={:.6}",
        field_f64(prelingual, "room_familiarity")
    );
    println!(
        "solitary_growth_score={:.6}",
        field_f64(prelingual, "solitary_growth_score")
    );
    println!(
        "phot_structure_observed={}",
        field_u8(nursery, "phot_structure_observed")
    );
    println!(
        "vocal_write_epochs={}",
        field_u32(nursery, "vocal_write_epochs")
    );
    println!("return_windows={}", field_u32(nursery, "return_windows"));
    println!(
        "temporal_pairing_observed={}",
        field_u8(nursery, "temporal_pairing_observed")
    );
    println!(
        "closure_or_reentry_observed={}",
        field_u8(nursery, "closure_or_reentry_observed")
    );
    println!(
        "communication_candidate={}",
        field_u8(nursery, "communication_candidate")
    );
    println!(
        "fire_live_closure_gate_pass={}",
        field_u8(fire, "fire_live_closure_gate_pass")
    );
}

fn field_u8(map: &BTreeMap<String, String>, key: &str) -> u8 {
    map.get(key).and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn field_u32(map: &BTreeMap<String, String>, key: &str) -> u32 {
    map.get(key).and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn field_f64(map: &BTreeMap<String, String>, key: &str) -> f64 {
    map.get(key).and_then(|s| s.parse().ok()).unwrap_or(0.0)
}

fn file_is_empty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.len() == 0)
        .unwrap_or(true)
}
