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

//use crate::body::Body;
//use ultraviolet::Vec2;

// pub fn uniform_disc(_n: usize) -> Vec<Body> {
//     let mut bodies = Vec::new();

//     // Particle A — stationary in the center
//     let pos_a = Vec2::new(0.0, 0.0);
//     let vel_a = Vec2::zero();
//     let mass_a = 1.0;
//     let radius_a = 10.0;
//     let charge_a = 1.0;

//     // Particle B — slightly overlapping, moving left
//     let pos_b = Vec2::new(15.0, 0.0); // slightly overlaps (10 + 10 = 20 > 15)
//     let vel_b = Vec2::new(-1.0, 0.0);
//     let mass_b = 1.0;
//     let radius_b = 10.0;
//     let charge_b = 0.0;

//     bodies.push(Body::new(pos_a, vel_a, mass_a, radius_a, charge_a));
//     bodies.push(Body::new(pos_b, vel_b, mass_b, radius_b, charge_b));

//     bodies
// }
