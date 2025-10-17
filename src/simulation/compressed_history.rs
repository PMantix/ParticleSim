// Optimized history storage with delta compression
// This module provides memory-efficient history tracking for simulation playback

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use ultraviolet::Vec2;

use crate::body::{Body, Species};
use crate::body::foil::Foil;
use crate::config::SimConfig;

/// Configuration for change detection thresholds
/// Optimized based on simulation physics and precision requirements
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct ChangeThresholds {
    /// Minimum position change to record (in Angstroms)
    /// Based on thermal motion scale: ~0.01 Å is typical atomic vibration amplitude
    pub position_epsilon: f32,
    /// Minimum velocity change to record (Å/fs)
    /// Based on thermal velocity scale: ~0.1 Å/fs is typical atomic velocity
    pub velocity_epsilon: f32, 
    /// Minimum charge change to record (elementary charges)
    /// Small fractional charges from polarization effects
    pub charge_epsilon: f32,
    /// Minimum electron position change to record (Å)
    /// Electrons can drift significantly within atoms
    pub electron_epsilon: f32,
    /// Minimum z-position change to record (Å)
    /// Out-of-plane motion threshold
    pub z_epsilon: f32,
    /// Threshold for considering a body "moved significantly"
    /// Used for electron surroundings recalculation
    pub significant_move_epsilon: f32,
}

impl Default for ChangeThresholds {
    fn default() -> Self {
        Self {
            // Position: 0.01 Å ≈ 1% of typical atomic radius
            // This catches meaningful atomic displacements while ignoring noise
            position_epsilon: 0.01,
            
            // Velocity: 0.1 Å/fs ≈ 100 m/s at atomic scale
            // Catches significant momentum changes
            velocity_epsilon: 0.1,
            
            // Charge: 0.01e ≈ 1% of elementary charge
            // Captures polarization and fractional charge effects
            charge_epsilon: 0.01, 
            
            // Electron drift: 0.1 Å is reasonable for electron cloud movement
            // Larger than atomic vibrations but smaller than hopping distances
            electron_epsilon: 0.1,
            
            // Z-motion: 0.05 Å for out-of-plane motion
            // More sensitive since z-motion should be constrained
            z_epsilon: 0.05,
            
            // Significant movement: 0.5 Å matches SURROUND_MOVE_THRESHOLD
            // Consistent with existing physics thresholds
            significant_move_epsilon: 0.5,
        }
    }
}

#[allow(dead_code)] // Will be used in GUI integration
impl ChangeThresholds {
    /// Create thresholds optimized for high-precision simulations
    /// Useful for detailed analysis or validation runs
    pub fn high_precision() -> Self {
        Self {
            position_epsilon: 0.001,    // 0.001 Å - very fine position changes
            velocity_epsilon: 0.01,     // 0.01 Å/fs - small velocity changes
            charge_epsilon: 0.001,      // 0.001e - tiny charge fluctuations
            electron_epsilon: 0.01,     // 0.01 Å - fine electron movement
            z_epsilon: 0.001,           // 0.001 Å - very sensitive z-motion
            significant_move_epsilon: 0.1, // 0.1 Å - sensitive movement detection
        }
    }
    
    /// Create thresholds optimized for memory efficiency
    /// Trades some precision for significant memory savings
    pub fn memory_optimized() -> Self {
        Self {
            position_epsilon: 0.1,      // 0.1 Å - only major displacements
            velocity_epsilon: 0.5,      // 0.5 Å/fs - significant velocity changes
            charge_epsilon: 0.05,       // 0.05e - substantial charge changes
            electron_epsilon: 0.5,      // 0.5 Å - large electron movements
            z_epsilon: 0.2,             // 0.2 Å - major z-motion only
            significant_move_epsilon: 2.0, // 2.0 Å - very significant movement
        }
    }
    
    /// Create thresholds optimized for specific simulation types
    pub fn for_simulation_type(sim_type: SimulationType) -> Self {
        match sim_type {
            SimulationType::Equilibration => Self::memory_optimized(),
            SimulationType::Production => Self::default(), 
            SimulationType::Analysis => Self::high_precision(),
        }
    }
    
    /// Adjust thresholds based on simulation temperature
    /// Higher temperatures require larger thresholds due to increased thermal motion
    pub fn temperature_adjusted(mut self, temperature_k: f32) -> Self {
        // Thermal energy scaling factor (relative to 300K)
        let thermal_factor = (temperature_k / 300.0).sqrt();
        
        // Scale position and velocity thresholds with thermal motion
        self.position_epsilon *= thermal_factor;
        self.velocity_epsilon *= thermal_factor;
        self.electron_epsilon *= thermal_factor;
        self.z_epsilon *= thermal_factor;
        
        // Charge and significant movement thresholds remain unchanged
        // as they're more related to chemical processes than thermal motion
        
        self
    }
}

/// Types of simulations with different precision requirements
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Will be used in GUI integration
pub enum SimulationType {
    /// Initial equilibration phase - can use loose thresholds
    Equilibration,
    /// Production run - balanced precision/performance
    Production,
    /// Detailed analysis - maximum precision
    Analysis,
}

/// Lightweight body representation that excludes computed values
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightBody {
    pub pos: Vec2,
    pub z: f32,
    pub vel: Vec2, 
    pub vz: f32,
    pub mass: f32,
    pub radius: f32,
    pub charge: f32,
    pub id: u64,
    pub species: Species,
    // Note: Excludes acc, az, e_field (computed values)
    // Note: Includes electrons but they can be optional in deltas
    pub electrons: Vec<crate::body::Electron>,
    pub surrounded_by_metal: bool,
    pub last_surround_pos: Vec2,
    pub last_surround_frame: usize,
}

impl From<&Body> for LightBody {
    fn from(body: &Body) -> Self {
        Self {
            pos: body.pos,
            z: body.z,
            vel: body.vel,
            vz: body.vz,
            mass: body.mass,
            radius: body.radius,
            charge: body.charge,
            id: body.id,
            species: body.species,
            electrons: body.electrons.iter().cloned().collect(),
            surrounded_by_metal: body.surrounded_by_metal,
            last_surround_pos: body.last_surround_pos,
            last_surround_frame: body.last_surround_frame,
        }
    }
}

