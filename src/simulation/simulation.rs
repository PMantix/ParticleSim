// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use crate::{body::{Body, Species}, quadtree::Quadtree};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use rand::seq::SliceRandom; // Add this at the top of the file if not already present

/// The main simulation state and logic for the particle system.
pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32,
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
    pub config:config::SimConfig, // 
}

impl Simulation {
    pub fn new() -> Self {
        let dt = config::DEFAULT_DT;
        let theta = config::QUADTREE_THETA;
        let epsilon = config::QUADTREE_EPSILON;
        let leaf_capacity = config::QUADTREE_LEAF_CAPACITY;
        let thread_capacity = config::QUADTREE_THREAD_CAPACITY;
        let bounds = config::DOMAIN_BOUNDS;
        // Start with no bodies; scenario setup is now done via SimCommand AddCircle/AddBody
        let bodies = Vec::new();
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let rewound_flags = vec![];
        let sim = Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            bounds,
            rewound_flags,
            background_e_field: Vec2::zero(),
            config: config::SimConfig::default(),
        };
        // Example: scenario setup using SimCommand (pseudo-code, actual sending is done in main.rs or GUI)
        // let left_center = Vec2::new(-bounds * 0.6, 0.0);
        // let right_center = Vec2::new(bounds * 0.6, 0.0);
        // let center = Vec2::zero();
        // let clump_radius = 10.0;
        // let metal_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
        // let ion_body = Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon);
        // SimCommand::AddCircle { body: metal_body, x: left_center.x, y: left_center.y, radius: clump_radius }
        // SimCommand::AddCircle { body: metal_body, x: right_center.x, y: right_center.y, radius: clump_radius }
        // SimCommand::AddCircle { body: ion_body, x: center.x, y: center.y, radius: clump_radius }
        sim
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
        let quadtree = &self.quadtree;
        let k_e = crate::simulation::forces::K_E;
        // Clone the bodies' positions and charges needed for field calculation
        let bodies_snapshot = self.bodies.clone();
        for body in &mut self.bodies {
            body.update_electrons(
                |pos| quadtree.field_at_point(&bodies_snapshot, pos, k_e) + self.background_e_field,
                self.dt,
            );
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
            let hop_radius = self.config.hop_radius_factor * src_body.radius;

            // 1️⃣ Collect all valid neighbor indices
            let mut candidate_neighbors = Vec::new();
            for (dst_idx, dst_body) in self.bodies.iter().enumerate() {
                if dst_idx == src_idx || received_electron[dst_idx] {
                    continue;
                }
                let d = (dst_body.pos - src_body.pos).mag();
                if d > hop_radius {
                    continue;
                }
                let can_accept = (dst_body.species == Species::LithiumMetal
                                    && dst_body.electrons.len() < src_body.electrons.len())
                                    || dst_body.species == Species::LithiumIon;
                if !can_accept {
                    continue;
                }
                candidate_neighbors.push(dst_idx);
            }

            // 2️⃣ Shuffle the neighbor list to remove directional bias
            let mut rng = rand::rng();
            candidate_neighbors.shuffle(&mut rng);

            // 3️⃣ Now process in random order
            for &dst_idx in &candidate_neighbors {
                let dst_body = &self.bodies[dst_idx];

                // compute overpotential Δφ
                let d_phi = dst_body.charge - src_body.charge;
                if d_phi <= 0.0 {
                    continue;
                }

                let rate = self.config.hop_rate_k0 * (self.config.hop_transfer_coeff * d_phi / self.config.hop_activation_energy).exp();
                let p_hop = 1.0 - (-rate * self.dt).exp();
                if rand::random::<f32>() < p_hop {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    // If you want to allow only one hop per src per step, uncomment the next line:
                    // break;
                }
            }
        }

        // Perform the hops
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
