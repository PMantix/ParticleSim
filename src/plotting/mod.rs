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
    #[allow(dead_code)]
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
                    SamplingMode::SingleTimestep => {
                        // For single timestep, update if manually triggered (last_update = 0) or if it's been a while
                        window.last_update == 0.0 || (current_time - window.last_update) > 0.1
                    },
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
                            Self::update_concentration_map_static(window, bodies, current_time, self.bounds);
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
        // Use analysis functions for better modularity
        match window.config.quantity {
            Quantity::Charge => {
                let charges = analysis::calculate_charge_distribution(bodies, is_x_axis, bounds, window.config.spatial_bins);
                window.data.x_data.clear();
                window.data.y_data.clear();
                
                let bin_size = (2.0 * bounds) / window.config.spatial_bins as f32;
                
                for (i, &charge) in charges.iter().enumerate() {
                    let x_pos = -bounds + (i as f32 + 0.5) * bin_size;
                    window.data.x_data.push(x_pos as f64);
                    window.data.y_data.push(charge as f64);
                }
            }
            Quantity::Velocity => {
                let (positions, velocities) = analysis::calculate_velocity_profile(bodies, is_x_axis, bounds, window.config.spatial_bins);
                window.data.x_data.clear();
                window.data.y_data.clear();
                
                for (i, &velocity) in velocities.iter().enumerate() {
                    window.data.x_data.push(positions[i] as f64);
                    window.data.y_data.push(velocity as f64);
                }
            }
            _ => {
                // Fallback to original binning approach for other quantities
                let bins = window.config.spatial_bins;
                let mut bin_values = vec![0.0; bins];
                let mut bin_counts = vec![0; bins];
                let bin_size = (2.0 * bounds) / bins as f32;

                for body in bodies {
                    let position = if is_x_axis { body.pos.x } else { body.pos.y };
                    // Fix binning calculation with proper bounds checking
                    let normalized_pos = (position + bounds) / (2.0 * bounds);
                    let bin_idx_f = normalized_pos * bins as f32;
                    
                    // Clamp to valid range and convert to usize
                    if bin_idx_f >= 0.0 && bin_idx_f < bins as f32 {
                        let bin_idx = bin_idx_f.floor() as usize;
                        if bin_idx < bins {
                            let value = match window.config.quantity {
                                Quantity::ElectronCount => body.electrons.len() as f32,
                                Quantity::SpeciesConcentration(species) => {
                                    if body.species == species { 1.0 } else { 0.0 }
                                }
                                Quantity::TotalSpeciesCount(species) => {
                                    if body.species == species { 1.0 } else { 0.0 }
                                }
                                _ => 0.0,
                            };
                            
                            bin_values[bin_idx] += value;
                            bin_counts[bin_idx] += 1;
                        }
                    }
                }

                // Calculate appropriate values for display
                window.data.x_data.clear();
                window.data.y_data.clear();
                
                for i in 0..bins {
                    let x_pos = -bounds + (i as f32 + 0.5) * bin_size;
                    let y_val = match window.config.quantity {
                        Quantity::SpeciesConcentration(_) => {
                            // For concentration, we want density (count per unit area)
                            let cell_area = bin_size * bin_size; // Approximate cell area
                            bin_values[i] / cell_area
                        }
                        Quantity::TotalSpeciesCount(_) => {
                            // For species count, show total count in each bin
                            bin_values[i]
                        }
                        _ => {
                            // For other quantities, show mean value
                            if bin_counts[i] > 0 {
                                bin_values[i] / bin_counts[i] as f32
                            } else {
                                0.0
                            }
                        }
                    };
                    
                    window.data.x_data.push(x_pos as f64);
                    window.data.y_data.push(y_val as f64);
                }
            }
        }
        
        // Handle timestamps for spatial profiles
        if matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            window.data.timestamps.clear();
        }
        window.data.timestamps.push(current_time as f64);
    }

    fn update_time_series_static(window: &mut PlotWindow, bodies: &[Body], foils: &[Foil], current_time: f32) {
        let value = match window.config.quantity {
            Quantity::TotalSpeciesCount(species) => {
                // Use analysis function for species populations
                let populations = analysis::calculate_species_populations(bodies);
                populations.get(&species).copied().unwrap_or(0) as f32
            }
            Quantity::FoilCurrent(foil_id) => {
                foils.iter()
                    .find(|f| f.id == foil_id)
                    .map(|f| f.current)
                    .unwrap_or(0.0)
            }
            Quantity::ElectronHopRate => {
                // Use analysis function for electron hop rate
                analysis::calculate_electron_hop_rate(bodies, 0.016) // Assume ~60fps timestep
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
                if bodies.is_empty() { 0.0 } else { total / bodies.len() as f32 }
            }
        };

        // For single timestep mode, clear existing data before adding new point
        if matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            window.data.x_data.clear();
            window.data.y_data.clear();
            window.data.timestamps.clear();
        }

        window.data.x_data.push(current_time as f64);
        window.data.y_data.push(value as f64);
        window.data.timestamps.push(current_time as f64);

        // Limit data to time window (only for continuous modes)
        if !matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            let cutoff_time = current_time - window.config.time_window;
            while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
                window.data.x_data.remove(0);
                window.data.y_data.remove(0);
                window.data.timestamps.remove(0);
            }
        }
    }

    fn update_species_population_static(window: &mut PlotWindow, bodies: &[Body], current_time: f32) {
        let count = match window.config.quantity {
            Quantity::TotalSpeciesCount(species) => {
                // Use analysis function for better modularity
                let populations = analysis::calculate_species_populations(bodies);
                populations.get(&species).copied().unwrap_or(0) as f32
            }
            _ => 0.0,
        };

        // For single timestep mode, clear existing data before adding new point
        if matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            window.data.x_data.clear();
            window.data.y_data.clear();
            window.data.timestamps.clear();
        }

        window.data.x_data.push(current_time as f64);
        window.data.y_data.push(count as f64);
        window.data.timestamps.push(current_time as f64);

        // Limit data to time window (only for continuous modes)
        if !matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            let cutoff_time = current_time - window.config.time_window;
            while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
                window.data.x_data.remove(0);
                window.data.y_data.remove(0);
                window.data.timestamps.remove(0);
            }
        }
    }

    fn update_current_analysis_static(window: &mut PlotWindow, bodies: &[Body], foils: &[Foil], current_time: f32) {
        // Use analysis functions for current analysis
        let current_analysis = analysis::calculate_current_analysis(foils, bodies, 0.016); // Assume ~60fps timestep
        
        if let Quantity::FoilCurrent(foil_id) = window.config.quantity {
            if let Some((command_current, _actual_current)) = current_analysis.get(&foil_id) {
                // For single timestep mode, clear existing data before adding new point
                if matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
                    window.data.x_data.clear();
                    window.data.y_data.clear();
                    window.data.timestamps.clear();
                }
                
                // Store command current (main data)
                window.data.x_data.push(current_time as f64);
                window.data.y_data.push(*command_current as f64);
                window.data.timestamps.push(current_time as f64);
                
                // Add metadata about actual current
                window.data.metadata.insert(
                    format!("actual_current_{}", current_time),
                    _actual_current.to_string()
                );
                
                // Limit data to time window (only for continuous modes)
                if !matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
                    let cutoff_time = current_time - window.config.time_window;
                    while !window.data.x_data.is_empty() && window.data.x_data[0] < cutoff_time as f64 {
                        window.data.x_data.remove(0);
                        window.data.y_data.remove(0);
                        window.data.timestamps.remove(0);
                    }
                }
            }
        }
    }

    fn update_concentration_map_static(window: &mut PlotWindow, bodies: &[Body], current_time: f32, bounds: f32) {
        if let Quantity::SpeciesConcentration(species) = window.config.quantity {
            // Use analysis function to calculate 2D concentration map
            let grid_size = window.config.spatial_bins; // Use spatial_bins as grid size
            let concentration_grid = analysis::calculate_concentration_map(bodies, species, bounds, grid_size);
            
            // For visualization, we'll flatten the 2D grid into 1D data
            // X-axis represents grid position, Y-axis represents concentration
            window.data.x_data.clear();
            window.data.y_data.clear();
            
            // Convert 2D grid to line plot data (sum along Y-axis for each X position)
            for x in 0..grid_size {
                let x_pos = -bounds + (x as f32 + 0.5) * (2.0 * bounds) / grid_size as f32;
                let y_sum: f32 = (0..grid_size).map(|y| concentration_grid[y][x]).sum();
                
                window.data.x_data.push(x_pos as f64);
                window.data.y_data.push(y_sum as f64);
            }
            
            // Store the full 2D grid in metadata for potential future use
            let grid_json = serde_json::to_string(&concentration_grid).unwrap_or_default();
            window.data.metadata.insert(
                format!("concentration_grid_{}", current_time),
                grid_json
            );
        }
        
        // Handle timestamps for concentration maps
        if matches!(window.config.sampling_mode, SamplingMode::SingleTimestep) {
            window.data.timestamps.clear();
        }
        window.data.timestamps.push(current_time as f64);
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