/// Lightweight foil representation that excludes PID history
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightFoil {
    pub id: u64,
    pub body_ids: Vec<u64>,
    pub dc_current: f32,
    pub ac_current: f32,
    pub switch_hz: f32,
    pub link_id: Option<u64>,
    pub mode: crate::body::foil::LinkMode,
    pub charging_mode: crate::body::foil::ChargingMode,
    // Note: Excludes overpotential_controller.history (saves ~60KB per foil!)
    pub overpotential_target: Option<f32>,
    pub overpotential_kp: Option<f32>,
    pub overpotential_ki: Option<f32>,
    pub overpotential_kd: Option<f32>,
    pub slave_overpotential_current: f32,
}

impl From<&Foil> for LightFoil {
    fn from(foil: &Foil) -> Self {
        Self {
            id: foil.id,
            body_ids: foil.body_ids.clone(),
            dc_current: foil.dc_current,
            ac_current: foil.ac_current,
            switch_hz: foil.switch_hz,
            link_id: foil.link_id,
            mode: foil.mode.clone(),
            charging_mode: foil.charging_mode.clone(),
            overpotential_target: foil.overpotential_controller.as_ref().map(|c| c.target_ratio),
            overpotential_kp: foil.overpotential_controller.as_ref().map(|c| c.kp),
            overpotential_ki: foil.overpotential_controller.as_ref().map(|c| c.ki),
            overpotential_kd: foil.overpotential_controller.as_ref().map(|c| c.kd),
            slave_overpotential_current: foil.slave_overpotential_current,
        }
    }
}

/// Complete lightweight snapshot (keyframe)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightSnapshot {
    pub frame: usize,
    pub sim_time: f32,
    pub dt: f32,
    pub last_thermostat_time: f32,
    
    pub bodies: Vec<LightBody>,
    pub foils: Vec<LightFoil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config: SimConfig,
    
    pub domain_width: f32,
    pub domain_height: f32, 
    pub domain_depth: f32,
}

/// Delta change for a single body
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BodyDelta {
    pub id: u64,
    pub pos: Option<Vec2>,
    pub z: Option<f32>,
    pub vel: Option<Vec2>,
    pub vz: Option<f32>,
    pub charge: Option<f32>,
    pub electrons: Option<Vec<crate::body::Electron>>,
    pub surrounded_by_metal: Option<bool>,
    pub last_surround_pos: Option<Vec2>,
    pub last_surround_frame: Option<usize>,
    // Note: mass, radius, species rarely change so not included in deltas
}

/// Delta change for a single foil
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FoilDelta {
    pub id: u64,
    pub dc_current: Option<f32>,
    pub ac_current: Option<f32>,
    pub switch_hz: Option<f32>,
    pub link_id: Option<Option<u64>>,
    pub mode: Option<crate::body::foil::LinkMode>,
    pub charging_mode: Option<crate::body::foil::ChargingMode>,
    pub overpotential_target: Option<f32>,
    pub overpotential_kp: Option<f32>,
    pub overpotential_ki: Option<f32>, 
    pub overpotential_kd: Option<f32>,
    pub slave_overpotential_current: Option<f32>,
    // Note: body_ids rarely change
}

/// Delta snapshot storing only changes since last frame
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeltaSnapshot {
    pub frame: usize,
    pub sim_time: f32,
    pub dt: Option<f32>,  // Only if changed
    pub last_thermostat_time: Option<f32>,  // Only if changed
    
    /// Only bodies that changed
    pub body_deltas: Vec<BodyDelta>,
    
    /// Only foils that changed
    pub foil_deltas: Vec<FoilDelta>,
    
    /// Only if config changed (rare)
    pub config: Option<SimConfig>,
    
    /// Only if domain changed (rare)
    pub domain_width: Option<f32>,
    pub domain_height: Option<f32>,
    pub domain_depth: Option<f32>,
    
    /// Only if body-foil mapping changed (rare)
    pub body_to_foil_changes: Option<HashMap<u64, Option<u64>>>, // None = removed mapping
}

/// Change detection for bodies
#[allow(dead_code)] // Will be used by CompressedHistorySystem
impl LightBody {
    pub fn create_delta(&self, previous: &LightBody, thresholds: &ChangeThresholds) -> Option<BodyDelta> {
        let mut delta = BodyDelta { id: self.id, ..Default::default() };
        let mut has_changes = false;
        
        // Check position change
        if (self.pos - previous.pos).mag() > thresholds.position_epsilon || 
           (self.z - previous.z).abs() > thresholds.position_epsilon {
            delta.pos = Some(self.pos);
            delta.z = Some(self.z);
            has_changes = true;
        }
        
        // Check velocity change
        if (self.vel - previous.vel).mag() > thresholds.velocity_epsilon ||
           (self.vz - previous.vz).abs() > thresholds.velocity_epsilon {
            delta.vel = Some(self.vel);
            delta.vz = Some(self.vz);
            has_changes = true;
        }
        
        // Check charge change
        if (self.charge - previous.charge).abs() > thresholds.charge_epsilon {
            delta.charge = Some(self.charge);
            has_changes = true;
        }
        
        // Check electron changes (simplified - could be more sophisticated)
        if self.electrons.len() != previous.electrons.len() ||
           self.electrons.iter().zip(previous.electrons.iter()).any(|(a, b)| {
               (a.rel_pos - b.rel_pos).mag() > thresholds.electron_epsilon ||
               (a.vel - b.vel).mag() > thresholds.electron_epsilon
           }) {
            delta.electrons = Some(self.electrons.clone());
            has_changes = true;
        }
        
        // Check other state changes
        if self.surrounded_by_metal != previous.surrounded_by_metal {
            delta.surrounded_by_metal = Some(self.surrounded_by_metal);
            has_changes = true;
        }
        
        if (self.last_surround_pos - previous.last_surround_pos).mag() > thresholds.position_epsilon {
            delta.last_surround_pos = Some(self.last_surround_pos);
            has_changes = true;
        }
        
        if self.last_surround_frame != previous.last_surround_frame {
            delta.last_surround_frame = Some(self.last_surround_frame);
            has_changes = true;
        }
        
        if has_changes { Some(delta) } else { None }
    }
}

