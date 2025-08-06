// simulation/simulation.rs
// Contains the Simulation struct and main methods (new, step, iterate, perform_electron_hopping)

use crate::{body::{Body, Species, Electron}, quadtree::Quadtree, cell_list::CellList};
use rayon::prelude::*;
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION, TIMESTEP, COLLISION_PASSES, SIM_TIME};
use ultraviolet::Vec2;
use super::forces;
use super::collision;
use crate::config;
use crate::simulation::utils::can_transfer_electron;
use rand::prelude::*; // Import all prelude traits for rand 0.9+
use crate::profile_scope;
use std::collections::HashMap;
use crate::body::foil::LinkMode;

/// The main simulation state and logic for the particle system.
pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub quadtree: Quadtree,
    pub cell_list: CellList,
    pub bounds: f32, // Legacy field for backward compatibility
    pub domain_width: f32,  // Half-width of the domain (from center to edge)
    pub domain_height: f32, // Half-height of the domain (from center to edge)
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
    pub foils: Vec<crate::body::foil::Foil>,
    pub body_to_foil: HashMap<u64, u64>,
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
        let cell_size = crate::species::max_lj_cutoff();
        let cell_list = CellList::new(bounds, cell_size);
        let rewound_flags = vec![];
        let sim = Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            cell_list,
            bounds,
            domain_width: bounds,   // Initialize with square domain, will be updated by SetDomainSize command
            domain_height: bounds,  // Initialize with square domain, will be updated by SetDomainSize command
            rewound_flags,
            background_e_field: Vec2::zero(),
            foils: Vec::new(),
            body_to_foil: HashMap::new(),
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
        profile_scope!("simulation_step");
        // Sync config from global LJ_CONFIG (updated by GUI)
        self.config = crate::config::LJ_CONFIG.lock().clone();

        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        self.background_e_field = Vec2::new(theta.cos(), theta.sin()) * mag;
        self.rewound_flags.par_iter_mut().for_each(|flag| *flag = false);
        self.dt = *TIMESTEP.lock();
        let time = self.frame as f32 * self.dt;

        // Update global simulation time for GUI access
        *SIM_TIME.lock() = time;

        // Propagate linked foil currents - removed since we now handle linking at the property level
        
        self.bodies.par_iter_mut().for_each(|body| {
            body.acc = Vec2::zero();
        });

        forces::attract(self);
        forces::apply_polar_forces(self);
        forces::apply_lj_forces(self);
        forces::apply_repulsive_forces(self);
        self.iterate();
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        self.update_surrounded_flags();

        // Track which bodies receive electrons from foil current this step
        let mut foil_current_recipients = vec![false; self.bodies.len()];
        // Apply foil current sources/sinks with linking logic
        let mut visited = vec![false; self.foils.len()];
        for i in 0..self.foils.len() {
            if visited[i] { continue; }
            if let Some(link_id) = self.foils[i].link_id {
                if let Some(j) = self.foils.iter().position(|f| f.id == link_id) {
                    if !visited[j] {
                        visited[i] = true;
                        visited[j] = true;
                        self.process_linked_pair(i, j, time, &mut foil_current_recipients);
                        continue;
                    }
                }
            }
            visited[i] = true;
            self.process_single_foil(i, time, &mut foil_current_recipients);
        }
        // Ensure all body charges are up-to-date after foil current changes
        self.bodies.par_iter_mut().for_each(|body| body.update_charge_from_electrons());
        // Rebuild the quadtree after charge/electron changes so field is correct for hopping
        self.quadtree.build(&mut self.bodies);

        let quadtree = &self.quadtree;
        let len = self.bodies.len();
        let bodies_ptr = self.bodies.as_ptr();
        for i in 0..len {
            let bodies_slice = unsafe { std::slice::from_raw_parts(bodies_ptr, len) };
            let body = &mut self.bodies[i];
            body.update_electrons(
                bodies_slice,
                quadtree,
                self.background_e_field,
                self.dt,
                self.config.coulomb_constant,
            );
            body.update_charge_from_electrons();
        }
        self.perform_electron_hopping_with_exclusions(&foil_current_recipients);
        self.frame += 1;

        #[cfg(test)]
        // After all updates, print debug info for anions
        for (i, body) in self.bodies.iter().enumerate() {
            if body.species == crate::body::Species::ElectrolyteAnion {
                println!("[DEBUG] Step {}: Anion {} charge = {}, pos = {:?}, vel = {:?}", self.frame, i, body.charge, body.pos, body.vel);
            }
        }
    }

    pub fn iterate(&mut self) {
        profile_scope!("iterate");
        // Damping factor scales with timestep and is user-configurable
        let dt = self.dt;
        let base_damping = self.config.damping_base.powf(dt / 0.01);
        let domain_width = self.domain_width;
        let domain_height = self.domain_height;
        self.bodies.par_iter_mut().for_each(|body| {
            body.vel += body.acc * dt;
            let damping = base_damping * body.species.damping();
            body.vel *= damping; // Simple damping without energy storage
            body.pos += body.vel * dt;
            
            // X-axis boundary enforcement
            if body.pos.x < -domain_width {
                body.pos.x = -domain_width;
                body.vel.x = -body.vel.x;
            } else if body.pos.x > domain_width {
                body.pos.x = domain_width;
                body.vel.x = -body.vel.x;
            }
            
            // Y-axis boundary enforcement
            if body.pos.y < -domain_height {
                body.pos.y = -domain_height;
                body.vel.y = -body.vel.y;
            } else if body.pos.y > domain_height {
                body.pos.y = domain_height;
                body.vel.y = -body.vel.y;
            }
        });
    }

    pub fn use_cell_list(&self) -> bool {
        let area = (2.0 * self.domain_width) * (2.0 * self.domain_height);
        let density = self.bodies.len() as f32 / area;
        density > self.config.cell_list_density_threshold
    }

    /// Attempt electron hops between nearby bodies.
    ///
    /// `exclude_donor` marks bodies that should not donate electrons this step
    /// (used for foil current sources). When `use_butler_volmer` is enabled
    /// in the configuration, hops between different species use the
    /// Butler-Volmer rate expression.
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
                    let dst_diff = dst_body.electrons.len() as i32 - dst_body.neutral_electron_count() as i32;
                    // Allow hop if donor is more excess than recipient
                    if src_diff >= dst_diff {
                        match dst_body.species {
                            Species::LithiumMetal | Species::FoilMetal | Species::LithiumIon => can_transfer_electron(src_body, dst_body),
                            _ => false,
                        }
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>();

            candidate_neighbors.shuffle(&mut rng);

            // Only check until the first successful hop
            if let Some(&dst_idx) = candidate_neighbors.iter().find(|&&dst_idx| {
                let dst_body = &self.bodies[dst_idx];
                let d_phi = dst_body.charge - src_body.charge;
                let hop_vec = dst_body.pos - src_body.pos;
                let hop_dir = if hop_vec.mag() > 1e-6 { hop_vec.normalized() } else { Vec2::zero() };
                let local_field = self.background_e_field
                    + self.quadtree.field_at_point(&self.bodies, src_body.pos, self.config.coulomb_constant);
                let field_dir = if local_field.mag() > 1e-6 { local_field.normalized() } else { Vec2::zero() };
                let mut alignment = (-hop_dir.dot(field_dir)).max(0.0);
                if field_dir == Vec2::zero() { alignment = 1.0; }
                if alignment < 1e-3 { return false; }

                let rate = if self.config.use_butler_volmer && src_body.species != dst_body.species {
                    // Butler-Volmer kinetics for inter-species electron transfer
                    let alpha = self.config.bv_transfer_coeff;
                    let scale = self.config.bv_overpotential_scale;
                    let i0 = self.config.bv_exchange_current;
                    let forward = (alpha * d_phi / scale).exp();
                    let backward = (-(1.0 - alpha) * d_phi / scale).exp();
                    i0 * (forward - backward)
                } else {
                    if d_phi <= 0.0 { return false; }
                    self.config.hop_rate_k0 * (self.config.hop_transfer_coeff * d_phi / self.config.hop_activation_energy).exp()
                };

                if rate <= 0.0 { return false; }
                let p_hop = alignment * (1.0 - (-rate * self.dt).exp());
                rand::random::<f32>() < p_hop
            }) {
                hops.push((src_idx, dst_idx));
                received_electron[dst_idx] = true;
                donated_electron[src_idx] = true;
            }
        }
        for (src_idx, dst_idx) in hops {
            if let Some(electron) = self.bodies[src_idx].electrons.pop() {
                self.bodies[dst_idx].electrons.push(electron);
                self.bodies[src_idx].update_charge_from_electrons();
                self.bodies[dst_idx].update_charge_from_electrons();
            }
        }
        // Split immutable borrows for rayon safety
        //let bodies_ref: Vec<Body> = self.bodies.iter().cloned().collect();
        //let quadtree_ref = &self.quadtree;
        profile_scope!("apply_redox");
        self.bodies.par_iter_mut().for_each(|body| {
            body.apply_redox();
        });
    }

    /// Update `surrounded_by_metal` for all bodies using either the cell list or quadtree.
    pub fn update_surrounded_flags(&mut self) {
        if self.bodies.is_empty() { return; }
        let use_cell = self.use_cell_list();
        let neighbor_radius = crate::species::max_lj_cutoff();
        if use_cell {
            self.cell_list.cell_size = neighbor_radius;
            self.cell_list.rebuild(&self.bodies);
        } else {
            self.quadtree.build(&mut self.bodies);
        }
        let quadtree = &self.quadtree;
        let cell_list = &self.cell_list;
        let frame = self.frame;
        // Collect the data needed for immutable borrow
        let bodies_snapshot: Vec<_> = self.bodies.iter().map(|b| b.clone()).collect();
        for (i, body) in self.bodies.iter_mut().enumerate() {
            body.maybe_update_surrounded(i, &bodies_snapshot, quadtree, cell_list, use_cell, frame);
        }
    }

    fn effective_current(foil: &crate::body::foil::Foil, time: f32) -> f32 {
        let mut current = foil.dc_current;
        if foil.switch_hz > 0.0 {
            let ac_component = if (time * foil.switch_hz) % 1.0 < 0.5 {
                foil.ac_current
            } else {
                -foil.ac_current
            };
            current += ac_component;
        }
        current
    }

    fn foil_can_add(&self, idx: usize) -> bool {
        let foil = &self.foils[idx];
        foil.body_ids.iter().any(|&id| {
            self.bodies.iter().any(|b| b.id == id && b.species == Species::FoilMetal && b.electrons.len() < crate::config::FOIL_MAX_ELECTRONS)
        })
    }

    fn foil_can_remove(&self, idx: usize) -> bool {
        let foil = &self.foils[idx];
        foil.body_ids.iter().any(|&id| {
            self.bodies.iter().any(|b| b.id == id && b.species == Species::FoilMetal && !b.electrons.is_empty())
        })
    }

    fn try_add_electron(&mut self, idx: usize, rng: &mut rand::rngs::ThreadRng, recipients: &mut [bool]) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some((body_idx, body)) = self.bodies.iter_mut().enumerate().find(|(_, b)| b.id == id && b.species == Species::FoilMetal) {
                if body.electrons.len() < crate::config::FOIL_MAX_ELECTRONS {
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                    recipients[body_idx] = true;
                    return true;
                }
            }
        }
        false
    }

    fn try_remove_electron(&mut self, idx: usize, rng: &mut rand::rngs::ThreadRng, recipients: &mut [bool]) -> bool {
        let foil = &mut self.foils[idx];
        if let Some(&id) = foil.body_ids.as_slice().choose(rng) {
            if let Some((body_idx, body)) = self.bodies.iter_mut().enumerate().find(|(_, b)| b.id == id && b.species == Species::FoilMetal) {
                if !body.electrons.is_empty() {
                    body.electrons.pop();
                    recipients[body_idx] = true;
                    return true;
                }
            }
        }
        false
    }

    fn process_single_foil(&mut self, idx: usize, time: f32, recipients: &mut [bool]) {
        let dt = self.dt;
        let current = Self::effective_current(&self.foils[idx], time);
        self.foils[idx].accum += current * dt;
        let mut rng = rand::rng();
        while self.foils[idx].accum >= 1.0 {
            let _ = self.try_add_electron(idx, &mut rng, recipients);
            self.foils[idx].accum -= 1.0;
        }
        while self.foils[idx].accum <= -1.0 {
            let _ = self.try_remove_electron(idx, &mut rng, recipients);
            self.foils[idx].accum += 1.0;
        }
    }

    fn process_linked_pair(&mut self, a: usize, b: usize, time: f32, recipients: &mut [bool]) {
        let dt = self.dt;
        let mode = self.foils[a].mode;
        let current_a = Self::effective_current(&self.foils[a], time);
        let current_b = Self::effective_current(&self.foils[b], time);
        self.foils[a].accum += current_a * dt;
        self.foils[b].accum += current_b * dt;
        let mut rng = rand::rng();
        loop {
            match mode {
                LinkMode::Parallel => {
                    if self.foils[a].accum >= 1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_add(a) && self.foil_can_add(b) {
                            if self.try_add_electron(a, &mut rng, recipients) && self.try_add_electron(b, &mut rng, recipients) {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum -= 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum <= -1.0 {
                        if self.foil_can_remove(a) && self.foil_can_remove(b) {
                            if self.try_remove_electron(a, &mut rng, recipients) && self.try_remove_electron(b, &mut rng, recipients) {
                                self.foils[a].accum += 1.0;
                                self.foils[b].accum += 1.0;
                                continue;
                            }
                        }
                    }
                    break;
                }
                LinkMode::Opposite => {
                    if self.foils[a].accum >= 1.0 && self.foils[b].accum <= -1.0 {
                        if self.foil_can_add(a) && self.foil_can_remove(b) {
                            if self.try_add_electron(a, &mut rng, recipients) && self.try_remove_electron(b, &mut rng, recipients) {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum += 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_remove(a) && self.foil_can_add(b) {
                            if self.try_remove_electron(a, &mut rng, recipients) && self.try_add_electron(b, &mut rng, recipients) {
                                self.foils[a].accum += 1.0;
                                self.foils[b].accum -= 1.0;
                                continue;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
}
