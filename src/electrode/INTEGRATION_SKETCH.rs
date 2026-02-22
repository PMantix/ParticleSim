// IMPLEMENTATION SKETCH: Integration with Simulation
// 
// This file shows how the electrode module integrates with the main simulation.
// This is NOT meant to be compiled directly - it's a reference for implementation.
//
// Key integration points:
// 1. Simulation struct - add active_material_regions field
// 2. Simulation::step() - add intercalation phase
// 3. Renderer - use region SOC for particle coloring
// 4. Commands - add CreateActiveMaterialRegion command

// ============================================================================
// PART 1: Simulation struct additions
// ============================================================================

// In src/simulation/simulation.rs, add to Simulation struct:
/*
use crate::electrode::{ActiveMaterialRegion, MaterialType, IntercalationConfig, IntercalationStats};

pub struct Simulation {
    // ... existing fields ...
    
    /// Active material regions (intercalation electrodes)
    pub active_regions: Vec<ActiveMaterialRegion>,
    
    /// Map body ID → region ID for quick lookup
    pub body_to_region: HashMap<u64, u64>,
    
    /// Intercalation physics configuration
    pub intercalation_config: IntercalationConfig,
    
    /// Statistics for the current step
    pub intercalation_stats: IntercalationStats,
}
*/

// ============================================================================
// PART 2: New simulation phase - perform_intercalation()
// ============================================================================

// Add to src/simulation/mod.rs or new file src/simulation/intercalation_sim.rs

