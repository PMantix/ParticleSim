// commands.rs
// Handles processing of SimCommand messages for the simulation

use std::sync::atomic::Ordering;
use crate::body::{Species, Electron};
use crate::renderer::state::{SimCommand, PAUSED};
use crate::simulation::Simulation;
use crate::io::{save_state, load_state};
use ultraviolet::Vec2;

#[cfg(feature = "profiling")]
use crate::PROFILER;

const RANDOM_ATTEMPTS: usize = 20;

/// Process a single SimCommand
pub fn process_command(cmd: SimCommand, simulation: &mut Simulation) {
    match cmd {
        // Change charge of a particle by id
        SimCommand::ChangeCharge { id, delta } => {
            handle_change_charge(simulation, id, delta);
        }

        // Add a new body with 1 valence electron, correct charge & species
        SimCommand::AddBody { mut body } => {
            handle_add_body(simulation, &mut body);
        }

        // Delete all bodies in the simulation
        SimCommand::DeleteAll => {
            handle_delete_all(simulation);
        }

        // Add a circle of bodies with given radius, position, count, and species
        SimCommand::AddCircle { body, x, y, radius } => {
            handle_add_circle(simulation, body, x, y, radius);
        }

        // Handle the StepOnce command
        SimCommand::StepOnce => {
            handle_step_once(simulation);
        }

        SimCommand::SaveState { path } => {
            handle_save_state(simulation, path);
        }

        SimCommand::LoadState { path } => {
            handle_load_state(simulation, path);
        }

        SimCommand::AddRing { body, x, y, radius } => {
            handle_add_ring(simulation, body, x, y, radius);
        }

        SimCommand::AddRectangle { body, x, y, width, height } => {
            handle_add_rectangle(simulation, body, x, y, width, height);
        }

        SimCommand::AddRandom { body, count, domain_width, domain_height } => {
            handle_add_random(simulation, body, count, domain_width, domain_height);
        }

        SimCommand::AddFoil { width, height, x, y, particle_radius, current } => {
            handle_add_foil(simulation, width, height, x, y, particle_radius, current);
        }

        SimCommand::SetFoilCurrent { foil_id, current } => {
            handle_set_foil_current(simulation, foil_id, current);
        }

        SimCommand::SetFoilDCCurrent { foil_id, dc_current } => {
            handle_set_foil_dc_current(simulation, foil_id, dc_current);
        }

        SimCommand::SetFoilACCurrent { foil_id, ac_current } => {
            handle_set_foil_ac_current(simulation, foil_id, ac_current);
        }

        SimCommand::SetFoilFrequency { foil_id, switch_hz } => {
            handle_set_foil_frequency(simulation, foil_id, switch_hz);
        }

        SimCommand::LinkFoils { a, b, mode } => {
            handle_link_foils(simulation, a, b, mode);
        }

        SimCommand::UnlinkFoils { a, b } => {
            handle_unlink_foils(simulation, a, b);
        }

        SimCommand::SetDomainSize { width, height } => {
            handle_set_domain_size(simulation, width, height);
        }
    }
}

fn handle_change_charge(simulation: &mut Simulation, id: u64, delta: f32) {
    if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
        // Add or remove electrons based on delta
        if delta > 0.0 {
            // Remove electrons (increase charge)
            for _ in 0..delta.round() as usize {
                body.electrons.pop();
            }
        } else if delta < 0.0 {
            // Add electrons (decrease charge)
            for _ in 0..(-delta).round() as usize {
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let rel_pos = ultraviolet::Vec2::new(angle.cos(), angle.sin()) * body.radius * crate::config::ELECTRON_DRIFT_RADIUS_FACTOR;
                body.electrons.push(crate::body::Electron { rel_pos, vel: ultraviolet::Vec2::zero() });
            }
        }
        body.update_charge_from_electrons();
        println!("Particle {} new charge: {}", id, body.charge);
        println!("Particle {} new electron count: {}", id, body.electrons.len());
        println!("Particle {} new species: {:?}", id, body.species);

        // Update species if charge crosses threshold
        let was_metal = body.species == crate::body::Species::LithiumMetal;
        let was_ion = body.species == crate::body::Species::LithiumIon;
        body.update_species();

        if was_metal && body.species == crate::body::Species::LithiumIon {
            println!();
            println!("Should become ion below...");
            println!("Particle {} new species: {:?}", id, body.species);
        }
        if was_ion && body.species == crate::body::Species::LithiumMetal {
            println!();
            println!("Should become metal below...");
            println!("Particle {} new species: {:?}", id, body.species);
        }

        println!("Particle {} new charge: {}", id, body.charge);
    }
}

fn handle_add_body(simulation: &mut Simulation, body: &mut crate::body::Body) {
    // ensure 1 valence electron, correct charge & species:
    body.electrons.clear();
    if matches!(body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
    }
    body.update_charge_from_electrons();
    body.update_species();
    simulation.bodies.push(body.clone());
}

