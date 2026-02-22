// simulation/intercalation.rs
// Intercalation physics: Li+ absorption/release at electrode surfaces

use super::simulation::Simulation;
use crate::body::Species;
use crate::electrode::ElectrodeRole;
use crate::profile_scope;
use rand::random;

/// Threshold distance (in body radii) for Li+ to be considered "at" the electrode surface
const INTERCALATION_DISTANCE_FACTOR: f32 = 2.5;

/// Minimum probability scaling factor for intercalation per timestep
const BASE_INTERCALATION_PROBABILITY: f32 = 0.01;

/// Check if a species is an intercalation electrode material
fn is_intercalation_electrode(species: Species) -> bool {
    matches!(
        species,
        Species::Graphite
            | Species::HardCarbon
            | Species::SiliconOxide
            | Species::LTO
            | Species::LFP
            | Species::LMFP
            | Species::NMC
            | Species::NCA
    )
}

impl Simulation {
    /// Perform intercalation: Li+ ions near electrode surfaces with excess electrons are absorbed.
    /// The reaction is: Li⁺ + e⁻ → Li(intercalated)
    /// This is called each simulation step after electron hopping.
    pub fn perform_intercalation(&mut self) {
        if self.active_regions.is_empty() {
            return;
        }
        profile_scope!("intercalation");

        let dt = self.dt;
        
        // Find electrode particles with excess electrons near Li+ ions
        // Collect candidates: (li_index, electrode_index, region_index)
        let mut candidates: Vec<(usize, usize, usize)> = Vec::new();
        
        // First, find electrode particles that have excess electrons (ready to accept Li+)
        for (elec_idx, elec_body) in self.bodies.iter().enumerate() {
            if !is_intercalation_electrode(elec_body.species) {
                continue;
            }
            
            // Check if electrode has excess electrons (received from foil via hopping)
            let neutral_count = elec_body.neutral_electron_count();
            if elec_body.electrons.len() <= neutral_count {
                continue; // No excess electrons, can't accept Li+
            }
            
            // Find which active region this electrode particle belongs to
            let region_idx = match self.find_region_for_electrode(elec_body.pos.x, elec_body.pos.y) {
                Some(idx) => idx,
                None => continue,
            };
            
            // Check if this is an anode (accepts Li+ during charge)
            if region_idx >= self.active_regions.len() {
                continue;
            }
            let region = &self.active_regions[region_idx];
            if region.material.role() != ElectrodeRole::Anode {
                continue; // Only anodes intercalate during charging
            }
            if region.is_full() {
                continue; // Electrode is fully lithiated
            }
            
            // Look for nearby Li+ ions
            let search_radius = elec_body.radius * INTERCALATION_DISTANCE_FACTOR;
            let neighbors = self.quadtree.find_neighbors_within(&self.bodies, elec_idx, search_radius);
            
            for &li_idx in &neighbors {
                if li_idx == elec_idx || li_idx >= self.bodies.len() {
                    continue;
                }
                
                let li_body = &self.bodies[li_idx];
                if li_body.species != Species::LithiumIon {
                    continue;
                }
                
                candidates.push((li_idx, elec_idx, region_idx));
                break; // Only one intercalation attempt per electrode per step
            }
        }
        
        // Process intercalation candidates
        // Track which Li+ ions to remove and which electrodes lose an electron
        let mut to_remove: Vec<usize> = Vec::new();
        let mut electrode_consume_electron: Vec<usize> = Vec::new();
        
        for (li_idx, electrode_idx, region_idx) in candidates {
            // Skip if already processed
            if to_remove.contains(&li_idx) || electrode_consume_electron.contains(&electrode_idx) {
                continue;
            }
            
            if region_idx >= self.active_regions.len() {
                continue;
            }
            
            let region = &self.active_regions[region_idx];
            
            // Probability based on:
            // 1. Base rate scaled by timestep
            // 2. Available capacity (more empty = faster absorption)
            // 3. Material kinetics (exchange current)
            let capacity_factor = 1.0 - region.state_of_charge;
            let kinetics_factor = region.material.exchange_current() / 1.0; // Normalized
            let prob = BASE_INTERCALATION_PROBABILITY * capacity_factor * kinetics_factor * dt;
            
            if random::<f32>() < prob {
                // Successful intercalation!
                // Mark Li+ for removal and electrode for electron consumption
                to_remove.push(li_idx);
                electrode_consume_electron.push(electrode_idx);
                
                // Update region (need mutable access)
                let region = &mut self.active_regions[region_idx];
                region.intercalate();
                region.total_intercalated += 1;
            }
        }
        
        // Consume electrons from electrodes that performed intercalation
        // Li⁺ + e⁻ → Li(intercalated), so the electron is "used up"
        for &elec_idx in &electrode_consume_electron {
            if elec_idx < self.bodies.len() && !self.bodies[elec_idx].electrons.is_empty() {
                self.bodies[elec_idx].electrons.pop();
                self.bodies[elec_idx].update_charge_from_electrons();
            }
        }
        
        // Remove absorbed Li+ ions (in reverse order to preserve indices)
        to_remove.sort_unstable();
        to_remove.reverse();
        for idx in to_remove {
            if idx < self.bodies.len() {
                let removed = self.bodies.remove(idx);
                println!("⚡ Li+ intercalated into anode (was body {})", removed.id);
            }
        }
    }
    
