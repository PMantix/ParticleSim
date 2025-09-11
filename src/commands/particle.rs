use crate::simulation::Simulation;
use crate::body::{Species, Electron};
use ultraviolet::Vec2;

/// Number of attempts when placing random particles.
const RANDOM_ATTEMPTS: usize = 20;

pub fn handle_change_charge(simulation: &mut Simulation, id: u64, delta: f32) {
    if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
        if delta > 0.0 {
            for _ in 0..delta.round() as usize {
                body.electrons.pop();
            }
        } else if delta < 0.0 {
            // Calculate maximum electrons allowed for this species
            let max_electrons = match body.species {
                Species::FoilMetal => crate::config::FOIL_MAX_ELECTRONS,
                Species::LithiumMetal => crate::config::LITHIUM_METAL_MAX_ELECTRONS,
                _ => usize::MAX, // No limit for other species
            };
            
            let requested_additions = (-delta).round() as usize;
            let current_count = body.electrons.len();
            let available_capacity = max_electrons.saturating_sub(current_count);
            let actual_additions = requested_additions.min(available_capacity);
            
            if actual_additions < requested_additions {
                println!("Warning: Particle {} can only accept {} more electrons (max: {}, current: {})", 
                         id, actual_additions, max_electrons, current_count);
            }
            
            for _ in 0..actual_additions {
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let rel_pos = Vec2::new(angle.cos(), angle.sin())
                    * body.radius
                    * body.species.polar_offset();
                body.electrons.push(Electron { rel_pos, vel: Vec2::zero() });
            }
        }
        body.update_charge_from_electrons();
        println!("Particle {} new charge: {}", id, body.charge);
        println!("Particle {} new electron count: {}", id, body.electrons.len());
        println!("Particle {} new species: {:?}", id, body.species);

        let was_metal = body.species == Species::LithiumMetal;
        let was_ion = body.species == Species::LithiumIon;
        body.update_species();

        if was_metal && body.species == Species::LithiumIon {
            println!();
            println!("Should become ion below...");
            println!("Particle {} new species: {:?}", id, body.species);
        }
        if was_ion && body.species == Species::LithiumMetal {
            println!();
            println!("Should become metal below...");
            println!("Particle {} new species: {:?}", id, body.species);
        }

        println!("Particle {} new charge: {}", id, body.charge);
    }
}

pub fn handle_add_body(simulation: &mut Simulation, body: &mut crate::body::Body) {
    body.electrons.clear();
    if matches!(
        body.species,
        Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC
    ) {
        body.electrons.push(Electron {
            rel_pos: Vec2::zero(),
            vel: Vec2::zero(),
        });
    }
    body.update_charge_from_electrons();
    body.update_species();
    simulation.bodies.push(body.clone());
}

pub fn handle_delete_all(simulation: &mut Simulation) {
    simulation.bodies.clear();
    simulation.foils.clear();
    simulation.body_to_foil.clear();
}

pub fn handle_delete_species(simulation: &mut Simulation, species: Species) {
    simulation.bodies.retain(|body| body.species != species);
    if species == Species::FoilMetal {
        use std::collections::HashSet;
        let remaining: HashSet<u64> = simulation
            .bodies
            .iter()
            .filter(|b| b.species == Species::FoilMetal)
            .map(|b| b.id)
            .collect();
        simulation
            .foils
            .retain(|foil| foil.body_ids.iter().any(|id| remaining.contains(id)));
        simulation.body_to_foil.retain(|id, _| remaining.contains(id));
    }
}

pub fn handle_add_circle(
    simulation: &mut Simulation,
    body: crate::body::Body,
    x: f32,
    y: f32,
    radius: f32,
) {
    let center = Vec2::new(x, y);
    let particle_radius = body.radius;
    let particle_diameter = 2.0 * particle_radius;
    let mut r = particle_radius;
    while r <= radius {
        let circumference = 2.0 * std::f32::consts::PI * r;
        let count = (circumference / particle_diameter).floor() as usize;
        if count == 0 {
            r += particle_diameter;
            continue;
        }
        for i in 0..count {
            let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
            let offset = Vec2::new(angle.cos(), angle.sin()) * r;
            let pos = center + offset;
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

pub fn handle_add_ring(
    simulation: &mut Simulation,
    body: crate::body::Body,
    x: f32,
    y: f32,
    radius: f32,
) {
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

pub fn handle_add_rectangle(
    simulation: &mut Simulation,
    body: crate::body::Body,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
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

pub fn handle_add_random(
    simulation: &mut Simulation,
    body: crate::body::Body,
    count: usize,
    domain_width: f32,
    domain_height: f32,
) {
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

pub fn handle_set_domain_size(simulation: &mut Simulation, width: f32, height: f32) {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    simulation.bodies.retain(|body| {
        body.pos.x >= -half_width &&
        body.pos.x <= half_width &&
        body.pos.y >= -half_height &&
        body.pos.y <= half_height
    });
    // Update rectangular domain dimensions
    simulation.domain_width = half_width;
    simulation.domain_height = half_height;
    simulation.cell_list.update_domain_size(half_width, half_height);
    
    // Update shared state for renderer
    *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
    *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
}

pub fn handle_set_temperature(simulation: &mut Simulation, temperature: f32) {
    let current = crate::simulation::utils::compute_temperature(&simulation.bodies);
    if current > 0.0 {
        let scale = (temperature / current).sqrt();
        for body in &mut simulation.bodies {
            body.vel *= scale;
        }
    }
    crate::config::LJ_CONFIG.lock().temperature = temperature;
}

pub fn overlaps_any(existing: &[crate::body::Body], pos: Vec2, radius: f32) -> Option<usize> {
    existing.iter().position(|b| (b.pos - pos).mag() < (b.radius + radius))
}

pub fn remove_body_with_foils(simulation: &mut Simulation, idx: usize) {
    let body = simulation.bodies.remove(idx);
    if let Some(foil_id) = simulation.body_to_foil.remove(&body.id) {
        if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
            foil.body_ids.retain(|&id| id != body.id);
        }
    }
}
