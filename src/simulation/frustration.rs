use std::collections::HashMap;
use ultraviolet::Vec2;
use crate::body::Body;

/// Tracks particles that are experiencing frustrated motion
#[derive(Debug)]
pub struct FrustrationTracker {
    /// Maps particle index to frustration data
    frustrated_particles: HashMap<usize, FrustrationData>,
    /// Configuration parameters
    pub config: FrustrationConfig,
}

#[derive(Debug, Clone)]
struct FrustrationData {
    /// How many consecutive timesteps this particle has been frustrated
    stuck_duration: u32,
    /// The particle's position when frustration started
    initial_stuck_position: Vec2,
    /// The magnitude of force trying to move the particle
    desired_force: f32,
    /// Recent position history for movement detection
    recent_positions: Vec<Vec2>,
}

#[derive(Debug, Clone)]
pub struct FrustrationConfig {
    /// Minimum force magnitude to consider for frustration (prevents noise)
    pub min_force_threshold: f32,
    /// Maximum distance particle can move while still considered "stuck"
    pub stuck_movement_threshold: f32,
    /// Number of timesteps needed to confirm frustration
    pub frustration_confirmation_steps: u32,
    /// How much to soften repulsion when frustrated (0.0 = no collision, 1.0 = normal collision)
    pub soft_repulsion_factor: f32,
    /// Maximum time a particle can remain in frustrated state
    pub max_frustration_duration: u32,
    /// Size of position history buffer
    pub position_history_size: usize,
}

impl Default for FrustrationConfig {
    fn default() -> Self {
        Self {
            min_force_threshold: 0.5,
            stuck_movement_threshold: 0.1,
            frustration_confirmation_steps: 8,
            soft_repulsion_factor: 0.3,
            max_frustration_duration: 50,
            position_history_size: 5,
        }
    }
}

impl FrustrationTracker {
    pub fn new() -> Self {
        Self {
            frustrated_particles: HashMap::new(),
            config: FrustrationConfig::default(),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_config(config: FrustrationConfig) -> Self {
        Self {
            frustrated_particles: HashMap::new(),
            config,
        }
    }

    /// Update frustration state for all particles
    pub fn update(&mut self, bodies: &[Body]) {
        // Check each particle for frustration
        for (i, body) in bodies.iter().enumerate() {
            self.update_particle_frustration(i, body);
        }

        // Clean up particles that are no longer frustrated or have exceeded max duration
        self.cleanup_resolved_frustration(bodies.len());
    }

    /// Check if a specific particle is currently frustrated
    pub fn is_frustrated(&self, particle_index: usize) -> bool {
        self.frustrated_particles.get(&particle_index)
            .map(|data| data.stuck_duration >= self.config.frustration_confirmation_steps)
            .unwrap_or(false)
    }

    /// Get the soft repulsion factor for a particle (1.0 = normal, <1.0 = softened)
    pub fn get_repulsion_factor(&self, particle_index: usize) -> f32 {
        if self.is_frustrated(particle_index) {
            self.config.soft_repulsion_factor
        } else {
            1.0 // Normal collision behavior
        }
    }

    /// Get frustration statistics for debugging
    #[allow(dead_code)]
    pub fn get_frustration_stats(&self) -> (usize, f32) {
        let frustrated_count = self.frustrated_particles.len();
        let avg_duration = if frustrated_count > 0 {
            self.frustrated_particles.values()
                .map(|data| data.stuck_duration as f32)
                .sum::<f32>() / frustrated_count as f32
        } else {
            0.0
        };
        (frustrated_count, avg_duration)
    }

    fn update_particle_frustration(&mut self, particle_index: usize, body: &Body) {
        let desired_force = body.acc.mag();
        
        // Only consider particles with significant forces acting on them
        if desired_force < self.config.min_force_threshold {
            self.frustrated_particles.remove(&particle_index);
            return;
        }

        let should_update_existing = self.frustrated_particles.contains_key(&particle_index);
        
        if should_update_existing {
            // Get data separately to avoid borrow issues
            let mut needs_removal = false;
            
            if let Some(frustration_data) = self.frustrated_particles.get_mut(&particle_index) {
                // Update position history
                frustration_data.recent_positions.push(body.pos);
                if frustration_data.recent_positions.len() > self.config.position_history_size {
                    frustration_data.recent_positions.remove(0);
                }

                // Check if particle is still stuck
                let movement_since_start = (body.pos - frustration_data.initial_stuck_position).mag();
                let is_still_stuck = movement_since_start < self.config.stuck_movement_threshold;

                if is_still_stuck && desired_force >= self.config.min_force_threshold {
                    // Still frustrated - increment duration
                    frustration_data.stuck_duration += 1;
                    frustration_data.desired_force = desired_force;
                } else {
                    // No longer stuck - mark for removal
                    needs_removal = true;
                }
            }
            
            if needs_removal {
                self.frustrated_particles.remove(&particle_index);
            }
        } else {
            // Start tracking potential frustration
            let frustration_data = FrustrationData {
                stuck_duration: 1,
                initial_stuck_position: body.pos,
                desired_force,
                recent_positions: vec![body.pos],
            };
            
            self.frustrated_particles.insert(particle_index, frustration_data);
        }
    }

    fn cleanup_resolved_frustration(&mut self, num_bodies: usize) {
        let max_duration = self.config.max_frustration_duration;
        
        self.frustrated_particles.retain(|&particle_index, data| {
            // Remove if particle no longer exists
            if particle_index >= num_bodies {
                return false;
            }

            // Remove if frustration resolved or exceeded max duration
            if data.stuck_duration == 0 || data.stuck_duration > max_duration {
                return false;
            }

            true
        });
    }
}

/// Apply frustrated motion soft repulsion to collision forces
pub fn apply_frustration_softening(
    tracker: &FrustrationTracker,
    particle1_index: usize,
    particle2_index: usize,
    collision_force: Vec2,
) -> Vec2 {
    // Get the minimum repulsion factor between the two particles
    let factor1 = tracker.get_repulsion_factor(particle1_index);
    let factor2 = tracker.get_repulsion_factor(particle2_index);
    let min_factor = factor1.min(factor2);
    
    // If either particle is frustrated, soften the collision
    collision_force * min_factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Species;

    #[test]
    fn test_frustration_detection() {
        let mut tracker = FrustrationTracker::new();
        
        // Create a test body that should be frustrated (high force, low movement)
        let mut body = Body::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            1.0,
            1.0,
            1.0,
            Species::LithiumCation,
        );
        body.acc = Vec2::new(2.0, 0.0); // High acceleration but can't move
        
        let bodies = vec![body];
        
        // Initially not frustrated
        assert!(!tracker.is_frustrated(0));
        
        // Update several times with same position (simulating stuck particle)
        for _ in 0..10 {
            tracker.update(&bodies);
        }
        
        // Should now be detected as frustrated
        assert!(tracker.is_frustrated(0));
        assert_eq!(tracker.get_repulsion_factor(0), tracker.config.soft_repulsion_factor);
    }
}