    /// Perform deintercalation: Release Li+ from cathodes during charging.
    /// The reaction is: Li(intercalated) → Li⁺ + e⁻
    /// This spawns new Li+ ions at electrode surfaces and adds electrons to cathode particles.
    pub fn perform_deintercalation(&mut self) {
        if self.active_regions.is_empty() {
            return;
        }
        profile_scope!("deintercalation");
        
        let dt = self.dt;
        
        // For each cathode region, check if we should deintercalate
        // We need to find cathode particles that can accept the released electron
        let mut deintercalation_events: Vec<(usize, f32, f32)> = Vec::new(); // (region_idx, spawn_x, spawn_y)
        
        for (region_idx, region) in self.active_regions.iter().enumerate() {
            if region.is_empty() {
                continue; // No Li to release
            }
            
            // Only cathodes release Li+ during charging
            if region.material.role() != ElectrodeRole::Cathode {
                continue;
            }
            
            // Probability based on SOC and kinetics
            let soc_factor = region.state_of_charge;
            let kinetics_factor = region.material.exchange_current() / 1.0;
            let prob = BASE_INTERCALATION_PROBABILITY * soc_factor * kinetics_factor * dt;
            
            if random::<f32>() < prob {
                // Calculate spawn position
                let spawn_x = region.center_x + (random::<f32>() - 0.5) * 20.0;
                let spawn_y = region.center_y + (random::<f32>() - 0.5) * 20.0;
                deintercalation_events.push((region_idx, spawn_x, spawn_y));
            }
        }
        
        // Process deintercalation events
        let mut spawned_bodies: Vec<crate::body::Body> = Vec::new();
        
        for (region_idx, spawn_x, spawn_y) in deintercalation_events {
            // Find a nearby cathode particle to receive the electron
            // The electron will then hop toward the foil
            let mut found_cathode_body = None;
            for (body_idx, body) in self.bodies.iter().enumerate() {
                if !is_intercalation_electrode(body.species) {
                    continue;
                }
                // Check if this is a cathode material
                if !matches!(body.species, Species::LFP | Species::LMFP | Species::NMC | Species::NCA) {
                    continue;
                }
                
                // Check distance to spawn point
                let dx = body.pos.x - spawn_x;
                let dy = body.pos.y - spawn_y;
                if dx * dx + dy * dy < 400.0 { // Within 20 units
                    // Check if can accept electron (not at max)
                    if body.electrons.len() < crate::config::ELECTRODE_CATHODE_MAX_ELECTRONS {
                        found_cathode_body = Some(body_idx);
                        break;
                    }
                }
            }
            
            // Only proceed if we found a cathode body to receive the electron
            if let Some(cathode_idx) = found_cathode_body {
                // Deintercalate from region
                let region = &mut self.active_regions[region_idx];
                if region.deintercalate() {
                    region.total_deintercalated += 1;
                    
                    // Add electron to cathode particle (will hop toward foil)
                    let angle = random::<f32>() * std::f32::consts::TAU;
                    let cathode = &mut self.bodies[cathode_idx];
                    let rel_pos = ultraviolet::Vec2::new(angle.cos(), angle.sin())
                        * cathode.radius * cathode.species.polar_offset();
                    cathode.electrons.push(crate::body::Electron {
                        rel_pos,
                        vel: ultraviolet::Vec2::zero(),
                    });
                    cathode.update_charge_from_electrons();
                    
                    // Spawn Li+ ion
                    let vel_x = (random::<f32>() - 0.5) * 0.1;
                    let vel_y = (random::<f32>() - 0.5) * 0.1;
                    
                    let li_ion = crate::body::Body::new(
                        ultraviolet::Vec2::new(spawn_x, spawn_y),
                        ultraviolet::Vec2::new(vel_x, vel_y),
                        Species::LithiumIon.mass(),
                        Species::LithiumIon.radius(),
                        1.0, // Li+ has +1 charge
                        Species::LithiumIon,
                    );
                    
                    spawned_bodies.push(li_ion);
                    println!("⚡ Li+ deintercalated from cathode at ({:.1}, {:.1})", spawn_x, spawn_y);
                }
            }
        }
        
        // Add spawned Li+ to simulation
        for body in spawned_bodies {
            self.bodies.push(body);
        }
    }
    
    /// Find which active region an electrode at (x, y) belongs to.
    /// Returns the region index if found.
    fn find_region_for_electrode(&self, x: f32, y: f32) -> Option<usize> {
        let mut best_idx = None;
        let mut best_dist_sq = f32::MAX;
        
        // Simple distance-based matching to region centers
        // In the future, could use body_ids in regions for precise matching
        for (idx, region) in self.active_regions.iter().enumerate() {
            let dx = x - region.center_x;
            let dy = y - region.center_y;
            let dist_sq = dx * dx + dy * dy;
            
            // Only match if within reasonable electrode size (50 units)
            if dist_sq < 2500.0 && dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_idx = Some(idx);
            }
        }
        
        best_idx
    }
}
