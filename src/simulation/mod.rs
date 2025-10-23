// simulation/mod.rs
// Re-exports and module declarations for simulation submodules

//pub mod core;
pub mod collision;
mod electron_hopping;
pub mod forces;
pub mod simulation;
pub mod thermal;
pub use simulation::*;
pub mod history;
pub use history::PlaybackProgress;
pub mod compressed_history;
pub mod utils;
pub use utils::compute_temperature;
pub mod out_of_plane;

#[cfg(test)]
// Module declarations removed dead test files
#[cfg(test)]
mod out_of_plane_tests;