/*
impl Simulation {
    /// Main intercalation processing for each timestep
    /// 
    /// This handles:
    /// 1. Li⁺ approaching electrode surfaces → desolvation → intercalation
    /// 2. Li deintercalation from electrodes under reverse overpotential
    pub fn perform_intercalation(&mut self) {
        if !self.intercalation_config.enabled {
            return;
        }
        
        // Skip if no active regions
        if self.active_regions.is_empty() {
            return;
        }
        
        profile_scope!("intercalation");
        
        let dt = self.dt;
        let config = self.intercalation_config.clone();
        
        // ---- Phase 1: Find Li⁺ candidates near active material surfaces ----
        
        let mut intercalation_events: Vec<(usize, u64)> = vec![]; // (li_idx, region_id)
        let mut deintercalation_spawns: Vec<(u64, Vec2)> = vec![]; // (region_id, spawn_pos)
        
        for (li_idx, body) in self.bodies.iter().enumerate() {
            // Only process Li⁺ ions
            if body.species != Species::LithiumIon {
                continue;
            }
            
            // Find nearby electrode surface particles
            let search_radius = config.surface_distance;
            let neighbors = self.quadtree.find_neighbors_within(&self.bodies, li_idx, search_radius);
            
            for &neighbor_idx in &neighbors {
                let neighbor = &self.bodies[neighbor_idx];
                
                // Check if neighbor is part of an active region
                if let Some(&region_id) = self.body_to_region.get(&neighbor.id) {
                    // Get the region
                    let region = self.active_regions.iter()
                        .find(|r| r.id == region_id)
                        .unwrap();
                    
                    // Skip if at capacity
                    if region.is_full() {
                        self.intercalation_stats.rejected_at_capacity += 1;
                        continue;
                    }
                    
                    // Count nearby solvent for desolvation barrier
                    let solvation_number = self.count_nearby_solvent(li_idx);
                    
                    // Calculate desolvation probability
                    let prob = crate::electrode::intercalation::desolvation_probability(
                        region.material,
                        solvation_number,
                        &config,
                        dt
                    );
                    
                    self.intercalation_stats.attempted_intercalations += 1;
                    
                    if rand::random::<f32>() < prob {
                        intercalation_events.push((li_idx, region_id));
                        break; // Only intercalate into one region
                    } else {
                        self.intercalation_stats.rejected_barrier += 1;
                    }
                }
            }
        }
        
        // ---- Phase 2: Process intercalation events ----
        
        // Sort by index descending so we can remove without shifting issues
        intercalation_events.sort_by(|a, b| b.0.cmp(&a.0));
        
        for (li_idx, region_id) in intercalation_events {
            // Find region and intercalate
            if let Some(region) = self.active_regions.iter_mut().find(|r| r.id == region_id) {
                if region.intercalate() {
                    // Mark Li⁺ for removal (absorbed into electrode)
                    self.bodies[li_idx].species = Species::Absorbed; // New species, or use a flag
                    self.intercalation_stats.successful_intercalations += 1;
                }
            }
        }
        
        // Remove absorbed particles
        self.bodies.retain(|b| b.species != Species::Absorbed);
        
        // ---- Phase 3: Process deintercalation (opposite direction) ----
        // This happens when overpotential drives Li out of electrode
        
        for region in &mut self.active_regions {
            if region.is_empty() {
                continue;
            }
            
            // Get foil potential if available
            let applied_potential = if let Some(foil_id) = region.foil_id {
                self.get_foil_potential(foil_id)
            } else {
                0.0
            };
            
            let direction = crate::electrode::intercalation::reaction_direction(
                region.material,
                region.state_of_charge,
                applied_potential
            );
            
            if direction == crate::electrode::intercalation::ReactionDirection::Oxidation {
                // Calculate deintercalation rate
                let rate = crate::electrode::intercalation::butler_volmer_rate(
                    region.material,
                    region.state_of_charge,
                    applied_potential,
                    &config,
                    dt
                );
                
                self.intercalation_stats.attempted_deintercalations += 1;
                
                if rand::random::<f32>() < rate {
                    // Find a surface body to spawn Li⁺ near
                    if let Some(&surface_body_id) = region.surface_body_ids.iter().next() {
                        if let Some(surface_body) = self.bodies.iter().find(|b| b.id == surface_body_id) {
                            let spawn_pos = surface_body.pos + random_offset(surface_body.radius);
                            deintercalation_spawns.push((region.id, spawn_pos));
                        }
                    }
                }
            }
        }
        
        // ---- Phase 4: Spawn deintercalated Li⁺ ----
        
        for (region_id, spawn_pos) in deintercalation_spawns {
            if let Some(region) = self.active_regions.iter_mut().find(|r| r.id == region_id) {
                if region.deintercalate() {
                    // Create new Li⁺ at spawn position
                    let new_li = Body::new_from_species(
                        spawn_pos,
                        Vec2::zero(),
                        1.0, // Li⁺ charge
                        Species::LithiumIon
                    );
                    self.bodies.push(new_li);
                    self.intercalation_stats.successful_deintercalations += 1;
                }
            }
        }
    }
    
    /// Count solvent molecules near a Li⁺ ion (for solvation shell)
    fn count_nearby_solvent(&self, li_idx: usize) -> usize {
        let search_radius = 5.0; // Å - typical solvation shell radius
        let neighbors = self.quadtree.find_neighbors_within(&self.bodies, li_idx, search_radius);
        
        neighbors.iter()
            .filter(|&&n| matches!(
                self.bodies[n].species,
                Species::EC | Species::DMC | Species::VC | Species::FEC | Species::EMC
            ))
            .count()
    }
    
    /// Get effective potential at a foil (placeholder - integrate with existing foil logic)
    fn get_foil_potential(&self, foil_id: u64) -> f32 {
        // This would integrate with the existing foil charging logic
        // For now, derive from electron count
        if let Some(foil) = self.foils.iter().find(|f| f.id == foil_id) {
            let body_ids = &foil.body_ids;
            let total_electrons: usize = self.bodies.iter()
                .filter(|b| body_ids.contains(&b.id))
                .map(|b| b.electrons.len())
                .sum();
            let neutral = body_ids.len(); // Assume 1 neutral electron per body
            let excess = total_electrons as f32 - neutral as f32;
            
            // Rough mapping: excess electrons → negative potential
            -excess * 0.1 // Very simplified!
        } else {
            0.0
        }
    }
}

fn random_offset(radius: f32) -> Vec2 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let angle = rng.gen::<f32>() * std::f32::consts::TAU;
    let r = radius * 1.5;
    Vec2::new(angle.cos() * r, angle.sin() * r)
}
*/

// ============================================================================
// PART 3: Integration into step() function
// ============================================================================

