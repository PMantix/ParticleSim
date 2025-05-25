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
            body.set_electron_count();
            body.update_electrons(body.e_field, self.dt);
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
        // -------------Electron hopping pass --------------
        // We'll collect all hops this frame, then apply them.
        let mut hops: Vec<(usize /*src_idx*/, usize /*dst_idx*/, usize /*e_idx*/)> = vec![];
        let n = self.bodies.len();

        for src_idx in 0..n {
            let body = &self.bodies[src_idx];
            if body.species != Species::LithiumMetal {
                continue;
            }
            // For each electron in this metal
            for (e_idx, e) in body.electrons.iter().enumerate() {
                // Compute electron’s world position
                let e_world = body.pos + e.rel_pos;
                let hop_radius = HOP_RADIUS_FACTOR * body.radius;

                // Find a candidate neighbor metal within hop_radius
                if let Some((dst_idx, dst_body)) = self.bodies
                    .iter()
                    .enumerate()
                    .filter(|&(j, b)| j != src_idx && b.species == Species::LithiumMetal)
                    .filter(|(_, b)| (b.pos - e_world).mag() <= hop_radius)
                    // pick the *most favorable* neighbor: largest potential drop
                    .max_by(|(_, a), (_, b)| {
                        let da = body.charge - a.charge;
                        let db = body.charge - b.charge;
                        da.partial_cmp(&db).unwrap()
                    })
                {
                    // Only hop if the charge difference exceeds threshold
                    if body.charge - dst_body.charge >= HOP_CHARGE_THRESHOLD {
                        hops.push((src_idx, dst_idx, e_idx));
                    }
                }
            }
        }

        // Apply hops
        for (src_idx, dst_idx, e_idx) in hops.into_iter().rev() {
            // Remove the electron from the source
            let e = self.bodies[src_idx].electrons.remove(e_idx);
            // Add it to the destination
            self.bodies[dst_idx].electrons.push(e);
        }
        // -------- END HOP PASS --------
    }
}
