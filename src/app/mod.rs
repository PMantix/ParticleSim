use crate::body::Species;
use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use crate::renderer::Renderer;
use crate::simulation::Simulation;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

mod command_loop;
mod simulation_loop;
mod spawn;

pub const RANDOM_ATTEMPTS: usize = 20;

pub fn run() {
    // Creates a global thread pool (using rayon) with threads = max(3, total cores - 2)
    let threads = std::thread::available_parallelism()
        .unwrap()
        .get()
        .max(crate::config::MIN_THREADS)
        - crate::config::THREADS_LEAVE_FREE;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap();

    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(
            crate::config::WINDOW_WIDTH,
            crate::config::WINDOW_HEIGHT,
        ),
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let simulation = Simulation::new();

    let init_config = match crate::init_config::InitConfig::load_default() {
        Ok(config) => {
            println!("Loaded initial configuration from init_config.toml");
            config
        }
        Err(e) => {
            eprintln!("Failed to load init_config.toml: {}", e);
            eprintln!("Using default hardcoded configuration");
            return run_with_hardcoded_config();
        }
    };

    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();

    // Determine the domain size from configuration
    let (global_width, global_height) = if let Some(ref sim_config) = init_config.simulation {
        let width = sim_config
            .domain_width
            .unwrap_or(crate::config::DOMAIN_BOUNDS * 2.0);
        let height = sim_config
            .domain_height
            .unwrap_or(crate::config::DOMAIN_BOUNDS * 2.0);
        println!("Setting domain size to {}x{}", width, height);
        tx.send(SimCommand::SetDomainSize { width, height }).unwrap();
        (width, height)
    } else {
        let size = crate::config::DOMAIN_BOUNDS * 2.0;
        (size, size)
    };

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
    // Bodies for solvent molecules
    let ec_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::EC.mass(),
        Species::EC.radius(),
        0.0,
        Species::EC,
    );
    let dmc_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::DMC.mass(),
        Species::DMC.radius(),
        0.0,
        Species::DMC,
    );

    for circle_config in &init_config.particles.circles {
        match circle_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                    Species::EC => ec_body.clone(),
                    Species::DMC => dmc_body.clone(),
                };
                tx.send(SimCommand::AddCircle {
                    body,
                    x: circle_config.x,
                    y: circle_config.y,
                    radius: circle_config.radius,
                })
                .unwrap();
                println!(
                    "Added circle: {} at ({}, {}) with radius {}",
                    circle_config.species, circle_config.x, circle_config.y, circle_config.radius
                );
            }
            Err(e) => eprintln!("Error in circle config: {}", e),
        }
    }

    for rect_config in &init_config.particles.metal_rectangles {
        match rect_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                    Species::EC => ec_body.clone(),
                    Species::DMC => dmc_body.clone(),
                };
                let (origin_x, origin_y) = rect_config.to_origin_coords();
                tx.send(SimCommand::AddRectangle {
                    body,
                    x: origin_x,
                    y: origin_y,
                    width: rect_config.width,
                    height: rect_config.height,
                })
                .unwrap();
                println!(
                    "Added {} rectangle: {}x{} at center ({}, {})",
                    rect_config.species,
                    rect_config.width,
                    rect_config.height,
                    rect_config.x,
                    rect_config.y
                );
            }
            Err(e) => eprintln!("Error in metal rectangle config: {}", e),
        }
    }

    for foil_config in &init_config.particles.foil_rectangles {
        let (origin_x, origin_y) = foil_config.to_origin_coords();
        tx.send(SimCommand::AddFoil {
            width: foil_config.width,
            height: foil_config.height,
            x: origin_x,
            y: origin_y,
            particle_radius: Species::FoilMetal.radius(),
            current: foil_config.current,
        })
        .unwrap();
        println!(
            "Added foil: {}x{} at center ({}, {}) with current {}",
            foil_config.width,
            foil_config.height,
            foil_config.x,
            foil_config.y,
            foil_config.current
        );
    }

    for random_config in &init_config.particles.random {
        match random_config.to_species() {
            Ok(species) => {
                let body = match species {
                    Species::LithiumMetal => metal_body.clone(),
                    Species::LithiumIon => ion_body.clone(),
                    Species::ElectrolyteAnion => anion_body.clone(),
                    Species::FoilMetal => foil_body.clone(),
                    Species::EC => ec_body.clone(),
                    Species::DMC => dmc_body.clone(),
                };
                let width = random_config
                    .domain_width
                    .unwrap_or(global_width);
                let height = random_config
                    .domain_height
                    .unwrap_or(global_height);
                tx.send(SimCommand::AddRandom {
                    body,
                    count: random_config.count,
                    domain_width: width,
                    domain_height: height,
                })
                .unwrap();
                println!(
                    "Added {} random {} particles in {}x{} domain",
                    random_config.count,
                    random_config.species,
                    width,
                    height
                );
            }
            Err(e) => eprintln!("Error in random config: {}", e),
        }
    }

    println!("Initial configuration loaded successfully!");

    std::thread::spawn(move || {
        simulation_loop::run_simulation_loop(rx, simulation);
    });

    quarkstrom::run::<Renderer>(config);
}

fn run_with_hardcoded_config() {
    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(
            crate::config::WINDOW_WIDTH,
            crate::config::WINDOW_HEIGHT,
        ),
    };
    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);
    let simulation = Simulation::new();
    let bounds = crate::config::DOMAIN_BOUNDS;
    let clump_radius = crate::config::CLUMP_RADIUS;
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
    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();
    tx.send(SimCommand::AddCircle {
        body: metal_body.clone(),
        x: left_center.x,
        y: left_center.y,
        radius: clump_radius,
    })
    .unwrap();
    tx.send(SimCommand::AddCircle {
        body: metal_body.clone(),
        x: right_center.x,
        y: right_center.y,
        radius: clump_radius,
    })
    .unwrap();
    tx.send(SimCommand::AddCircle {
        body: ion_body,
        x: center.x,
        y: center.y,
        radius: clump_radius,
    })
    .unwrap();
    tx.send(SimCommand::AddCircle {
        body: anion_body,
        x: center.x,
        y: bounds * 0.6,
        radius: clump_radius,
    })
    .unwrap();
    std::thread::spawn(move || {
        simulation_loop::run_simulation_loop(rx, simulation);
    });
    quarkstrom::run::<Renderer>(config);
}
