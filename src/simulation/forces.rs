// simulation/forces.rs
// Contains force calculation functions (attract, apply_lj_forces)

use crate::body::Species;
// Removed unused imports: Body, Quadtree, Vec2
use super::core::Simulation;

pub const K_E: f32 = 8.988e2 * 0.5;

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

pub fn apply_lj_forces(sim: &mut Simulation) {
    let sigma = 0.8;
    let epsilon = 500.0;
    let cutoff = 2.5 * sigma;
    for i in 0..sim.bodies.len() {
        if sim.bodies[i].species != Species::LithiumMetal {
            continue;
        }
        let neighbors = sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff);
        for &j in &neighbors {
            if j <= i { continue; }
            let (a, b) = {
                let (left, right) = sim.bodies.split_at_mut(j);
                (&mut left[i], &mut right[0])
            };
            if a.species == Species::LithiumMetal && b.species == Species::LithiumMetal {
                let r_vec = b.pos - a.pos;
                let r = r_vec.mag();
                if r < cutoff && r > 1e-6 {
                    let sr6 = (sigma / r).powi(6);
                    let max_lj_force = 100.0;
                    let unclamped_force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;
                    let force_mag = unclamped_force_mag.clamp(-max_lj_force, max_lj_force);
                    let force = force_mag * r_vec.normalized();
                    a.acc -= force / a.mass;
                    b.acc += force / b.mass;
                }
            }
        }
    }
}
