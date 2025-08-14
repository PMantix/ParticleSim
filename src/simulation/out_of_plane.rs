use rayon::prelude::*;
use crate::body::Species;

use super::simulation::Simulation;

/// Apply out-of-plane forces to all bodies.
/// Enforces that particles cannot move above or below metal/foil particles,
/// and applies z-direction spring and damping forces.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    // debug log removed
    
    if !sim.config.enable_out_of_plane {
        // debug log removed
        // If disabled, reset all z-components to keep simulation strictly 2D
        sim.bodies.par_iter_mut().for_each(|b| b.reset_z());
        return;
    }

    let stiffness = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;
    let frustration = sim.config.z_frustration_strength;
    
    // debug log removed
    
    // Safety checks to prevent crashes
    if !stiffness.is_finite() || !damping.is_finite() || !max_z.is_finite() || !frustration.is_finite() {
        eprintln!("[ERROR] Invalid out-of-plane parameters detected! Disabling for safety.");
        return;
    }
    
    if max_z <= 0.0 {
        eprintln!("[ERROR] max_z must be positive! Got: {}", max_z);
        return;
    }

    // debug log removed
    // First pass: apply basic z-forces to all particles
    sim.bodies.par_iter_mut().for_each(|body| {
        // Only non-metal particles can move in z-direction
        if !matches!(body.species, Species::LithiumMetal | Species::FoilMetal) {
            // Safety check for NaN/infinite values
            if !body.z.is_finite() || !body.vz.is_finite() {
                body.reset_z(); // Emergency reset
                return;
            }
            
            // Frustration redirects in-plane acceleration into the z-axis
            let acc_mag = body.acc.mag();
            if acc_mag.is_finite() {
                let frustration_force = acc_mag * frustration;
                if frustration_force.is_finite() {
                    body.az += -stiffness * body.z - damping * body.vz + frustration_force;
                }
            }
            
            // Additional safety check after force calculation
            if !body.az.is_finite() {
                body.az = 0.0;
            }
        } else {
            // Metal/foil particles stay fixed at z=0
            body.z = 0.0;
            body.vz = 0.0;
            body.az = 0.0;
        }
    });

    // debug log removed
    // Second pass: enforce z-constraints based on metal/foil boundaries
    enforce_metal_z_boundaries(sim, max_z);
    // debug log removed
}

/// Enforce that particles cannot move above or below metal/foil particles
/// Optimized version using spatial filtering to reduce O(N²) to approximately O(N)
fn enforce_metal_z_boundaries(sim: &mut Simulation, max_z: f32) {
    // Safety check
    if !max_z.is_finite() || max_z <= 0.0 {
        eprintln!("[ERROR] Invalid max_z in boundary enforcement: {}", max_z);
        return;
    }
    // Early out if no metals present
    let any_metal = sim.bodies.iter().any(|b| matches!(b.species, Species::LithiumMetal | Species::FoilMetal));
    if !any_metal { return; }

    // Choose neighbor structure based on density
    let use_cell = sim.use_cell_list();
    // Pick a conservative cell size / ensure structures are up to date
    if use_cell {
        let metal_max_r = Species::LithiumMetal.radius().max(Species::FoilMetal.radius());
        sim.cell_list.cell_size = 4.0 * metal_max_r;
        sim.cell_list.rebuild(&sim.bodies);
    } else {
        // Reuse quadtree but ensure it's built
        sim.quadtree.build(&mut sim.bodies);
    }

    // Process non-metal particles
    for i in 0..sim.bodies.len() {
        if matches!(sim.bodies[i].species, Species::LithiumMetal | Species::FoilMetal) { continue; }
        let body_pos = sim.bodies[i].pos;
        let body_radius = sim.bodies[i].radius;
        let metal_max_r = Species::LithiumMetal.radius().max(Species::FoilMetal.radius());
        let cutoff = 3.0 * body_radius + metal_max_r;
        let neighbors = if use_cell {
            sim.cell_list.find_neighbors_within(&sim.bodies, i, cutoff)
        } else {
            sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff)
        };

        let mut min_z_constraint = -max_z;
        let mut max_z_constraint = max_z;
        let mut constraints_applied = 0;
        for &j in &neighbors {
            if !matches!(sim.bodies[j].species, Species::LithiumMetal | Species::FoilMetal) { continue; }
            if j == i { continue; }

            // Limit constraint checks to prevent performance degradation
            constraints_applied += 1;
            if constraints_applied > 5 { break; }

            let metal_pos = sim.bodies[j].pos;
            let metal_radius = sim.bodies[j].radius;
            let dx = body_pos.x - metal_pos.x;
            let dy = body_pos.y - metal_pos.y;
            let distance_sq = dx * dx + dy * dy;
            let thresh = (body_radius + metal_radius + 2.0 * body_radius).powi(2);
            if distance_sq > thresh { continue; }

            let distance_2d = distance_sq.sqrt();
            if distance_2d < body_radius + metal_radius + 2.0 * body_radius {
                let metal_z = sim.bodies[j].z; // typically 0 for metals
                let metal_thickness = metal_radius;
                let constraint_margin = 0.01;
                let lower_bound = metal_z - metal_thickness - constraint_margin;
                let upper_bound = metal_z + metal_thickness + constraint_margin;
                if lower_bound < upper_bound {
                    min_z_constraint = min_z_constraint.max(lower_bound);
                    max_z_constraint = max_z_constraint.min(upper_bound);
                }
            }
        }

        if min_z_constraint > max_z_constraint {
            let safe_z = 0.0;
            min_z_constraint = safe_z - 0.1;
            max_z_constraint = safe_z + 0.1;
        }

        let body = &mut sim.bodies[i];
        if body.z < min_z_constraint {
            body.z = min_z_constraint;
            if body.vz < 0.0 { body.vz = 0.0; }
        }
        if body.z > max_z_constraint {
            body.z = max_z_constraint;
            if body.vz > 0.0 { body.vz = 0.0; }
        }
        body.clamp_z(max_z);
    }
}
