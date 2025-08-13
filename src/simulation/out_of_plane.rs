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
fn enforce_metal_z_boundaries(sim: &mut Simulation, max_z: f32) {
    let len = sim.bodies.len();
    
    for i in 0..len {
        // Skip if this is a metal/foil particle (they stay at z=0)
        if matches!(sim.bodies[i].species, Species::LithiumMetal | Species::FoilMetal) {
            continue;
        }

        // Find nearby metal/foil particles that could constrain z-movement
        let mut min_z_constraint = -max_z;
        let mut max_z_constraint = max_z;
        
        let search_radius = sim.bodies[i].radius * 3.0; // Search nearby
        
        for j in 0..len {
            if i == j { continue; }
            
            // Only check against metal/foil particles
            if !matches!(sim.bodies[j].species, Species::LithiumMetal | Species::FoilMetal) {
                continue;
            }
            
            let distance_2d = (sim.bodies[i].pos - sim.bodies[j].pos).mag();
            
            // If we're within the 2D footprint of a metal particle
            if distance_2d < sim.bodies[i].radius + sim.bodies[j].radius + search_radius {
                let metal_z = sim.bodies[j].z; // Should be 0 for metals
                let metal_thickness = sim.bodies[j].radius; // Use radius as "thickness"
                
                // Metal creates a barrier - particles can't go through it
                min_z_constraint = min_z_constraint.max(metal_z - metal_thickness);
                max_z_constraint = max_z_constraint.min(metal_z + metal_thickness);
            }
        }
        
        // Apply the constraints
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
