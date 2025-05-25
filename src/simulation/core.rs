// simulation/core.rs
// Contains the Simulation struct and main methods (new, step)

use crate::{body::{Body, Species}, quadtree::Quadtree, utils};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use crate::config::{HOP_RADIUS_FACTOR, HOP_CHARGE_THRESHOLD};

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
        let dt = config::DEFAULT_DT;
        let n = config::DEFAULT_PARTICLE_COUNT;
        let theta = config::QUADTREE_THETA;
        let epsilon = config::QUADTREE_EPSILON;
        let leaf_capacity = config::QUADTREE_LEAF_CAPACITY;
        let thread_capacity = config::QUADTREE_THREAD_CAPACITY;
        let clump_size = config::CLUMP_SIZE;
        let clump_radius = config::CLUMP_RADIUS;
        let bounds = config::DOMAIN_BOUNDS;
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
            //body.set_electron_count();
            body.update_electrons(body.e_field, self.dt);
            body.update_charge_from_electrons();
        }

        // Perform electron hopping pass
        self.perform_electron_hopping();

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

    pub fn perform_electron_hopping(&mut self) {
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];

        for src_idx in 0..n {
            let src_body = &self.bodies[src_idx];
            if src_body.species != Species::LithiumMetal || src_body.electrons.len() <= 1 {
                continue;
            }
            let hop_radius = HOP_RADIUS_FACTOR * src_body.radius;
            // Find a neighbor with higher charge (less negative)
            if let Some(dst_idx) = self.bodies
                .iter()
                .enumerate()
                .filter(|&(j, b)| j != src_idx && b.species == Species::LithiumMetal)
                .filter(|(_, b)| (b.pos - src_body.pos).mag() <= hop_radius)
                .filter(|(_, b)| b.charge > src_body.charge) // dst is less negative
                .min_by(|(_, a), (_, b)| {
                    let da = a.charge - src_body.charge;
                    let db = b.charge - src_body.charge;
                    da.partial_cmp(&db).unwrap()
                })
                .map(|(j, _)| j)
            {
                let dst_body = &self.bodies[dst_idx];
                if dst_body.charge - src_body.charge >= HOP_CHARGE_THRESHOLD {
                    println!(
                        "Trying hop: src={} (charge={}), dst={} (charge={}), dist={}",
                        src_idx, src_body.charge, dst_idx, dst_body.charge, (src_body.pos - dst_body.pos).mag()
                    );
                    hops.push((src_idx, dst_idx));
                }
            }
        }

        // Apply hops
        for (src_idx, dst_idx) in hops {
            if self.bodies[src_idx].electrons.len() > 1 {
                if let Some(e) = self.bodies[src_idx].electrons.pop() {
                    self.bodies[dst_idx].electrons.push(e);
                    self.bodies[src_idx].update_charge_from_electrons();
                    self.bodies[dst_idx].update_charge_from_electrons();
                }
            }
        }
    }
}
