// simulation/collision.rs
// Contains collision detection and resolution functions

// Removed unused import: Body
use crate::renderer::state::COLLISION_PASSES;
use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;
use crate::simulation::Simulation;
use crate::profile_scope;
use std::f32::consts::TAU;

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
    // Snapshot current state into locals (avoid holding immutable borrows)
    let mut p1 = sim.bodies[i].pos;
    let mut p2 = sim.bodies[j].pos;
    let mut z1 = sim.bodies[i].z;
    let mut z2 = sim.bodies[j].z;
    let r1 = sim.bodies[i].radius;
    let r2 = sim.bodies[j].radius;
    let mut d_xy = p2 - p1;
    let mut dz = z2 - z1;
    let r = r1 + r2;
    let mut dist_sq = d_xy.mag_sq() + dz * dz;

    // Check inputs (positions/velocities) and sanitize bodies if needed
    let mut v1 = sim.bodies[i].vel;
    let mut v2 = sim.bodies[j].vel;
    let mut v1z = sim.bodies[i].vz;
    let mut v2z = sim.bodies[j].vz;
    let mut need_sanitize = false;
    if !(p1.x.is_finite() && p1.y.is_finite() && z1.is_finite() && v1.x.is_finite() && v1.y.is_finite() && v1z.is_finite()) {
        need_sanitize = true;
    }
    if !(p2.x.is_finite() && p2.y.is_finite() && z2.is_finite() && v2.x.is_finite() && v2.y.is_finite() && v2z.is_finite()) {
        need_sanitize = true;
    }
    if need_sanitize || !dist_sq.is_finite() {
        eprintln!("[DIAG][resolve] i={} j={} sanitizing invalid inputs", i, j);
        for &k in &[i, j] {
            let b = &mut sim.bodies[k];
            if !b.pos.x.is_finite() { b.pos.x = 0.0; }
            if !b.pos.y.is_finite() { b.pos.y = 0.0; }
            if !b.vel.x.is_finite() { b.vel.x = 0.0; }
            if !b.vel.y.is_finite() { b.vel.y = 0.0; }
            if !b.z.is_finite() { b.z = 0.0; }
            if !b.vz.is_finite() { b.vz = 0.0; }
            if !b.az.is_finite() { b.az = 0.0; }
        }
        // Reload sanitized state
        p1 = sim.bodies[i].pos;
        p2 = sim.bodies[j].pos;
        z1 = sim.bodies[i].z;
        z2 = sim.bodies[j].z;
        d_xy = p2 - p1;
        dz = z2 - z1;
        dist_sq = d_xy.mag_sq() + dz * dz;
        v1 = sim.bodies[i].vel;
        v2 = sim.bodies[j].vel;
        v1z = sim.bodies[i].vz;
        v2z = sim.bodies[j].vz;
    }
    if dist_sq > r * r {
        return;
    }
    let v_xy = v2 - v1;
    let vz = v2z - v1z;
    let d_dot_v = d_xy.dot(v_xy) + dz * vz;
    let m1 = sim.bodies[i].mass;
    let m2 = sim.bodies[j].mass;
    let weight1 = m2 / (m1 + m2);
    let weight2 = m1 / (m1 + m2);

    if d_dot_v >= 0.0 && dist_sq > 0.0 && dist_sq.is_finite() {
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
    // Fallback: if distances or velocities are degenerate/non-finite, perform deterministic separation and exit
    if !d_sq.is_finite() || d_sq <= 1.0e-8 || !v_sq.is_finite() {
        // Separate along a deterministic direction based on indices
        let angle = ((i as u64) ^ ((j as u64).rotate_left(13))) as f32 * (TAU / 1024.0);
        let (s, c) = angle.sin_cos();
        let dir = Vec2::new(c, s);
        let sep = r * 1.001;
        let mid = (sim.bodies[i].pos + sim.bodies[j].pos) * 0.5;
        sim.bodies[i].pos = mid - dir * (sep * weight1);
        sim.bodies[j].pos = mid + dir * (sep * weight2);
        // Keep z together and clamped in range
        let depth = sim.domain_depth;
        let midz = ((sim.bodies[i].z + sim.bodies[j].z) * 0.5).clamp(-depth, depth);
        sim.bodies[i].z = midz;
        sim.bodies[j].z = midz;
        // Zero any non-finite velocities
        for &k in &[i, j] {
            let b = &mut sim.bodies[k];
            if !b.vel.x.is_finite() { b.vel.x = 0.0; }
            if !b.vel.y.is_finite() { b.vel.y = 0.0; }
            if !b.vz.is_finite() { b.vz = 0.0; }
        }
        return;
    }
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
        // Fallback to purely positional correction
        let dist = d_sq.sqrt();
        if dist.is_finite() && dist > 0.0 {
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
        }
        return;
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
    let scale = if d_sq.is_finite() && d_sq > 0.0 { 1.5 * d_dot_v / d_sq } else { 0.0 };
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
