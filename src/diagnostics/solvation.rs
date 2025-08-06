// diagnostics/solvation.rs
// Calculates coordination numbers and solvation state distribution

use crate::body::{Body, Species};

pub struct SolvationDiagnostic {
    pub temperature: f32,
    pub avg_li_coordination: f32,
    pub avg_anion_coordination: f32,
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
            temperature: 0.0,
            avg_li_coordination: 0.0,
            avg_anion_coordination: 0.0,
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

    pub fn calculate(&mut self, bodies: &[Body]) {
        const CATION_SHELL_FACTOR: f32 = 4.5; // Larger shell for small lithium ions to capture solvents
        const ANION_SHELL_FACTOR: f32 = 2.5;  // Smaller shell for larger anions
        const CONTACT_BUFFER: f32 = 0.1;

        let mut li_coord_total = 0usize;
        let mut an_coord_total = 0usize;
        let mut li_count = 0usize;
        let mut anion_count = 0usize;

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
                    
                    li_count += 1;
                    let li_shell = body.radius * CATION_SHELL_FACTOR; // Use larger shell for small lithium
                    let li_solvent_ids: Vec<u64> = bodies.iter().enumerate()
                        .filter(|(k, b)| *k != i && matches!(b.species, Species::EC | Species::DMC) && (b.pos - body.pos).mag() < li_shell)
                        .map(|(_, b)| b.id)
                        .collect();
                    let li_solvents = li_solvent_ids.len();
                    li_coord_total += li_solvents;

                    if let Some((j, dist)) = nearest_body_with_species(bodies, i, Species::ElectrolyteAnion) {
                        // Calculate max pairing distance: cation radius + anion radius + 2*average solvent radius
                        let max_pairing_distance = body.radius + bodies[j].radius + 2.0 * avg_solvent_radius;
                        
                        if dist > max_pairing_distance {
                            // Too far from any anion to be considered paired
                            self.fd_ion_ids.push(body.id);
                            self.fd_cations.push((body.id, li_solvent_ids));
                        } else {
                            let an_shell = bodies[j].radius * ANION_SHELL_FACTOR; // Use smaller shell for larger anion
                            let an_solvent_ids: Vec<u64> = bodies.iter().enumerate()
                                .filter(|(k, b)| *k != j && matches!(b.species, Species::EC | Species::DMC) && (b.pos - bodies[j].pos).mag() < an_shell)
                                .map(|(_, b)| b.id)
                                .collect();
                            let an_solvents = an_solvent_ids.len();
                            an_coord_total += an_solvents;

                            let contact_cutoff = body.radius + bodies[j].radius + CONTACT_BUFFER;
                            if dist < contact_cutoff {
                                self.cip_ion_ids.push(body.id);
                                self.cip_pairs.push((body.id, bodies[j].id, li_solvent_ids, an_solvent_ids));
                            } else if li_solvents >= 4 && an_solvents >= 3 { // Relaxed S2IP criteria: at least 1 solvent each
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
                    anion_count += 1;
                    let an_shell = body.radius * ANION_SHELL_FACTOR; // Use anion shell factor
                    let solvents = count_solvent_neighbors(bodies, i, an_shell);
                    an_coord_total += solvents;
                }
                Species::LithiumMetal | Species::FoilMetal | Species::EC | Species::DMC => {}
            }
        }

        // Calculate temperature as average kinetic energy per unit mass (physically correct)
        let undamped_bodies: Vec<&crate::body::Body> = bodies.iter()
            .filter(|body| body.species.damping() >= 1.0)
            .collect();
        self.temperature = if !undamped_bodies.is_empty() {
            let total_temp: f32 = undamped_bodies.iter()
                .map(|b| 0.5 * b.mass * b.vel.mag_sq() / b.mass) // KE/mass per particle
                .sum();
            total_temp / undamped_bodies.len() as f32
        } else {
            0.0
        };
        self.avg_li_coordination = if li_count > 0 { li_coord_total as f32 / li_count as f32 } else { 0.0 };
        self.avg_anion_coordination = if anion_count > 0 { an_coord_total as f32 / anion_count as f32 } else { 0.0 };
        if li_count > 0 {
            self.cip_fraction = self.cip_ion_ids.len() as f32 / li_count as f32;
            self.sip_fraction = self.sip_ion_ids.len() as f32 / li_count as f32;
            self.s2ip_fraction = self.s2ip_ion_ids.len() as f32 / li_count as f32;
            self.fd_fraction = self.fd_ion_ids.len() as f32 / li_count as f32;
        } else {
            self.cip_fraction = 0.0;
            self.sip_fraction = 0.0;
            self.s2ip_fraction = 0.0;
            self.fd_fraction = 0.0;
        }
    }
}

fn count_solvent_neighbors(bodies: &[Body], index: usize, radius: f32) -> usize {
    let pos = bodies[index].pos;
    bodies
        .iter()
        .enumerate()
        .filter(|(i, b)| {
            *i != index && matches!(b.species, Species::EC | Species::DMC) && (b.pos - pos).mag() < radius
        })
        .count()
}

fn nearest_body_with_species(bodies: &[Body], index: usize, species: Species) -> Option<(usize, f32)> {
    let pos = bodies[index].pos;
    let mut best = None;
    let mut best_dist = f32::INFINITY;
    for (i, b) in bodies.iter().enumerate() {
        if i == index || b.species != species {
            continue;
        }
        let dist = (b.pos - pos).mag();
        if dist < best_dist {
            best_dist = dist;
            best = Some(i);
        }
    }
    best.map(|i| (i, best_dist))
}

