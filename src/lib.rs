pub mod body;
pub mod partition;
pub mod quadtree;
pub mod cell_list;
pub mod renderer;
pub mod renderer_utils;
pub mod simulation;
pub mod utils;
pub mod config;
pub mod units;
pub mod profiler;
pub mod io;
pub mod species;
pub mod plotting;
pub mod init_config;
pub mod diagnostics;
pub mod commands;
pub mod scenario;
pub mod switch_charging;
pub mod doe;
pub mod manual_measurement;

pub mod app;

#[cfg(feature = "profiling")]
use once_cell::sync::Lazy;
#[cfg(feature = "profiling")]
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> = Lazy::new(|| {
    Mutex::new(profiler::Profiler::new())
});
