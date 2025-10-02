//! Manual measurement system for interactive recording during simulation
//! 
//! This module provides a flexible measurement system where users can:
//! - Define measurement points manually in the GUI
//! - Save/load measurement configurations
//! - Automatically record measurements to CSV at specified intervals
//! - Control when recording starts/stops manually

use crate::body::{Body, Species};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualMeasurementPoint {
    /// X-coordinate of measurement center (Angstroms)
    pub x: f32,
    /// Y-coordinate of measurement center (Angstroms)
    pub y: f32,
    /// Vertical height of measurement region (Angstroms)
    pub height: f32,
    /// Horizontal width of measurement region (Angstroms)
    pub width: f32,
    /// Direction to measure: "left", "right", "up", "down"
    pub direction: String,
    /// Label for this measurement point
    pub label: String,
}

impl Default for ManualMeasurementPoint {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            height: 50.0,
            width: 70.0,
            direction: "left".to_string(),
            label: "Measurement_1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualMeasurementConfig {
    /// Name of this measurement configuration
    pub name: String,
    /// Measurement points
    pub points: Vec<ManualMeasurementPoint>,
    /// Interval between measurements in femtoseconds
    pub interval_fs: f32,
    /// Output CSV file name (relative to doe_results/)
    pub output_file: String,
}

impl Default for ManualMeasurementConfig {
    fn default() -> Self {
        Self {
            name: "Manual Measurement".to_string(),
            points: vec![ManualMeasurementPoint::default()],
            interval_fs: 1000.0,
            output_file: "manual_measurements.csv".to_string(),
        }
    }
}

impl ManualMeasurementConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementResult {
    pub label: String,
    pub edge_position: f32,
    pub li_metal_count: usize,
    pub li_ion_count: usize,
}

pub struct ManualMeasurementRecorder {
    config: ManualMeasurementConfig,
    is_recording: bool,
    last_measurement_time: f32,
    csv_file: Option<File>,
    measurement_count: usize,
}

impl ManualMeasurementRecorder {
    pub fn new(config: ManualMeasurementConfig) -> Self {
        Self {
            config,
            is_recording: false,
            last_measurement_time: -999999.0,
            csv_file: None,
            measurement_count: 0,
        }
    }

    pub fn config(&self) -> &ManualMeasurementConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ManualMeasurementConfig {
        &mut self.config
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn measurement_count(&self) -> usize {
        self.measurement_count
    }

    /// Start recording measurements to CSV
    pub fn start_recording(&mut self, simulation_time_fs: f32) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_recording {
            return Ok(());
        }

        // Create doe_results directory if it doesn't exist
        std::fs::create_dir_all("doe_results")?;

        // Open CSV file
        let path = format!("doe_results/{}", self.config.output_file);
        let mut file = File::create(&path)?;

        // Write header
        write!(file, "frame,time_fs")?;
        for point in &self.config.points {
            write!(file, ",{}_edge,{}_li_metal,{}_li_ion", 
                point.label, point.label, point.label)?;
        }
        writeln!(file)?;

        self.csv_file = Some(file);
        self.is_recording = true;
        self.last_measurement_time = simulation_time_fs - self.config.interval_fs; // Ensure first measurement happens
        self.measurement_count = 0;

        println!("✓ Started recording measurements to: {}", path);
        Ok(())
    }

    /// Stop recording measurements
    pub fn stop_recording(&mut self) {
        if !self.is_recording {
            return;
        }

        self.csv_file = None;
        self.is_recording = false;
        println!("✓ Stopped recording. Total measurements: {}", self.measurement_count);
    }

    /// Update and potentially record measurements
    pub fn update(&mut self, bodies: &[Body], frame: usize, simulation_time_fs: f32) -> Vec<MeasurementResult> {
        let mut results = Vec::new();

        // Check if it's time to measure
        let should_measure = simulation_time_fs - self.last_measurement_time >= self.config.interval_fs;

        if !should_measure && !self.is_recording {
            return results;
        }

        if should_measure {
            self.last_measurement_time = simulation_time_fs;

            // Perform measurements at each point
            for point in &self.config.points {
                let result = self.measure_at_point(bodies, point);
                results.push(result);
            }

            // Write to CSV if recording
            if self.is_recording {
                if let Some(file) = &mut self.csv_file {
                    let _ = write!(file, "{},{}", frame, simulation_time_fs);
                    for result in &results {
                        let _ = write!(file, ",{},{},{}", 
                            result.edge_position, 
                            result.li_metal_count, 
                            result.li_ion_count);
                    }
                    let _ = writeln!(file);
                    let _ = file.flush();
                    self.measurement_count += 1;
                }
            }
        }

        results
    }

    /// Measure at a single point
    fn measure_at_point(&self, bodies: &[Body], point: &ManualMeasurementPoint) -> MeasurementResult {
        // Define measurement region bounds
        let half_width = point.width / 2.0;
        let half_height = point.height / 2.0;

        // Measurement box is always centered at (x, y)
        let x_min = point.x - half_width;
        let x_max = point.x + half_width;
        let y_min = point.y - half_height;
        let y_max = point.y + half_height;

        // Collect particles in region
        let mut li_metal_positions = Vec::new();
        let mut li_ion_count = 0;

        for body in bodies {
            let pos = body.pos;
            if pos.x >= x_min && pos.x <= x_max && pos.y >= y_min && pos.y <= y_max {
                match body.species {
                    Species::LithiumMetal => {
                        li_metal_positions.push(pos);
                    }
                    Species::LithiumIon => {
                        li_ion_count += 1;
                    }
                    _ => {}
                }
            }
        }

        // Find leading edge (furthest in measurement direction)
        let edge_position = if li_metal_positions.is_empty() {
            point.x // No metal found, return starting position
        } else {
            match point.direction.as_str() {
                "left" => li_metal_positions.iter()
                    .map(|p| p.x)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.x),
                "right" => li_metal_positions.iter()
                    .map(|p| p.x)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.x),
                "up" => li_metal_positions.iter()
                    .map(|p| p.y)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.y),
                "down" => li_metal_positions.iter()
                    .map(|p| p.y)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(point.y),
                _ => point.x,
            }
        };

        MeasurementResult {
            label: point.label.clone(),
            edge_position,
            li_metal_count: li_metal_positions.len(),
            li_ion_count,
        }
    }

    /// Get the last measurement results without waiting for interval
    pub fn get_current_measurements(&self, bodies: &[Body]) -> Vec<MeasurementResult> {
        self.config.points
            .iter()
            .map(|point| self.measure_at_point(bodies, point))
            .collect()
    }
}
