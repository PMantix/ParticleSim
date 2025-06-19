// simulation/mod.rs
// Re-exports and module declarations for simulation submodules

//pub mod core;
pub mod forces;
pub mod collision;
pub mod simulation;
pub use simulation::*;
pub mod utils;
pub mod foil_link;
pub use foil_link::{FoilLink, FoilLinkType};

#[cfg(test)]
mod tests;

