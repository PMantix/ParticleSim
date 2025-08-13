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
    let z1 = b1.z;
    let z2 = b2.z;
    let r1 = b1.radius;
    let r2 = b2.radius;
    let d_xy = p2 - p1;
    let dz = z2 - z1;
    let r = r1 + r2;
    let dist_sq = d_xy.mag_sq() + dz * dz;
    if dist_sq > r * r {
        return;
    }
    let v1 = b1.vel;
    let v2 = b2.vel;
    let v1z = b1.vz;
    let v2z = b2.vz;
    let v_xy = v2 - v1;
    let vz = v2z - v1z;
    let d_dot_v = d_xy.dot(v_xy) + dz * vz;
    let m1 = b1.mass;
    let m2 = b2.mass;
    let weight1 = m2 / (m1 + m2);
    let weight2 = m1 / (m1 + m2);

    if d_dot_v >= 0.0 && dist_sq > 0.0 {
        let dist = dist_sq.sqrt();
        let corr = r / dist - 1.0;
        let tmpx = d_xy.x * corr;
        let tmpy = d_xy.y * corr;
        let tmpz = dz * corr;
        sim.bodies[i].pos.x -= weight1 * tmpx;
        sim.bodies[i].pos.y -= weight1 * tmpy;
        sim.bodies[i].z -= weight1 * tmpz;
        sim.bodies[j].pos.x += weight2 * tmpx;
        sim.bodies[j].pos.y += weight2 * tmpy;
        sim.bodies[j].z += weight2 * tmpz;
        return;
    }
    let v_sq = v_xy.mag_sq() + vz * vz;
    let d_sq = dist_sq;
    let r_sq = r * r;
    let correction_scale = 1.0 / num_passes as f32;
    let t = correction_scale * (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;
    sim.bodies[i].pos -= v1 * t;
    sim.bodies[i].z -= v1z * t;
    sim.bodies[j].pos -= v2 * t;
    sim.bodies[j].z -= v2z * t;
    let p1 = sim.bodies[i].pos;
    let p2 = sim.bodies[j].pos;
    let z1 = sim.bodies[i].z;
    let z2 = sim.bodies[j].z;
    let d_xy = p2 - p1;
    let dz = z2 - z1;
    let d_dot_v = d_xy.dot(v_xy) + dz * vz;
    let d_sq = d_xy.mag_sq() + dz * dz;
    let scale = 1.5 * d_dot_v / d_sq;
    let tmpx = d_xy.x * scale;
    let tmpy = d_xy.y * scale;
    let tmpz = dz * scale;
    let v1x = v1.x + tmpx * weight1;
    let v1y = v1.y + tmpy * weight1;
    let v1z_new = v1z + tmpz * weight1;
    let v2x = v2.x - tmpx * weight2;
    let v2y = v2.y - tmpy * weight2;
    let v2z_new = v2z - tmpz * weight2;
    sim.bodies[i].vel = Vec2::new(v1x, v1y);
    sim.bodies[i].vz = v1z_new;
    sim.bodies[j].vel = Vec2::new(v2x, v2y);
    sim.bodies[j].vz = v2z_new;
    sim.bodies[i].pos += Vec2::new(v1x, v1y) * t;
    sim.bodies[i].z += v1z_new * t;
    sim.bodies[j].pos += Vec2::new(v2x, v2y) * t;
    sim.bodies[j].z += v2z_new * t;
}
