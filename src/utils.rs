// Handles the generation of the initial conditions for the simulation.
//
// This module generates a uniform distribution of particles in a disc.
// The particles are assigned random positions, velocities, and masses.
// The particles are then sorted by their distance from the center of the disc.
// The mass of each particle is calculated based on its radius.
// The velocity of each particle is calculated based on the mass of the particles

use crate::body::{Body, Species, Electron};
use ultraviolet::Vec2;

pub fn _uniform_disc(n: usize) -> Vec<Body> {
    fastrand::seed(0);
    let inner_radius = 0.0;
    let outer_radius = (n as f32).sqrt() * 1.50;

    let mut bodies: Vec<Body> = Vec::with_capacity(n);

    while bodies.len() < n {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let t = inner_radius / outer_radius;
        let r = fastrand::f32() * (1.0 - t * t) + t * t;
        let pos = Vec2::new(cos, sin) * outer_radius * r.sqrt();

        let vel = Vec2::new(0.0, 0.0);
        let mass = 0.5f32;
        let radius = 1.0 * mass.cbrt();

        let charge = if pos.x < 0.0 { 1.0 } else { -1.0 };

        let species = if charge > 0.5 {
            Species::LithiumIon
        } else if charge <= 0.0 {
            Species::LithiumMetal
        } else {
            Species::LithiumIon
        };

        bodies.push(Body::new(pos, vel, mass, radius, charge, species));
    }

    bodies.sort_by(|a, b| a.pos.mag_sq().total_cmp(&b.pos.mag_sq()));
    let mut mass = 0.0;
    for i in 0..n {
        mass += bodies[i].mass;
        if bodies[i].pos == Vec2::zero() {
            continue;
        }

        let v = (mass / bodies[i].pos.mag()).sqrt();
        bodies[i].vel *= v;
    }

    bodies
}

pub fn two_lithium_clumps_with_ions(
    n: usize,
    clump_size: usize,
    clump_radius: f32,
    domain_half_width: f32,
) -> Vec<Body> {
    fastrand::seed(0);
    let mut bodies = Vec::with_capacity(n);

    // Left clump center
    let left_center = Vec2::new(-domain_half_width * 0.6, 0.0);
    // Right clump center
    let right_center = Vec2::new(domain_half_width * 0.6, 0.0);
    // Middle clump center
    let center = Vec2::new(domain_half_width * 0.0, 0.0);

    // Helper to generate a random point in a disc
    let random_in_disc = |center: Vec2| {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let r = fastrand::f32().sqrt() * clump_radius;
        center + Vec2::new(cos, sin) * r
    };

    // Generate left clump
    for _ in 0..clump_size {
        let pos = random_in_disc(left_center);
        let vel = Vec2::zero();
        let mass:f32 = 1.0;
        let radius:f32 = 1.0 * mass.cbrt();
        let mut body = Body::new(pos, vel, mass, radius, 0.0, Species::LithiumMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }];
        body.update_charge_from_electrons();
        bodies.push(body);
    }

    // Generate right clump
    for _ in 0..clump_size {
        let pos = random_in_disc(right_center);
        let vel = Vec2::zero();
        let mass:f32 = 1.0;
        let radius:f32 = 1.0 * mass.cbrt();
        let mut body = Body::new(pos, vel, mass, radius, 0.0, Species::LithiumMetal);
        body.electrons = vec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }];
        body.update_charge_from_electrons();
        bodies.push(body);
    }

    // Generate middle lump (ions)
    for _ in 0..clump_size*2 {
        let pos = random_in_disc(center);
        let vel = Vec2::zero();
        let mass:f32 = 1.0;
        let radius = 1.0 * mass.cbrt();
        let mut body = Body::new(pos, vel, mass, radius, 0.0, Species::LithiumIon);
        body.electrons.clear();
        body.update_charge_from_electrons();
        bodies.push(body);
    }


    /*// Fill the rest with ions, randomly distributed
    let ions_to_add = n.saturating_sub(2 * clump_size);
    for _ in 0..ions_to_add {
        let x = fastrand::f32() * 2.0 * domain_half_width - domain_half_width;
        let y = (fastrand::f32() - 0.5) * 2.0 * domain_half_width;
        let pos = Vec2::new(x, y);
        let vel = Vec2::zero();
        let mass:f32 = 0.5;
        let radius = 1.0 * mass.cbrt();
        let charge = 1.0;
        bodies.push(Body::new(pos, vel, mass, radius, charge, Species::LithiumIon));
    }*/

    bodies
}
