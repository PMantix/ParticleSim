// simulation/collision.rs
// Contains collision detection and resolution functions

use crate::profile_scope;
use crate::renderer::state::COLLISION_PASSES;
use crate::simulation::Simulation;
use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;

pub fn collide(sim: &mut Simulation) {
    profile_scope!("collision");
    let mut rects = sim
        .bodies
        .iter()
        .enumerate()
        .map(|(index, body)| {
            let pos = body.pos;
            let radius = body.radius;
            let min = pos - Vec2::one() * radius;
            let max = pos + Vec2::one() * radius;
            (Rect::new(min.x, max.x, min.y, max.y), index)
        })
        .collect::<Vec<_>>();
    let mut broccoli = broccoli::Tree::par_new(&mut rects);
    let ptr = sim as *mut Simulation as usize;
    let num_passes = *COLLISION_PASSES.lock();
    broccoli.par_find_colliding_pairs(|i, j| {
        let sim = unsafe { &mut *(ptr as *mut Simulation) };
        let i = *i.unpack_inner();
        let j = *j.unpack_inner();
        resolve(sim, i, j, num_passes);
    });
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
    sim.bodies[i].vel = v1_final;
    sim.bodies[j].vel = v2_final;
    sim.bodies[i].pos += v1_final * t;
    sim.bodies[j].pos += v2_final * t;

    // Thermal energy transfer - simple approach
    let damped1 = sim.bodies[i].species.damping() < 1.0;
    let damped2 = sim.bodies[j].species.damping() < 1.0;
    
    // New approach: Track energy lost from undamped particles and restore it randomly
    if damped1 ^ damped2 {
        // Identify which is undamped
        let (u_idx, m_u) = if damped1 {
            (j, m2)
        } else {
            (i, m1)
        };
        
        // Calculate energy of undamped particle before collision
        let v_u_before = if damped1 { v2_initial } else { v1_initial };
        let energy_before = 0.5 * m_u * v_u_before.mag_sq();
        
        // Get energy of undamped particle after collision
        let v_u_after = sim.bodies[u_idx].vel;
        let energy_after = 0.5 * m_u * v_u_after.mag_sq();
        
        // Calculate energy lost by the undamped particle
        let energy_lost = energy_before - energy_after;
        
        // If undamped particle lost energy, restore it with a random direction
        if energy_lost > 0.0 {
            // Generate a random unit vector for energy restoration
            let random_angle = rand::random::<f32>() * 2.0 * std::f32::consts::PI;
            let random_direction = Vec2::new(random_angle.cos(), random_angle.sin());
            
            // Calculate velocity magnitude needed to restore the lost energy
            let speed_to_add = (2.0 * energy_lost / m_u).sqrt();
            let velocity_to_add = random_direction * speed_to_add;
            
            // Add the random thermal velocity to the undamped particle
            sim.bodies[u_idx].vel += velocity_to_add;
        }
    }
    // Do NOT allow energy transfer or reservoir increase for damped-damped collisions
}
