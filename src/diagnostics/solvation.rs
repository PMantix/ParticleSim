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
        }
    }

    pub fn calculate(&mut self, bodies: &[Body]) {
        const SHELL_FACTOR: f32 = 3.0;
        const CONTACT_BUFFER: f32 = 0.1;

        let mut li_coord_total = 0usize;
        let mut an_coord_total = 0usize;
        let mut li_count = 0usize;
        let mut anion_count = 0usize;

        let mut cip = 0usize;
        let mut sip = 0usize;
        let mut s2ip = 0usize;
        let mut fd = 0usize;

        for (i, body) in bodies.iter().enumerate() {
            match body.species {
                Species::LithiumIon => {
                    li_count += 1;
                    let shell = body.radius * SHELL_FACTOR;
                    let li_solvents = count_solvent_neighbors(bodies, i, shell);
                    li_coord_total += li_solvents;

                    if let Some((j, dist)) = nearest_body_with_species(bodies, i, Species::ElectrolyteAnion) {
                        let an_shell = bodies[j].radius * SHELL_FACTOR;
                        let an_solvents = count_solvent_neighbors(bodies, j, an_shell);
                        an_coord_total += an_solvents;

                        let contact_cutoff = body.radius + bodies[j].radius + CONTACT_BUFFER;
                        if dist < contact_cutoff {
                            cip += 1;
                        } else if li_solvents >= 4 && an_solvents >= 4 {
                            s2ip += 1;
                        } else {
                            sip += 1;
                        }
                    } else {
                        fd += 1;
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
            self.cip_fraction = cip as f32 / li_count as f32;
            self.sip_fraction = sip as f32 / li_count as f32;
            self.s2ip_fraction = s2ip as f32 / li_count as f32;
            self.fd_fraction = fd as f32 / li_count as f32;
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

