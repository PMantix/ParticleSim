//! Force calculation functions for the particle simulation.
//!
//! Provides routines for computing electric (Coulomb) forces and Lennard-Jones (LJ) forces between bodies.
//! Used by the main simulation loop to update accelerations and fields.

use crate::body::{Body, Species};
use crate::config;
use crate::simulation::Simulation;
use crate::profile_scope;
use rayon::prelude::*;

/// Coulomb's constant (scaled for simulation units).
pub const K_E: f32 = 8.988e3 * 0.5;

/// Compute electric field and force on all bodies using the quadtree.
///
/// - Builds the quadtree for the current body positions.
/// - Computes the electric field at each body due to all others.
/// - Adds background field and updates acceleration (F = qE).
pub fn attract(sim: &mut Simulation) {
    profile_scope!("forces_attract");
    sim.quadtree.build(&mut sim.bodies);
    sim.quadtree.field(&mut sim.bodies, K_E);
    for body in &mut sim.bodies {
        body.e_field += sim.background_e_field;
    }
    for body in &mut sim.bodies {
        // Convert force (qE) to acceleration by dividing by mass (a = F / m)
        body.acc = (body.charge * body.e_field) / body.mass;

    }
}

/// Apply Lennard-Jones (LJ) repulsive/attractive forces between lithium metals.
///
/// - Only applies to pairs of lithium metal atoms within a cutoff distance.
/// - Uses the quadtree to find neighbors efficiently.
/// - Forces are clamped to avoid instability.
pub fn apply_lj_forces(sim: &mut Simulation) {
    profile_scope!("forces_lj");
    let sigma = sim.config.lj_force_sigma;
    let epsilon = sim.config.lj_force_epsilon;
    let cutoff = sim.config.lj_force_cutoff * sigma;

    let bodies_ptr = std::ptr::addr_of!(sim.bodies) as *const Vec<Body>;
    let quadtree = &sim.quadtree;

    sim.bodies
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, body)| {
            if !(body.species == Species::LithiumMetal || body.species == Species::FoilMetal) {
                return;
            }

            let bodies = unsafe { &*bodies_ptr };
            let neighbors = quadtree.find_neighbors_within(bodies, i, cutoff);
            for &j in &neighbors {
                let other = &bodies[j];
                if !(other.species == Species::LithiumMetal || other.species == Species::FoilMetal) {
                    continue;
                }
                let r_vec = other.pos - body.pos;
                let r = r_vec.mag();
                if r < cutoff && r > 1e-6 {
                    let sr6 = (sigma / r).powi(6);
                    let max_lj_force = config::COLLISION_PASSES as f32 * config::LJ_FORCE_MAX;
                    let force_mag = (24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r)
                        .clamp(-max_lj_force, max_lj_force);
                    let force = force_mag * r_vec.normalized();
                    body.acc -= force / body.mass;
                }
            }
        });
}
