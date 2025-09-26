// scenario.rs
// Handles loading and applying initial simulation scenarios from configuration files

use crate::body::Species;
use crate::init_config::InitConfig;
use crate::renderer::state::{SIM_COMMAND_SENDER, SimCommand};
use ultraviolet::Vec2;

/// Load and apply the initial scenario configuration
pub fn load_and_apply_scenario() -> Result<(), Box<dyn std::error::Error>> {
    // Load initial configuration from init_config.toml
    let init_config = match InitConfig::load_default() {
        Ok(config) => {
            println!("Loaded initial configuration from init_config.toml");
            config
        }
        Err(e) => {
            eprintln!("Failed to load init_config.toml: {}", e);
            eprintln!("Using default hardcoded configuration");
            return Err(e);
        }
    };

    apply_configuration(init_config)?;
    Ok(())
}

/// Apply the loaded configuration to the simulation
fn apply_configuration(init_config: InitConfig) -> Result<(), Box<dyn std::error::Error>> {
    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();
    
    // Reset time to 0 when loading a new scenario
    eprintln!("[scenario-debug] Sending ResetTime command");
    tx.send(SimCommand::ResetTime)?;
    eprintln!("[scenario-debug] ResetTime sent successfully");
    
    // Determine domain size from config or fallback constant
    let (global_width, global_height) = if let Some(ref sim_config) = init_config.simulation {
        let (width, height) = sim_config.domain_size();
        println!("Setting domain size to {}x{}", width, height);
        *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
        *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
        tx.send(SimCommand::SetDomainSize { width, height })?;
        (width, height)
    } else {
        let size = crate::config::DOMAIN_BOUNDS * 2.0;
        *crate::renderer::state::DOMAIN_WIDTH.lock() = size;
        *crate::renderer::state::DOMAIN_HEIGHT.lock() = size;
        (size, size)
    };
    
    // Create template bodies for each species
    let body_templates = create_body_templates();

    // Add circles
    for circle_config in &init_config.particles.circles {
        match circle_config.to_species() {
            Ok(species) => {
                let body = get_body_for_species(&body_templates, species);
                tx.send(SimCommand::AddCircle { 
                    body, 
                    x: circle_config.x, 
                    y: circle_config.y, 
                    radius: circle_config.radius 
                })?;
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
                let body = get_body_for_species(&body_templates, species);
                let (origin_x, origin_y) = rect_config.to_origin_coords();
                tx.send(SimCommand::AddRectangle { 
                    body, 
                    x: origin_x, 
                    y: origin_y, 
                    width: rect_config.width, 
                    height: rect_config.height 
                })?;
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
        })?;
        println!("Added foil: {}x{} at center ({}, {}) with current {}", 
                 foil_config.width, foil_config.height, 
                 foil_config.x, foil_config.y, foil_config.current);
    }

    // Add random particles
    for random_config in &init_config.particles.random {
        match random_config.to_species() {
            Ok(species) => {
                let body = get_body_for_species(&body_templates, species);
                let width = random_config.domain_width.unwrap_or(global_width);
                let height = random_config.domain_height.unwrap_or(global_height);
                eprintln!("[scenario-debug] Sending AddRandom command for {} {} particles", random_config.count, random_config.species);
                tx.send(SimCommand::AddRandom {
                    body,
                    count: random_config.count,
                    domain_width: width,
                    domain_height: height
                })?;
                eprintln!("[scenario-debug] AddRandom sent successfully");
                println!("Added {} random {} particles in {}x{} domain",
                         random_config.count, random_config.species, width, height);
            }
            Err(e) => eprintln!("Error in random config: {}", e),
        }
    }

    println!("Initial configuration loaded successfully!");
    Ok(())
}

/// Create template bodies for each species
fn create_body_templates() -> BodyTemplates {
    BodyTemplates {
        metal_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::LithiumMetal.mass(),
            Species::LithiumMetal.radius(),
            0.0,
            Species::LithiumMetal,
        ),
        ion_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::LithiumCation.mass(),
            Species::LithiumCation.radius(),
            1.0,
            Species::LithiumCation,
        ),
        anion_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::Pf6Anion.mass(),
            Species::Pf6Anion.radius(),
            -1.0,
            Species::Pf6Anion,
        ),
        foil_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::FoilMetal.mass(),
            Species::FoilMetal.radius(),
            0.0,
            Species::FoilMetal,
        ),
        ec_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::EC.mass(),
            Species::EC.radius(),
            0.0,
            Species::EC,
        ),
        dmc_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::DMC.mass(),
            Species::DMC.radius(),
            0.0,
            Species::DMC,
        ),
    }
}

/// Structure to hold template bodies for each species
struct BodyTemplates {
    metal_body: crate::body::Body,
    ion_body: crate::body::Body,
    anion_body: crate::body::Body,
    foil_body: crate::body::Body,
    ec_body: crate::body::Body,
    dmc_body: crate::body::Body,
}

/// Get the appropriate body template for a given species
fn get_body_for_species(templates: &BodyTemplates, species: Species) -> crate::body::Body {
    match species {
        Species::LithiumMetal => templates.metal_body.clone(),
        Species::LithiumCation => templates.ion_body.clone(),
        Species::Pf6Anion => templates.anion_body.clone(),
        Species::FoilMetal => templates.foil_body.clone(),
        Species::EC => templates.ec_body.clone(),
        Species::DMC => templates.dmc_body.clone(),
    }
}

/// Load and apply the hardcoded fallback scenario
pub fn load_hardcoded_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let tx = SIM_COMMAND_SENDER.lock().as_ref().unwrap().clone();
    
    // Reset time to 0 when loading hardcoded scenario
    tx.send(SimCommand::ResetTime)?;
    
    // Hardcoded Scenario setup: Add two 10mm lithium clumps and a central ion clump
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
        Species::LithiumCation.mass(),
        Species::LithiumCation.radius(),
        1.0,
        Species::LithiumCation,
    );
    let anion_body = crate::body::Body::new(
        Vec2::zero(),
        Vec2::zero(),
        Species::Pf6Anion.mass(),
        Species::Pf6Anion.radius(),
        -1.0,
        Species::Pf6Anion,
    );
    
    // Send SimCommands to populate the simulation
    let width = bounds * 2.0;
    let height = bounds * 2.0;
    *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
    *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
    tx.send(SimCommand::SetDomainSize { width, height })?;
    tx.send(SimCommand::AddCircle { body: metal_body.clone(), x: left_center.x, y: left_center.y, radius: clump_radius })?;
    tx.send(SimCommand::AddCircle { body: metal_body.clone(), x: right_center.x, y: right_center.y, radius: clump_radius })?;
    tx.send(SimCommand::AddCircle { body: ion_body, x: center.x, y: center.y, radius: clump_radius })?;
    tx.send(SimCommand::AddCircle { body: anion_body, x: center.x, y: bounds * 0.6, radius: clump_radius })?;
    
    println!("Hardcoded scenario loaded successfully!");
    Ok(())
}
