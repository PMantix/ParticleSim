/// Design of Experiments (DOE) module for automated parametric studies
/// 
/// This module provides functionality to:
/// - Define experimental test cases with varying parameters
/// - Run simulations headlessly without GUI interaction
/// - Automatically perform measurements at specified locations
/// - Export results for statistical analysis

#[cfg(feature = "doe")]
pub mod config;
#[cfg(feature = "doe")]
pub mod runner;
#[cfg(feature = "doe")]
pub mod measurement;
#[cfg(feature = "doe")]
pub mod export;

