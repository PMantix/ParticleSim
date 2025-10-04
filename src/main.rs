mod body;
mod partition;
mod quadtree;
mod cell_list;
mod renderer;
mod renderer_utils;
mod simulation;
mod utils;
mod config;
mod profiler;
mod io;
mod species;
mod plotting;
mod init_config;
mod diagnostics;
mod commands;
mod app;
mod units;
mod scenario;
mod switch_charging;
mod doe;
mod manual_measurement;
mod manual_measurement_filename;

#[cfg(feature = "profiling")]
use once_cell::sync::Lazy;
#[cfg(feature = "profiling")]
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> = Lazy::new(|| {
    Mutex::new(profiler::Profiler::new())
});

// Updated main
fn main() {
    app::run();
}
