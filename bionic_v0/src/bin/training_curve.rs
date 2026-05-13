use std::env;

use bionic_v0::training_curve::summarize_paths;

fn main() {
    let paths = env::args().skip(1).collect::<Vec<_>>();
    if paths.is_empty() {
        std::process::exit(2);
    }

    match summarize_paths(&paths) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(_) => std::process::exit(1),
    }
}
