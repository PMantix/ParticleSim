//! Force calculation functions for the particle simulation.
//!
//! Provides routines for computing electric (Coulomb) forces and Lennard-Jones (LJ) forces between bodies.
//! Used by the main simulation loop to update accelerations and fields.

use crate::body::Species;
use crate::config;
use crate::simulation::Simulation;

/// Coulomb's constant (scaled for simulation units).
pub const K_E: f32 = 8.988e2 * 0.5;

/// Compute electric field and force on all bodies using the quadtree.
///
/// - Builds the quadtree for the current body positions.
/// - Computes the electric field at each body due to all others.
/// - Adds background field and updates acceleration (F = qE).
pub fn attract(sim: &mut Simulation) {
    sim.quadtree.build(&mut sim.bodies);
    sim.quadtree.field(&mut sim.bodies, K_E);
    for body in &mut sim.bodies {
        body.e_field += sim.background_e_field;
    }
    for body in &mut sim.bodies {
        body.acc = body.charge * body.e_field;
    }
}

/// Apply Lennard-Jones (LJ) repulsive/attractive forces between lithium metals.
///
/// - Only applies to pairs of lithium metal atoms within a cutoff distance.
/// - Uses the quadtree to find neighbors efficiently.
/// - Forces are clamped to avoid instability.
pub fn apply_lj_forces(sim: &mut Simulation) {
    // Debug: Print all lithium metals in the simulation
    let mut metal_indices = vec![];
    for (i, b) in sim.bodies.iter().enumerate() {
        if b.species == Species::LithiumMetal {
            metal_indices.push(i);
        }
    }

    let sigma = sim.config.lj_force_sigma;
    let epsilon = sim.config.lj_force_epsilon;
    let cutoff = sim.config.lj_force_cutoff * sigma;
    for i in 0..sim.bodies.len() {
        // Only apply LJ to LithiumMetal or FoilMetal
        if !(sim.bodies[i].species == Species::LithiumMetal || sim.bodies[i].species == Species::FoilMetal) {
            continue;
        }
        let neighbors = sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff);
        for &j in &neighbors {
            if j <= i { continue; }
            let (a, b) = {
                let (left, right) = sim.bodies.split_at_mut(j);
                (&mut left[i], &mut right[0])
            };
            // Apply LJ between LithiumMetal and/or FoilMetal
            if (a.species == Species::LithiumMetal || a.species == Species::FoilMetal) &&
               (b.species == Species::LithiumMetal || b.species == Species::FoilMetal) {
                let r_vec = b.pos - a.pos;
                let r = r_vec.mag();
                if r < cutoff && r > 1e-6 {
                    let sr6 = (sigma / r).powi(6);
                    let max_lj_force = config::COLLISION_PASSES as f32 * config::LJ_FORCE_MAX;
                    let unclamped_force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;
                    let force_mag = unclamped_force_mag.clamp(-max_lj_force, max_lj_force);
                    let force = force_mag * r_vec.normalized();
                    //println!("LJ DEBUG: r={:.3}, sr6={:.3}, force_mag={:.3}, force=({:.3},{:.3})", r, sr6, force_mag, force.x, force.y);
                    // Update acceleration (SWAPPED SIGNS)
                    a.acc -= force / a.mass;
                    b.acc += force / b.mass;

                    // Debug print: LJ vs Coulomb force ratio if enabled
                    if sim.config.show_lj_vs_coulomb_ratio {
                        // Only print for nonzero charge pairs
                        if a.charge.abs() > 1e-6 && b.charge.abs() > 1e-6 {
                            let coulomb_force_mag = crate::simulation::forces::K_E * a.charge * b.charge / (r * r);
                            let ratio = if coulomb_force_mag.abs() > 1e-12 {
                                force_mag.abs() / coulomb_force_mag.abs()
                            } else {
                                0.0
                            };
                            println!("LJ force: {:.3e}, Coulomb force: {:.3e}, ratio (LJ/Coulomb): {:.3}", force_mag, coulomb_force_mag, ratio);
                        }
                    }

                    // Debug print: Show LJ force vector, positions, and direction
                    //println!("LJ DEBUG 2: i={}, j={}, r={:.4}, a.pos=({:.4},{:.4}), b.pos=({:.4},{:.4}), force=({:.4},{:.4})", i, j, r, a.pos.x, a.pos.y, b.pos.x, b.pos.y, force.x, force.y);
                }
            }
        }
    }
}
