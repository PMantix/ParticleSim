mod app;
mod body;
mod cell_list;
mod commands;
mod config;
mod diagnostics;
mod doe;
mod electrode;
mod init_config;
mod io;
mod manual_measurement;
mod measurement_csv;
mod partition;
mod plotting;
mod profiler;
mod quadtree;
mod renderer;
mod renderer_utils;
mod scenario;
mod simulation;
mod species;
mod switch_charging;
mod units;
mod utils;

#[cfg(feature = "profiling")]
use once_cell::sync::Lazy;
#[cfg(feature = "profiling")]
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> =
    Lazy::new(|| Mutex::new(profiler::Profiler::new()));

fn main() {
    parse_cli_args();
    app::run();
}

fn parse_cli_args() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                let path = args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--scenario requires a path argument");
                    print_usage_and_exit();
                });
                *scenario::SCENARIO_PATH.lock() = Some(path);
            }
            "--help" | "-h" => print_usage_and_exit(),
            other => {
                eprintln!("Unknown argument: {}", other);
                print_usage_and_exit();
            }
        }
        i += 1;
    }
}

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: particle_sim [--scenario <path.toml>]");
    eprintln!();
    eprintln!("  --scenario <path>   Override the default init_config.toml.");
    eprintln!("                      Useful for measurement_configs/*.toml.");
    std::process::exit(2);
}
