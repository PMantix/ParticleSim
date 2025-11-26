// scenario.rs
// Handles loading and applying initial simulation scenarios from configuration files

use crate::body::Species;
use crate::init_config::InitConfig;
use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
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
                    radius: circle_config.radius,
                })?;
                println!(
                    "Added circle: {} at ({}, {}) with radius {}",
                    circle_config.species, circle_config.x, circle_config.y, circle_config.radius
                );
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
                    height: rect_config.height,
                })?;
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

    // Add foil rectangles
    for foil_config in &init_config.particles.foil_rectangles {
        let (origin_x, origin_y) = foil_config.to_origin_coords();
        tx.send(SimCommand::AddFoil {
            width: foil_config.width,
            height: foil_config.height,
            x: origin_x,
            y: origin_y,
            particle_radius: Species::FoilMetal.radius(),
            current: foil_config.current,
        })?;
        println!(
            "Added foil: {}x{} at center ({}, {}) with current {}",
            foil_config.width,
            foil_config.height,
            foil_config.x,
            foil_config.y,
            foil_config.current
        );
    }

    // Add random particles
    for random_config in &init_config.particles.random {
        match random_config.to_species() {
            Ok(species) => {
                let body = get_body_for_species(&body_templates, species);
                let width = random_config.domain_width.unwrap_or(global_width);
                let height = random_config.domain_height.unwrap_or(global_height);
                eprintln!(
                    "[scenario-debug] Sending AddRandom command for {} {} particles",
                    random_config.count, random_config.species
                );
                tx.send(SimCommand::AddRandom {
                    body,
                    count: random_config.count,
                    domain_width: width,
                    domain_height: height,
                })?;
                eprintln!("[scenario-debug] AddRandom sent successfully");
                println!(
                    "Added {} random {} particles in {}x{} domain",
                    random_config.count, random_config.species, width, height
                );
            }
            Err(e) => eprintln!("Error in random config: {}", e),
        }
    }

    // Add default 1M LiPF6 electrolyte solution with 5471 total particles
    add_default_electrolyte(tx.clone(), global_width, global_height)?;

    println!("Initial configuration loaded successfully!");
    Ok(())
}

