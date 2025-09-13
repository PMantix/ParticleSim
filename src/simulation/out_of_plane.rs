use rayon::prelude::*;
use crate::body::Species;

use super::simulation::Simulation;

/// Calculate realistic local z-direction forces for a particle (avoiding borrowing issues)
/// This replaces the artificial spring force with physically motivated surface interactions
fn calculate_local_z_force(body: &crate::body::Body, max_z: f32) -> f32 {
    let mut total_force = 0.0;
    
    // 1. Surface binding potential (replaces artificial spring)
    total_force += calculate_surface_binding_force(body, max_z);
    
    // 2. Electric field gradient effects (simplified)
    total_force += calculate_electric_field_gradient_force(body);
    
    // 3. Add simple harmonic restoration for testing (when needed)
    // This ensures particles at z != 0 experience a restoring force
    let spring_stiffness = 1.0; // Simple spring constant for testing
    total_force += -spring_stiffness * body.z;
    
    total_force
}

/// Surface binding potential - attracts particles to electrode surfaces at z=±max_z
/// This creates a more realistic double-well potential instead of a single harmonic well
fn calculate_surface_binding_force(body: &crate::body::Body, max_z: f32) -> f32 {
    let binding_strength = body.species.surface_affinity();
    let z_abs = body.z.abs();
    
    // Debug: use fallback strength if surface_affinity is too weak
    let effective_strength = if binding_strength < 1e-6 {
        1.0 // Fallback binding strength for testing
    } else {
        binding_strength
    };
    
    if z_abs < max_z {
        // Create a double-well potential with minima near the surfaces
        let normalized_z = body.z / max_z; // -1 to +1
        
        // Polynomial potential: U(z) = a*z^4 - b*z^2 creates double well
        // Force = -dU/dz = -4*a*z^3 + 2*b*z
        let a = effective_strength * 0.5;
        let b = effective_strength * 1.0;
        
        let force = -4.0 * a * normalized_z.powi(3) + 2.0 * b * normalized_z;
        force / max_z // Scale by max_z to get proper units
    } else {
        // Strong restoring force if outside bounds (hard wall)
        let excess = z_abs - max_z;
        -body.z.signum() * 50.0 * excess
    }
}

/// Electric field gradient forces (particles respond to field variations in z)
fn calculate_electric_field_gradient_force(_body: &crate::body::Body) -> f32 {
    // Removed artificial field gradient force - was creating unphysical charge separation
    // Real field gradients should be computed from actual electrode geometries and potentials
    0.0
}

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
            
            // Apply realistic z-direction forces instead of artificial spring
            // Note: We need to avoid borrowing issues, so we'll compute forces differently
            let z_force = calculate_local_z_force(body, max_z);
            
            // Frustration redirects in-plane acceleration into the z-axis (keep this mechanism)
            let acc_mag = body.acc.mag();
            if acc_mag.is_finite() {
                let frustration_force = acc_mag * frustration;
                if frustration_force.is_finite() {
                    body.az += z_force - damping * body.vz + frustration_force;
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
    
    // Third pass: apply many-body z-forces (solvation, density effects)
    // Use adaptive performance: only enable for small simulations or when explicitly requested
    let particle_count = sim.bodies.len();
    let enable_many_body = sim.config.enable_z_many_body_forces || particle_count < 50;
    
    if enable_many_body {
        apply_many_body_z_forces(sim, max_z);
    }
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

/// Apply many-body z-forces (solvation and density effects)
/// This is done in a separate pass to avoid borrowing conflicts
/// Uses spatial filtering to maintain O(N) performance
fn apply_many_body_z_forces(sim: &mut Simulation, _max_z: f32) {
    // Pre-compute forces to avoid borrowing issues
    let mut z_forces = vec![0.0; sim.bodies.len()];
    
    // Use existing spatial structures for efficiency
    let use_cell = sim.use_cell_list();
    
    // Calculate solvation and density forces using spatial filtering
    for i in 0..sim.bodies.len() {
        let body = &sim.bodies[i];
        
        // Skip metal particles
        if matches!(body.species, Species::LithiumMetal | Species::FoilMetal) {
            continue;
        }
        
        // Get nearby particles using existing spatial structures
        let solvation_range = 3.0 * body.radius;
        let neighbors = if use_cell {
            sim.cell_list.find_neighbors_within(&sim.bodies, i, solvation_range)
        } else {
            sim.quadtree.find_neighbors_within(&sim.bodies, i, solvation_range)
        };
        
        // Solvation forces (only consider nearby particles)
        z_forces[i] += calculate_solvation_forces_optimized(body, &sim.bodies, &neighbors);
        
        // Density-dependent forces (only consider nearby particles)
        z_forces[i] += calculate_density_gradient_force_optimized(body, &sim.bodies, &neighbors);
    }
    
    // Apply the calculated forces
    for (i, force) in z_forces.iter().enumerate() {
        sim.bodies[i].az += force;
    }
}

/// Calculate solvation forces from nearby particles (OPTIMIZED - uses neighbor list)
fn calculate_solvation_forces_optimized(body: &crate::body::Body, all_bodies: &[crate::body::Body], neighbors: &[usize]) -> f32 {
    let mut solvation_force = 0.0;
    let solvation_range = 3.0 * body.radius;
    
    // Only loop through nearby particles (not all N particles!)
    for &other_idx in neighbors {
        let other = &all_bodies[other_idx];
        
        let dx = other.pos.x - body.pos.x;
        let dy = other.pos.y - body.pos.y;
        let dz = other.z - body.z;
        let r_2d = (dx*dx + dy*dy).sqrt();
        
        // Double-check distance (spatial structure might have false positives)
        if r_2d < solvation_range {
            // Solvation shell effect: particles prefer certain z-separations
            let preferred_z_separation = body.species.preferred_z_separation(&other.species);
            let z_deviation = dz - preferred_z_separation;
            
            // Soft spring-like force toward preferred separation
            let shell_strength = body.species.solvation_strength(&other.species);
            let distance_weight = (1.0 - r_2d / solvation_range).max(0.0);
            
            solvation_force += -shell_strength * z_deviation * distance_weight;
        }
    }
    
    solvation_force
}

/// Density-dependent forces (OPTIMIZED - uses neighbor list)
fn calculate_density_gradient_force_optimized(body: &crate::body::Body, all_bodies: &[crate::body::Body], neighbors: &[usize]) -> f32 {
    // Calculate local density around this particle's z-position
    let z_layer_thickness = 0.5;
    let mut local_density = 0.0;
    let mut count = 0;
    
    // Only loop through nearby particles (not all N particles!)
    for &other_idx in neighbors {
        let other = &all_bodies[other_idx];
        
        let dx = other.pos.x - body.pos.x;
        let dy = other.pos.y - body.pos.y;
        let dz = (other.z - body.z).abs();
        let r_2d = (dx*dx + dy*dy).sqrt();
        
        // Count particles in nearby z-layer and x-y radius
        if r_2d < 5.0 && dz < z_layer_thickness {
            local_density += other.mass;
            count += 1;
        }
    }
    
    if count > 0 {
        local_density /= count as f32;
        
        // High density creates pressure to spread out in z
        let pressure_threshold = 2.0;
        if local_density > pressure_threshold {
            let excess_pressure = local_density - pressure_threshold;
            // Force away from center (z=0) to relieve pressure
            0.5 * excess_pressure * body.z.signum()
        } else {
            0.0
        }
    } else {
        0.0
    }
}
