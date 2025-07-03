// Entry point for the simulation.
// Sets up threading, simulation, and rendering loop. 
// Handles synchronization between simulation and renderer.

use std::sync::atomic::Ordering;
use crate::renderer::state::{
    PAUSED, UPDATE_LOCK, SPAWN, BODIES, QUADTREE, FOILS,
};

mod body;
mod partition;
mod quadtree;
mod cell_list;
mod renderer;
mod simulation;
mod utils;
mod config;
mod profiler;
mod io;
mod species;
mod plotting;
mod init_config;

use crate::body::Species;
//use crate::body::foil::{Foil, LinkMode};
use renderer::Renderer;
use renderer::state::{SIM_COMMAND_SENDER, SimCommand};
use std::sync::mpsc::channel;
use simulation::Simulation;
use crate::body::Electron;
use ultraviolet::Vec2;
use crate::io::{save_state, load_state};

const RANDOM_ATTEMPTS: usize = 20;

#[cfg(feature = "profiling")]   
use once_cell::sync::Lazy;

#[cfg(feature = "profiling")]   
use parking_lot::Mutex;

#[cfg(feature = "profiling")]
pub static PROFILER: Lazy<Mutex<profiler::Profiler>> = Lazy::new(|| {
    Mutex::new(profiler::Profiler::new())
});

fn main() {
    // Creates a global thread pool (using rayon) with threads = max(3, total cores - 2). Leaves 1-2 cores free to avoid hogging system resources.
    let threads = std::thread::available_parallelism().unwrap().get().max(config::MIN_THREADS) - config::THREADS_LEAVE_FREE;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap();

    // Example: update window config
    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(config::WINDOW_WIDTH, config::WINDOW_HEIGHT),
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let simulation = Simulation::new();

    // === Load initial configuration from init_config.toml ===
    let init_config = match init_config::InitConfig::load_default() {
        Ok(config) => {
            println!("Loaded initial configuration from init_config.toml");
            config
        }
        Err(e) => {
            eprintln!("Failed to load init_config.toml: {}", e);
            eprintln!("Using default hardcoded configuration");
            // Fallback to hardcoded configuration
            return main_with_hardcoded_config();
        }
    };

    // Apply configuration
    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();
    
    // Apply domain bounds if specified in config
    if let Some(ref sim_config) = init_config.simulation {
        if let Some(domain_bounds) = sim_config.domain_bounds {
            println!("Setting domain bounds to: {}", domain_bounds);
            // Update the simulation bounds - domain_bounds is the radius, so full width/height is 2x
            let domain_size = domain_bounds * 2.0;
            tx.send(SimCommand::SetDomainSize { 
                width: domain_size, 
                height: domain_size 
            }).unwrap();
        }
    }
    
    // Create template bodies for each species
    let metal_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::LithiumMetal.mass(),
        Species::LithiumMetal.radius(),
        0.0,
        Species::LithiumMetal,
    );
    let ion_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::LithiumIon.mass(),
        Species::LithiumIon.radius(),
        1.0,
        Species::LithiumIon,
    );
    let anion_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::ElectrolyteAnion.mass(),
        Species::ElectrolyteAnion.radius(),
        -1.0,
        Species::ElectrolyteAnion,
    );
    let foil_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::FoilMetal.mass(),
        Species::FoilMetal.radius(),
        0.0,
        Species::FoilMetal,
    );

    // Add circles
    for circle_config in &init_config.particles.circles {
        match circle_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                };
                tx.send(SimCommand::AddCircle { 
                    body, 
                    x: circle_config.x, 
                    y: circle_config.y, 
                    radius: circle_config.radius 
                }).unwrap();
                println!("Added circle: {} at ({}, {}) with radius {}", 
                         circle_config.species, circle_config.x, circle_config.y, circle_config.radius);
            }
            Err(e) => eprintln!("Error in circle config: {}", e),
        }
    }

    // Add metal rectangles
    for rect_config in &init_config.particles.metal_rectangles {
        match rect_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                };
                let (origin_x, origin_y) = rect_config.to_origin_coords();
                tx.send(SimCommand::AddRectangle { 
                    body, 
                    x: origin_x, 
                    y: origin_y, 
                    width: rect_config.width, 
                    height: rect_config.height 
                }).unwrap();
                println!("Added {} rectangle: {}x{} at center ({}, {})", 
                         rect_config.species, rect_config.width, rect_config.height, 
                         rect_config.x, rect_config.y);
            }
            Err(e) => eprintln!("Error in metal rectangle config: {}", e),
        }
    }

    // Add foil rectangles
    for foil_config in &init_config.particles.foil_rectangles {
        let (origin_x, origin_y) = foil_config.to_origin_coords();
        tx.send(SimCommand::AddFoil { 
            width: foil_config.width, 
            height: foil_config.height, 
            x: origin_x, 
            y: origin_y, 
            particle_radius: Species::FoilMetal.radius(), 
            current: foil_config.current 
        }).unwrap();
        println!("Added foil: {}x{} at center ({}, {}) with current {}", 
                 foil_config.width, foil_config.height, 
                 foil_config.x, foil_config.y, foil_config.current);
    }

    // Add random particles
    for random_config in &init_config.particles.random {
        match random_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                };
                tx.send(SimCommand::AddRandom { 
                    body, 
                    count: random_config.count, 
                    domain_width: random_config.domain_width, 
                    domain_height: random_config.domain_height 
                }).unwrap();
                println!("Added {} random {} particles in {}x{} domain", 
                         random_config.count, random_config.species, 
                         random_config.domain_width, random_config.domain_height);
            }
            Err(e) => eprintln!("Error in random config: {}", e),
        }
    }

    println!("Initial configuration loaded successfully!");
    // === End configuration loading ===

    std::thread::spawn(move || {
        run_simulation_loop(rx, simulation);
    });

    quarkstrom::run::<Renderer>(config);
}

