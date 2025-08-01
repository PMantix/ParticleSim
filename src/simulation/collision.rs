// simulation/collision.rs
// Contains collision detection and resolution functions

// Removed unused import: Body
use crate::profile_scope;
use crate::renderer::state::COLLISION_PASSES;
use crate::simulation::Simulation;
use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;

fn elastic_result(v1: Vec2, v2: Vec2, m1: f32, m2: f32, normal: Vec2) -> (Vec2, Vec2) {
    if normal == Vec2::zero() {
        return (v1, v2);
    }
    let v1n = v1.dot(normal);
    let v2n = v2.dot(normal);
    let v1t = v1 - normal * v1n;
    let v2t = v2 - normal * v2n;
    let v1n_after = (v1n * (m1 - m2) + 2.0 * m2 * v2n) / (m1 + m2);
    let v2n_after = (v2n * (m2 - m1) + 2.0 * m1 * v1n) / (m1 + m2);
    let v1_final = v1t + normal * v1n_after;
    let v2_final = v2t + normal * v2n_after;
    (v1_final, v2_final)
}

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

    // Thermal energy transfer
    let normal = if d != Vec2::zero() {
        d.normalized()
    } else {
        Vec2::zero()
    };
    let damped1 = sim.bodies[i].species.damping() < 1.0;
    let damped2 = sim.bodies[j].species.damping() < 1.0;
    if damped1 ^ damped2 {
        let (d_idx, u_idx, m_d, m_u, v_d_pre, v_u_pre) = if damped1 {
            (i, j, m1, m2, v1_initial, v2_initial)
        } else {
            (j, i, m2, m1, v2_initial, v1_initial)
        };
        let reservoir_before = sim.bodies[d_idx].thermal_reservoir;
        if reservoir_before > 0.0 {
            let thermal_speed = (2.0 * reservoir_before / m_d).sqrt();
            let thermal_vel = normal * thermal_speed;
            let (_v_d_base, v_u_base) = elastic_result(v_d_pre, v_u_pre, m_d, m_u, normal);
            let (_v_d_therm, v_u_therm) =
                elastic_result(v_d_pre + thermal_vel, v_u_pre, m_d, m_u, normal);
            let delta_e_full = 0.5 * m_u * (v_u_therm.mag_sq() - v_u_base.mag_sq());
            if delta_e_full > 0.0 {
                let injection = delta_e_full.min(reservoir_before);
                let delta_v_full = v_u_therm - v_u_base;
                let scale = (injection / delta_e_full).sqrt();
                let delta_v = delta_v_full * scale;
                sim.bodies[u_idx].vel += delta_v;
                sim.bodies[d_idx].thermal_reservoir -= injection;
            }
        }
        // Store any energy the undamped particle lost in the collision
        let v_u_after = sim.bodies[u_idx].vel;
        let delta_e = 0.5 * m_u * (v_u_after.mag_sq() - v_u_pre.mag_sq());
        if delta_e < 0.0 {
            sim.bodies[d_idx].thermal_reservoir += -delta_e;
        }
        if sim.bodies[d_idx].thermal_reservoir < 0.0 {
            sim.bodies[d_idx].thermal_reservoir = 0.0;
        }
    }
}
