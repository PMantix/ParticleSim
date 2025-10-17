// diagnostics/solvation.rs
// Calculates coordination numbers and solvation state distribution

use crate::body::{Body, Species};
use crate::quadtree::Quadtree;
use crate::profile_scope;

/// Calculate 3D distance between two bodies, accounting for z-coordinates
fn distance_3d(body1: &Body, body2: &Body) -> f32 {
    let dx = body1.pos.x - body2.pos.x;
    let dy = body1.pos.y - body2.pos.y;
    let dz = body1.z - body2.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub struct SolvationDiagnostic {
    pub cip_fraction: f32,
    pub sip_fraction: f32,
    pub s2ip_fraction: f32,
    pub fd_fraction: f32,
    
    // Ion ID lists for visual overlays
    pub cip_ion_ids: Vec<u64>,
    pub sip_ion_ids: Vec<u64>,
    pub s2ip_ion_ids: Vec<u64>,
    pub fd_ion_ids: Vec<u64>,
    // New: For overlays, track paired anion and solvent IDs for each cation
    pub cip_pairs: Vec<(u64, u64, Vec<u64>, Vec<u64>)>, // (cation_id, anion_id, cation_solvent_ids, anion_solvent_ids)
    pub sip_pairs: Vec<(u64, u64, Vec<u64>, Vec<u64>)>,
    pub s2ip_pairs: Vec<(u64, u64, Vec<u64>, Vec<u64>)>,
    pub fd_cations: Vec<(u64, Vec<u64>)>, // (cation_id, cation_solvent_ids)
}

impl SolvationDiagnostic {
    pub fn new() -> Self {
        Self {
            cip_fraction: 0.0,
            sip_fraction: 0.0,
            s2ip_fraction: 0.0,
            fd_fraction: 0.0,
            
            // Initialize empty ion ID lists
            cip_ion_ids: Vec::new(),
            sip_ion_ids: Vec::new(),
            s2ip_ion_ids: Vec::new(),
            fd_ion_ids: Vec::new(),
            cip_pairs: Vec::new(),
            sip_pairs: Vec::new(),
            s2ip_pairs: Vec::new(),
            fd_cations: Vec::new(),
        }
    }

    /// Calculate solvation statistics using quadtree for spatial optimization
    pub fn calculate(&mut self, bodies: &[Body], quadtree: &Quadtree) {
        profile_scope!("solvation_calculation_internal");
        const CATION_SHELL_FACTOR: f32 = 4.5; // Larger shell for small lithium ions to capture solvents
        const ANION_SHELL_FACTOR: f32 = 2.5;  // Smaller shell for larger anions
        const CONTACT_BUFFER: f32 = 0.1;

    // Coordination counts removed for performance (no longer reported)

        // Clear previous ion ID lists
        self.cip_ion_ids.clear();
        self.sip_ion_ids.clear();
        self.s2ip_ion_ids.clear();
        self.fd_ion_ids.clear();
        self.cip_pairs.clear();
        self.sip_pairs.clear();
        self.s2ip_pairs.clear();
        self.fd_cations.clear();

        // Get typical solvent radius (average of EC and DMC)
        let ec_radius = crate::body::Species::EC.radius();
        let dmc_radius = crate::body::Species::DMC.radius();
        let avg_solvent_radius = (ec_radius + dmc_radius) / 2.0;

        for (i, body) in bodies.iter().enumerate() {
            match body.species {
                Species::LithiumIon => {
                    // Skip ions that are surrounded by metal (not truly ionic)
                    if body.surrounded_by_metal {
                        continue;
                    }
                    
                    let li_shell = body.radius * CATION_SHELL_FACTOR;
                    
                    // Use quadtree to find nearby particles instead of linear search
                    let nearby_indices = quadtree.find_neighbors_within(bodies, i, li_shell);
                    let li_solvent_ids: Vec<u64> = nearby_indices.iter()
                        .filter_map(|&idx| {
                            let neighbor = &bodies[idx];
                            if matches!(neighbor.species, Species::EC | Species::DMC) {
                                Some(neighbor.id)
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    let li_solvents = li_solvent_ids.len();

                    // Find nearest anion using quadtree - search in expanding radius
                    let max_search_radius = body.radius + bodies.iter()
                        .filter(|b| b.species == Species::ElectrolyteAnion)
                        .map(|b| b.radius)
                        .fold(0.0, f32::max) + 2.0 * avg_solvent_radius + 50.0; // Add buffer
                    
                    if let Some((j, dist)) = self.find_nearest_anion_with_quadtree(bodies, quadtree, i, max_search_radius) {
                        // Calculate max pairing distance: cation radius + anion radius + 2*average solvent radius
                        let max_pairing_distance = body.radius + bodies[j].radius + 2.0 * avg_solvent_radius;
                        
                        if dist > max_pairing_distance {
                            // Too far from any anion to be considered paired
                            self.fd_ion_ids.push(body.id);
                            self.fd_cations.push((body.id, li_solvent_ids));
                        } else {
                            let an_shell = bodies[j].radius * ANION_SHELL_FACTOR;
                            
                            // Use quadtree for anion's solvent neighbors too
                            let anion_nearby_indices = quadtree.find_neighbors_within(bodies, j, an_shell);
                            let an_solvent_ids: Vec<u64> = anion_nearby_indices.iter()
                                .filter_map(|&idx| {
                                    let neighbor = &bodies[idx];
                                    if matches!(neighbor.species, Species::EC | Species::DMC) {
                                        Some(neighbor.id)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            
                            let an_solvents = an_solvent_ids.len();

                            let contact_cutoff = body.radius + bodies[j].radius + CONTACT_BUFFER;
                            if dist < contact_cutoff {
                                self.cip_ion_ids.push(body.id);
                                self.cip_pairs.push((body.id, bodies[j].id, li_solvent_ids, an_solvent_ids));
                            } else if li_solvents >= 3 && an_solvents >= 2 {
                                self.s2ip_ion_ids.push(body.id);
                                self.s2ip_pairs.push((body.id, bodies[j].id, li_solvent_ids, an_solvent_ids));
                            } else {
                                self.sip_ion_ids.push(body.id);
                                self.sip_pairs.push((body.id, bodies[j].id, li_solvent_ids, an_solvent_ids));
                            }
                        }
                    } else {
                        // No anions exist in the simulation
                        self.fd_ion_ids.push(body.id);
                        self.fd_cations.push((body.id, li_solvent_ids));
                    }
                }
                Species::ElectrolyteAnion => {
                    let _an_shell = body.radius * ANION_SHELL_FACTOR;
                    // Skipping anion coordination calculation (not used)
                }
                Species::LithiumMetal | Species::FoilMetal | Species::EC | Species::DMC => {}
            }
        }

        let total_ions = self.cip_ion_ids.len() + self.sip_ion_ids.len() + self.s2ip_ion_ids.len() + self.fd_ion_ids.len();
        if total_ions > 0 {
            self.cip_fraction = self.cip_ion_ids.len() as f32 / total_ions as f32;
            self.sip_fraction = self.sip_ion_ids.len() as f32 / total_ions as f32;
            self.s2ip_fraction = self.s2ip_ion_ids.len() as f32 / total_ions as f32;
            self.fd_fraction = self.fd_ion_ids.len() as f32 / total_ions as f32;
        } else {
            self.cip_fraction = 0.0;
            self.sip_fraction = 0.0;
            self.s2ip_fraction = 0.0;
            self.fd_fraction = 0.0;
        }
    }

    /// Helper method to find nearest anion using quadtree with expanding search radius
    fn find_nearest_anion_with_quadtree(&self, bodies: &[Body], quadtree: &Quadtree, index: usize, max_radius: f32) -> Option<(usize, f32)> {
        let body_ref = &bodies[index];
        let mut search_radius = body_ref.radius * 3.0; // Start with small radius
        
        while search_radius <= max_radius {
            let nearby_indices = quadtree.find_neighbors_within(bodies, index, search_radius);
            
            let mut best = None;
            let mut best_dist = f32::INFINITY;
            
            for &idx in &nearby_indices {
                if bodies[idx].species == Species::ElectrolyteAnion {
                    let dist = distance_3d(&bodies[idx], body_ref);
                    if dist < best_dist {
                        best_dist = dist;
                        best = Some(idx);
                    }
                }
            }
            
            if best.is_some() {
                return best.map(|i| (i, best_dist));
            }
            
            // Expand search radius
            search_radius *= 2.0;
        }
        
        None
    }
}

