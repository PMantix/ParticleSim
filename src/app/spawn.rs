use crate::body::{Electron, Species};
use crate::simulation::Simulation;
use rand_distr::{Distribution, StandardNormal};
use rand::thread_rng;
use smallvec::smallvec;
use ultraviolet::Vec2;

const RANDOM_ATTEMPTS: usize = super::RANDOM_ATTEMPTS;

/// Sample a random velocity vector from a Maxwell-Boltzmann distribution.
pub fn sample_velocity(mass: f32, temperature: f32) -> Vec2 {
    let sigma = (temperature / mass).sqrt();
    let mut rng = thread_rng();
    let vx: f64 = StandardNormal.sample(&mut rng);
    let vy: f64 = StandardNormal.sample(&mut rng);
    Vec2::new(vx as f32 * sigma, vy as f32 * sigma)
}

pub fn overlaps_any(existing: &[crate::body::Body], pos: Vec2, radius: f32) -> Option<usize> {
    existing
        .iter()
        .position(|b| (b.pos - pos).mag() < (b.radius + radius))
}

pub fn remove_body_with_foils(simulation: &mut Simulation, idx: usize) {
    let body = simulation.bodies.remove(idx);
    if let Some(foil_id) = simulation.body_to_foil.remove(&body.id) {
        if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
            foil.body_ids.retain(|&id| id != body.id);
        }
    }
}

pub fn add_circle(
    simulation: &mut Simulation,
    body: crate::body::Body,
    x: f32,
    y: f32,
    radius: f32,
) {
    let temp = crate::config::LJ_CONFIG.lock().temperature;
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
            new_body.vel = sample_velocity(new_body.mass, temp);
            new_body.electrons.clear();
            if matches!(
                new_body.species,
                Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC
            ) {
                new_body.electrons.push(Electron {
                    rel_pos: Vec2::zero(),
                    vel: Vec2::zero(),
                });
            }
            new_body.update_charge_from_electrons();
            new_body.update_species();
            simulation.bodies.push(new_body);
        }
        r += particle_diameter;
    }
}

pub fn add_ring(simulation: &mut Simulation, body: crate::body::Body, x: f32, y: f32, radius: f32) {
    let temp = crate::config::LJ_CONFIG.lock().temperature;
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
        let mut new_body =
            crate::body::Body::new(pos, Vec2::zero(), body.mass, body.radius, 0.0, body.species);
        new_body.vel = sample_velocity(new_body.mass, temp);
        new_body.electrons.clear();
        if matches!(
            new_body.species,
            Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC
        ) {
            new_body.electrons.push(Electron {
                rel_pos: Vec2::zero(),
                vel: Vec2::zero(),
            });
        }
        new_body.update_charge_from_electrons();
        new_body.update_species();
        simulation.bodies.push(new_body);
    }
}

pub fn add_rectangle(
    simulation: &mut Simulation,
    body: crate::body::Body,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    let temp = crate::config::LJ_CONFIG.lock().temperature;
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
            if matches!(
                new_body.species,
                Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC
            ) {
                new_body.electrons.push(Electron {
                    rel_pos: Vec2::zero(),
                    vel: Vec2::zero(),
                });
            }
            new_body.update_charge_from_electrons();
            new_body.update_species();
            simulation.bodies.push(new_body);
        }
    }
}

pub fn add_random(
    simulation: &mut Simulation,
    body: crate::body::Body,
    count: usize,
    domain_width: f32,
    domain_height: f32,
) {
    // Attempt to place 'count' random bodies, tracking failures
    let mut failures = 0;
    let temp = crate::config::LJ_CONFIG.lock().temperature;
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
                new_body.vel = sample_velocity(new_body.mass, temp);
                new_body.electrons.clear();
                if matches!(new_body.species, Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
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
            failures += 1;
        }
    }
    // Report summary of placement failures only once
    if failures > 0 {
        eprintln!("Failed to place {} random bodies out of {} after {} attempts each", failures, count, RANDOM_ATTEMPTS);
    }
}

pub fn add_foil(
    simulation: &mut Simulation,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    particle_radius: f32,
    current: f32,
) {
    let temp = crate::config::LJ_CONFIG.lock().temperature;
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
            new_body.vel = sample_velocity(new_body.mass, temp);
            new_body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
            new_body.update_charge_from_electrons();
            body_ids.push(new_body.id);
            simulation.bodies.push(new_body);
        }
    }
    let foil = crate::body::foil::Foil::new(body_ids.clone(), origin, width, height, current, 0.0);
    for id in &body_ids {
        simulation.body_to_foil.insert(*id, foil.id);
    }
    simulation.foils.push(foil);
}
