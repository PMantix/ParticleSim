//! Force calculation functions for the particle simulation.
//!
//! Provides routines for computing electric (Coulomb) forces and Lennard-Jones (LJ) forces between bodies.
//! Used by the main simulation loop to update accelerations and fields.

use crate::config;
use crate::simulation::Simulation;
use crate::profile_scope;

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
            let k_e = sim.config.coulomb_constant;
            let field_from_source = |point: ultraviolet::Vec2,
                                     point_radius: f32,
                                     src_pos: ultraviolet::Vec2,
                                     src_radius: f32,
                                     src_charge: f32| {
                if src_charge.abs() < f32::EPSILON { return ultraviolet::Vec2::zero(); }
                let d = point - src_pos;
                let dist = d.mag();
                let min_sep = point_radius + src_radius;
                let r_eff = dist.max(min_sep);
                let denom = (r_eff * r_eff + epsilon_sq) * r_eff;
                d * (k_e * src_charge / denom)
            };

            // Total field at i's nucleus/electron from j's net charge and (if present) j's dipole
            let i_nuc_pos = sim.bodies[i].pos;
            let i_nuc_rad = sim.bodies[i].radius;
            let i_ele_pos = e_pos;

            // j source terms
            let j_pos = sim.bodies[j].pos;
            let j_rad = sim.bodies[j].radius;
            let j_q = sim.bodies[j].charge;
            let j_has_dipole = matches!(sim.bodies[j].species, Species::EC | Species::DMC)
                && !sim.bodies[j].electrons.is_empty();
            let j_e_pos = if j_has_dipole {
                j_pos + sim.bodies[j].electrons[0].rel_pos
            } else { j_pos };
            let j_q_eff = if j_has_dipole { sim.bodies[j].species.polar_charge() } else { 0.0 };

            // Field at nucleus
            let mut field_nucleus = ultraviolet::Vec2::zero();
            // From j net charge
            field_nucleus += field_from_source(i_nuc_pos, i_nuc_rad, j_pos, j_rad, j_q);
            // From j dipole (+q_eff at nucleus, -q_eff at electron)
            if j_has_dipole {
                field_nucleus += field_from_source(i_nuc_pos, i_nuc_rad, j_pos, j_rad,  j_q_eff);
                field_nucleus -= field_from_source(i_nuc_pos, i_nuc_rad, j_e_pos, 0.0,  j_q_eff);
            }

            // Field at electron
            let mut field_electron = ultraviolet::Vec2::zero();
            field_electron += field_from_source(i_ele_pos, 0.0, j_pos, j_rad, j_q);
            if j_has_dipole {
                field_electron += field_from_source(i_ele_pos, 0.0, j_pos, j_rad,  j_q_eff);
                field_electron -= field_from_source(i_ele_pos, 0.0, j_e_pos, 0.0,  j_q_eff);
            }

            // If j contributes neither charge nor dipole, skip
            if field_nucleus == ultraviolet::Vec2::zero() && field_electron == ultraviolet::Vec2::zero() {
                continue;
            }

            let q_eff_i = sim.bodies[i].species.polar_charge();
            let force = (field_nucleus - field_electron) * q_eff_i;

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

/// Compute soft-core repulsive force between two bodies.
pub fn compute_repulsive_force(p1: &crate::body::Body, p2: &crate::body::Body, r_vec: ultraviolet::Vec2, r: f32) -> ultraviolet::Vec2 {
    let r0 = 0.5 * (p1.species.repulsion_cutoff() + p2.species.repulsion_cutoff());
    if r >= r0 || r <= 0.0 {
        return ultraviolet::Vec2::zero();
    }
    let k = 0.5 * (p1.species.repulsion_strength() + p2.species.repulsion_strength());
    let mag = k * (1.0 - r / r0) / r;
    r_vec * mag
}

/// Apply short-range repulsive forces when enabled for both species.
pub fn apply_repulsive_forces(sim: &mut Simulation) {
    profile_scope!("forces_repulsion");
    let max_cutoff = crate::species::max_repulsion_cutoff();
    if max_cutoff <= 0.0 { return; }
    let use_cell = sim.use_cell_list();
    if use_cell {
        sim.cell_list.cell_size = max_cutoff;
        sim.cell_list.rebuild(&sim.bodies);
    } else {
        sim.quadtree.build(&mut sim.bodies);
    }

    for i in 0..sim.bodies.len() {
        if !sim.bodies[i].species.repulsion_enabled() { continue; }
        let cutoff = sim.bodies[i].species.repulsion_cutoff();
        let neighbors = if use_cell {
            sim.cell_list.find_neighbors_within(&sim.bodies, i, cutoff)
        } else {
            sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff)
        };
        for &j in &neighbors {
            if j <= i { continue; }
            if !sim.bodies[j].species.repulsion_enabled() { continue; }
            let r_vec = sim.bodies[j].pos - sim.bodies[i].pos;
            let r = r_vec.mag();
            let f = compute_repulsive_force(&sim.bodies[i], &sim.bodies[j], r_vec, r);
            if f != ultraviolet::Vec2::zero() {
                let (a, b) = {
                    let (left, right) = sim.bodies.split_at_mut(j);
                    (&mut left[i], &mut right[0])
                };
                a.acc -= f / a.mass;
                b.acc += f / b.mass;
            }
        }
    }
}
