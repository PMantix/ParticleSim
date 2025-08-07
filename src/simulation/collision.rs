// simulation/collision.rs
// Contains collision detection and resolution functions

use crate::profile_scope;
use crate::renderer::state::COLLISION_PASSES;
use crate::simulation::Simulation;
use ultraviolet::Vec2;

pub fn collide(sim: &mut Simulation) {
    profile_scope!("collision");
    if sim.quadtree_dirty {
        sim.quadtree.build(&mut sim.bodies);
        sim.quadtree_dirty = false;
    }
    let num_passes = *COLLISION_PASSES.lock();
    let max_radius = sim
        .bodies
        .iter()
        .map(|b| b.radius)
        .fold(0.0_f32, f32::max);
    let mut neighbors = Vec::new();
    for i in 0..sim.bodies.len() {
        let cutoff = sim.bodies[i].radius + max_radius;
        neighbors.clear();
        sim.quadtree
            .find_neighbors_within(&sim.bodies, i, cutoff, &mut neighbors);
        for &j in &neighbors {
            if j <= i {
                continue;
            }
            resolve(sim, i, j, num_passes);
        }
    }
    // Positions may have changed during resolution
    sim.quadtree_dirty = true;
}

fn resolve(sim: &mut Simulation, i: usize, j: usize, num_passes: usize) {
    let b1 = &sim.bodies[i];
    let b2 = &sim.bodies[j];
    let p1 = b1.pos;
    let p2 = b2.pos;
    let r1 = b1.radius;
    let r2 = b2.radius;
    let d = p2 - p1;
    let r = r1 + r2;
    if d.mag_sq() > r * r {
        return;
    }
    // Pre-collision velocities
    let v1_initial = b1.vel;
    let v2_initial = b2.vel;
    let v = v2_initial - v1_initial;
    let d_dot_v = d.dot(v);
    let m1 = b1.mass;
    let m2 = b2.mass;
    let weight1 = m2 / (m1 + m2);
    let weight2 = m1 / (m1 + m2);

    if d_dot_v >= 0.0 && d != Vec2::zero() {
        let tmp = d * (r / d.mag() - 1.0);
        sim.bodies[i].pos -= weight1 * tmp;
        sim.bodies[j].pos += weight2 * tmp;
        return;
    }
    let v_sq = v.mag_sq();
    let d_sq = d.mag_sq();
    let r_sq = r * r;
    let correction_scale = 1.0 / num_passes as f32;
    let t = correction_scale
        * (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt())
        / v_sq;
    sim.bodies[i].pos -= v1_initial * t;
    sim.bodies[j].pos -= v2_initial * t;
    let p1 = sim.bodies[i].pos;
    let p2 = sim.bodies[j].pos;
    let d = p2 - p1;
    let d_dot_v = d.dot(v);
    let d_sq = d.mag_sq();
    let tmp = d * (1.5 * d_dot_v / d_sq);
    let v1_final = v1_initial + tmp * weight1;
    let v2_final = v2_initial - tmp * weight2;
    
    // Apply standard collision velocities - simple approach since thermostat will handle temperature
    sim.bodies[i].vel = v1_final;
    sim.bodies[j].vel = v2_final;
    
    // Store final velocities to avoid borrow conflicts
    let final_vel1 = sim.bodies[i].vel;
    let final_vel2 = sim.bodies[j].vel;
    sim.bodies[i].pos += final_vel1 * t;
    sim.bodies[j].pos += final_vel2 * t;
}
