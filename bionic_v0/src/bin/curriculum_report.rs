use bionic_v0::curriculum::{
    run_curriculum_episodes, run_persistent_curriculum_session, CurriculumConfig,
};

fn parse_args() -> (u32, bool) {
    let mut args = std::env::args().skip(1);
    let mut episodes = 3;
    let mut persistent = false;
    while let Some(arg) = args.next() {
        if arg == "--episodes" {
            if let Some(value) = args.next().and_then(|s| s.parse::<u32>().ok()) {
                episodes = value.max(1);
            }
        } else if arg == "--persistent" {
            persistent = true;
        }
    }
    (episodes, persistent)
}

fn main() -> std::io::Result<()> {
    let (episodes, persistent) = parse_args();

    println!("curriculum_report_v0");
    if persistent {
        let trend = run_persistent_curriculum_session(CurriculumConfig::default(), episodes)?;
        println!("mode=persistent");
        println!("episodes_run={}", trend.episodes_run);
        println!("courses_run={}", trend.courses_run);
        println!("total_steps_run={}", trend.total_steps_run);
        println!("first_worldline_err={:.6}", trend.first_worldline_err);
        println!("last_worldline_err={:.6}", trend.last_worldline_err);
        println!("mean_worldline_err={:.6}", trend.mean_worldline_err);
        println!("total_pairs_formed={}", trend.total_pairs_formed);
        println!("final_pairs={}", trend.final_pairs);
        println!("final_learned_events={}", trend.final_learned_events);
        println!("max_learned_abs_delta={:.6}", trend.max_learned_abs_delta);
    } else {
        let trend = run_curriculum_episodes(CurriculumConfig::default(), episodes)?;
        println!("mode=fresh");
        println!("episodes_run={}", trend.episodes_run);
        println!("courses_run={}", trend.courses_run);
        println!("total_steps_run={}", trend.total_steps_run);
        println!("first_worldline_err={:.6}", trend.first_worldline_err);
        println!("last_worldline_err={:.6}", trend.last_worldline_err);
        println!("mean_worldline_err={:.6}", trend.mean_worldline_err);
        println!("total_pairs_formed={}", trend.total_pairs_formed);
        println!("total_learned_events={}", trend.total_learned_events);
        println!("max_learned_abs_delta={:.6}", trend.max_learned_abs_delta);
    }

    Ok(())
}
