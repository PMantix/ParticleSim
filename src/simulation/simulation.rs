// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use crate::{body::{Body, Species}, quadtree::Quadtree, utils};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use crate::config::{HOP_RADIUS_FACTOR, HOP_CHARGE_THRESHOLD};

/// The main simulation state and logic for the particle system.
pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32,
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
        // ...existing code...
        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        self.background_e_field = Vec2::new(theta.cos(), theta.sin()) * mag;
        for flag in &mut self.rewound_flags {
            *flag = false;
        }
        self.dt = *TIMESTEP.lock();
        for body in &mut self.bodies {
            body.acc = Vec2::zero();
        }
        forces::attract(self);
        forces::apply_lj_forces(self);
        self.iterate();
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        for body in &mut self.bodies {
            body.update_electrons(body.e_field, self.dt);
            body.update_charge_from_electrons();
        }
        self.perform_electron_hopping();
        self.frame += 1;
    }

    pub fn iterate(&mut self) {
        let damping = 0.999;
        for body in &mut self.bodies {
            body.vel += body.acc * self.dt;
            body.vel *= damping;
            body.pos += body.vel * self.dt;
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
        let mut received_electron = vec![false; n];
        for src_idx in 0..n {
            let src_body = &self.bodies[src_idx];
            if src_body.species != Species::LithiumMetal || src_body.electrons.len() <= 1 {
                continue;
            }
            let hop_radius = HOP_RADIUS_FACTOR * src_body.radius;
            if let Some(dst_idx) = self.bodies
                .iter()
                .enumerate()
                .filter(|&(j, b)| {
                    j != src_idx &&
                    !received_electron[j] &&
                    (
                        (b.species == Species::LithiumMetal && b.electrons.len() < src_body.electrons.len() && b.charge > src_body.charge)
                        ||
                        (b.species == Species::LithiumIon)
                    )
                })
                .filter(|(_, b)| (b.pos - src_body.pos).mag() <= hop_radius)
                .filter(|(_, b)| {
                    (b.charge > src_body.charge) && (b.electrons.len() < src_body.electrons.len())
                })
                .min_by(|(_, a), (_, b)| {
                    let da = a.charge - src_body.charge;
                    let db = b.charge - src_body.charge;
                    da.partial_cmp(&db).unwrap()
                })
                .map(|(j, _)| j)
            {
                let dst_body = &self.bodies[dst_idx];
                if dst_body.charge - src_body.charge >= HOP_CHARGE_THRESHOLD {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                }
            }
        }
        for (src_idx, dst_idx) in hops {
            let (first, second) = self.bodies.split_at_mut(std::cmp::max(src_idx, dst_idx));
            let (src, dst) = if src_idx < dst_idx {
                (&mut first[src_idx], &mut second[0])
            } else {
                (&mut second[0], &mut first[dst_idx])
            };
            if src.electrons.len() > 1 {
                if let Some(e) = src.electrons.pop() {
                    dst.electrons.push(e);
                    src.apply_redox();
                    dst.apply_redox();
                }
            }
        }
    }
}