fn handle_delete_all(simulation: &mut Simulation) {
    simulation.bodies.clear();
    simulation.foils.clear(); // Also clear all foils when deleting all particles
    simulation.body_to_foil.clear();
    // Optionally clear other simulation state if needed
}

fn handle_add_circle(simulation: &mut Simulation, body: crate::body::Body, x: f32, y: f32, radius: f32) {
    let center = Vec2::new(x, y);
    let particle_radius = body.radius;
    let particle_diameter = 2.0 * particle_radius;
    let mut r = particle_radius;
    while r <= radius {
        let circumference = 2.0 * std::f32::consts::PI * r;
        let count = (circumference / particle_diameter).floor() as usize;
        if count == 0 { r += particle_diameter; continue; }
        for i in 0..count {
            let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
            let offset = Vec2::new(angle.cos(), angle.sin()) * r;
            let pos = center + offset;
            // Remove any overlapping particle
            while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
                remove_body_with_foils(simulation, idx);
            }
            let mut new_body = crate::body::Body::new(
                pos,
                Vec2::zero(),
                body.mass,
                body.radius,
                0.0,
                body.species,
            );
            new_body.electrons.clear();
            if matches!(new_body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
                new_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
            new_body.update_charge_from_electrons();
            new_body.update_species();
            simulation.bodies.push(new_body);
        }
        r += particle_diameter;
    }
}

fn handle_step_once(simulation: &mut Simulation) {
    // Manually step the simulation one frame
    simulation.step();
    crate::renderer_utils::render(simulation);
    #[cfg(feature = "profiling")]
    {
        PROFILER.lock().print_and_clear(Some(simulation), None);
    }
    // Optionally, pause the simulation if desired:
    PAUSED.store(true, Ordering::Relaxed);
}

fn handle_save_state(simulation: &Simulation, path: String) {
    if let Err(e) = save_state(path, simulation) {
        eprintln!("Failed to save state: {}", e);
    }
}

fn handle_load_state(simulation: &mut Simulation, path: String) {
    match load_state(path) {
        Ok(state) => state.apply_to(simulation),
        Err(e) => eprintln!("Failed to load state: {}", e),
    }
}

fn handle_add_ring(simulation: &mut Simulation, body: crate::body::Body, x: f32, y: f32, radius: f32) {
    let center = Vec2::new(x, y);
    let particle_radius = body.radius;
    let particle_diameter = 2.0 * particle_radius;
    let circumference = 2.0 * std::f32::consts::PI * radius;
    let count = (circumference / particle_diameter).floor() as usize;
    for i in 0..count {
        let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
        let pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
            remove_body_with_foils(simulation, idx);
        }
    let mut new_body = crate::body::Body::new(
        pos,
        Vec2::zero(),
        body.mass,
        body.radius,
        0.0,
        body.species,
    );
    new_body.electrons.clear();
    if matches!(new_body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
        new_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
    }
        new_body.update_charge_from_electrons();
        new_body.update_species();
        simulation.bodies.push(new_body);
    }
}

fn handle_add_rectangle(simulation: &mut Simulation, body: crate::body::Body, x: f32, y: f32, width: f32, height: f32) {
    let origin = Vec2::new(x, y);
    let particle_radius = body.radius;
    let particle_diameter = 2.0 * particle_radius;
    let cols = (width / particle_diameter).floor() as usize;
    let rows = (height / particle_diameter).floor() as usize;
    for row in 0..rows {
        for col in 0..cols {
            let pos = origin
                + Vec2::new(
                    (col as f32 + 0.5) * particle_diameter,
                    (row as f32 + 0.5) * particle_diameter,
                );
            while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
                remove_body_with_foils(simulation, idx);
            }
            let mut new_body = crate::body::Body::new(
                pos,
                Vec2::zero(),
                body.mass,
                body.radius,
                0.0,
                body.species,
            );
            new_body.electrons.clear();
            if matches!(new_body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
                new_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
            new_body.update_charge_from_electrons();
            new_body.update_species();
            simulation.bodies.push(new_body);
        }
    }
}

