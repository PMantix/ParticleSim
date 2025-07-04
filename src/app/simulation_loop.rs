use std::sync::atomic::Ordering;
use crate::renderer::state::{
    PAUSED, UPDATE_LOCK, SPAWN, BODIES, QUADTREE, FOILS, SimCommand,
};
use crate::simulation::Simulation;
use crate::body::{self, Electron, Species};
use crate::io::{save_state, load_state};
use ultraviolet::Vec2;

use smallvec::smallvec; // used in AddFoil branch

const RANDOM_ATTEMPTS: usize = super::RANDOM_ATTEMPTS;

pub fn render(simulation: &mut Simulation) {
    let mut lock = UPDATE_LOCK.lock();
    for body in SPAWN.lock().drain(..) {
        simulation.bodies.push(body);
    }
    {
        let mut lock = BODIES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.bodies);
    }
    {
        let mut lock = QUADTREE.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.quadtree.nodes);
    }
    {
        let mut lock = FOILS.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.foils);
    }
    *lock |= true;
}

fn overlaps_any(existing: &[crate::body::Body], pos: Vec2, radius: f32) -> Option<usize> {
    existing.iter().position(|b| (b.pos - pos).mag() < (b.radius + radius))
}

fn remove_body_with_foils(simulation: &mut Simulation, idx: usize) {
    let body = simulation.bodies.remove(idx);
    if let Some(foil_id) = simulation.body_to_foil.remove(&body.id) {
        if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
            foil.body_ids.retain(|&id| id != body.id);
        }
    }
}

