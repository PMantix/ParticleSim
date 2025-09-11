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
    pub domain_width: f32,  // Half-width of the domain (from center to edge)
    pub domain_height: f32, // Half-height of the domain (from center to edge)
    pub domain_depth: f32,  // Half-depth of the domain (for z-direction)
    pub rewound_flags: Vec<bool>,
    pub background_e_field: Vec2,
    pub foils: Vec<crate::body::foil::Foil>,
    pub body_to_foil: HashMap<u64, u64>,
    pub config:config::SimConfig, //
    /// Track when thermostat was last applied (in simulation time)
    pub last_thermostat_time: f32,
}

impl Simulation {
    pub fn new() -> Self {
        let dt = config::DEFAULT_DT_FS;
        let theta = config::QUADTREE_THETA;
        let epsilon = config::QUADTREE_EPSILON;
        let leaf_capacity = config::QUADTREE_LEAF_CAPACITY;
        let thread_capacity = config::QUADTREE_THREAD_CAPACITY;
        let bounds = config::DOMAIN_BOUNDS;
        // Start with no bodies; scenario setup is now done via SimCommand AddCircle/AddBody
        let bodies = Vec::new();
        let quadtree = Quadtree::new(theta, epsilon, leaf_capacity, thread_capacity);
        let cell_size = crate::species::max_lj_cutoff();
        let cell_list = CellList::new(bounds, bounds, cell_size);
        let rewound_flags = vec![];
        let sim = Self {
            dt,
            frame: 0,
            bodies,
            quadtree,
            cell_list,
            domain_width: bounds,   // Initialize with square domain, will be updated by SetDomainSize command
            domain_height: bounds,  // Initialize with square domain, will be updated by SetDomainSize command
            domain_depth: bounds,   // Initialize with square domain depth
            rewound_flags,
            background_e_field: Vec2::zero(),
            foils: Vec::new(),
            body_to_foil: HashMap::new(),
            config: config::SimConfig::default(),
            last_thermostat_time: 0.0,
        };
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

        // Check for NaN values at start of step
        let nan_count = self.bodies.iter().filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.charge.is_finite()).count();
        if nan_count > 0 {
            // NaN values detected at step start
        }

        // Propagate linked foil currents - removed since we now handle linking at the property level
        
        self.bodies.par_iter_mut().for_each(|body| {
            body.acc = Vec2::zero();
            body.az = 0.0; // Reset z-acceleration as well
        });

        forces::attract(self);
        forces::apply_polar_forces(self);
        forces::apply_lj_forces(self);
        forces::apply_repulsive_forces(self);
        
        // Check for NaN values after force calculations
        let nan_count = self.bodies.iter().filter(|b| !b.acc.x.is_finite() || !b.acc.y.is_finite() || !b.az.is_finite()).count();
        if nan_count > 0 {
            // NaN values detected after force calculations
        }
        
        // Apply out-of-plane forces if enabled
        if self.config.enable_out_of_plane {
            super::out_of_plane::apply_out_of_plane(self);
        }
        
