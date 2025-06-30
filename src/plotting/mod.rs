// plotting/mod.rs
// Data analysis and plotting module for the particle simulation

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub mod analysis;
pub mod export;
pub mod gui;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlotType {
    SpatialProfileX,     // Mean quantity vs X position
    SpatialProfileY,     // Mean quantity vs Y position
    TimeSeries,          // Quantity vs time
    ConcentrationMap,    // 2D concentration heatmap
    ChargeDistribution,  // Charge vs position/time
    SpeciesPopulation,   // Species counts vs time
    CurrentAnalysis,     // Command vs actual current
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Quantity {
    Charge,
    ElectronCount,
    Velocity,
    SpeciesConcentration(Species),
    TotalSpeciesCount(Species),
    FoilCurrent(u64), // foil_id
    ElectronHopRate,
    LocalFieldStrength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotConfig {
    pub plot_type: PlotType,
    pub quantity: Quantity,
    pub title: String,
    pub sampling_mode: SamplingMode,
    pub spatial_bins: usize,
    pub time_window: f32, // seconds
    pub update_frequency: f32, // Hz
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SamplingMode {
    SingleTimestep,
    TimeAveraged { window: f32 },
    Continuous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotData {
    pub config: PlotConfig,
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub timestamps: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PlotWindow {
    pub id: String,
    pub config: PlotConfig,
    pub data: PlotData,
    pub is_open: bool,
    pub last_update: f32,
}

pub struct PlottingSystem {
    pub windows: HashMap<String, PlotWindow>,
    pub next_window_id: usize,
    pub bounds: f32,
}

impl PlottingSystem {
    pub fn new(bounds: f32) -> Self {
        Self {
            windows: HashMap::new(),
            next_window_id: 0,
            bounds,
        }
    }

    pub fn create_plot_window(&mut self, config: PlotConfig) -> String {
        let window_id = format!("plot_{}", self.next_window_id);
        self.next_window_id += 1;

        let window = PlotWindow {
            id: window_id.clone(),
            config: config.clone(),
            data: PlotData {
                config,
                x_data: Vec::new(),
                y_data: Vec::new(),
                timestamps: Vec::new(),
                metadata: HashMap::new(),
            },
            is_open: true,
            last_update: 0.0,
        };

        self.windows.insert(window_id.clone(), window);
        window_id
    }

    pub fn update_plots(&mut self, bodies: &[Body], foils: &[Foil], current_time: f32) {
        let window_ids: Vec<String> = self.windows.keys().cloned().collect();
        
        for window_id in window_ids {
            if let Some(window) = self.windows.get_mut(&window_id) {
                if !window.is_open {
                    continue;
                }

                let should_update = match window.config.sampling_mode {
                    SamplingMode::SingleTimestep => false, // Only update on manual trigger
                    SamplingMode::Continuous => {
                        current_time - window.last_update >= 1.0 / window.config.update_frequency
                    }
                    SamplingMode::TimeAveraged { .. } => {
                        current_time - window.last_update >= 1.0 / window.config.update_frequency
                    }
                };

                if should_update {
                    // Update data based on plot type
                    match window.config.plot_type {
                        PlotType::SpatialProfileX => {
                            Self::update_spatial_profile_static(window, bodies, current_time, true, self.bounds);
                        }
                        PlotType::SpatialProfileY => {
                            Self::update_spatial_profile_static(window, bodies, current_time, false, self.bounds);
                        }
                        PlotType::TimeSeries => {
                            Self::update_time_series_static(window, bodies, foils, current_time);
                        }
                        PlotType::ConcentrationMap => {
                            // TODO: Implement 2D concentration map
                        }
                        PlotType::ChargeDistribution => {
                            Self::update_spatial_profile_static(window, bodies, current_time, true, self.bounds);
                        }
                        PlotType::SpeciesPopulation => {
                            Self::update_species_population_static(window, bodies, current_time);
                        }
                        PlotType::CurrentAnalysis => {
                            Self::update_current_analysis_static(window, bodies, foils, current_time);
                        }
                    }
                    window.last_update = current_time;
                }
            }
        }
    }

    fn update_spatial_profile_static(window: &mut PlotWindow, bodies: &[Body], current_time: f32, is_x_axis: bool, bounds: f32) {
        let bins = window.config.spatial_bins;
        let mut bin_values = vec![0.0; bins];
        let mut bin_counts = vec![0; bins];

        for body in bodies {
            let position = if is_x_axis { body.pos.x } else { body.pos.y };
            let bin_idx = ((position + bounds) / (2.0 * bounds) * bins as f32) as usize;
            
            if bin_idx < bins {
                let value = match window.config.quantity {
                    Quantity::Charge => body.charge,
                    Quantity::ElectronCount => body.electrons.len() as f32,
                    Quantity::Velocity => if is_x_axis { body.vel.x } else { body.vel.y },
                    Quantity::SpeciesConcentration(species) => {
                        if body.species == species { 1.0 } else { 0.0 }
                    }
                    _ => 0.0,
                };
                
                bin_values[bin_idx] += value;
                bin_counts[bin_idx] += 1;
            }
        }

        // Calculate mean values
        window.data.x_data.clear();
        window.data.y_data.clear();
        
        for i in 0..bins {
            let x_pos = -bounds + (i as f32 + 0.5) * (2.0 * bounds) / bins as f32;
            let y_val = if bin_counts[i] > 0 {
                bin_values[i] / bin_counts[i] as f32
            } else {
                0.0
            };
            
            window.data.x_data.push(x_pos as f64);
            window.data.y_data.push(y_val as f64);
        }
        
        window.data.timestamps.push(current_time as f64);
    }

    fn update_time_series_static(window: &mut PlotWindow, bodies: &[Body], foils: &[Foil], current_time: f32) {
        let value = match window.config.quantity {
            Quantity::TotalSpeciesCount(species) => {
                bodies.iter().filter(|b| b.species == species).count() as f32
            }
            Quantity::FoilCurrent(foil_id) => {
                foils.iter()
                    .find(|f| f.id == foil_id)
                    .map(|f| f.current)
                    .unwrap_or(0.0)
            }
            Quantity::ElectronHopRate => {
                // TODO: Track electron hop events
                0.0
            }
            _ => {
                // Calculate aggregate values
                let total: f32 = bodies.iter().map(|body| {
                    match window.config.quantity {
                        Quantity::Charge => body.charge,
                        Quantity::ElectronCount => body.electrons.len() as f32,
                        _ => 0.0,
                    }
                }).sum();
                total / bodies.len().max(1) as f32
            }
        };

        window.data.x_data.push(current_time as f64);
        window.data.y_data.push(value as f64);
        window.data.timestamps.push(current_time as f64);

        // Limit data to time window
        let cutoff_time = current_time - window.config.time_window;
        while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
            window.data.x_data.remove(0);
            window.data.y_data.remove(0);
            window.data.timestamps.remove(0);
        }
    }

    fn update_species_population_static(window: &mut PlotWindow, bodies: &[Body], current_time: f32) {
        let count = match window.config.quantity {
            Quantity::TotalSpeciesCount(species) => {
                bodies.iter().filter(|b| b.species == species).count() as f32
            }
            _ => 0.0,
        };

        window.data.x_data.push(current_time as f64);
        window.data.y_data.push(count as f64);
        window.data.timestamps.push(current_time as f64);

        // Limit data to time window
        let cutoff_time = current_time - window.config.time_window;
        while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
            window.data.x_data.remove(0);
            window.data.y_data.remove(0);
            window.data.timestamps.remove(0);
        }
    }

    fn update_current_analysis_static(window: &mut PlotWindow, _bodies: &[Body], foils: &[Foil], current_time: f32) {
        // TODO: Implement current analysis - compare command current vs actual electron flow
        let _command_current: f32 = foils.iter().map(|f| f.current).sum();
        let _actual_current = 0.0; // Would need to track electron movements
        
        // For now, just track command current
        if let Quantity::FoilCurrent(foil_id) = window.config.quantity {
            if let Some(foil) = foils.iter().find(|f| f.id == foil_id) {
                window.data.x_data.push(current_time as f64);
                window.data.y_data.push(foil.current as f64);
                window.data.timestamps.push(current_time as f64);
            }
        }
    }

    // Legacy methods - keeping for backward compatibility
    fn update_charge_distribution(&self, window: &mut PlotWindow, bodies: &[Body], current_time: f32) {
        // Similar to spatial profile but for charge specifically
        Self::update_spatial_profile_static(window, bodies, current_time, true, self.bounds);
    }

    fn update_species_population(&self, window: &mut PlotWindow, bodies: &[Body], current_time: f32) {
        let count = match window.config.quantity {
            Quantity::TotalSpeciesCount(species) => {
                bodies.iter().filter(|b| b.species == species).count() as f32
            }
            _ => 0.0,
        };

        window.data.x_data.push(current_time as f64);
        window.data.y_data.push(count as f64);
        window.data.timestamps.push(current_time as f64);

        // Limit data to time window
        let cutoff_time = current_time - window.config.time_window;
        while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
            window.data.x_data.remove(0);
            window.data.y_data.remove(0);
            window.data.timestamps.remove(0);
        }
    }

    fn update_current_analysis(&self, window: &mut PlotWindow, _bodies: &[Body], foils: &[Foil], current_time: f32) {
        // TODO: Implement current analysis - compare command current vs actual electron flow
        let _command_current: f32 = foils.iter().map(|f| f.current).sum();
        let _actual_current = 0.0; // Would need to track electron movements
        
        // For now, just track command current
        if let Quantity::FoilCurrent(foil_id) = window.config.quantity {
            if let Some(foil) = foils.iter().find(|f| f.id == foil_id) {
                window.data.x_data.push(current_time as f64);
                window.data.y_data.push(foil.current as f64);
                window.data.timestamps.push(current_time as f64);
            }
        }
    }

    pub fn remove_window(&mut self, window_id: &str) {
        self.windows.remove(window_id);
    }

    pub fn export_data(&self, window_id: &str, format: ExportFormat) -> Result<String, String> {
        if let Some(window) = self.windows.get(window_id) {
            export::export_plot_data(&window.data, format)
        } else {
            Err("Window not found".to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    CSV,
    JSON,
    TSV,
}
