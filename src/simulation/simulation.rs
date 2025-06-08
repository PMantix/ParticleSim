// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use crate::{body::{Body, Species, Electron}, quadtree::Quadtree};
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use crate::simulation::utils::can_transfer_electron;
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

        // Track which bodies receive electrons from foil current this step
        let mut foil_current_recipients = vec![false; self.bodies.len()];
        // Apply foil current sources/sinks
        for (_, foil) in self.foils.iter_mut().enumerate() {
            // Accumulate current for this foil
            foil.accum += foil.current * self.dt;

            let mut rng = rand::rng();
            // Print electron counts for all foil bodies before
            #[cfg(debug_assertions)]
            for &id in &foil.body_ids {
                if let Some(body) = self.bodies.iter().find(|b| b.id == id && b.species == Species::FoilMetal) {
                    println!("[Foil Debug]   Body id {}: electrons before = {}", id, body.electrons.len());
                }
            }
            while foil.accum >= 1.0 {

                if let Some(&id) = foil.body_ids.as_slice().choose(&mut rng) {
                    if let Some((body_idx, body)) = self.bodies.iter_mut().enumerate().find(|(_, b)| b.id == id && b.species == Species::FoilMetal) {
                        if body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                            body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                            foil_current_recipients[body_idx] = true;
                        }
                    }
                }
                foil.accum -= 1.0;
            }
            while foil.accum <= -1.0 {
                if let Some(&id) = foil.body_ids.as_slice().choose(&mut rng) {
                    if let Some((body_idx, body)) = self.bodies.iter_mut().enumerate().find(|(_, b)| b.id == id && b.species == Species::FoilMetal) {
                        if !body.electrons.is_empty() {
                            body.electrons.pop();
                            foil_current_recipients[body_idx] = true;
                        }
                    }
                }
                foil.accum += 1.0;
            }
            // Print electron counts for all foil bodies after
            #[cfg(debug_assertions)]
            for &id in &foil.body_ids {
                if let Some(body) = self.bodies.iter().find(|b| b.id == id && b.species == Species::FoilMetal) {
                     println!("[Foil Debug]   Body id {}: electrons after = {}", id, body.electrons.len());
                }
            }
        }
        // Ensure all body charges are up-to-date after foil current changes
        for body in &mut self.bodies {
            body.update_charge_from_electrons();
        }
        // Rebuild the quadtree after charge/electron changes so field is correct for hopping
        self.quadtree.build(&mut self.bodies);

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
        self.perform_electron_hopping_with_exclusions(&foil_current_recipients);
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

    /*pub fn perform_electron_hopping(&mut self) {
        if self.bodies.is_empty() { return; }
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];
        let mut received_electron = vec![false; n];
        let mut donated_electron = vec![false; n]; // Track if a donor has already donated this step
        // Shuffle source indices to remove directional bias
        let mut src_indices: Vec<usize> = (0..n).collect();
        let mut rng = rand::rng();
        src_indices.shuffle(&mut rng);
        for &src_idx in &src_indices {
            if donated_electron[src_idx] { continue; } // Only allow one hop per donor per step
            let src_body = &self.bodies[src_idx];
            let src_diff = src_body.electrons.len() as i32 - src_body.neutral_electron_count() as i32;
            // Only skip if the donor is so depleted that it cannot donate
            if !(src_body.species == Species::LithiumMetal || src_body.species == Species::FoilMetal) || src_diff < 0 {
                continue;
            }
            let hop_radius = self.config.hop_radius_factor * src_body.radius;

            // Use quadtree for neighbor search!
            let mut candidate_neighbors = self.quadtree
                .find_neighbors_within(&self.bodies, src_idx, hop_radius)
                .into_iter()
                .filter(|&dst_idx| dst_idx != src_idx && !received_electron[dst_idx])
                .filter(|&dst_idx| {
                    let dst_body = &self.bodies[dst_idx];
                    match dst_body.species {
                        Species::LithiumMetal | Species::FoilMetal => can_transfer_electron(src_body, dst_body),
                        Species::LithiumIon => true,
                    }
                })
                .collect::<Vec<_>>();

            candidate_neighbors.shuffle(&mut rng);

            for &dst_idx in &candidate_neighbors {
                if donated_electron[src_idx] { break; } // Only allow one hop per donor per step
                let dst_body = &self.bodies[dst_idx];

                // If destination is an ion, always allow the hop (aggressive redox cycling)
                if dst_body.species == Species::LithiumIon {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    donated_electron[src_idx] = true;
                    break;
                }

                // compute overpotential Δφ
                let d_phi = dst_body.charge - src_body.charge;
                if d_phi <= 0.0 { continue; }

                // --- Field-biased hopping ---
                let hop_vec = dst_body.pos - src_body.pos;
                let hop_dir = if hop_vec.mag() > 1e-6 { hop_vec.normalized() } else { Vec2::zero() };
                let local_field = self.background_e_field
                    + self.quadtree.field_at_point(&self.bodies, src_body.pos, crate::simulation::forces::K_E);
                let field_dir = if local_field.mag() > 1e-6 { local_field.normalized() } else { Vec2::zero() };
                let mut alignment = (-hop_dir.dot(field_dir)).max(0.0); // Inverted: Only positive alignment in field direction
                if field_dir == Vec2::zero() {
                    // No external (or significant) field -> assume perfect alignment.
                    alignment = 1.0;
                }
                if alignment < 1e-3 { continue; }
                let rate = self.config.hop_rate_k0 * (self.config.hop_transfer_coeff * d_phi / self.config.hop_activation_energy).exp();
                let p_hop = alignment * (1.0 - (-rate * self.dt).exp());
                if rand::random::<f32>() < p_hop {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    donated_electron[src_idx] = true;
                    break; // Only allow one hop per donor per step
                }
            }
        }

        for (src_idx, dst_idx) in hops {
            if let Some(electron) = self.bodies[src_idx].electrons.pop() {
                self.bodies[dst_idx].electrons.push(electron);
                self.bodies[src_idx].update_charge_from_electrons();
                self.bodies[dst_idx].update_charge_from_electrons();
            }
        }

        for body in &mut self.bodies {
            body.apply_redox();
        }
    }*/

    pub fn perform_electron_hopping_with_exclusions(&mut self, exclude_donor: &[bool]) {
        if self.bodies.is_empty() { return; }
        let n = self.bodies.len();
        let mut hops: Vec<(usize, usize)> = vec![];
        let mut received_electron = vec![false; n];
        let mut donated_electron = vec![false; n];
        let mut src_indices: Vec<usize> = (0..n).collect();
        let mut rng = rand::rng();
        src_indices.shuffle(&mut rng);
        for &src_idx in &src_indices {
            if donated_electron[src_idx] || exclude_donor[src_idx] { continue; }
            let src_body = &self.bodies[src_idx];
            let src_diff = src_body.electrons.len() as i32 - src_body.neutral_electron_count() as i32;
            if !(src_body.species == Species::LithiumMetal || src_body.species == Species::FoilMetal) || src_diff < 0 {
                continue;
            }
            let hop_radius = self.config.hop_radius_factor * src_body.radius;

            // Use quadtree for neighbor search!
            let mut candidate_neighbors = self.quadtree
                .find_neighbors_within(&self.bodies, src_idx, hop_radius)
                .into_iter()
                .filter(|&dst_idx| dst_idx != src_idx && !received_electron[dst_idx])
                .filter(|&dst_idx| {
                    let dst_body = &self.bodies[dst_idx];
                    match dst_body.species {
                        Species::LithiumMetal | Species::FoilMetal => can_transfer_electron(src_body, dst_body),
                        Species::LithiumIon => true,
                    }
                })
                .collect::<Vec<_>>();

            candidate_neighbors.shuffle(&mut rng);

            for &dst_idx in &candidate_neighbors {
                if donated_electron[src_idx] { break; }
                let dst_body = &self.bodies[dst_idx];
                /*if dst_body.species == Species::LithiumIon {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    donated_electron[src_idx] = true;
                    break;
                }*/
                let d_phi = dst_body.charge - src_body.charge;
                if d_phi <= 0.0 { continue; }
                let hop_vec = dst_body.pos - src_body.pos;
                let hop_dir = if hop_vec.mag() > 1e-6 { hop_vec.normalized() } else { Vec2::zero() };
                let local_field = self.background_e_field
                    + self.quadtree.field_at_point(&self.bodies, src_body.pos, crate::simulation::forces::K_E);
                let field_dir = if local_field.mag() > 1e-6 { local_field.normalized() } else { Vec2::zero() };
                let mut alignment = (-hop_dir.dot(field_dir)).max(0.0);
                if field_dir == Vec2::zero() { alignment = 1.0; }
                if alignment < 1e-3 { continue; }
                let rate = self.config.hop_rate_k0 * (self.config.hop_transfer_coeff * d_phi / self.config.hop_activation_energy).exp();
                let p_hop = alignment * (1.0 - (-rate * self.dt).exp());
                if rand::random::<f32>() < p_hop {
                    hops.push((src_idx, dst_idx));
                    received_electron[dst_idx] = true;
                    donated_electron[src_idx] = true;
                    break;
                }
            }
        }
        for (src_idx, dst_idx) in hops {
            if let Some(electron) = self.bodies[src_idx].electrons.pop() {
                self.bodies[dst_idx].electrons.push(electron);
                self.bodies[src_idx].update_charge_from_electrons();
                self.bodies[dst_idx].update_charge_from_electrons();
            }
        }
        for body in &mut self.bodies {
            body.apply_redox();
        }
    }
}