/*
// In Simulation::step(), add intercalation phase:

pub fn step(&mut self) {
    // ... existing phases: attract, apply_polar_forces, etc. ...
    
    // Electron hopping
    self.perform_electron_hopping();
    
    // === NEW: Intercalation processing ===
    self.perform_intercalation();
    
    // SEI formation
    self.perform_sei_formation();
    
    // ... remaining phases: collision, thermostat, etc. ...
}
*/

// ============================================================================
// PART 4: Renderer integration for SOC-based coloring
// ============================================================================

/*
// In src/renderer/draw/mod.rs, modify body coloring:

// Add this to Renderer struct:
pub active_regions: Vec<ActiveMaterialRegion>,
pub body_to_region: HashMap<u64, u64>,

// Modify the body drawing loop:
for body in &self.bodies {
    let color = if let Some(&region_id) = self.body_to_region.get(&body.id) {
        // This body is part of an active material region
        // Color based on SOC
        if let Some(region) = self.active_regions.iter().find(|r| r.id == region_id) {
            region.current_color()
        } else {
            body.species.color()
        }
    } else {
        // Normal species coloring
        body.species.color()
    };
    
    // ... rest of drawing logic ...
}
*/

// ============================================================================
// PART 5: New SimCommand variants
// ============================================================================

/*
// In src/renderer/state.rs or wherever SimCommand is defined:

pub enum SimCommand {
    // ... existing variants ...
    
    /// Create an active material region from existing particles
    CreateActiveMaterialRegion {
        material: MaterialType,
        body_ids: Vec<u64>,
        foil_id: Option<u64>,
        initial_soc: f32,
    },
    
    /// Set state of charge for a region (for testing/manual control)
    SetRegionSOC {
        region_id: u64,
        soc: f32,
    },
    
    /// Configure intercalation physics
    ConfigureIntercalation {
        config: IntercalationConfig,
    },
}
*/

// ============================================================================
// PART 6: Example usage in scenario configuration
// ============================================================================

/*
// In init_config.toml:

[[electrodes]]
name = "graphite_anode"
material = "Graphite"
role = "anode"
x = -150.0
y = 0.0
width = 50.0
height = 350.0
initial_soc = 1.0  # Start fully lithiated (charged state)

[[electrodes]]
name = "lfp_cathode"
material = "LFP"
role = "cathode"
x = 150.0
y = 0.0
width = 50.0
height = 350.0
initial_soc = 0.0  # Start delithiated (charged state)

[electrolyte]
molarity = 1.0
solvents = [
    { species = "EC", volume_fraction = 0.3 },
    { species = "DMC", volume_fraction = 0.7 },
]
*/

// ============================================================================
// PART 7: Species enum additions (minimal)
// ============================================================================

/*
// In src/body/types.rs:

pub enum Species {
    // ... existing species ...
    
    // Marker for particles being removed via intercalation
    // (alternative: use a separate Vec of indices to remove)
    Absorbed,
    
    // Active material surface particles (if we want distinct rendering)
    // These replace some of the LithiumMetal particles at electrode surfaces
    GraphiteSurface,
    LFPSurface,
    NMCSurface,
    // etc.
}

// However, for the color-gradient approach, we might NOT need new species.
// Instead, we just:
// 1. Keep existing LithiumMetal (or add GraphiteSurface) particles
// 2. Override their color based on region SOC at render time
// 3. Track lithiation as a counter, not as particle species changes
*/

// ============================================================================
// SUMMARY OF CHANGES NEEDED
// ============================================================================

/*
Files to modify:
1. src/lib.rs - add `pub mod electrode;`
2. src/simulation/simulation.rs - add active_regions field, body_to_region map
3. src/simulation/mod.rs - add intercalation module or inline in simulation.rs
4. src/renderer/state.rs - add new SimCommand variants
5. src/renderer/draw/mod.rs - modify coloring logic
6. src/init_config.rs - add electrode configuration parsing
7. src/scenario.rs - add electrode region creation

New files (already created as sketches):
- src/electrode/mod.rs
- src/electrode/material.rs
- src/electrode/region.rs
- src/electrode/intercalation.rs

Optional UI additions:
- src/renderer/gui/electrode_material_tab.rs - new tab for material configuration
*/