/// Change detection for foils
#[allow(dead_code)] // Will be used by CompressedHistorySystem
impl LightFoil {
    pub fn create_delta(&self, previous: &LightFoil) -> Option<FoilDelta> {
        let mut delta = FoilDelta { id: self.id, ..Default::default() };
        let mut has_changes = false;
        
        if (self.dc_current - previous.dc_current).abs() > 1e-6 {
            delta.dc_current = Some(self.dc_current);
            has_changes = true;
        }
        
        if (self.ac_current - previous.ac_current).abs() > 1e-6 {
            delta.ac_current = Some(self.ac_current);
            has_changes = true;
        }
        
        if (self.switch_hz - previous.switch_hz).abs() > 1e-6 {
            delta.switch_hz = Some(self.switch_hz);
            has_changes = true;
        }
        
        if self.link_id != previous.link_id {
            delta.link_id = Some(self.link_id);
            has_changes = true;
        }
        
        if self.mode != previous.mode {
            delta.mode = Some(self.mode.clone());
            has_changes = true;
        }
        
        if (self.slave_overpotential_current - previous.slave_overpotential_current).abs() > 1e-6 {
            delta.slave_overpotential_current = Some(self.slave_overpotential_current);
            has_changes = true;
        }
        
        if self.charging_mode != previous.charging_mode {
            delta.charging_mode = Some(self.charging_mode.clone());
            has_changes = true;
        }
        
        // Check overpotential parameters
        if self.overpotential_target != previous.overpotential_target {
            delta.overpotential_target = self.overpotential_target;
            has_changes = true;
        }
        
        if self.overpotential_kp != previous.overpotential_kp {
            delta.overpotential_kp = self.overpotential_kp;
            has_changes = true;
        }
        
        if self.overpotential_ki != previous.overpotential_ki {
            delta.overpotential_ki = self.overpotential_ki;
            has_changes = true;
        }
        
        if self.overpotential_kd != previous.overpotential_kd {
            delta.overpotential_kd = self.overpotential_kd;
            has_changes = true;
        }
        
        if has_changes { Some(delta) } else { None }
    }
}

/// Configuration for the compressed history system
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct CompressionConfig {
    /// How often to create keyframes (every N frames)
    pub keyframe_interval: usize,
    /// Maximum number of keyframes to keep
    pub max_keyframes: usize,
    /// Maximum number of delta frames to keep
    pub max_deltas: usize,
    /// Change detection thresholds
    pub thresholds: ChangeThresholds,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            keyframe_interval: 100,     // Keyframe every 100 frames
            max_keyframes: 50,          // Keep 50 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::default(),
        }
    }
}

#[allow(dead_code)] // Will be used in GUI integration
impl CompressionConfig {
    /// Create configuration optimized for memory efficiency
    /// Fewer keyframes, larger intervals, loose thresholds
    pub fn memory_optimized() -> Self {
        Self {
            keyframe_interval: 200,     // Keyframe every 200 frames
            max_keyframes: 25,          // Keep 25 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::memory_optimized(),
        }
    }
    
    /// Create configuration optimized for precision
    /// More keyframes, smaller intervals, tight thresholds
    pub fn high_precision() -> Self {
        Self {
            keyframe_interval: 50,      // Keyframe every 50 frames
            max_keyframes: 100,         // Keep 100 keyframes (5000 frames span)
            max_deltas: 5000,           // Keep 5000 delta frames
            thresholds: ChangeThresholds::high_precision(),
        }
    }
    
    /// Create configuration for large simulations (many particles)
    /// Balanced between memory and precision for scalability
    pub fn large_simulation() -> Self {
        Self {
            keyframe_interval: 150,     // Keyframe every 150 frames
            max_keyframes: 33,          // Keep 33 keyframes (~5000 frames)
            max_deltas: 3000,           // Reduced delta storage
            thresholds: ChangeThresholds::default(),
        }
    }
    
    /// Create configuration based on simulation parameters
    pub fn for_simulation(particle_count: usize, temperature_k: f32, sim_type: SimulationType) -> Self {
        let base_config = match sim_type {
            SimulationType::Equilibration => Self::memory_optimized(),
            SimulationType::Production => Self::default(),
            SimulationType::Analysis => Self::high_precision(),
        };
        
        // Adjust for particle count
        let mut config = if particle_count > 50000 {
            // Very large simulations need aggressive memory optimization
            Self {
                keyframe_interval: base_config.keyframe_interval * 2,
                max_keyframes: base_config.max_keyframes / 2,
                max_deltas: base_config.max_deltas / 2,
                thresholds: base_config.thresholds,
            }
        } else if particle_count > 10000 {
            // Large simulations
            Self::large_simulation()
        } else {
            base_config
        };
        
        // Adjust thresholds for temperature
        config.thresholds = config.thresholds.temperature_adjusted(temperature_k);
        
        config
    }
    
    /// Update configuration to maintain target memory usage (in MB)
    pub fn target_memory_mb(mut self, target_mb: f64, particle_count: usize) -> Self {
        // Estimate memory per frame (rough calculation)
        let kb_per_particle = 0.2; // ~200 bytes per particle in lightweight format
        let estimated_mb_per_keyframe = (particle_count as f64 * kb_per_particle) / 1024.0;
        let estimated_mb_per_delta = estimated_mb_per_keyframe * 0.05; // Deltas ~5% of keyframe
        
        // Calculate current estimated memory usage
        let current_keyframe_mb = self.max_keyframes as f64 * estimated_mb_per_keyframe;
        let current_delta_mb = self.max_deltas as f64 * estimated_mb_per_delta;
        let current_total_mb = current_keyframe_mb + current_delta_mb;
        
        if current_total_mb > target_mb {
            // Need to reduce memory usage
            let reduction_factor = target_mb / current_total_mb;
            
            // Primarily reduce by increasing keyframe interval and reducing counts
            self.keyframe_interval = (self.keyframe_interval as f64 / reduction_factor.sqrt()) as usize;
            self.max_keyframes = ((self.max_keyframes as f64 * reduction_factor.sqrt()) as usize).max(5);
            self.max_deltas = ((self.max_deltas as f64 * reduction_factor) as usize).max(100);
            
            // Also use looser thresholds to reduce delta frequency
            let threshold_factor = (2.0 - reduction_factor).max(1.0) as f32;
            self.thresholds.position_epsilon *= threshold_factor;
            self.thresholds.velocity_epsilon *= threshold_factor;
            self.thresholds.charge_epsilon *= threshold_factor;
        }
        
        self
    }
}