fn render(simulation: &mut Simulation) {
    let mut lock = UPDATE_LOCK.lock();
    //if new body was created, add to simulation
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

// Fallback function with hardcoded configuration for when init_config.toml can't be loaded
fn main_with_hardcoded_config() {
    // Example: update window config
    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(config::WINDOW_WIDTH, config::WINDOW_HEIGHT),
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let simulation = Simulation::new();

    // === Hardcoded Scenario setup: Add two 10mm lithium clumps and a central ion clump ===
    let bounds = config::DOMAIN_BOUNDS;
    let clump_radius = config::CLUMP_RADIUS;
    let left_center = Vec2::new(-bounds * 0.6, 0.0);
    let right_center = Vec2::new(bounds * 0.6, 0.0);
    let center = Vec2::zero();
    let metal_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::LithiumMetal.mass(),
        Species::LithiumMetal.radius(),
        0.0,
        Species::LithiumMetal,
    );
    let ion_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::LithiumIon.mass(),
        Species::LithiumIon.radius(),
        1.0,
        Species::LithiumIon,
    );
    let anion_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::ElectrolyteAnion.mass(),
        Species::ElectrolyteAnion.radius(),
        -1.0,
        Species::ElectrolyteAnion,
    );
    // Send SimCommands to populate the simulation
    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();
    tx.send(SimCommand::AddCircle { body: metal_body.clone(), x: left_center.x, y: left_center.y, radius: clump_radius }).unwrap();
    tx.send(SimCommand::AddCircle { body: metal_body.clone(), x: right_center.x, y: right_center.y, radius: clump_radius }).unwrap();
    tx.send(SimCommand::AddCircle { body: ion_body, x: center.x, y: center.y, radius: clump_radius }).unwrap();
    tx.send(SimCommand::AddCircle { body: anion_body, x: center.x, y: bounds * 0.6, radius: clump_radius }).unwrap();
    // === End hardcoded scenario setup ===

    std::thread::spawn(move || {
        run_simulation_loop(rx, simulation);
    });

    quarkstrom::run::<Renderer>(config);
}

// Extracted simulation loop to be shared between main and fallback
fn run_simulation_loop(rx: std::sync::mpsc::Receiver<SimCommand>, mut simulation: Simulation) {
    loop {

        // Handle commands
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                // Change charge of a particle by id
                SimCommand::ChangeCharge { id, delta } => {
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

                // Add a new body with 1 valence electron, correct charge & species
                SimCommand::AddBody { mut body } => {
                    // ensure 1 valence electron, correct charge & species:
                    body.electrons.clear();
                    if matches!(body.species, Species::LithiumMetal | Species::ElectrolyteAnion) {
                        body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                    }
                    body.update_charge_from_electrons();
                    body.update_species();
                    simulation.bodies.push(body);
                }

                // Delete all bodies in the simulation
                SimCommand::DeleteAll => {
                    simulation.bodies.clear();
                    simulation.foils.clear(); // Also clear all foils when deleting all particles
                    simulation.body_to_foil.clear();
                    // Optionally clear other simulation state if needed
                }

                // Add a circle of bodies with given radius, position, count, and species
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
                            // Remove any overlapping particle
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
                },

                // Handle the StepOnce command (currently does nothing)
                SimCommand::StepOnce => {
                    // Manually step the simulation one frame
                    simulation.step();
                    render(&mut simulation);
                    #[cfg(feature = "profiling")]
                    {
                        PROFILER.lock().print_and_clear(Some(&simulation), None);
                    }
                    // Optionally, pause the simulation if desired:
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
                            new_body.electrons = smallvec::smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
                            new_body.update_charge_from_electrons();
                            // new_body.fixed = true; // No longer needed
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
                        // Also update DC current to maintain compatibility
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

                SimCommand::UnlinkFoils { a, b } => {
                    if let Some(foil_a) = simulation.foils.iter_mut().find(|f| f.id == a && f.link_id == Some(b)) {
                        foil_a.link_id = None;
                    }
                    if let Some(foil_b) = simulation.foils.iter_mut().find(|f| f.id == b && f.link_id == Some(a)) {
                        foil_b.link_id = None;
                    }
                }

                SimCommand::SetDomainSize { width, height } => {
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
                    // (You may need to adjust this depending on your simulation structure)
                    simulation.bounds = width.max(height) / 2.0;
                }

                //SimCommand::Plate { foil_id, amount } => { /* ... */ }
                //SimCommand::Strip { foil_id, amount } => { /* ... */ }
                //SimCommand::AddElectron { pos, vel } => { /* ... */ }
                //SimCommand::RemoveElectron { id } => { /* ... */ }
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
            PROFILER.lock().print_and_clear_if_running(!PAUSED.load(Ordering::Relaxed), Some(&simulation), None);
        }
    }
}
