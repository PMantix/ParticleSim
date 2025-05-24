// simulation/core.rs
// Contains the Simulation struct and main methods (new, step)

use crate::{body::Body, quadtree::Quadtree, utils};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32, // half size of the bounding box
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
}

impl Simulation {
    pub fn new() -> Self {
        let dt = 0.0025;
        let n = 50000;
        let theta = 1.0;
        let epsilon = 2.0;
        let leaf_capacity = 1;
        let thread_capacity = 1024;
        let clump_size = 1000;
        let clump_radius = 20.0;
        let bounds = 350.0;
        let bodies = utils::two_lithium_clumps_with_ions(n, clump_size, clump_radius, bounds);
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let rewound_flags = vec![false; bodies.len()];
        Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            bounds,
            rewound_flags,
            background_e_field: Vec2::zero(),
        }
    }

    pub fn step(&mut self) {
        // Update uniform E-field from sliders
        {
            let mag = *FIELD_MAGNITUDE.lock();
            let theta = (*FIELD_DIRECTION.lock()).to_radians();
            self.background_e_field = Vec2::new(theta.cos(), theta.sin()) * mag;
        }
        // Reset rewound flags
        for flag in &mut self.rewound_flags {
            *flag = false;
        }
        self.dt = *TIMESTEP.lock();
        // Reset all accelerations
        for body in &mut self.bodies {
            body.acc = Vec2::zero();
        }
        // Compute forces
        forces::attract(self);
        forces::apply_lj_forces(self);
        // Integrate equations of motion
        self.iterate();
        // Collision passes
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        // Update electrons for each Li metal atom
        for body in &mut self.bodies {
            body.set_electron_count();
            body.update_electrons(body.e_field, self.dt);
        }
        self.frame += 1;
    }

    pub fn iterate(&mut self) {
        let damping = 0.999;
        for body in &mut self.bodies {
            body.vel += body.acc * self.dt;
            body.vel *= damping;
            body.pos += body.vel * self.dt;
            // Reflect from walls
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
}
