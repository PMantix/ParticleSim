// Main compressed history system implementation
// Separated from other modules to reduce compilation coupling

use std::collections::{HashMap, VecDeque};
use super::{
    config::CompressionConfig,
    types::{LightSnapshot, LightBody, LightFoil},
    delta::{DeltaSnapshot, ReconstructionError, apply_body_delta, apply_foil_delta}
};

/// Memory usage statistics for the compressed history system
#[derive(Clone, Debug)]
pub struct MemoryStats {
    pub keyframe_count: usize,
    pub delta_count: usize,
    pub keyframe_memory_bytes: usize,
    pub delta_memory_bytes: usize,
    pub total_memory_bytes: usize,
}

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

/// Internal keyframe entry
#[derive(Clone, Debug)]
struct KeyframeEntry {
    frame: usize,
    snapshot: LightSnapshot,
}

/// Internal delta entry
#[derive(Clone, Debug)]
struct DeltaEntry {
    frame: usize,
    delta: DeltaSnapshot,
    keyframe_frame: usize, // Reference to the keyframe this delta is based on
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
        
        if self.should_create_keyframe(frame) {
            self.create_keyframe(snapshot);
        } else {
            self.create_delta(snapshot);
        }
        
        self.cleanup_old_data();
        self.cursor_frame = frame;
    }
    
    /// Check if we should create a keyframe for this frame
    #[inline]
    fn should_create_keyframe(&self, frame: usize) -> bool {
        // Create keyframe if no keyframes exist yet
        if self.keyframes.is_empty() {
            return true;
        }
        
        // Create keyframe at regular intervals
        let last_keyframe = self.keyframes.back().unwrap().frame;
        frame - last_keyframe >= self.config.keyframe_interval
    }
    
    /// Create a new keyframe entry
    fn create_keyframe(&mut self, snapshot: LightSnapshot) {
        let entry = KeyframeEntry {
            frame: snapshot.frame,
            snapshot: snapshot.clone(),
        };
        
        self.keyframes.push_back(entry);
        self.last_state = Some(snapshot);
    }
    
    /// Create a new delta entry
    fn create_delta(&mut self, snapshot: LightSnapshot) {
        if let Some(ref previous) = self.last_state {
            if let Some(delta) = self.generate_delta(previous, &snapshot) {
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
    }
    
    /// Generate delta between two snapshots
    fn generate_delta(&self, previous: &LightSnapshot, current: &LightSnapshot) -> Option<DeltaSnapshot> {
        let mut body_deltas = Vec::new();
        let mut new_bodies = Vec::new();
        let mut foil_deltas = Vec::new();
        let mut new_foils = Vec::new();
        
        // Compare bodies
        let mut body_map: HashMap<u64, &LightBody> = HashMap::new();
        for body in &previous.bodies {
            body_map.insert(body.species as u64, body); // Using species as ID for now
        }
        
        for (i, current_body) in current.bodies.iter().enumerate() {
            let body_id = current_body.species as u64;
            if let Some(previous_body) = body_map.get(&body_id) {
                if let Some(mut delta) = super::delta::compute_body_delta(current_body, previous_body) {
                    delta.id = i as u64;
                    body_deltas.push(delta);
                }
            } else {
                new_bodies.push(current_body.clone());
            }
        }
        
        // Compare foils
        let mut foil_map: HashMap<u64, &LightFoil> = HashMap::new();
        for foil in &previous.foils {
            foil_map.insert(foil.id, foil);
        }
        
        for current_foil in &current.foils {
            if let Some(previous_foil) = foil_map.get(&current_foil.id) {
                if let Some(delta) = super::delta::compute_foil_delta(current_foil, previous_foil) {
                    foil_deltas.push(delta);
                }
            } else {
                new_foils.push(current_foil.clone());
            }
        }
        
        // Check if anything actually changed
        if body_deltas.is_empty() && new_bodies.is_empty() && 
           foil_deltas.is_empty() && new_foils.is_empty() &&
           current.dt == previous.dt && current.last_thermostat_time == previous.last_thermostat_time {
            return None;
        }
        
        Some(DeltaSnapshot {
            frame: current.frame,
            dt_delta: if current.dt != previous.dt { Some(current.dt - previous.dt) } else { None },
            thermostat_delta: if current.last_thermostat_time != previous.last_thermostat_time { 
                Some(current.last_thermostat_time - previous.last_thermostat_time) 
            } else { 
                None 
            },
            body_deltas,
            new_bodies,
            foil_deltas,
            new_foils,
            body_to_foil_changes: if current.body_to_foil != previous.body_to_foil {
                // Compute actual changes rather than storing entire map
                let mut changes = HashMap::new();
                for (k, v) in &current.body_to_foil {
                    if previous.body_to_foil.get(k) != Some(v) {
                        changes.insert(*k, Some(*v));
                    }
                }
                for k in previous.body_to_foil.keys() {
                    if !current.body_to_foil.contains_key(k) {
                        changes.insert(*k, None);
                    }
                }
                Some(changes)
            } else {
                None
            },
            config_delta: None, // Config changes are rare
            domain_delta: if (current.domain_width, current.domain_height, current.domain_depth) != 
                            (previous.domain_width, previous.domain_height, previous.domain_depth) {
                Some((current.domain_width, current.domain_height, current.domain_depth))
            } else {
                None
            }
        })
    }
    
    /// Find the keyframe frame number that should be used as base for given frame
    #[inline]
    fn find_keyframe_for_frame(&self, frame: usize) -> usize {
        self.keyframes.iter()
            .rev()
            .find(|kf| kf.frame <= frame)
            .map(|kf| kf.frame)
            .unwrap_or(0)
    }
    
    /// Clean up old data based on configuration limits
    fn cleanup_old_data(&mut self) {
        // Remove old keyframes
        while self.keyframes.len() > self.config.max_keyframes {
            self.keyframes.pop_front();
        }
        
        // Remove old deltas
        while self.deltas.len() > self.config.max_deltas {
            self.deltas.pop_front();
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        let keyframe_memory = self.keyframes.len() * std::mem::size_of::<KeyframeEntry>();
        let delta_memory = self.deltas.len() * std::mem::size_of::<DeltaEntry>();
        
        MemoryStats {
            keyframe_count: self.keyframes.len(),
            delta_count: self.deltas.len(),
            keyframe_memory_bytes: keyframe_memory,
            delta_memory_bytes: delta_memory,
            total_memory_bytes: keyframe_memory + delta_memory,
        }
    }
    
    /// Check if a specific frame is available in history
    pub fn has_frame(&self, frame: usize) -> bool {
        self.keyframes.iter().any(|kf| kf.frame == frame) ||
        self.deltas.iter().any(|d| d.frame == frame)
    }
    
    /// Get the range of frames available in history
    pub fn get_frame_range(&self) -> Option<(usize, usize)> {
        let min_keyframe = self.keyframes.front().map(|kf| kf.frame);
        let min_delta = self.deltas.front().map(|d| d.frame);
        let min = match (min_keyframe, min_delta) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        
        let max_keyframe = self.keyframes.back().map(|kf| kf.frame);
        let max_delta = self.deltas.back().map(|d| d.frame);
        let max = match (max_keyframe, max_delta) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        
        match (min, max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }
    
    /// Reconstruct a specific frame from keyframes and deltas
    /// Returns the complete LightSnapshot for the requested frame
    pub fn reconstruct_frame(&self, frame: usize) -> Result<LightSnapshot, ReconstructionError> {
        // Find the most recent keyframe at or before the requested frame
        let keyframe = self.keyframes.iter()
            .rev()
            .find(|kf| kf.frame <= frame)
            .ok_or(ReconstructionError::MissingKeyframe(frame))?;
        
        let mut snapshot = keyframe.snapshot.clone();
        
        // Apply all deltas from keyframe to target frame
        for delta_entry in self.deltas.iter() {
            if delta_entry.keyframe_frame == keyframe.frame && 
               delta_entry.frame > keyframe.frame && 
               delta_entry.frame <= frame {
                
                // Apply delta to snapshot
                if let Some(dt_delta) = delta_entry.delta.dt_delta {
                    snapshot.dt += dt_delta;
                }
                
                if let Some(thermostat_delta) = delta_entry.delta.thermostat_delta {
                    snapshot.last_thermostat_time += thermostat_delta;
                }
                
                // Apply body deltas
                for body_delta in &delta_entry.delta.body_deltas {
                    if let Some(body) = snapshot.bodies.get_mut(body_delta.id as usize) {
                        apply_body_delta(body, body_delta)?;
                    }
                }
                
                // Add new bodies
                snapshot.bodies.extend(delta_entry.delta.new_bodies.iter().cloned());
                
                // Apply foil deltas
                for foil_delta in &delta_entry.delta.foil_deltas {
                    if let Some(foil) = snapshot.foils.iter_mut().find(|f| f.id == foil_delta.id) {
                        apply_foil_delta(foil, foil_delta)?;
                    }
                }
                
                // Add new foils
                snapshot.foils.extend(delta_entry.delta.new_foils.iter().cloned());
                
                // Apply body-to-foil changes
                if let Some(ref changes) = delta_entry.delta.body_to_foil_changes {
                    for (body_id, foil_id) in changes {
                        if let Some(foil_id) = foil_id {
                            snapshot.body_to_foil.insert(*body_id, *foil_id);
                        } else {
                            snapshot.body_to_foil.remove(body_id);
                        }
                    }
                }
                
                // Apply domain changes
                if let Some((width, height, depth)) = delta_entry.delta.domain_delta {
                    snapshot.domain_width = width;
                    snapshot.domain_height = height; 
                    snapshot.domain_depth = depth;
                }
                
                snapshot.frame = delta_entry.frame;
            }
        }
        
        Ok(snapshot)
    }
}