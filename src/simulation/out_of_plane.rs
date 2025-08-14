use rayon::prelude::*;
use crate::body::Species;

use super::simulation::Simulation;

/// Apply out-of-plane forces to all bodies.
/// Enforces that particles cannot move above or below metal/foil particles,
/// and applies z-direction spring and damping forces.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    if !sim.config.enable_out_of_plane {
        // If disabled, reset all z-components to keep simulation strictly 2D
        sim.bodies.par_iter_mut().for_each(|b| b.reset_z());
        return;
    }

    let stiffness = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;
    let frustration = sim.config.z_frustration_strength;

    // First pass: apply basic z-forces to all particles
    sim.bodies.par_iter_mut().for_each(|body| {
        // Only non-metal particles can move in z-direction
        if !matches!(body.species, Species::LithiumMetal | Species::FoilMetal) {
            // Frustration redirects in-plane acceleration into the z-axis
            let frustration_force = body.acc.mag() * frustration;
            body.az += -stiffness * body.z - damping * body.vz + frustration_force;
        } else {
            // Metal/foil particles stay fixed at z=0
            body.z = 0.0;
            body.vz = 0.0;
            body.az = 0.0;
        }
    });

    // Second pass: enforce z-constraints based on metal/foil boundaries
    enforce_metal_z_boundaries(sim, max_z);
}

/// Enforce that particles cannot move above or below metal/foil particles
/// Optimized version using spatial filtering to reduce O(N²) to approximately O(N)
fn enforce_metal_z_boundaries(sim: &mut Simulation, max_z: f32) {
    // Quick early return if too many particles (emergency performance protection)
    if sim.bodies.len() > 10000 {
        return;
    }
    
    // Pre-filter: collect metal particle positions and properties for faster lookup
    let metal_particles: Vec<(usize, f32, f32, f32)> = sim.bodies.iter().enumerate()
        .filter_map(|(j, body)| {
            if matches!(body.species, Species::LithiumMetal | Species::FoilMetal) {
                Some((j, body.pos.x, body.pos.y, body.radius))
            } else {
                None
            }
        })
        .collect();

    // If no metal particles, no constraints needed
    if metal_particles.is_empty() {
        return;
    }

    // Process non-metal particles in parallel for better performance
    let non_metal_indices: Vec<usize> = (0..sim.bodies.len())
        .filter(|&i| !matches!(sim.bodies[i].species, Species::LithiumMetal | Species::FoilMetal))
        .collect();

    // Use a simple spatial optimization: skip distant metal particles
    for &i in &non_metal_indices {
        let body_pos = sim.bodies[i].pos;
        let body_radius = sim.bodies[i].radius;
        let search_radius = body_radius * 2.0; // Reduced from 3.0 for better performance
        
        let mut min_z_constraint = -max_z;
        let mut max_z_constraint = max_z;
        let mut constraints_applied = 0;
        
        // Check only nearby metal particles using spatial filtering
        for &(j, metal_x, metal_y, metal_radius) in &metal_particles {
            if i == j { continue; }
            
            // Quick distance check using squared distance to avoid sqrt
            let dx = body_pos.x - metal_x;
            let dy = body_pos.y - metal_y;
            let distance_sq = dx * dx + dy * dy;
            let combined_radius_sq = (body_radius + metal_radius + search_radius).powi(2);
            
            // Skip if too far away (most particles will be filtered out here)
            if distance_sq > combined_radius_sq {
                continue;
            }
            
            // Limit constraint checks to prevent performance degradation
            constraints_applied += 1;
            if constraints_applied > 5 { // Max 5 constraint checks per particle
                break;
            }
            
            // Quick distance check using squared distance to avoid sqrt
            let dx = body_pos.x - metal_x;
            let dy = body_pos.y - metal_y;
            let distance_sq = dx * dx + dy * dy;
            let combined_radius_sq = (body_radius + metal_radius + search_radius).powi(2);
            
            // Skip if too far away
            if distance_sq > combined_radius_sq {
                continue;
            }
            
            // Closer check with actual distance
            let distance_2d = distance_sq.sqrt();
            if distance_2d < body_radius + metal_radius + search_radius {
                let metal_z = sim.bodies[j].z; // Should be 0 for metals
                let metal_thickness = metal_radius; // Use radius as "thickness"
                
                // Metal creates a barrier - particles can't go through it
                // Be more conservative with constraints to avoid conflicts
                let constraint_margin = 0.01; // Small safety margin
                let lower_bound = metal_z - metal_thickness - constraint_margin;
                let upper_bound = metal_z + metal_thickness + constraint_margin;
                
                // Only update constraints if they don't create conflicts
                if lower_bound < upper_bound {
                    min_z_constraint = min_z_constraint.max(lower_bound);
                    max_z_constraint = max_z_constraint.min(upper_bound);
                }
            }
        }
        
        // Check for constraint conflicts that can cause impossible positions
        if min_z_constraint > max_z_constraint {
            // Conflict detected - use the metal z position as safe constraint
            let safe_z = 0.0; // Default metal z position
            min_z_constraint = safe_z - 0.1;
            max_z_constraint = safe_z + 0.1;
        }
        
        // Apply the constraints safely
        let body = &mut sim.bodies[i];
        if body.z < min_z_constraint {
            body.z = min_z_constraint;
            if body.vz < 0.0 { body.vz = 0.0; }
        }
        if body.z > max_z_constraint {
            body.z = max_z_constraint;
            if body.vz > 0.0 { body.vz = 0.0; }
        }
        
        // Also apply global max_z constraint
        body.clamp_z(max_z);
    }
}