        // Check for NaN values after out-of-plane physics
        let nan_count = self.bodies.iter().filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.charge.is_finite() || !b.z.is_finite()).count();
        if nan_count > 0 {
            // NaN values detected after out-of-plane physics
        }
        
        // Apply Li+ mobility enhancement (pressure-dependent collision softening)
        // super::li_mobility::apply_li_mobility_enhancement(self);
        self.iterate();
        
        // Check for NaN values after iterate
        let nan_count = self.bodies.iter().filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.charge.is_finite()).count();
        if nan_count > 0 {
            // NaN values detected after iterate
        }
        
        let num_passes = *COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            collision::collide(self);
        }
        self.update_surrounded_flags();

        // Track which bodies receive electrons from foil current this step
        let mut foil_current_recipients = vec![false; self.bodies.len()];
        // Apply foil current sources/sinks with charge conservation
        self.process_foils_with_charge_conservation(time, &mut foil_current_recipients);
        // Ensure all body charges are up-to-date after foil current changes
        self.bodies.par_iter_mut().for_each(|body| body.update_charge_from_electrons());
        
        // Rebuild the quadtree after charge/electron changes so field is correct for hopping
        // Use domain-aware build to respect the configured domain boundaries
        self.quadtree.build_with_domain(&mut self.bodies, self.domain_width, self.domain_height);

        let quadtree = &self.quadtree;
        let len = self.bodies.len();
        let bodies_ptr = self.bodies.as_ptr();
        for i in 0..len {
            if i % 1000 == 0 && i > 0 {
                // Processing electron updates in batches
            }
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
        
        // Apply periodic thermostat if enough time has passed
        if time - self.last_thermostat_time >= self.config.thermostat_frequency {
            self.apply_thermostat();
            self.last_thermostat_time = time;
        }
        
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
        let domain_depth = self.domain_depth;
        let enable_out_of_plane = self.config.enable_out_of_plane;
        self.bodies.par_iter_mut().for_each(|body| {
            body.vel += body.acc * dt;
            let damping = base_damping * body.species.damping();
            body.vel *= damping;
            body.pos += body.vel * dt;
            
            // Z-coordinate integration (if out-of-plane is enabled)
            if enable_out_of_plane {
                body.vz += body.az * dt;
                body.vz *= damping; // Apply same damping to z-velocity
                body.z += body.vz * dt;
                
                // Z-axis boundary enforcement
                if body.z < -domain_depth {
                    body.z = -domain_depth;
                    body.vz = -body.vz;
                } else if body.z > domain_depth {
                    body.z = domain_depth;
                    body.vz = -body.vz;
                }
            }
            
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
            self.quadtree.build_with_domain(&mut self.bodies, self.domain_width, self.domain_height);
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

    /// Process foils with charge conservation - electrons can only be added if another foil removes one
    fn process_foils_with_charge_conservation(&mut self, time: f32, recipients: &mut [bool]) {
        let dt = self.dt;
        let mut rng = rand::rng();
        
        // Update all accumulators first
        for i in 0..self.foils.len() {
            let current = Self::effective_current(&self.foils[i], time);
            self.foils[i].accum += current * dt;
        }
        
        // Handle linked pairs first (they have priority and built-in charge conservation)
        let mut visited = vec![false; self.foils.len()];
        for i in 0..self.foils.len() {
            if visited[i] { continue; }
            if let Some(link_id) = self.foils[i].link_id {
                if let Some(j) = self.foils.iter().position(|f| f.id == link_id) {
                    if !visited[j] {
                        visited[i] = true;
                        visited[j] = true;
                        self.process_linked_pair_conservative(i, j, &mut rng, recipients);
                        continue;
                    }
                }
            }
        }
        
        // For unlinked foils, enforce global charge conservation
        let mut add_ready: Vec<usize> = Vec::new();
        let mut remove_ready: Vec<usize> = Vec::new();
        
        for i in 0..self.foils.len() {
            if visited[i] { continue; }
            
            // Check if foil is ready to add electrons (positive accumulator)
            if self.foils[i].accum >= 1.0 && self.foil_can_add(i) {
                add_ready.push(i);
            }
            // Check if foil is ready to remove electrons (negative accumulator)
            else if self.foils[i].accum <= -1.0 && self.foil_can_remove(i) {
                remove_ready.push(i);
            }
        }
        
        // Shuffle to ensure random pairing
        add_ready.shuffle(&mut rng);
        remove_ready.shuffle(&mut rng);
        
        // Process charge-conserving pairs: one adds, one removes
        let num_pairs = add_ready.len().min(remove_ready.len());
        
        for pair_idx in 0..num_pairs {
            let add_foil_idx = add_ready[pair_idx];
            let remove_foil_idx = remove_ready[pair_idx];
            
            // Attempt the charge-conserving pair operation
            if self.try_add_electron(add_foil_idx, &mut rng, recipients) && 
               self.try_remove_electron(remove_foil_idx, &mut rng, recipients) {
                self.foils[add_foil_idx].accum -= 1.0;
                self.foils[remove_foil_idx].accum += 1.0;
            }
        }
    }

    /// Process linked pair with charge conservation (similar to existing but renamed for clarity)
    fn process_linked_pair_conservative(&mut self, a: usize, b: usize, rng: &mut rand::rngs::ThreadRng, recipients: &mut [bool]) {
        let mode = self.foils[a].mode;
        loop {
            match mode {
                LinkMode::Parallel => {
                    if self.foils[a].accum >= 1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_add(a) && self.foil_can_add(b) {
                            if self.try_add_electron(a, rng, recipients) && self.try_add_electron(b, rng, recipients) {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum -= 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum <= -1.0 {
                        if self.foil_can_remove(a) && self.foil_can_remove(b) {
                            if self.try_remove_electron(a, rng, recipients) && self.try_remove_electron(b, rng, recipients) {
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
                            if self.try_add_electron(a, rng, recipients) && self.try_remove_electron(b, rng, recipients) {
                                self.foils[a].accum -= 1.0;
                                self.foils[b].accum += 1.0;
                                continue;
                            }
                        }
                    }
                    if self.foils[a].accum <= -1.0 && self.foils[b].accum >= 1.0 {
                        if self.foil_can_remove(a) && self.foil_can_add(b) {
                            if self.try_remove_electron(a, rng, recipients) && self.try_add_electron(b, rng, recipients) {
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
    
    /// Apply Maxwell-Boltzmann thermostat to maintain target temperature
    /// Only applies to solvent particles (EC/DMC), excludes metals
    pub fn apply_thermostat(&mut self) {
        use crate::body::Species;
        use crate::units::BOLTZMANN_CONSTANT;
        
        let target_temp = self.config.temperature;
        if target_temp <= 0.0 {
            return;
        }
        
        // Calculate current temperature of solvent particles only
        let mut solvent_ke = 0.0;
        let mut solvent_count = 0;
        
        for body in &self.bodies {
            match body.species {
                Species::EC | Species::DMC => {
                    solvent_ke += 0.5 * body.mass * body.vel.mag_sq();
                    solvent_count += 1;
                }
                _ => {} // Skip metals and ions
            }
        }
        
        if solvent_count == 0 {
            return; // No solvent particles to thermostat
        }
        
        // For 2D: <E> = k_B * T, so T = <E> / k_B
        let avg_kinetic_energy = solvent_ke / solvent_count as f32;
        let current_temp = avg_kinetic_energy / BOLTZMANN_CONSTANT;
        
        if current_temp > 0.0 {
            let scale = (target_temp / current_temp).sqrt();
            
            // Scale velocities of solvent particles only
            for body in &mut self.bodies {
                match body.species {
                    Species::EC | Species::DMC => {
                        body.vel *= scale;
                    }
                    _ => {} // Don't modify metals or ions
                }
            }
        }
    }
}

#[cfg(test)]
mod charge_conservation_tests {
    use super::*;
    use crate::body::foil::Foil;

    fn create_test_simulation_with_foils() -> Simulation {
        let mut sim = Simulation::new();
        
        // Create test foil bodies
        let foil_body1 = Body::new(
            Vec2::new(-10.0, 0.0), 
            Vec2::zero(), 
            1.0, 
            1.0, 
            0.0, 
            Species::FoilMetal
        );
        let foil_body2 = Body::new(
            Vec2::new(10.0, 0.0), 
            Vec2::zero(), 
            1.0, 
            1.0, 
            0.0, 
            Species::FoilMetal
        );
        
        sim.bodies.push(foil_body1);
        sim.bodies.push(foil_body2);
        
        // Create foils with positive and negative currents
        let mut foil1 = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil1.accum = 1.5; // Ready to add electrons
        
        let mut foil2 = Foil::new(vec![sim.bodies[1].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil2.accum = -1.5; // Ready to remove electrons
        
        sim.foils.push(foil1);
        sim.foils.push(foil2);
        
        sim
    }

    #[test]
    fn test_single_foil_with_positive_accum_does_nothing() {
        let mut sim = Simulation::new();
        
        // Create a single foil body
        let foil_body = Body::new(
            Vec2::zero(), 
            Vec2::zero(), 
            1.0, 
            1.0, 
            0.0, 
            Species::FoilMetal
        );
        sim.bodies.push(foil_body);
        
        // Create a single foil with positive current (wants to add electrons)
        let mut foil = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil.accum = 1.5; // Ready to add electrons
        sim.foils.push(foil);
        
        let initial_electron_count = sim.bodies[0].electrons.len();
        let initial_accum = sim.foils[0].accum;
        let dt = sim.dt;
        let current = 2.0; // foil dc_current
        
        // Process foils - should do nothing since no partner to remove electrons
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);
        
        // Verify no electrons were added but accumulator updated by current
        assert_eq!(sim.bodies[0].electrons.len(), initial_electron_count, 
                   "Single foil should not add electrons without a removal partner");
        assert_eq!(sim.foils[0].accum, initial_accum + current * dt, 
                   "Accumulator should be updated by current flow even when no operations occur");
        assert!(!recipients[0], "Body should not be marked as recipient");
    }

    #[test]
    fn test_single_foil_with_negative_accum_does_nothing() {
        let mut sim = Simulation::new();
        
        // Create a single foil body with an electron to remove
        let mut foil_body = Body::new(
            Vec2::zero(), 
            Vec2::zero(), 
            1.0, 
            1.0, 
            0.0, 
            Species::FoilMetal
        );
        foil_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        sim.bodies.push(foil_body);
        
        // Create a single foil with negative current (wants to remove electrons)
        let mut foil = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil.accum = -1.5; // Ready to remove electrons
        sim.foils.push(foil);
        
        let initial_electron_count = sim.bodies[0].electrons.len();
        let initial_accum = sim.foils[0].accum;
        let dt = sim.dt;
        let current = -2.0; // foil dc_current
        
        // Process foils - should do nothing since no partner to add electrons
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);
        
        // Verify no electrons were removed but accumulator updated by current
        assert_eq!(sim.bodies[0].electrons.len(), initial_electron_count, 
                   "Single foil should not remove electrons without an addition partner");
        assert_eq!(sim.foils[0].accum, initial_accum + current * dt, 
                   "Accumulator should be updated by current flow even when no operations occur");
        assert!(!recipients[0], "Body should not be marked as recipient");
    }

    #[test]
    fn test_paired_foils_execute_charge_conserving_operations() {
        let mut sim = create_test_simulation_with_foils();
        
        let initial_electrons_foil1 = sim.bodies[0].electrons.len();
        let initial_electrons_foil2 = sim.bodies[1].electrons.len();
        let initial_accum1 = sim.foils[0].accum;
        let initial_accum2 = sim.foils[1].accum;
        let dt = sim.dt;
        let current1 = 2.0;  // foil1 dc_current
        let current2 = -2.0; // foil2 dc_current
        
        // Add an electron to foil2 so it can be removed
        sim.bodies[1].electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        
        // Process foils - should execute charge-conserving pair
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);
        
        // Verify charge-conserving operations occurred
        assert_eq!(sim.bodies[0].electrons.len(), initial_electrons_foil1 + 1, 
                   "Foil 1 should have gained an electron");
        assert_eq!(sim.bodies[1].electrons.len(), initial_electrons_foil2, // Had 1, lost 1, still 0
                   "Foil 2 should have lost an electron");
        
        // Verify accumulators: updated by current, then decremented/incremented by operations
        let expected_accum1 = initial_accum1 + current1 * dt - 1.0;
        let expected_accum2 = initial_accum2 + current2 * dt + 1.0;
        assert_eq!(sim.foils[0].accum, expected_accum1, 
                   "Foil 1 accumulator should be updated by current then decremented by operation");
        assert_eq!(sim.foils[1].accum, expected_accum2, 
                   "Foil 2 accumulator should be updated by current then incremented by operation");
        
        // Verify recipients were marked
        assert!(recipients[0], "Foil 1 body should be marked as recipient");
        assert!(recipients[1], "Foil 2 body should be marked as recipient");
    }

    #[test]
    fn test_total_electron_count_conservation() {
        let mut sim = create_test_simulation_with_foils();
        
        // Add some initial electrons
        sim.bodies[0].electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        sim.bodies[1].electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        sim.bodies[1].electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        
        let initial_total_electrons: usize = sim.bodies.iter()
            .map(|body| body.electrons.len())
            .sum();
        
        // Process foils multiple times
        for _ in 0..5 {
            let mut recipients = vec![false; sim.bodies.len()];
            sim.process_foils_with_charge_conservation(0.0, &mut recipients);
            
            // Check total electron count remains constant
            let current_total_electrons: usize = sim.bodies.iter()
                .map(|body| body.electrons.len())
                .sum();
            
            assert_eq!(current_total_electrons, initial_total_electrons, 
                       "Total electron count should be conserved throughout simulation");
        }
    }

    #[test]
    fn test_foils_at_capacity_limits() {
        let mut sim = Simulation::new();
        
        // Create foil bodies at max capacity
        let mut foil_body1 = Body::new(Vec2::new(-10.0, 0.0), Vec2::zero(), 1.0, 1.0, 0.0, Species::FoilMetal);
        let foil_body2 = Body::new(Vec2::new(10.0, 0.0), Vec2::zero(), 1.0, 1.1, 0.0, Species::FoilMetal);
        
        // Fill foil1 to max capacity
        for _ in 0..crate::config::FOIL_MAX_ELECTRONS {
            foil_body1.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
        }
        
        sim.bodies.push(foil_body1);
        sim.bodies.push(foil_body2);
        
        // Create foils ready to operate
        let mut foil1 = Foil::new(vec![sim.bodies[0].id], Vec2::zero(), 1.0, 1.0, 2.0, 0.0);
        foil1.accum = 1.5; // Wants to add but can't (at capacity)
        
        let mut foil2 = Foil::new(vec![sim.bodies[1].id], Vec2::zero(), 1.0, 1.0, -2.0, 0.0);
        foil2.accum = -1.5; // Wants to remove but can't (empty)
        
        sim.foils.push(foil1);
        sim.foils.push(foil2);
        
        let initial_electrons_1 = sim.bodies[0].electrons.len();
        let initial_electrons_2 = sim.bodies[1].electrons.len();
        let initial_accum1 = sim.foils[0].accum;
        let initial_accum2 = sim.foils[1].accum;
        let dt = sim.dt;
        let current1 = 2.0;  // foil1 dc_current  
        let current2 = -2.0; // foil2 dc_current
        
        // Process foils - should do nothing due to capacity constraints
        let mut recipients = vec![false; sim.bodies.len()];
        sim.process_foils_with_charge_conservation(0.0, &mut recipients);
        
        // Verify no operations occurred due to capacity limits
        assert_eq!(sim.bodies[0].electrons.len(), initial_electrons_1, 
                   "Foil at max capacity should not gain electrons");
        assert_eq!(sim.bodies[1].electrons.len(), initial_electrons_2, 
                   "Empty foil should not lose electrons");
        
        // Accumulators should still be updated by current flow
        let expected_accum1 = initial_accum1 + current1 * dt;
        let expected_accum2 = initial_accum2 + current2 * dt;
        assert_eq!(sim.foils[0].accum, expected_accum1, 
                   "Accumulator should be updated by current even when operation fails");
        assert_eq!(sim.foils[1].accum, expected_accum2, 
                   "Accumulator should be updated by current even when operation fails");
    }
}
