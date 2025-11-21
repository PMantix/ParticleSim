pub mod body;
pub mod cell_list;
pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod doe;
pub mod init_config;
pub mod io;
pub mod manual_measurement;
pub mod measurement_csv;
pub mod partition;
pub mod plotting;
pub mod profiler;
pub mod quadtree;
pub mod renderer;
pub mod renderer_utils;
pub mod scenario;
pub mod simulation;
pub mod species;
pub mod switch_charging;
pub mod units;
pub mod utils;

pub mod app;

#[cfg(feature = "profiling")]
use once_cell::sync::Lazy;
#[cfg(feature = "profiling")]
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> =
    Lazy::new(|| Mutex::new(profiler::Profiler::new()));
