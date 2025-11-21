mod app;
mod body;
mod cell_list;
mod commands;
mod config;
mod diagnostics;
mod doe;
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

// Updated main
fn main() {
    app::run();
}
