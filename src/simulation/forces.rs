//! Force calculation functions for the particle simulation.
//!
//! Provides routines for computing electric (Coulomb) forces and Lennard-Jones (LJ) forces between bodies.
//! Used by the main simulation loop to update accelerations and fields.

use crate::config;
use crate::simulation::Simulation;
use crate::profile_scope;

/// Coulomb's constant (scaled for simulation units). 8.988e3 * 0.5;
pub const K_E: f32 = 8.988e3 * 0.5;

/// Compute electric field and force on all bodies using the quadtree.
///
/// - Builds the quadtree for the current body positions.
/// - Computes the electric field at each body due to all others.
/// - Adds background field and updates acceleration (F = qE).
pub fn attract(sim: &mut Simulation) {
    profile_scope!("forces_attract");
    sim.quadtree.build(&mut sim.bodies);
    sim.quadtree.field(&mut sim.bodies, sim.config.coulomb_constant);
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

    if sim.bodies.is_empty() {
        return;
    }

    let epsilon_sq = config::QUADTREE_EPSILON * config::QUADTREE_EPSILON;

    // Optionally use the cell list for neighbor search at high densities.
    let use_cell = sim.use_cell_list();
    if use_cell {
        let max_cutoff = 3.0 * crate::species::max_lj_cutoff();
        sim.cell_list.cell_size = max_cutoff;
        sim.cell_list.rebuild(&sim.bodies);
    }

    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::EC | Species::DMC) {
            continue;
        }
        if sim.bodies[i].electrons.is_empty() {
            continue;
        }

        let e_pos = sim.bodies[i].pos + sim.bodies[i].electrons[0].rel_pos;
        let cutoff = 3.0 * sim.bodies[i].radius;
        let neighbors = if use_cell {
            sim.cell_list.find_neighbors_within(&sim.bodies, i, cutoff)
        } else {
            sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff)
        };

        for &j in &neighbors {
            if sim.bodies[j].charge.abs() < f32::EPSILON {
                continue;
            }

            let k_e = sim.config.coulomb_constant;
            let field_from = |point: ultraviolet::Vec2, point_radius: f32| {
                let d = point - sim.bodies[j].pos;
                let dist = d.mag();
                let min_sep = point_radius + sim.bodies[j].radius;
                let r_eff = dist.max(min_sep);
                let denom = (r_eff * r_eff + epsilon_sq) * r_eff;
                d * (k_e * sim.bodies[j].charge / denom)
            };

            let field_nucleus = field_from(sim.bodies[i].pos, sim.bodies[i].radius);
            let field_electron = field_from(e_pos, 0.0);
            let q_eff = sim.bodies[i].species.polar_charge();
            let force = (field_nucleus - field_electron) * q_eff;

            if i < j {
                let (left, right) = sim.bodies.split_at_mut(j);
                let a = &mut left[i];
                let b = &mut right[0];
                a.acc += force / a.mass;
                b.acc -= force / b.mass;
            } else {
                let (left, right) = sim.bodies.split_at_mut(i);
                let b = &mut left[j];
                let a = &mut right[0];
                a.acc += force / a.mass;
                b.acc -= force / b.mass;
            }
        }
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