/// Compressed history storage system using keyframes + deltas
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct CompressedHistorySystem {
    /// Full snapshots at regular intervals (keyframes)
    keyframes: VecDeque<KeyframeEntry>,
    
    /// Delta changes between keyframes
    deltas: VecDeque<DeltaEntry>,
    
    /// Configuration settings
    config: CompressionConfig,
    
    /// Current cursor position for playback
    cursor_frame: usize,
    
    /// Last full state for delta generation
    last_state: Option<LightSnapshot>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // Internal structure for CompressedHistorySystem
struct KeyframeEntry {
    frame: usize,
    snapshot: LightSnapshot,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // Internal structure for CompressedHistorySystem
struct DeltaEntry {
    frame: usize,
    delta: DeltaSnapshot,
    /// Reference to the keyframe this delta is based on
    keyframe_frame: usize,
}

#[allow(dead_code)] // Will be used in GUI integration
impl CompressedHistorySystem {
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            keyframes: VecDeque::new(),
            deltas: VecDeque::new(),
            config,
            cursor_frame: 0,
            last_state: None,
        }
    }
    
    pub fn new_default() -> Self {
        Self::new(CompressionConfig::default())
    }
    
    /// Add a new frame to the history
    pub fn push_frame(&mut self, snapshot: LightSnapshot) {
        let frame = snapshot.frame;
        
        // Decide whether this should be a keyframe or delta
        if self.should_create_keyframe(frame) {
            self.create_keyframe(snapshot);
        } else {
            self.create_delta(snapshot);
        }
        
        // Clean up old data if needed
        self.cleanup_old_data();
        
        // Update cursor to latest frame
        self.cursor_frame = frame;
    }
    
    /// Check if we should create a keyframe for this frame
    fn should_create_keyframe(&self, frame: usize) -> bool {
        // Always create keyframe for first frame
        if self.keyframes.is_empty() {
            return true;
        }
        
        // Create keyframe at regular intervals
        let last_keyframe_frame = self.keyframes.back().unwrap().frame;
        frame >= last_keyframe_frame + self.config.keyframe_interval
    }
    
    /// Create a new keyframe entry
    fn create_keyframe(&mut self, snapshot: LightSnapshot) {
        let entry = KeyframeEntry {
            frame: snapshot.frame,
            snapshot: snapshot.clone(),
        };
        
        self.keyframes.push_back(entry);
        self.last_state = Some(snapshot);
        
        // Remove excess keyframes
        while self.keyframes.len() > self.config.max_keyframes {
            let removed = self.keyframes.pop_front().unwrap();
            // Remove all deltas that depend on this keyframe
            self.deltas.retain(|delta| delta.keyframe_frame != removed.frame);
        }
    }
    
    /// Create a new delta entry
    fn create_delta(&mut self, snapshot: LightSnapshot) {
        if let Some(ref last_state) = self.last_state {
            // Generate delta from last state
            if let Some(delta) = self.generate_delta(last_state, &snapshot) {
                let keyframe_frame = self.find_keyframe_for_frame(snapshot.frame);
                
                let entry = DeltaEntry {
                    frame: snapshot.frame,
                    delta,
                    keyframe_frame,
                };
                
                self.deltas.push_back(entry);
            }
        }
        
        self.last_state = Some(snapshot);
        
        // Remove excess deltas
        while self.deltas.len() > self.config.max_deltas {
            self.deltas.pop_front();
        }
    }
    
    /// Generate delta between two snapshots
    fn generate_delta(&self, previous: &LightSnapshot, current: &LightSnapshot) -> Option<DeltaSnapshot> {
        let mut body_deltas = Vec::new();
        let mut foil_deltas = Vec::new();
        
        // Create HashMap for O(1) lookup of previous bodies by ID
        let previous_bodies: std::collections::HashMap<u64, &LightBody> = 
            previous.bodies.iter().map(|b| (b.id, b)).collect();
        
        // Generate body deltas with O(N) performance
        for current_body in &current.bodies {
            if let Some(prev_body) = previous_bodies.get(&current_body.id) {
                if let Some(delta) = current_body.create_delta(prev_body, &self.config.thresholds) {
                    body_deltas.push(delta);
                }
            } else {
                // New body - store full state as delta
                body_deltas.push(BodyDelta {
                    id: current_body.id,
                    pos: Some(current_body.pos),
                    z: Some(current_body.z),
                    vel: Some(current_body.vel),
                    vz: Some(current_body.vz),
                    charge: Some(current_body.charge),
                    electrons: Some(current_body.electrons.clone()),
                    surrounded_by_metal: Some(current_body.surrounded_by_metal),
                    last_surround_pos: Some(current_body.last_surround_pos),
                    last_surround_frame: Some(current_body.last_surround_frame),
                });
            }
        }
        
        // Create HashMap for O(1) lookup of previous foils by ID  
        let previous_foils: std::collections::HashMap<u64, &LightFoil> =
            previous.foils.iter().map(|f| (f.id, f)).collect();
        
        // Generate foil deltas with O(N) performance
        for current_foil in &current.foils {
            if let Some(prev_foil) = previous_foils.get(&current_foil.id) {
                if let Some(delta) = current_foil.create_delta(prev_foil) {
                    foil_deltas.push(delta);
                }
            }
        }
        
        // Check for other changes
        let dt_changed = (current.dt - previous.dt).abs() > 1e-9;
        let thermostat_changed = (current.last_thermostat_time - previous.last_thermostat_time).abs() > 1e-6;
        let config_changed = !std::ptr::eq(&current.config, &previous.config); // Simple comparison
        let domain_changed = (current.domain_width - previous.domain_width).abs() > 1e-6 ||
                            (current.domain_height - previous.domain_height).abs() > 1e-6 ||
                            (current.domain_depth - previous.domain_depth).abs() > 1e-6;
        let mapping_changed = current.body_to_foil != previous.body_to_foil;
        
        // Only create delta if there are actual changes
        if body_deltas.is_empty() && foil_deltas.is_empty() && 
           !dt_changed && !thermostat_changed && !config_changed && !domain_changed && !mapping_changed {
            return None;
        }
        
        Some(DeltaSnapshot {
            frame: current.frame,
            sim_time: current.sim_time,
            dt: if dt_changed { Some(current.dt) } else { None },
            last_thermostat_time: if thermostat_changed { Some(current.last_thermostat_time) } else { None },
            body_deltas,
            foil_deltas,
            config: if config_changed { Some(current.config.clone()) } else { None },
            domain_width: if domain_changed { Some(current.domain_width) } else { None },
            domain_height: if domain_changed { Some(current.domain_height) } else { None },
            domain_depth: if domain_changed { Some(current.domain_depth) } else { None },
            body_to_foil_changes: if mapping_changed { 
                // For simplicity, store the full mapping when it changes
                // Could be optimized to store only changes
                Some(current.body_to_foil.iter().map(|(k, v)| (*k, Some(*v))).collect())
            } else { 
                None 
            },
        })
    }
    
    /// Find the keyframe frame number that should be used as base for given frame
    fn find_keyframe_for_frame(&self, frame: usize) -> usize {
        self.keyframes
            .iter()
            .rev()
            .find(|kf| kf.frame <= frame)
            .map(|kf| kf.frame)
            .unwrap_or(0)
    }
    
    /// Clean up old data based on configuration limits
    fn cleanup_old_data(&mut self) {
        // Keyframe cleanup is handled in create_keyframe
        // Delta cleanup is handled in create_delta
        
        // Additional cleanup: remove deltas that are older than oldest keyframe
        if let Some(oldest_keyframe) = self.keyframes.front() {
            self.deltas.retain(|delta| delta.frame >= oldest_keyframe.frame);
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        let keyframe_count = self.keyframes.len();
        let delta_count = self.deltas.len();
        
        // Rough estimates (could be made more precise with actual serialization)
        let avg_keyframe_size = 200_000; // ~200KB per keyframe (estimated)
        let avg_delta_size = 10_000;     // ~10KB per delta (estimated)
        
        let keyframe_memory = keyframe_count * avg_keyframe_size;
        let delta_memory = delta_count * avg_delta_size;
        let total_memory = keyframe_memory + delta_memory;
        
        MemoryStats {
            keyframe_count,
            delta_count,
            keyframe_memory_bytes: keyframe_memory,
            delta_memory_bytes: delta_memory,
            total_memory_bytes: total_memory,
            oldest_frame: self.keyframes.front().map(|kf| kf.frame).unwrap_or(0),
            newest_frame: self.keyframes.back().map(|kf| kf.frame)
                .max(self.deltas.back().map(|d| d.frame))
                .unwrap_or(0),
        }
    }
    
    /// Check if a specific frame is available in history
    pub fn has_frame(&self, frame: usize) -> bool {
        // Check if frame is in keyframes
        if self.keyframes.iter().any(|kf| kf.frame == frame) {
            return true;
        }
        
        // Check if frame can be reconstructed from keyframe + deltas
        if let Some(base_keyframe) = self.keyframes.iter().rev().find(|kf| kf.frame <= frame) {
            // Check if all deltas from keyframe to target frame are available
            let mut current_frame = base_keyframe.frame;
            while current_frame < frame {
                current_frame += 1;
                if !self.deltas.iter().any(|d| d.frame == current_frame && d.keyframe_frame <= base_keyframe.frame) {
                    return false;
                }
            }
            return true;
        }
        
        false
    }
    
    /// Get the range of frames available in history
    pub fn get_frame_range(&self) -> Option<(usize, usize)> {
        let oldest = self.keyframes.front().map(|kf| kf.frame)?;
        let newest = self.keyframes.back().map(|kf| kf.frame)
            .max(self.deltas.back().map(|d| d.frame))?;
        Some((oldest, newest))
    }
    
    /// Reconstruct a specific frame from keyframes and deltas
    /// Returns the complete LightSnapshot for the requested frame
    pub fn reconstruct_frame(&self, frame: usize) -> Result<LightSnapshot, ReconstructionError> {
        // Find the appropriate keyframe (latest keyframe <= target frame)
        let keyframe_entry = self.keyframes
            .iter()
            .rev()
            .find(|kf| kf.frame <= frame)
            .ok_or(ReconstructionError::NoKeyframeAvailable { frame })?;
            
        // If requesting the keyframe itself, return it directly
        if keyframe_entry.frame == frame {
            return Ok(keyframe_entry.snapshot.clone());
        }
        
        // Start with the keyframe and apply deltas
        let mut reconstructed = keyframe_entry.snapshot.clone();
        
        // Collect all deltas from keyframe to target frame
        let mut deltas_to_apply: Vec<&DeltaEntry> = self.deltas
            .iter()
            .filter(|delta| {
                delta.frame > keyframe_entry.frame && 
                delta.frame <= frame &&
                delta.keyframe_frame <= keyframe_entry.frame
            })
            .collect();
            
        // Sort deltas by frame number to apply them in chronological order
        deltas_to_apply.sort_by_key(|delta| delta.frame);
        
        // Apply each delta to reconstruct the frame
        for delta_entry in &deltas_to_apply {
            // Update frame metadata
            reconstructed.frame = delta_entry.delta.frame;
            reconstructed.sim_time = delta_entry.delta.sim_time;
            
            // Apply scalar field changes
            if let Some(dt) = delta_entry.delta.dt {
                reconstructed.dt = dt;
            }
            if let Some(thermostat_time) = delta_entry.delta.last_thermostat_time {
                reconstructed.last_thermostat_time = thermostat_time;
            }
            if let Some(config) = &delta_entry.delta.config {
                reconstructed.config = config.clone();
            }
            if let Some(width) = delta_entry.delta.domain_width {
                reconstructed.domain_width = width;
            }
            if let Some(height) = delta_entry.delta.domain_height {
                reconstructed.domain_height = height;
            }
            if let Some(depth) = delta_entry.delta.domain_depth {
                reconstructed.domain_depth = depth;
            }
            
            // Apply body changes
            for body_delta in &delta_entry.delta.body_deltas {
                if let Some(body) = reconstructed.bodies.iter_mut().find(|b| b.id == body_delta.id) {
                    self.apply_body_delta(body, body_delta)?;
                } else {
                    // New body - this shouldn't happen in normal operation
                    return Err(ReconstructionError::UnknownBodyId { id: body_delta.id, frame });
                }
            }
            
            // Apply foil changes
            for foil_delta in &delta_entry.delta.foil_deltas {
                if let Some(foil) = reconstructed.foils.iter_mut().find(|f| f.id == foil_delta.id) {
                    self.apply_foil_delta(foil, foil_delta)?;
                } else {
                    return Err(ReconstructionError::UnknownFoilId { id: foil_delta.id, frame });
                }
            }
            
            // Apply body-to-foil mapping changes
            if let Some(mapping_changes) = &delta_entry.delta.body_to_foil_changes {
                for (body_id, foil_id_opt) in mapping_changes {
                    match foil_id_opt {
                        Some(foil_id) => {
                            reconstructed.body_to_foil.insert(*body_id, *foil_id);
                        }
                        None => {
                            reconstructed.body_to_foil.remove(body_id);
                        }
                    }
                }
            }
        }
        
        // Verify we reconstructed the correct frame
        if reconstructed.frame != frame {
            // Check if we have a gap in the delta chain
            let missing_frame = (keyframe_entry.frame + 1..=frame)
                .find(|f| !deltas_to_apply.iter().any(|d| d.frame == *f));
                
            if let Some(missing) = missing_frame {
                return Err(ReconstructionError::MissingDelta { 
                    frame: missing, 
                    target_frame: frame 
                });
            }
        }
        
        Ok(reconstructed)
    }
    
    /// Apply a body delta to a LightBody
    fn apply_body_delta(&self, body: &mut LightBody, delta: &BodyDelta) -> Result<(), ReconstructionError> {
        // Apply position changes
        if let Some(pos) = delta.pos {
            body.pos = pos;
        }
        if let Some(z) = delta.z {
            body.z = z;
        }
        
        // Apply velocity changes  
        if let Some(vel) = delta.vel {
            body.vel = vel;
        }
        if let Some(vz) = delta.vz {
            body.vz = vz;
        }
        
        // Apply charge changes
        if let Some(charge) = delta.charge {
            body.charge = charge;
        }
        
        // Apply electron changes
        if let Some(ref electrons) = delta.electrons {
            body.electrons = electrons.clone();
        }
        
        // Apply boolean field changes
        if let Some(surrounded) = delta.surrounded_by_metal {
            body.surrounded_by_metal = surrounded;
        }
        if let Some(surround_pos) = delta.last_surround_pos {
            body.last_surround_pos = surround_pos;
        }
        if let Some(surround_frame) = delta.last_surround_frame {
            body.last_surround_frame = surround_frame;
        }
        
        Ok(())
    }
    
    /// Apply a foil delta to a LightFoil
    fn apply_foil_delta(&self, foil: &mut LightFoil, delta: &FoilDelta) -> Result<(), ReconstructionError> {
        // Apply current changes
        if let Some(dc_current) = delta.dc_current {
            foil.dc_current = dc_current;
        }
        if let Some(ac_current) = delta.ac_current {
            foil.ac_current = ac_current;
        }
        
        // Apply switch frequency
        if let Some(switch_hz) = delta.switch_hz {
            foil.switch_hz = switch_hz;
        }
        
        // Apply link changes
        if let Some(link_id) = delta.link_id {
            foil.link_id = link_id;
        }
        
        // Apply mode changes
        if let Some(mode) = delta.mode {
            foil.mode = mode;
        }
        if let Some(charging_mode) = delta.charging_mode {
            foil.charging_mode = charging_mode;
        }
        
        // Apply overpotential controller parameters
        if let Some(overpotential_target) = delta.overpotential_target {
            foil.overpotential_target = Some(overpotential_target);
        }
        if let Some(overpotential_kp) = delta.overpotential_kp {
            foil.overpotential_kp = Some(overpotential_kp);
        }
        if let Some(overpotential_ki) = delta.overpotential_ki {
            foil.overpotential_ki = Some(overpotential_ki);
        }
        if let Some(overpotential_kd) = delta.overpotential_kd {
            foil.overpotential_kd = Some(overpotential_kd);
        }
        
        // Apply slave overpotential current
        if let Some(slave_overpotential_current) = delta.slave_overpotential_current {
            foil.slave_overpotential_current = slave_overpotential_current;
        }
        
        Ok(())
    }
    
    /// Batch reconstruct multiple frames efficiently
    /// Optimizes reconstruction by reusing intermediate results
    pub fn reconstruct_frames(&self, frames: &[usize]) -> Result<Vec<LightSnapshot>, ReconstructionError> {
        let mut results = Vec::with_capacity(frames.len());
        let mut sorted_frames = frames.to_vec();
        sorted_frames.sort_unstable();
        
        // Group frames by their keyframe to optimize reconstruction
        let mut frames_by_keyframe: HashMap<usize, Vec<usize>> = HashMap::new();
        
        for &frame in &sorted_frames {
            if let Some(keyframe) = self.keyframes.iter().rev().find(|kf| kf.frame <= frame) {
                frames_by_keyframe.entry(keyframe.frame).or_default().push(frame);
            } else {
                return Err(ReconstructionError::NoKeyframeAvailable { frame });
            }
        }
        
        // Reconstruct frames grouped by keyframe for efficiency
        for (&keyframe_frame, group_frames) in &frames_by_keyframe {
            let keyframe_snapshot = self.keyframes
                .iter()
                .find(|kf| kf.frame == keyframe_frame)
                .unwrap()
                .snapshot
                .clone();
                
            // For each frame in this group, reconstruct from the keyframe
            for &frame in group_frames {
                let reconstructed = if frame == keyframe_frame {
                    keyframe_snapshot.clone()
                } else {
                    // Apply deltas from keyframe to target frame
                    let mut current = keyframe_snapshot.clone();
                    
                    let deltas: Vec<&DeltaEntry> = self.deltas
                        .iter()
                        .filter(|d| d.frame > keyframe_frame && d.frame <= frame)
                        .collect();
                        
                    // Apply deltas in chronological order
                    for delta_entry in deltas {
                        self.apply_delta_to_snapshot(&mut current, &delta_entry.delta)?;
                    }
                    
                    current.frame = frame;
                    current
                };
                
                results.push(reconstructed);
            }
        }
        
        // Sort results to match input order
        let frame_to_snapshot: HashMap<usize, LightSnapshot> = 
            sorted_frames.iter().zip(results.into_iter()).map(|(&f, s)| (f, s)).collect();
            
        let final_results: Result<Vec<_>, _> = frames
            .iter()
            .map(|&frame| {
                frame_to_snapshot.get(&frame)
                    .cloned()
                    .ok_or(ReconstructionError::ReconstructionFailed { frame })
            })
            .collect();
            
        final_results
    }
    
    /// Apply a delta to a snapshot (helper function)
    fn apply_delta_to_snapshot(&self, snapshot: &mut LightSnapshot, delta: &DeltaSnapshot) -> Result<(), ReconstructionError> {
        // Update metadata
        snapshot.frame = delta.frame;
        snapshot.sim_time = delta.sim_time;
        
        // Apply scalar changes
        if let Some(dt) = delta.dt {
            snapshot.dt = dt;
        }
        if let Some(thermostat_time) = delta.last_thermostat_time {
            snapshot.last_thermostat_time = thermostat_time;
        }
        if let Some(ref config) = delta.config {
            snapshot.config = config.clone();
        }
        
        // Apply domain changes
        if let Some(width) = delta.domain_width {
            snapshot.domain_width = width;
        }
        if let Some(height) = delta.domain_height {
            snapshot.domain_height = height;
        }
        if let Some(depth) = delta.domain_depth {
            snapshot.domain_depth = depth;
        }
        
        // Apply body deltas
        for body_delta in &delta.body_deltas {
            if let Some(body) = snapshot.bodies.iter_mut().find(|b| b.id == body_delta.id) {
                self.apply_body_delta(body, body_delta)?;
            }
        }
        
        // Apply foil deltas
        for foil_delta in &delta.foil_deltas {
            if let Some(foil) = snapshot.foils.iter_mut().find(|f| f.id == foil_delta.id) {
                self.apply_foil_delta(foil, foil_delta)?;
            }
        }
        
        // Apply mapping changes
        if let Some(mapping_changes) = &delta.body_to_foil_changes {
            for (body_id, foil_id_opt) in mapping_changes {
                match foil_id_opt {
                    Some(foil_id) => {
                        snapshot.body_to_foil.insert(*body_id, *foil_id);
                    }
                    None => {
                        snapshot.body_to_foil.remove(body_id);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate the integrity of reconstructed data
    pub fn validate_reconstruction(&self, frame: usize, reconstructed: &LightSnapshot) -> Result<ValidationResult, ReconstructionError> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        
        // Basic frame consistency
        if reconstructed.frame != frame {
            issues.push(ValidationIssue::FrameMismatch {
                expected: frame,
                actual: reconstructed.frame,
            });
        }
        
        // Check for reasonable physical values
        for body in &reconstructed.bodies {
            // Position bounds check
            if body.pos.x.abs() > reconstructed.domain_width * 2.0 ||
               body.pos.y.abs() > reconstructed.domain_height * 2.0 {
                warnings.push(ValidationIssue::BodyOutOfBounds {
                    id: body.id,
                    pos: body.pos,
                });
            }
            
            // Velocity sanity check (very high velocities might indicate corruption)
            let speed = body.vel.mag();
            if speed > 100.0 { // 100 Å/fs is extremely fast
                warnings.push(ValidationIssue::ExtremeVelocity {
                    id: body.id,
                    velocity: speed,
                });
            }
            
            // Charge reasonableness
            if body.charge.abs() > 10.0 { // More than 10 elementary charges is unusual
                warnings.push(ValidationIssue::ExtremeCharge {
                    id: body.id, 
                    charge: body.charge,
                });
            }
        }
        
        // Check body-to-foil mapping consistency
        for (&body_id, &foil_id) in &reconstructed.body_to_foil {
            if !reconstructed.bodies.iter().any(|b| b.id == body_id) {
                issues.push(ValidationIssue::OrphanedMapping {
                    body_id,
                    foil_id,
                });
            }
            if !reconstructed.foils.iter().any(|f| f.id == foil_id) {
                issues.push(ValidationIssue::OrphanedMapping {
                    body_id,
                    foil_id,
                });
            }
        }
        
        Ok(ValidationResult {
            frame,
            is_valid: issues.is_empty(),
            issues,
            warnings,
        })
    }
}

/// Errors that can occur during frame reconstruction
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub enum ReconstructionError {
    /// No keyframe available for the requested frame
    NoKeyframeAvailable { frame: usize },
    /// Required delta is missing from the chain
    MissingDelta { frame: usize, target_frame: usize },
    /// Unknown body ID encountered in delta
    UnknownBodyId { id: u64, frame: usize },
    /// Unknown foil ID encountered in delta
    UnknownFoilId { id: u64, frame: usize },
    /// General reconstruction failure
    ReconstructionFailed { frame: usize },
}

impl std::fmt::Display for ReconstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKeyframeAvailable { frame } => {
                write!(f, "No keyframe available for frame {}", frame)
            }
            Self::MissingDelta { frame, target_frame } => {
                write!(f, "Missing delta for frame {} (reconstructing frame {})", frame, target_frame)
            }
            Self::UnknownBodyId { id, frame } => {
                write!(f, "Unknown body ID {} at frame {}", id, frame)
            }
            Self::UnknownFoilId { id, frame } => {
                write!(f, "Unknown foil ID {} at frame {}", id, frame)
            }
            Self::ReconstructionFailed { frame } => {
                write!(f, "Failed to reconstruct frame {}", frame)
            }
        }
    }
}

impl std::error::Error for ReconstructionError {}

/// Result of validation checks on reconstructed data
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct ValidationResult {
    pub frame: usize,
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

/// Issues found during validation
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub enum ValidationIssue {
    /// Frame number mismatch
    FrameMismatch { expected: usize, actual: usize },
    /// Body position outside reasonable bounds
    BodyOutOfBounds { id: u64, pos: Vec2 },
    /// Extreme velocity detected
    ExtremeVelocity { id: u64, velocity: f32 },
    /// Extreme charge detected  
    ExtremeCharge { id: u64, charge: f32 },
    /// Body-to-foil mapping references non-existent entities
    OrphanedMapping { body_id: u64, foil_id: u64 },
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in GUI integration
pub struct MemoryStats {
    pub keyframe_count: usize,
    pub delta_count: usize,
    pub keyframe_memory_bytes: usize,
    pub delta_memory_bytes: usize,
    pub total_memory_bytes: usize,
    pub oldest_frame: usize,
    pub newest_frame: usize,
}

#[allow(dead_code)] // Will be used in GUI integration
impl MemoryStats {
    pub fn total_memory_mb(&self) -> f64 {
        self.total_memory_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn keyframe_memory_mb(&self) -> f64 {
        self.keyframe_memory_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn delta_memory_mb(&self) -> f64 {
        self.delta_memory_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn compression_ratio(&self) -> f64 {
        if self.keyframe_count == 0 { return 0.0; }
        let total_frames = self.keyframe_count + self.delta_count;
        let uncompressed_size = total_frames * 200_000; // Estimated uncompressed size per frame
        self.total_memory_bytes as f64 / uncompressed_size as f64
    }
}

use crate::simulation::Simulation;
use crate::io::SimulationState;

/// Conversion from current simulation state to lightweight format
impl From<&Simulation> for LightSnapshot {
    fn from(sim: &Simulation) -> Self {
        Self {
            frame: sim.frame,
            sim_time: sim.frame as f32 * sim.dt,
            dt: sim.dt,
            last_thermostat_time: sim.last_thermostat_time,
            
            bodies: sim.bodies.iter().map(LightBody::from).collect(),
            foils: sim.foils.iter().map(LightFoil::from).collect(),
            body_to_foil: sim.body_to_foil.clone(),
            config: sim.config.clone(),
            
            domain_width: sim.domain_width,
            domain_height: sim.domain_height,
            domain_depth: sim.domain_depth,
        }
    }
}

impl From<&SimulationState> for LightSnapshot {
    fn from(state: &SimulationState) -> Self {
        Self {
            frame: state.frame,
            sim_time: state.sim_time,
            dt: state.dt,
            last_thermostat_time: state.last_thermostat_time,
            
            bodies: state.bodies.iter().map(LightBody::from).collect(),
            foils: state.foils.iter().map(LightFoil::from).collect(),
            body_to_foil: state.body_to_foil.clone(),
            config: state.config.clone(),
            
            domain_width: state.domain_width,
            domain_height: state.domain_height,
            domain_depth: state.domain_depth,
        }
    }
}

/// Conversion back to full body (with computed values zeroed)
impl From<&LightBody> for Body {
    fn from(light: &LightBody) -> Self {
        let mut body = Body::new(
            light.pos,
            light.vel,
            light.mass,
            light.radius,
            light.charge,
            light.species,
        );
        body.id = light.id;
        body.z = light.z;
        body.vz = light.vz;
        body.electrons = light.electrons.iter().cloned().collect();
        body.surrounded_by_metal = light.surrounded_by_metal;
        body.last_surround_pos = light.last_surround_pos;
        body.last_surround_frame = light.last_surround_frame;
        // Note: acc, az, e_field are zeroed - will be recalculated
        body
    }
}

/// Conversion back to full foil (with PID history empty)
impl From<&LightFoil> for Foil {
    fn from(light: &LightFoil) -> Self {
        use crate::body::foil::{Foil, OverpotentialController};
        use std::collections::VecDeque;
        
        let overpotential_controller = if let (Some(target), Some(kp), Some(ki), Some(kd)) = 
            (light.overpotential_target, light.overpotential_kp, light.overpotential_ki, light.overpotential_kd) {
            Some(OverpotentialController {
                target_ratio: target,
                kp,
                ki,
                kd,
                integral_error: 0.0,
                previous_error: 0.0,
                max_current: 100.0,  // Default value
                last_output_current: 0.0,
                history: VecDeque::new(),  // Empty history - major memory savings!
                max_history_size: 1000,
                master_foil_id: None,
            })
        } else {
            None
        };
        
        Foil {
            id: light.id,
            body_ids: light.body_ids.clone(),
            dc_current: light.dc_current,
            ac_current: light.ac_current,
            accum: 0.0,  // Reset accumulator
            switch_hz: light.switch_hz,
            link_id: light.link_id,
            mode: light.mode.clone(),
            charging_mode: light.charging_mode.clone(),
            overpotential_controller,
            slave_overpotential_current: light.slave_overpotential_current,
        }
    }
}

/// Conversion from lightweight snapshot back to SimulationState
impl From<&LightSnapshot> for SimulationState {
    fn from(light: &LightSnapshot) -> Self {
        Self {
            frame: light.frame,
            sim_time: light.sim_time,
            dt: light.dt,
            last_thermostat_time: light.last_thermostat_time,
            
            bodies: light.bodies.iter().map(Body::from).collect(),
            foils: light.foils.iter().map(Foil::from).collect(),
            body_to_foil: light.body_to_foil.clone(),
            config: light.config.clone(),
            switch_config: crate::switch_charging::SwitchChargingConfig::default(),
            
            domain_width: light.domain_width,
            domain_height: light.domain_height,
            domain_depth: light.domain_depth,
            switch_step: None,
            group_a: Vec::new(),
            group_b: Vec::new(),
        }
    }
}