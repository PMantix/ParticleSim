mod body;
mod partition;
mod quadtree;
mod cell_list;
mod renderer;
mod simulation;
mod utils;
mod config;
mod profiler;
mod io;
mod species;
mod plotting;
mod init_config;
mod diagnostics;
mod app;

#[cfg(feature = "profiling")]
use once_cell::sync::Lazy;
#[cfg(feature = "profiling")]
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> = Lazy::new(|| {
    Mutex::new(profiler::Profiler::new())
});

fn main() {
    app::run();
}
