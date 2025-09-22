// Main compressed history module that re-exports public API
// This reduces compilation time by organizing code into focused modules

pub mod types;
pub mod config;
pub mod delta;
pub mod system;

// Re-export the main public API
pub use types::{LightSnapshot, LightBody, LightFoil, SimulationSnapshot};
pub use config::{CompressionConfig, ChangeThresholds, SimulationType};
pub use delta::{DeltaSnapshot, BodyDelta, FoilDelta, ReconstructionError};
pub use system::{CompressedHistorySystem, MemoryStats};

// Re-export the conversion traits
pub use types::{FromSnapshot, ToSnapshot};