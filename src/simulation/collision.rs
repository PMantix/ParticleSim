// simulation/collision.rs
// Contains collision detection and resolution functions

// Removed unused import: Body
use crate::renderer::state::COLLISION_PASSES;
use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;
use crate::simulation::Simulation;
use crate::profile_scope;

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
    let v1 = b1.vel;
    let v2 = b2.vel;
    let v = v2 - v1;
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
    let t = correction_scale * (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;
    sim.bodies[i].pos -= v1 * t;
    sim.bodies[j].pos -= v2 * t;
    let p1 = sim.bodies[i].pos;
    let p2 = sim.bodies[j].pos;
    let d = p2 - p1;
    let d_dot_v = d.dot(v);
    let d_sq = d.mag_sq();
    let tmp = d * (1.5 * d_dot_v / d_sq);
    let v1 = v1 + tmp * weight1;
    let v2 = v2 - tmp * weight2;
    sim.bodies[i].vel = v1;
    sim.bodies[j].vel = v2;
    sim.bodies[i].pos += v1 * t;
    sim.bodies[j].pos += v2 * t;
}
