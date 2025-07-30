// diagnostics/solvation.rs
// Calculates coordination numbers and solvation state distribution

use crate::body::{Body, Species};
use crate::simulation::compute_temperature;

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
        }
    }

    pub fn calculate(&mut self, bodies: &[Body]) {
        const SHELL_FACTOR: f32 = 2.0; // Reduced from 3.0 for tighter solvation shell
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

        // Get typical solvent radius (average of EC and DMC)
        let ec_radius = crate::body::Species::EC.radius();
        let dmc_radius = crate::body::Species::DMC.radius();
        let avg_solvent_radius = (ec_radius + dmc_radius) / 2.0;

        for (i, body) in bodies.iter().enumerate() {
            match body.species {
                Species::LithiumIon => {
                    li_count += 1;
                    let shell = body.radius * SHELL_FACTOR;
                    let li_solvents = count_solvent_neighbors(bodies, i, shell);
                    li_coord_total += li_solvents;

                    if let Some((j, dist)) = nearest_body_with_species(bodies, i, Species::ElectrolyteAnion) {
                        // Calculate max pairing distance: Li+ radius + 1.5*solvent radius + anion radius
                        let max_pairing_distance = body.radius + 1.5 * avg_solvent_radius + bodies[j].radius;
                        
                        if dist > max_pairing_distance {
                            // Too far from any anion to be considered paired
                            self.fd_ion_ids.push(body.id);
                        } else {
                            let an_shell = bodies[j].radius * SHELL_FACTOR;
                            let an_solvents = count_solvent_neighbors(bodies, j, an_shell);
                            an_coord_total += an_solvents;

                            let contact_cutoff = body.radius + bodies[j].radius + CONTACT_BUFFER;
                            if dist < contact_cutoff {
                                self.cip_ion_ids.push(body.id);
                            } else if li_solvents >= 2 && an_solvents >= 2 {
                                self.s2ip_ion_ids.push(body.id);
                            } else {
                                self.sip_ion_ids.push(body.id);
                            }
                        }
                    } else {
                        // No anions exist in the simulation
                        self.fd_ion_ids.push(body.id);
                    }
                }
                Species::ElectrolyteAnion => {
                    anion_count += 1;
                    let shell = body.radius * SHELL_FACTOR;
                    let solvents = count_solvent_neighbors(bodies, i, shell);
                    an_coord_total += solvents;
                }
                _ => {}
            }
        }

        self.temperature = compute_temperature(bodies);
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

