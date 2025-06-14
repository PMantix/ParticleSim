use crate::profile_scope;
use rayon::prelude::*;

/// Apply Lennard-Jones (LJ) forces between metals.
///
/// - Applies only between LithiumMetal and FoilMetal species.
/// - Selects between cell list or quadtree based on density.
/// - Uses parallel iteration for better performance.
pub fn apply_lj_forces(sim: &mut Simulation) {
    profile_scope!("forces_lj");

    let sigma = sim.config.lj_force_sigma;
    let epsilon = sim.config.lj_force_epsilon;
    let cutoff = sim.config.lj_force_cutoff * sigma;
    let max_lj_force = config::COLLISION_PASSES as f32 * config::LJ_FORCE_MAX;

    let use_cell = sim.use_cell_list();

    if use_cell {
        sim.cell_list.cell_size = cutoff;
        sim.cell_list.rebuild(&sim.bodies);
    } else {
        sim.quadtree.build(&mut sim.bodies);
    }

    // Shared immutable reference to bodies
    let bodies_ptr = &sim.bodies as *const Vec<Body>;

    sim.bodies
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, body)| {
            if !(body.species == Species::LithiumMetal || body.species == Species::FoilMetal) {
                return;
            }

            let bodies = unsafe { &*bodies_ptr };

            let neighbors = if use_cell {
                sim.cell_list.find_neighbors_within(bodies, i, cutoff)
            } else {
                sim.quadtree.find_neighbors_within(bodies, i, cutoff)
            };

            for &j in &neighbors {
                if j == i {
                    continue;
                }

                let other = &bodies[j];
                if !(other.species == Species::LithiumMetal || other.species == Species::FoilMetal) {
                    continue;
                }

                let r_vec = other.pos - body.pos;
                let r = r_vec.mag();

                if r < cutoff && r > 1e-6 {
                    let sr6 = (sigma / r).powi(6);
                    let force_mag = (24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r)
                        .clamp(-max_lj_force, max_lj_force);
                    let force = force_mag * r_vec.normalized();

                    body.acc -= force / body.mass;
                }
            }
        });
}
