// Handles the generation of the initial conditions for the simulation.
//
// This module generates a uniform distribution of particles in a disc.
// The particles are assigned random positions, velocities, and masses.
// The particles are then sorted by their distance from the center of the disc.
// The mass of each particle is calculated based on its radius.
// The velocity of each particle is calculated based on the mass of the particles


use crate::body::Body;
use ultraviolet::Vec2;
//use rand::{rngs::ThreadRng};

pub fn uniform_disc(n: usize) -> Vec<Body> {
    fastrand::seed(0);
    let inner_radius = 0.0;
    let outer_radius = (n as f32).sqrt() * 1.50;

    let mut bodies: Vec<Body> = Vec::with_capacity(n);
	
    //let m = 1e9;
	//let rng: ThreadRng = rand::rng(); // new replacement for thread_rng()
    //let center = Body::new(Vec2::zero(), Vec2::zero(), m as f32, inner_radius, 1.0);
    //let center = Body::new(Vec2 {x:-100.0, y:0.00}, Vec2::zero(), m as f32, inner_radius, 1.0);
   // bodies.push(center);

    while bodies.len() < n {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let t = inner_radius / outer_radius;
        let r = fastrand::f32() * (1.0 - t * t) + t * t;
        let pos = Vec2::new(cos, sin) * outer_radius * r.sqrt();
		
        //let vel = Vec2::new(sin, -cos);
		let vel = Vec2::new(0.0,0.0);
        let mass = 1.0f32;
        let radius = mass.cbrt();

		let charge = if pos.x < 0.0 { 1.0 } else { -1.0 };
        bodies.push(Body::new(pos, vel, mass, radius, charge));
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