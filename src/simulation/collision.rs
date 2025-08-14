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
    // DIAGNOSTIC: Validate inputs before building spatial tree to catch NaN/Inf or invalid radii
    let mut invalid_bodies: Vec<(usize, &str)> = Vec::new();
    for (idx, b) in sim.bodies.iter().enumerate() {
        if !b.pos.x.is_finite() || !b.pos.y.is_finite() {
            invalid_bodies.push((idx, "pos"));
        }
        if !b.vel.x.is_finite() || !b.vel.y.is_finite() {
            invalid_bodies.push((idx, "vel"));
        }
        if !b.z.is_finite() {
            invalid_bodies.push((idx, "z"));
        }
        if !b.vz.is_finite() {
            invalid_bodies.push((idx, "vz"));
        }
        if !b.radius.is_finite() || b.radius <= 0.0 {
            invalid_bodies.push((idx, "radius"));
        }
        if !b.mass.is_finite() || b.mass <= 0.0 {
            invalid_bodies.push((idx, "mass"));
        }
        if !b.charge.is_finite() {
            invalid_bodies.push((idx, "charge"));
        }
    }
    if !invalid_bodies.is_empty() {
        eprintln!("[DIAG][collision] Found {} invalid bodies before tree build", invalid_bodies.len());
        for (i, (idx, what)) in invalid_bodies.iter().take(8).enumerate() {
            let b = &sim.bodies[*idx];
            eprintln!(
                "[DIAG][collision] #{i} idx={idx} bad={what} pos=({:.3},{:.3}) z={:.3} r={:.3} vel=({:.3},{:.3}) vz={:.3} mass={:.3} q={:.3}",
                b.pos.x, b.pos.y, b.z, b.radius, b.vel.x, b.vel.y, b.vz, b.mass, b.charge
            );
        }
        if invalid_bodies.len() > 8 {
            eprintln!("[DIAG][collision] ... and {} more", invalid_bodies.len() - 8);
        }
    }
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
    // DIAGNOSTIC: sanity-check rects (recompute extents from bodies to avoid relying on Rect API)
    let mut bad_rects = 0usize;
    for (_rect, idx) in &rects {
        let b = &sim.bodies[*idx];
        let minx = b.pos.x - b.radius;
        let maxx = b.pos.x + b.radius;
        let miny = b.pos.y - b.radius;
        let maxy = b.pos.y + b.radius;
        let any_non_finite = !(minx.is_finite() && maxx.is_finite() && miny.is_finite() && maxy.is_finite());
        let bad_order = minx > maxx || miny > maxy;
        if any_non_finite || bad_order {
            if bad_rects < 8 {
                eprintln!(
                    "[DIAG][collision] bad rect for idx={} rect=({:.3},{:.3},{:.3},{:.3}) pos=({:.3},{:.3}) z={:.3} r={:.3}",
                    idx, minx, maxx, miny, maxy, b.pos.x, b.pos.y, b.z, b.radius
                );
            }
            bad_rects += 1;
        }
    }
    if bad_rects > 0 {
        eprintln!("[DIAG][collision] Found {} bad rects before tree build", bad_rects);
    }

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
    // DIAGNOSTIC: compute components of t and log anomalies
    let disc_term = (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0);
    let sqrt_disc = disc_term.sqrt();
    let numerator = d_dot_v + sqrt_disc;
    if !v_sq.is_finite() || v_sq == 0.0 || !numerator.is_finite() || !correction_scale.is_finite() {
        eprintln!(
            "[DIAG][resolve] i={} j={} anomalous t components: v_sq={:.6e} numerator={:.6e} disc_term={:.6e} d_dot_v={:.6e} d_sq={:.6e} r_sq={:.6e}",
            i, j, v_sq, numerator, disc_term, d_dot_v, d_sq, r_sq
        );
    }
    let t = correction_scale * numerator / v_sq;
    if !t.is_finite() {
        eprintln!(
            "[DIAG][resolve] i={} j={} non-finite t: t={:.6e} (scale={:.6e}, numerator={:.6e}, v_sq={:.6e})",
            i, j, t, correction_scale, numerator, v_sq
        );
    }
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
    if !scale.is_finite() {
        eprintln!(
            "[DIAG][resolve] i={} j={} non-finite scale: scale={:.6e} d_dot_v={:.6e} d_sq={:.6e}",
            i, j, scale, d_dot_v, d_sq
        );
    }
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

    // DIAGNOSTIC: verify results remain finite
    let bi = &sim.bodies[i];
    let bj = &sim.bodies[j];
    if !(bi.pos.x.is_finite() && bi.pos.y.is_finite() && bi.z.is_finite() && bi.vel.x.is_finite() && bi.vel.y.is_finite() && bi.vz.is_finite()) {
        eprintln!(
            "[DIAG][resolve] i={} produced non-finite state: pos=({:.3},{:.3}) z={:.3} vel=({:.3},{:.3}) vz={:.3}",
            i, bi.pos.x, bi.pos.y, bi.z, bi.vel.x, bi.vel.y, bi.vz
        );
    }
    if !(bj.pos.x.is_finite() && bj.pos.y.is_finite() && bj.z.is_finite() && bj.vel.x.is_finite() && bj.vel.y.is_finite() && bj.vz.is_finite()) {
        eprintln!(
            "[DIAG][resolve] j={} produced non-finite state: pos=({:.3},{:.3}) z={:.3} vel=({:.3},{:.3}) vz={:.3}",
            j, bj.pos.x, bj.pos.y, bj.z, bj.vel.x, bj.vel.y, bj.vz
        );
    }
}
