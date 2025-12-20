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
    /// Perform intercalation: Li+ ions near electrode surfaces may be absorbed.
    /// This is called each simulation step after electron hopping.
    pub fn perform_intercalation(&mut self) {
        if self.active_regions.is_empty() {
            return;
        }
        profile_scope!("intercalation");

        let dt = self.dt;
        
        // Find Li+ ions near intercalation electrode surfaces
        // Collect candidates: (li_index, region_index, electrode_body_index)
        let mut candidates: Vec<(usize, usize, usize)> = Vec::new();
        
        for (li_idx, li_body) in self.bodies.iter().enumerate() {
            if li_body.species != Species::LithiumIon {
                continue;
            }
            
            // Check distance to each active region's electrode particles
            let search_radius = li_body.radius * INTERCALATION_DISTANCE_FACTOR;
            let neighbors = self.quadtree.find_neighbors_within(&self.bodies, li_idx, search_radius);
            
            for &neighbor_idx in &neighbors {
                if neighbor_idx == li_idx || neighbor_idx >= self.bodies.len() {
                    continue;
                }
                
                let neighbor = &self.bodies[neighbor_idx];
                if !is_intercalation_electrode(neighbor.species) {
                    continue;
                }
                
                // Find which active region this electrode particle belongs to
                // For now, use position-based matching (closest region center)
                if let Some(region_idx) = self.find_region_for_electrode(neighbor.pos.x, neighbor.pos.y) {
                    candidates.push((li_idx, region_idx, neighbor_idx));
                    break; // Only one intercalation attempt per Li+ per step
                }
            }
        }
        
        // Process intercalation candidates
        // Track which Li+ ions to remove
        let mut to_remove: Vec<usize> = Vec::new();
        
        for (li_idx, region_idx, _electrode_idx) in candidates {
            // Check if region can accept more Li
            if region_idx >= self.active_regions.len() {
                continue;
            }
            
            let region = &self.active_regions[region_idx];
            if region.is_full() {
                continue; // Electrode is fully lithiated
            }
            
            // Electrochemical driving force based on overpotential
            // For now, use a simplified model:
            // - Anodes (low voltage) accept Li+ during charge
            // - Cathodes (high voltage) release Li+ during charge
            let role = region.material.role();
            let accept_li = match role {
                ElectrodeRole::Anode => true,  // Anodes accept Li+ during charging
                ElectrodeRole::Cathode => false, // Cathodes release Li+ during charging
            };
            
            if !accept_li {
                continue;
            }
            
            // Probability based on:
            // 1. Base rate scaled by timestep
            // 2. Available capacity (more empty = faster absorption)
            // 3. Material kinetics (exchange current)
            let capacity_factor = 1.0 - region.state_of_charge;
            let kinetics_factor = region.material.exchange_current() / 1.0; // Normalized
            let prob = BASE_INTERCALATION_PROBABILITY * capacity_factor * kinetics_factor * dt;
            
            if random::<f32>() < prob {
                // Successful intercalation!
                // Mark Li+ for removal and update region
                to_remove.push(li_idx);
                
                // Update region (need mutable access)
                let region = &mut self.active_regions[region_idx];
                region.intercalate();
                region.total_intercalated += 1;
            }
        }
        
        // Remove absorbed Li+ ions (in reverse order to preserve indices)
        to_remove.sort_unstable();
        to_remove.reverse();
        for idx in to_remove {
            if idx < self.bodies.len() {
                let removed = self.bodies.remove(idx);
                println!("⚡ Li+ intercalated into electrode (was body {})", removed.id);
            }
        }
    }
    
    /// Perform deintercalation: Release Li+ from electrodes during discharge.
    /// This spawns new Li+ ions at electrode surfaces.
    pub fn perform_deintercalation(&mut self) {
        if self.active_regions.is_empty() {
            return;
        }
        profile_scope!("deintercalation");
        
        let dt = self.dt;
        let mut spawned_bodies: Vec<crate::body::Body> = Vec::new();
        
        for region in &mut self.active_regions {
            if region.is_empty() {
                continue; // No Li to release
            }
            
            // Only cathodes release Li+ during charging (for now)
            let role = region.material.role();
            let release_li = match role {
                ElectrodeRole::Anode => false,
                ElectrodeRole::Cathode => true,
            };
            
            if !release_li {
                continue;
            }
            
            // Probability based on SOC and kinetics
            let soc_factor = region.state_of_charge;
            let kinetics_factor = region.material.exchange_current() / 1.0;
            let prob = BASE_INTERCALATION_PROBABILITY * soc_factor * kinetics_factor * dt;
            
            if random::<f32>() < prob {
                // Deintercalate one Li
                if region.deintercalate() {
                    region.total_deintercalated += 1;
                    
                    // Spawn a new Li+ ion near the electrode surface
                    let spawn_x = region.center_x + (random::<f32>() - 0.5) * 20.0;
                    let spawn_y = region.center_y + (random::<f32>() - 0.5) * 20.0;
                    
                    // Initial velocity pointing away from electrode
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
