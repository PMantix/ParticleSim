// Contains the simulation struct and its methods
// for updating the simulation. The simulation struct contains the bodies, quadtree, and other parameters.
// Handles the simulation step, collision detection, and resolution.


pub const K_E: f32 = 8.988e1*0.5;  // Coulomb's constant
use crate::{body::Body, quadtree::Quadtree, utils};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use crate::body::Species;

use broccoli::aabb::Rect;
use broccoli_rayon::{build::RayonBuildPar, prelude::RayonQueryPar};
use ultraviolet::Vec2;

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32, // half size of the bounding box
    pub rewound_flags: Vec<bool>,
    //uniform background E field F = q*E
    pub background_e_field: Vec2
}

impl Simulation {
    pub fn new() -> Self {
        let dt = 0.0025;
        let n = 50000;
        let theta = 1.0;
        let epsilon = 2.0;
        let leaf_capacity = 1;
        let thread_capacity = 1024;

        let bounds = 350.0;

        let bodies: Vec<Body> = utils::uniform_disc(n);
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let rewound_flags = vec![false; bodies.len()];

        Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            bounds,
            rewound_flags,
            background_e_field:Vec2::zero(),
        }
    }

    pub fn step(&mut self) {
        //1.read the E-field sliders and update the uniform field ——
        {
            let mag   = *FIELD_MAGNITUDE.lock();
            let theta = (*FIELD_DIRECTION.lock()).to_radians();
            self.background_e_field = Vec2::new(theta.cos(), theta.sin()) * mag;
        }
        //2.reset the rewound flags, allowing particles to be "hit" again
        for flag in &mut self.rewound_flags {
            *flag = false;
        }

        self.dt = *TIMESTEP.lock();

        // 3. Reset all accelerations
        for body in &mut self.bodies {
            body.acc = Vec2::zero();
        }

        //4. compute the forces on the particles
        self.attract();
        self.apply_lj_forces();

        //5. Integrate the equations of motion
        self.iterate();

        // 6. Check for collisions
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes  {
            self.collide();
        }

        // 5b. Update electrons for each Li metal atom
        for body in &mut self.bodies {
            body.set_electron_count();
            // Use the net field (acceleration) already computed for this body
            body.update_electrons(body.e_field, self.dt);
        }
        
        self.frame += 1;
    }

    pub fn attract(&mut self) {
        self.quadtree.build(&mut self.bodies);
        self.quadtree.acc(&mut self.bodies, K_E);

        // Add the uniform E-field term: F = qE
        for body in &mut self.bodies {
            body.acc += body.charge * self.background_e_field;
        }

        // Store the net electric field (Coulomb + background) for electron polarization
        for body in &mut self.bodies {
            body.e_field = body.acc; // Save before LJ is applied!
        }
    }

    pub fn apply_lj_forces(&mut self) {
        let sigma = 1.0;   // tune for your system
        let epsilon = 80.0; // tune for your system

        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (a, b) = {
                    let (left, right) = self.bodies.split_at_mut(j);
                    (&mut left[i], &mut right[0])
                };
                // All code using a and b must be inside this block!
                if a.species == Species::LithiumMetal && b.species == Species::LithiumMetal {
                    let r_vec = b.pos - a.pos;
                    let r = r_vec.mag();
                    let cutoff = 2.5 * sigma;
                    if r < cutoff && r > 1e-6 {
                        let sr6 = (sigma / r).powi(6);
                        let force_mag = 24.0 * epsilon * (2.0 * sr6 * sr6 - sr6) / r;
                        let force = force_mag * r_vec.normalized();
                        a.acc -= force / a.mass;
                        b.acc += force / b.mass;
                    }
                }
            }
        }
    }

    pub fn iterate(&mut self) {
        let damping = 0.999; // Try 0.999 or 0.995 for stronger damping
        for body in &mut self.bodies {
            body.vel += body.acc * self.dt;
            body.vel *= damping; // <-- Damping applied here
            body.pos += body.vel * self.dt;

            // Reflect from walls (existing code)
            for axis in 0..2 {
                let pos = if axis == 0 { &mut body.pos.x } else { &mut body.pos.y };
                let vel = if axis == 0 { &mut body.vel.x } else { &mut body.vel.y };

                if *pos < -self.bounds {
                    *pos = -self.bounds;
                    *vel = -(*vel);
                } else if *pos > self.bounds {
                    *pos = self.bounds;
                    *vel = -(*vel);
                }
            }
        }
    }

    pub fn collide(&mut self) {
        let mut rects = self
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

        let ptr = self as *mut Self as usize;
        
        let num_passes = *COLLISION_PASSES.lock();

        broccoli.par_find_colliding_pairs(|i, j| {
            let sim = unsafe { &mut *(ptr as *mut Self) };

            let i = *i.unpack_inner();
            let j = *j.unpack_inner();

            sim.resolve(i, j, num_passes);
        });
    }

    fn resolve(&mut self, i: usize, j: usize, num_passes: usize) {
        let b1 = &self.bodies[i];
        let b2 = &self.bodies[j];

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
            self.bodies[i].pos -= weight1 * tmp;
            self.bodies[j].pos += weight2 * tmp;
            return;
        }

        let v_sq = v.mag_sq();
        let d_sq = d.mag_sq();
        let r_sq = r * r;

        let correction_scale = 1.0 / num_passes as f32; // e.g. 0.5 if running 2 passes
        let t = correction_scale * (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        //let t = (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        self.bodies[i].pos -= v1 * t;
        self.bodies[j].pos -= v2 * t;

        let p1 = self.bodies[i].pos;
        let p2 = self.bodies[j].pos;
        let d = p2 - p1;
        let d_dot_v = d.dot(v);
        let d_sq = d.mag_sq();

        let tmp = d * (1.5 * d_dot_v / d_sq);
        let v1 = v1 + tmp * weight1;
        let v2 = v2 - tmp * weight2;

        self.bodies[i].vel = v1;
        self.bodies[j].vel = v2;
        self.bodies[i].pos += v1 * t;
        self.bodies[j].pos += v2 * t;

    }
}

#[test]
fn test_body_id_unique() {
    use crate::body::{Body, Species};
    use ultraviolet::Vec2;
    let b1 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
    let b2 = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
    assert_ne!(b1.id, b2.id);
}

#[test]
fn test_lj_force_repulsion() {
    use crate::simulation::Simulation;
    use crate::body::{Body, Species};
    use ultraviolet::Vec2;
    let mut sim = Simulation::new();
    sim.bodies = vec![
        Body::new(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal),
        Body::new(Vec2::new(0.9, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal),
    ];
    sim.apply_lj_forces();
    // Should have nonzero acceleration in opposite directions
    assert!(sim.bodies[0].acc.x < 0.0);
    assert!(sim.bodies[1].acc.x > 0.0);
}

#[test]
fn test_change_charge_command() {
    use crate::renderer::state::{SimCommand};
    use crate::body::{Body, Species};
    use ultraviolet::Vec2;
    let mut body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
    let id = body.id;
    let mut bodies = vec![body];
    let cmd = SimCommand::ChangeCharge { id, delta: 2.0 };
    if let SimCommand::ChangeCharge { id, delta } = cmd {
        if let Some(b) = bodies.iter_mut().find(|b| b.id == id) {
            b.charge += delta;
            b.update_species(); // <-- fix here
        }
    }
    assert_eq!(bodies[0].charge, 2.0);
}
