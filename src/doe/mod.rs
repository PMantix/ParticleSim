/// Design of Experiments (DOE) module for automated parametric studies
/// 
/// This module provides functionality to:
/// - Define experimental test cases with varying parameters
/// - Run simulations headlessly without GUI interaction
/// - Automatically perform measurements at specified locations
/// - Export results for statistical analysis

pub mod config;
pub mod runner;
pub mod measurement;
pub mod export;

pub use config::{DoeConfig, TestCase, ChargingMode, MeasurementPoint};
pub use runner::DoeRunner;
pub use export::export_results_to_csv;
