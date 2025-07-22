//! Force calculation functions for the particle simulation.
//!
//! Provides routines for computing electric (Coulomb) forces and Lennard-Jones (LJ) forces between bodies.
//! Used by the main simulation loop to update accelerations and fields.

use crate::config;
use crate::simulation::Simulation;
use crate::profile_scope;

/// Coulomb's constant (scaled for simulation units). 8.988e3 * 0.5;
pub const K_E: f32 = 8.988e4 * 0.5;

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

/// Apply polarization forces for polar solvent molecules.
///
/// Each EC or DMC molecule carries a single bound electron that can drift
/// relative to the molecular center. The nucleus experiences the field at the
/// body position while the electron feels the field at its displaced position.
/// The force difference creates an effective dipole interaction.
pub fn apply_polar_forces(sim: &mut Simulation) {
    use crate::body::Species;
    profile_scope!("forces_polar");
    if sim.bodies.is_empty() { return; }
    let bodies_snapshot = sim.bodies.clone();
    let quadtree = &sim.quadtree;
    for (_i, body) in sim.bodies.iter_mut().enumerate() {
        if !matches!(body.species, Species::EC | Species::DMC) {
            continue;
        }
        if body.electrons.is_empty() { continue; }
        let e_pos = body.pos + body.electrons[0].rel_pos;
        let electron_field = quadtree.field_at_point(&bodies_snapshot, e_pos, K_E) + sim.background_e_field;
        let force = body.e_field - electron_field;
        body.acc += force / body.mass;
    }
}

/// Apply Lennard-Jones (LJ) forces between metals.
///
/// - Only applies to pairs within the LJ cutoff distance.
/// - Uses either the quadtree or a cell list depending on particle density.
/// - Forces are clamped to avoid instability.
pub fn apply_lj_forces(sim: &mut Simulation) {
    profile_scope!("forces_lj");
    let max_cutoff = crate::species::max_lj_cutoff();
    let use_cell = sim.use_cell_list();
    if use_cell {
        sim.cell_list.cell_size = max_cutoff;
        sim.cell_list.rebuild(&sim.bodies);
    } else {
        sim.quadtree.build(&mut sim.bodies);
    }

    for i in 0..sim.bodies.len() {
        if !sim.bodies[i].species.lj_enabled() { continue; }
        let neighbors = if use_cell {
            sim.cell_list.find_neighbors_within(&sim.bodies, i, max_cutoff)
        } else {
            sim.quadtree.find_neighbors_within(&sim.bodies, i, max_cutoff)
        };
        for &j in &neighbors {
            if j <= i { continue; }
            // Check if both particles have LJ enabled
            if !sim.bodies[i].species.lj_enabled() || !sim.bodies[j].species.lj_enabled() { continue; }
            let (a, b) = {
                let (left, right) = sim.bodies.split_at_mut(j);
                (&mut left[i], &mut right[0])
            };
            // Use per-species LJ parameters only
            let sigma = (a.species.lj_sigma() + b.species.lj_sigma()) * 0.5;
            let epsilon = (a.species.lj_epsilon() * b.species.lj_epsilon()).sqrt();
            let cutoff = 0.5 * (a.species.lj_cutoff() * a.species.lj_sigma() + b.species.lj_cutoff() * b.species.lj_sigma());
            let r_vec = b.pos - a.pos;
            let r = r_vec.mag();
            if r < cutoff && r > 1e-6 {
                let sr6 = (sigma / r).powi(6);
                let max_lj_force = config::COLLISION_PASSES as f32 * config::LJ_FORCE_MAX;
                let unclamped_force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;
                let force_mag = unclamped_force_mag.clamp(-max_lj_force, max_lj_force);
                let force = force_mag * r_vec.normalized();

                a.acc -= force / a.mass;
                b.acc += force / b.mass;
            }
        }
    }
}