pub fn run_simulation_loop(rx: std::sync::mpsc::Receiver<SimCommand>, mut simulation: Simulation) {
    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                SimCommand::ChangeCharge { id, delta } => {
                    if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
                        if delta > 0.0 {
                            for _ in 0..delta.round() as usize { body.electrons.pop(); }
                        } else if delta < 0.0 {
                            for _ in 0..(-delta).round() as usize {
                                let angle = fastrand::f32() * std::f32::consts::TAU;
                                let rel_pos = ultraviolet::Vec2::new(angle.cos(), angle.sin())
                                    * body.radius * crate::config::ELECTRON_DRIFT_RADIUS_FACTOR;
                                body.electrons.push(Electron { rel_pos, vel: ultraviolet::Vec2::zero() });
                            }
                        }
                        body.update_charge_from_electrons();
                        println!("Particle {} new charge: {}", id, body.charge);
                        println!("Particle {} new electron count: {}", id, body.electrons.len());
                        println!("Particle {} new species: {:?}", id, body.species);
                        let was_metal = body.species == body::Species::LithiumMetal;
                        let was_ion = body.species == body::Species::LithiumIon;
                        body.update_species();
                        if was_metal && body.species == body::Species::LithiumIon {
                            println!();
                            println!("Should become ion below...");
                            println!("Particle {} new species: {:?}", id, body.species);
                        }
                        if was_ion && body.species == body::Species::LithiumMetal {
                            println!();
                            println!("Should become metal below...");
                            println!("Particle {} new species: {:?}", id, body.species);
                        }
                        println!("Particle {} new charge: {}", id, body.charge);
                    }
                }
                SimCommand::AddBody { mut body } => {
                    body.electrons.clear();
                    if matches!(body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
                        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                    }
                    body.update_charge_from_electrons();
                    body.update_species();
                    simulation.bodies.push(body);
                }
                SimCommand::DeleteAll => {
                    simulation.bodies.clear();
                    simulation.foils.clear();
                    simulation.body_to_foil.clear();
                }
                SimCommand::AddCircle { body, x, y, radius } => {
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
                            while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
                                remove_body_with_foils(&mut simulation, idx);
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
                SimCommand::StepOnce => {
                    simulation.step();
                    render(&mut simulation);
                    #[cfg(feature = "profiling")]
                    {
                        crate::PROFILER.lock().print_and_clear(Some(&simulation), None);
                    }
                    PAUSED.store(true, Ordering::Relaxed);
                }
                SimCommand::SaveState { path } => {
                    if let Err(e) = save_state(path, &simulation) {
                        eprintln!("Failed to save state: {}", e);
                    }
                }
                SimCommand::LoadState { path } => {
                    match load_state(path) {
                        Ok(state) => state.apply_to(&mut simulation),
                        Err(e) => eprintln!("Failed to load state: {}", e),
                    }
                }
                SimCommand::AddRing { body, x, y, radius } => {
                    let center = Vec2::new(x, y);
                    let particle_radius = body.radius;
                    let particle_diameter = 2.0 * particle_radius;
                    let circumference = 2.0 * std::f32::consts::PI * radius;
                    let count = (circumference / particle_diameter).floor() as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                        let pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;
                        while let Some(idx) = overlaps_any(&simulation.bodies, pos, particle_radius) {
                            remove_body_with_foils(&mut simulation, idx);
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
                SimCommand::AddRectangle { body, x, y, width, height } => {
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
                                remove_body_with_foils(&mut simulation, idx);
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
                SimCommand::AddRandom { body, count, domain_width, domain_height } => {
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
                SimCommand::AddFoil { width, height, x, y, particle_radius, current } => {
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
                                remove_body_with_foils(&mut simulation, idx);
                            }
                            let mut new_body = crate::body::Body::new(
                                pos,
                                Vec2::zero(),
                                Species::FoilMetal.mass(),
                                particle_radius,
                                0.0,
                                Species::FoilMetal,
                            );
                            new_body.electrons = smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
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
                SimCommand::SetFoilCurrent { foil_id, current } => {
                    if let Some(foil) = simulation
                        .foils
                        .iter_mut()
                        .find(|f| f.body_ids.contains(&foil_id))
                    {
                        foil.current = current;
                        foil.dc_current = current;
                    }
                }
                SimCommand::SetFoilDCCurrent { foil_id, dc_current } => {
                    if let Some(foil) = simulation
                        .foils
                        .iter_mut()
                        .find(|f| f.body_ids.contains(&foil_id))
                    {
                        foil.dc_current = dc_current;
                    }
                }
                SimCommand::SetFoilACCurrent { foil_id, ac_current } => {
                    if let Some(foil) = simulation
                        .foils
                        .iter_mut()
                        .find(|f| f.body_ids.contains(&foil_id))
                    {
                        foil.ac_current = ac_current;
                    }
                }
                SimCommand::SetFoilFrequency { foil_id, switch_hz } => {
                    if let Some(foil) = simulation
                        .foils
                        .iter_mut()
                        .find(|f| f.body_ids.contains(&foil_id))
                    {
                        foil.switch_hz = switch_hz;
                    }
                }
                SimCommand::LinkFoils { a, b, mode } => {
                    let a_idx = simulation.foils.iter().position(|f| f.id == a);
                    let b_idx = simulation.foils.iter().position(|f| f.id == b);
                    if let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) {
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
                SimCommand::UnlinkFoils { a, b } => {
                    if let Some(foil_a) = simulation.foils.iter_mut().find(|f| f.id == a && f.link_id == Some(b)) {
                        foil_a.link_id = None;
                    }
                    if let Some(foil_b) = simulation.foils.iter_mut().find(|f| f.id == b && f.link_id == Some(a)) {
                        foil_b.link_id = None;
                    }
                }
                SimCommand::SetDomainSize { width, height } => {
                    let half_width = width / 2.0;
                    let half_height = height / 2.0;
                    simulation.bodies.retain(|body| {
                        body.pos.x >= -half_width &&
                        body.pos.x <= half_width &&
                        body.pos.y >= -half_height &&
                        body.pos.y <= half_height
                    });
                    simulation.bounds = width.max(height) / 2.0;
                }
            }
        }
        if PAUSED.load(Ordering::Relaxed) {
            std::thread::yield_now();
        } else {
            simulation.step();
        }
        render(&mut simulation);
        #[cfg(feature = "profiling")]
        {
            crate::PROFILER.lock().print_and_clear_if_running(!PAUSED.load(Ordering::Relaxed), Some(&simulation), None);
        }
    }
}
