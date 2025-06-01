// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use crate::{body::{Body, Species, Electron}, quadtree::Quadtree};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use rand::prelude::*; // Import all prelude traits for rand 0.9+

/// The main simulation state and logic for the particle system.
pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub bounds: f32,
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
    pub foils: Vec<crate::body::foil::Foil>,
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
            foils: Vec::new(),
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
        // Sync config from global LJ_CONFIG (updated by GUI)
        self.config = crate::config::LJ_CONFIG.lock().clone();

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
        // No longer forcibly fix FoilMetal
        // for body in &mut self.bodies {
        //     if body.species == Species::FoilMetal {
        //         body.fixed = true;
        //     }
        // }
        forces::attract(self);
        forces::apply_lj_forces(self);
        self.iterate();
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }

        // Apply foil current sources/sinks
        for foil in &mut self.foils {
            // Integrate current into accumulator
            foil.accum += foil.current * self.dt;
            //println!("[DEBUG] Foil accum value: {} (current: {})", foil.accum, foil.current);
            let mut rng = rand::rng();
            while foil.accum >= 1.0 {
                if let Some(&id) = foil.body_ids.as_slice().choose(&mut rng) {
                    if let Some(body) = self.bodies.iter_mut().find(|b| b.id == id && b.species == Species::FoilMetal) {
                        if body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                            body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                            body.update_charge_from_electrons();
                        }
                    }
                }
                foil.accum -= 1.0;
            }
            while foil.accum <= -1.0 {
                if let Some(&id) = foil.body_ids.as_slice().choose(&mut rng) {
                    if let Some(body) = self.bodies.iter_mut().find(|b| b.id == id && b.species == Species::FoilMetal) {
                        if !body.electrons.is_empty() {
                            body.electrons.pop();
                            body.update_charge_from_electrons();
                        }
                    }
                }
                foil.accum += 1.0;
            }
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
        // Damping factor scales with timestep and is user-configurable
        let dt = self.dt;
        let damping = self.config.damping_base.powf(dt / 0.01);
        for body in &mut self.bodies {
            body.vel += body.acc * self.dt;
            body.vel *= damping;
            body.pos += body.vel * self.dt;
            for axis in 0..2 {
                let pos = if axis == 0 { &mut body.pos.x } else { &mut body.pos.y };
                let vel = if axis == 0 { &mut body.vel.x } else { &mut body.vel.y };
                if *pos < -self.bounds {
                    *pos = -self.bounds;
                    *vel = -*vel;
                } else if *pos > self.bounds {
                    *pos = self.bounds;
                    *vel = -*vel;
                }
            }
        }
    }

    pub fn perform_electron_hopping(&mut self) {
        if self.bodies.is_empty() { return; }
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];
        let mut received_electron = vec![false; n];
        // Shuffle source indices to remove directional bias
        let mut src_indices: Vec<usize> = (0..n).collect();
        let mut rng = rand::rng();
        src_indices.shuffle(&mut rng);
        for &src_idx in &src_indices {
            let src_body = &self.bodies[src_idx];
            if !(src_body.species == Species::LithiumMetal || src_body.species == Species::FoilMetal) || src_body.electrons.len() <= 1 {
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
                let can_accept = match dst_body.species {
                    Species::LithiumMetal | Species::FoilMetal => {
                        // Allow if destination is at or below neutral and source is above neutral
                        dst_body.electrons.len() <= dst_body.neutral_electron_count()
                            && src_body.electrons.len() >= src_body.neutral_electron_count()
                    }
                    Species::LithiumIon => true, // Ions can always accept
                };
                if !can_accept {
                    continue;
                }
                candidate_neighbors.push(dst_idx);
            }

            // 2️⃣ Shuffle the neighbor list to remove directional bias
            candidate_neighbors.shuffle(&mut rng);

            // 3️⃣ Now process in random order
            for &dst_idx in &candidate_neighbors {
                let dst_body = &self.bodies[dst_idx];

                // If destination is an ion, always allow the hop (aggressive redox cycling)
                if dst_body.species == Species::LithiumIon {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    continue;
                }

                // compute overpotential Δφ
                let d_phi = dst_body.charge - src_body.charge;
                if d_phi <= 0.0 {
                    continue;
                }

                // --- Field-biased hopping ---
                let hop_vec = dst_body.pos - src_body.pos;
                let hop_dir = if hop_vec.mag() > 1e-6 { hop_vec.normalized() } else { Vec2::zero() };
                let local_field = self.background_e_field
                    + self.quadtree.field_at_point(&self.bodies, src_body.pos, crate::simulation::forces::K_E);
                let field_dir = if local_field.mag() > 1e-6 { local_field.normalized() } else { Vec2::zero() };
                let alignment = (-hop_dir.dot(field_dir)).max(0.0); // Inverted: Only positive alignment in field direction
                if alignment < 1e-3 { continue; } // Only allow hops in field direction

                let rate = self.config.hop_rate_k0 * (self.config.hop_transfer_coeff * d_phi / self.config.hop_activation_energy).exp();
                let p_hop = alignment * (1.0 - (-rate * self.dt).exp());
                if rand::random::<f32>() < p_hop {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
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
                }
            }
        }
        // Aggressively apply redox after hopping
        for body in &mut self.bodies {
            body.apply_redox();
        }
    }
}
