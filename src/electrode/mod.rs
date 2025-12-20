// electrode/mod.rs
// Module for intercalation electrode materials (graphite, LFP, NMC, etc.)
//
// This module handles:
// - Active material regions with state-of-charge tracking
// - Intercalation/deintercalation reactions
// - Material-specific properties (OCV curves, kinetics)
// - Desolvation barriers at electrode surfaces

pub mod material;
pub mod region;
pub mod intercalation;

pub use material::*;
pub use region::*;