fn handle_add_random(simulation: &mut Simulation, body: crate::body::Body, count: usize, domain_width: f32, domain_height: f32) {
    for _ in 0..count {
        let mut placed = false;
        for _ in 0..RANDOM_ATTEMPTS {
            let pos = Vec2::new(
                fastrand::f32() * domain_width - domain_width / 2.0,
                fastrand::f32() * domain_height - domain_height / 2.0,
            );
            if overlaps_any(&simulation.bodies, pos, body.radius).is_none() {
                let mut new_body = crate::body::Body::new(
                    pos,
                    Vec2::zero(),
                    body.mass,
                    body.radius,
                    0.0,
                    body.species,
                );
                new_body.electrons.clear();
                if matches!(new_body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
                    new_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
                new_body.update_charge_from_electrons();
                new_body.update_species();
                simulation.bodies.push(new_body);
                placed = true;
                break;
            }
        }
        if !placed {
            eprintln!("Failed to place random body after {} attempts", RANDOM_ATTEMPTS);
        }
    }
}

fn handle_add_foil(simulation: &mut Simulation, width: f32, height: f32, x: f32, y: f32, particle_radius: f32, current: f32) {
    let origin = Vec2::new(x, y);
    let particle_diameter = 2.0 * particle_radius;
    let cols = (width / particle_diameter).floor() as usize;
    let rows = (height / particle_diameter).floor() as usize;
    let mut body_ids = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let pos = origin
                + Vec2::new(
                    (col as f32 + 0.5) * particle_diameter,
                    (row as f32 + 0.5) * particle_diameter,
                );
            while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
                remove_body_with_foils(simulation, idx);
            }
            let mut new_body = crate::body::Body::new(
                pos,
                Vec2::zero(),
                Species::FoilMetal.mass(),
                particle_radius,
                0.0,
                Species::FoilMetal,
            );
            new_body.electrons = smallvec::smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
            new_body.update_charge_from_electrons();
            body_ids.push(new_body.id);
            simulation.bodies.push(new_body);
        }
    }
    let foil = crate::body::foil::Foil::new(
        body_ids.clone(),
        origin,
        width,
        height,
        current,
        0.0,
    );
    for id in &body_ids {
        simulation.body_to_foil.insert(*id, foil.id);
    }
    simulation.foils.push(foil);
}

fn handle_set_foil_current(simulation: &mut Simulation, foil_id: u64, current: f32) {
    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.body_ids.contains(&foil_id))
    {
        foil.current = current;
        // Also update DC current to maintain compatibility
        foil.dc_current = current;
    }
}

fn handle_set_foil_dc_current(simulation: &mut Simulation, foil_id: u64, dc_current: f32) {
    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.body_ids.contains(&foil_id))
    {
        foil.dc_current = dc_current;
    }
}

fn handle_set_foil_ac_current(simulation: &mut Simulation, foil_id: u64, ac_current: f32) {
    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.body_ids.contains(&foil_id))
    {
        foil.ac_current = ac_current;
    }
}

fn handle_set_foil_frequency(simulation: &mut Simulation, foil_id: u32, switch_hz: f32) {
    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.body_ids.contains(&foil_id))
    {
        foil.switch_hz = switch_hz;
    }
}

fn handle_link_foils(simulation: &mut Simulation, a: u32, b: u32, mode: crate::body::foil::LinkMode) {
    let a_idx = simulation.foils.iter().position(|f| f.id == a);
    let b_idx = simulation.foils.iter().position(|f| f.id == b);
    if let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) {
        // Safe because indices are unique and not equal
        let (first, second) = if a_idx < b_idx {
            let (left, right) = simulation.foils.split_at_mut(b_idx);
            (&mut left[a_idx], &mut right[0])
        } else {
            let (left, right) = simulation.foils.split_at_mut(a_idx);
            (&mut right[0], &mut left[b_idx])
        };
        first.link_id = Some(b);
        second.link_id = Some(a);
        first.mode = mode;
        second.mode = mode;
    }
}

fn handle_unlink_foils(simulation: &mut Simulation, a: u32, b: u32) {
    if let Some(foil_a) = simulation.foils.iter_mut().find(|f| f.id == a && f.link_id == Some(b)) {
        foil_a.link_id = None;
    }
    if let Some(foil_b) = simulation.foils.iter_mut().find(|f| f.id == b && f.link_id == Some(a)) {
        foil_b.link_id = None;
    }
}

fn handle_set_domain_size(simulation: &mut Simulation, width: f32, height: f32) {
    // Update the domain bounds
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    
    // Remove particles that are outside the new domain bounds
    simulation.bodies.retain(|body| {
        body.pos.x >= -half_width &&
        body.pos.x <= half_width &&
        body.pos.y >= -half_height &&
        body.pos.y <= half_height
    });
    
    // Update any simulation domain bounds if they exist
    simulation.bounds = width.max(height) / 2.0;
}

/// Check if a position overlaps with any existing body
fn overlaps_any(existing: &[crate::body::Body], pos: Vec2, radius: f32) -> Option<usize> {
    existing.iter().position(|b| (b.pos - pos).mag() < (b.radius + radius))
}

/// Remove a body and clean up associated foils
fn remove_body_with_foils(simulation: &mut Simulation, idx: usize) {
    let body = simulation.bodies.remove(idx);
    if let Some(foil_id) = simulation.body_to_foil.remove(&body.id) {
        if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
            foil.body_ids.retain(|&id| id != body.id);
        }
    }
}