/// Add default 1M LiPF6 electrolyte solution with 5471 total particles
/// Composition: 342 Li+, 342 PF6-, 2393 EC, 2394 DMC
fn add_default_electrolyte(
    tx: std::sync::mpsc::Sender<SimCommand>,
    domain_width: f32,
    domain_height: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let molarity = 1.0;
    let total: usize = 5471;
    
    // Calculations for 1M solution with 5471 total particles
    let solvent_to_salt_ratio = 15.0;
    let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
    let lipf6_count = (total as f32 * salt_fraction * molarity / 1.0).round() as usize;
    let li_count = lipf6_count;
    let pf6_count = lipf6_count;
    let remaining = total.saturating_sub(li_count + pf6_count);
    
    // Calculate EC and DMC counts based on 1:1 volume ratio
    // This accounts for different densities and molar masses
    let solvent_parts = vec![
        (Species::EC, 1.0),   // 1 part by volume
        (Species::DMC, 1.0),  // 1 part by volume
    ];
    let solvent_counts = crate::species::calculate_solvent_particle_counts(&solvent_parts, remaining);
    
    let ec_count = solvent_counts.iter()
        .find(|(s, _)| *s == Species::EC)
        .map(|(_, c)| *c)
        .unwrap_or(0);
    let dmc_count = solvent_counts.iter()
        .find(|(s, _)| *s == Species::DMC)
        .map(|(_, c)| *c)
        .unwrap_or(0);

    // Add Li+ ions
    if li_count > 0 {
        let li_body = crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::LithiumIon.mass(),
            Species::LithiumIon.radius(),
            1.0,
            Species::LithiumIon,
        );
        tx.send(SimCommand::AddRandom {
            body: li_body,
            count: li_count,
            domain_width,
            domain_height,
        })?;
    }

    // Add PF6- anions
    if pf6_count > 0 {
        let pf6_body = crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::ElectrolyteAnion.mass(),
            Species::ElectrolyteAnion.radius(),
            -1.0,
            Species::ElectrolyteAnion,
        );
        tx.send(SimCommand::AddRandom {
            body: pf6_body,
            count: pf6_count,
            domain_width,
            domain_height,
        })?;
    }

    // Add EC solvent
    if ec_count > 0 {
        let ec_body = crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::EC.mass(),
            Species::EC.radius(),
            0.0,
            Species::EC,
        );
        tx.send(SimCommand::AddRandom {
            body: ec_body,
            count: ec_count,
            domain_width,
            domain_height,
        })?;
    }

    // Add DMC solvent
    if dmc_count > 0 {
        let dmc_body = crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::DMC.mass(),
            Species::DMC.radius(),
            0.0,
            Species::DMC,
        );
        tx.send(SimCommand::AddRandom {
            body: dmc_body,
            count: dmc_count,
            domain_width,
            domain_height,
        })?;
    }

    eprintln!(
        "[Electrolyte] Loaded default molarity={:.2}M: {} Li+, {} PF6-, {} EC, {} DMC (total {} particles)",
        molarity, li_count, pf6_count, ec_count, dmc_count, total
    );
    
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
            Species::LithiumIon.mass(),
            Species::LithiumIon.radius(),
            1.0,
            Species::LithiumIon,
        ),
        anion_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::ElectrolyteAnion.mass(),
            Species::ElectrolyteAnion.radius(),
            -1.0,
            Species::ElectrolyteAnion,
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
        vc_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::VC.mass(),
            Species::VC.radius(),
            0.0,
            Species::VC,
        ),
        fec_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::FEC.mass(),
            Species::FEC.radius(),
            0.0,
            Species::FEC,
        ),
        emc_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::EMC.mass(),
            Species::EMC.radius(),
            0.0,
            Species::EMC,
        ),
        llzo_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::LLZO.mass(),
            Species::LLZO.radius(),
            0.0,
            Species::LLZO,
        ),
        llzt_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::LLZT.mass(),
            Species::LLZT.radius(),
            0.0,
            Species::LLZT,
        ),
        s40b_body: crate::body::Body::new(
            Vec2::zero(),
            Vec2::zero(),
            Species::S40B.mass(),
            Species::S40B.radius(),
            0.0,
            Species::S40B,
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
    vc_body: crate::body::Body,
    fec_body: crate::body::Body,
    emc_body: crate::body::Body,
    llzo_body: crate::body::Body,
    llzt_body: crate::body::Body,
    s40b_body: crate::body::Body,
}

/// Get the appropriate body template for a given species
fn get_body_for_species(templates: &BodyTemplates, species: Species) -> crate::body::Body {
    match species {
        Species::LithiumMetal => templates.metal_body.clone(),
        Species::LithiumIon => templates.ion_body.clone(),
        Species::ElectrolyteAnion => templates.anion_body.clone(),
        Species::FoilMetal => templates.foil_body.clone(),
        Species::EC => templates.ec_body.clone(),
        Species::DMC => templates.dmc_body.clone(),
        Species::VC => templates.vc_body.clone(),
        Species::FEC => templates.fec_body.clone(),
        Species::EMC => templates.emc_body.clone(),
        Species::LLZO => templates.llzo_body.clone(),
        Species::LLZT => templates.llzt_body.clone(),
        Species::S40B => templates.s40b_body.clone(),
        Species::SEI => templates.ec_body.clone(), // Fallback to EC body for SEI template for now
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
    let width = bounds * 2.0;
    let height = bounds * 2.0;
    *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
    *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
    tx.send(SimCommand::SetDomainSize { width, height })?;
    tx.send(SimCommand::AddCircle {
        body: metal_body.clone(),
        x: left_center.x,
        y: left_center.y,
        radius: clump_radius,
    })?;
    tx.send(SimCommand::AddCircle {
        body: metal_body.clone(),
        x: right_center.x,
        y: right_center.y,
        radius: clump_radius,
    })?;
    tx.send(SimCommand::AddCircle {
        body: ion_body,
        x: center.x,
        y: center.y,
        radius: clump_radius,
    })?;
    tx.send(SimCommand::AddCircle {
        body: anion_body,
        x: center.x,
        y: bounds * 0.6,
        radius: clump_radius,
    })?;

    println!("Hardcoded scenario loaded successfully!");
    Ok(())
}
