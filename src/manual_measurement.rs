//! Manual measurement system for interactive recording during simulation
//! 
//! This module provides a flexible measurement system where users can:
//! - Define measurement points manually in the GUI
//! - Save/load measurement configurations
//! - Automatically record measurements to CSV at specified intervals
//! - Control when recording starts/stops manually

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use crate::quadtree::Quadtree;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::species::get_species_props;
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
    /// Host foil ID this point measures relative to (if known). When present, connectivity
    /// is computed with respect to this foil instead of using a nearest-foil heuristic.
    #[serde(default)]
    pub host_foil_id: Option<u64>,
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
            host_foil_id: None,
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
    /// Optionally auto-pause simulation when this frame count is reached (None = no auto-pause)
    #[serde(default)]
    pub auto_pause_frame: Option<usize>,
}

impl Default for ManualMeasurementConfig {
    fn default() -> Self {
        Self {
            name: "Manual Measurement".to_string(),
            points: vec![ManualMeasurementPoint::default()],
            interval_fs: 1000.0,
            output_file: "manual_measurements.csv".to_string(),
            auto_pause_frame: None,
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

        // Write header: only frame, time_fs, and edge columns per request
        write!(file, "frame,time_fs")?;
        for point in &self.config.points {
            write!(file, ",{}_edge", point.label)?;
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
    pub fn update(&mut self, bodies: &[Body], foils: &[Foil], quadtree: &Quadtree, frame: usize, simulation_time_fs: f32) -> Vec<MeasurementResult> {
        let mut results = Vec::new();

        // Check if it's time to measure
        let should_measure = simulation_time_fs - self.last_measurement_time >= self.config.interval_fs;

        if !should_measure && !self.is_recording {
            return results;
        }

        if should_measure {
            self.last_measurement_time = simulation_time_fs;

            // Cache connectivity per host foil to avoid repeated BFS
            let mut connected_by_foil: HashMap<u64, HashSet<usize>> = HashMap::new();
            let id_to_index: HashMap<u64, usize> = bodies.iter().enumerate().map(|(i, b)| (b.id, i)).collect();

            // Perform measurements at each point
            for point in &self.config.points {
                // Determine host foil: prefer explicit host_foil_id if present, else fallback to nearest foil
                let mut host_foil_id: Option<u64> = point.host_foil_id;
                if host_foil_id.is_none() {
                    let mut best_dist_sq = f32::INFINITY;
                    for foil in foils {
                        for bid in &foil.body_ids {
                            if let Some(&idx) = id_to_index.get(bid) {
                                let d = bodies[idx].pos - ultraviolet::Vec2::new(point.x, point.y);
                                let dsq = d.mag_sq();
                                if dsq < best_dist_sq {
                                    best_dist_sq = dsq;
                                    host_foil_id = Some(foil.id);
                                }
                            }
                        }
                    }
                }

                // Compute or reuse connected set for the host foil
                if let Some(fid) = host_foil_id {
                    if !connected_by_foil.contains_key(&fid) {
                        let connected = bfs_connected_metals_for_foil(fid, bodies, foils, quadtree, &id_to_index);
                        connected_by_foil.insert(fid, connected);
                    }
                }

                let result = self.measure_at_point_connected(bodies, point, host_foil_id, connected_by_foil.get(&host_foil_id.unwrap_or(0)));
                results.push(result);
            }

            // Write to CSV if recording
            if self.is_recording {
                if let Some(file) = &mut self.csv_file {
                    let _ = write!(file, "{},{}", frame, simulation_time_fs);
                    for result in &results {
                        let _ = write!(file, ",{}", result.edge_position);
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

        // Measurement box: for left/right, width represents max length from center toward the direction
        // For up/down, height represents max length from center toward that direction
        let (x_min, x_max) = match point.direction.as_str() {
            "left" => (point.x - point.width, point.x),
            "right" => (point.x, point.x + point.width),
            _ => (point.x - half_width, point.x + half_width),
        };
        let (y_min, y_max) = match point.direction.as_str() {
            "up" => (point.y, point.y + point.height),
            "down" => (point.y - point.height, point.y),
            _ => (point.y - half_height, point.y + half_height),
        };

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


    /// Measure using only metals connected to the selected host foil if provided
    fn measure_at_point_connected(
        &self,
        bodies: &[Body],
        point: &ManualMeasurementPoint,
        host_foil_id: Option<u64>,
        connected_set_opt: Option<&HashSet<usize>>,
    ) -> MeasurementResult {
        // Fallback to basic measurement if we have no host foil context
        if host_foil_id.is_none() || connected_set_opt.is_none() {
            return self.measure_at_point(bodies, point);
        }

        let connected_set = connected_set_opt.unwrap();

        // Define region as before
        let half_width = point.width / 2.0;
        let half_height = point.height / 2.0;
        let (x_min, x_max) = match point.direction.as_str() {
            "left" => (point.x - point.width, point.x),
            "right" => (point.x, point.x + point.width),
            _ => (point.x - half_width, point.x + half_width),
        };
        let (y_min, y_max) = match point.direction.as_str() {
            "up" => (point.y, point.y + point.height),
            "down" => (point.y - point.height, point.y),
            _ => (point.y - half_height, point.y + half_height),
        };

        let mut li_metal_positions = Vec::new();
        let mut li_ion_count = 0;

        for (idx, body) in bodies.iter().enumerate() {
            let pos = body.pos;
            if pos.x >= x_min && pos.x <= x_max && pos.y >= y_min && pos.y <= y_max {
                match body.species {
                    Species::LithiumMetal => {
                        // Only include if connected to host foil
                        if connected_set.contains(&idx) {
                            li_metal_positions.push(pos);
                        }
                    }
                    Species::LithiumIon => {
                        li_ion_count += 1;
                    }
                    _ => {}
                }
            }
        }

        let edge_position = if li_metal_positions.is_empty() {
            point.x
        } else {
            match point.direction.as_str() {
                "left" => li_metal_positions.iter().map(|p| p.x).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(point.x),
                "right" => li_metal_positions.iter().map(|p| p.x).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(point.x),
                "up" => li_metal_positions.iter().map(|p| p.y).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(point.y),
                "down" => li_metal_positions.iter().map(|p| p.y).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(point.y),
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
}

/// Compute set of indices of metal bodies connected to the specified foil via contact connections.
fn bfs_connected_metals_for_foil(
    foil_id: u64,
    bodies: &[Body],
    foils: &[Foil],
    quadtree: &Quadtree,
    id_to_index: &HashMap<u64, usize>,
) -> HashSet<usize> {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut queue: VecDeque<usize> = VecDeque::new();

    // Seed queue with foil bodies
    let host_foil = if let Some(foil) = foils.iter().find(|f| f.id == foil_id) {
        foil
    } else {
        return visited; // No such foil
    };

    // Build a fast lookup set of indices for host foil bodies (to prevent cross-foil traversal)
    let mut host_foil_indices: HashSet<usize> = HashSet::new();
    for bid in &host_foil.body_ids {
        if let Some(&idx) = id_to_index.get(bid) {
            host_foil_indices.insert(idx);
        }
    }
    for &idx in &host_foil_indices {
        visited.insert(idx);
        queue.push_back(idx);
    }

    while let Some(idx) = queue.pop_front() {
        let body = &bodies[idx];
        // Allow a small gap less than one metal diameter to still count as connected.
        let li_props = get_species_props(Species::LithiumMetal);
        let foil_props = get_species_props(Species::FoilMetal);
        let metal_diameter = 2.0 * li_props.radius;

        // Increase neighbor search radius to accommodate this relaxed threshold
        let max_nb_r = foil_props.radius.max(li_props.radius);
        let search_radius = body.radius + max_nb_r + metal_diameter + 0.1;
        let neighbors = quadtree.find_neighbors_within(bodies, idx, search_radius);
        for &n_idx in &neighbors {
            if visited.contains(&n_idx) { continue; }
            let nb = &bodies[n_idx];
            // Only hop through metals; FoilMetal must belong to the host foil
            match nb.species {
                Species::LithiumMetal => {
                    // Relaxed contact check: allow a gap < one metal diameter
                    let threshold = (body.radius + nb.radius) + metal_diameter;
                    if (body.pos - nb.pos).mag() <= threshold {
                        visited.insert(n_idx);
                        queue.push_back(n_idx);
                    }
                }
                Species::FoilMetal => {
                    // Only traverse foil nodes that are part of the host foil
                    if !host_foil_indices.contains(&n_idx) { continue; }
                    let threshold = (body.radius + nb.radius) + metal_diameter;
                    if (body.pos - nb.pos).mag() <= threshold {
                        visited.insert(n_idx);
                        queue.push_back(n_idx);
                    }
                }
                _ => { /* do not traverse through non-metals */ }
            }
        }
    }

    visited
}
