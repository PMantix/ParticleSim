//! Headless diagnostic: track Li⁺ ion kinematics step-by-step in the C.1
//! reduction-pathway scenario to diagnose the "chugging" behavior.
//!
//! Loads C1_reduction_pathway.toml, runs N steps, and prints per-step CSV
//! for every Li⁺ ion: step, sim_time, ion_id, x, y, vx, vy, speed, ax, ay,
//! species (to catch any transient conversions).

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn template_body(species: Species) -> Body {
    let charge = match species {
        Species::LithiumIon => 1.0,
        Species::ElectrolyteAnion => -1.0,
        _ => 0.0,
    };
    Body::new(
        Vec2::zero(),
        Vec2::zero(),
        species.mass(),
        species.radius(),
        charge,
        species,
    )
}

fn main() {
    let n_steps: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);

    // Install a dummy command sender (required by handle_command)
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let scenario_path = "measurement_configs/electrode_mechanics/C1_reduction_pathway.toml";
    let config = InitConfig::load_from_file(scenario_path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}", scenario_path, e);
        std::process::exit(1);
    });
    let (full_w, full_h) = config
        .simulation
        .as_ref()
        .map(|s| s.domain_size())
        .unwrap_or((120.0, 60.0));

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    // Spawn metal rectangles
    for rect in &config.particles.metal_rectangles {
        let species = rect.to_species().unwrap_or_else(|e| {
            eprintln!("metal rect species: {}", e);
            std::process::exit(1);
        });
        let body = template_body(species);
        let (x, y) = rect.to_origin_coords();
        handle_command(
            SimCommand::AddRectangle {
                body,
                x,
                y,
                width: rect.width,
                height: rect.height,
            },
            &mut sim,
        );
    }

    // Spawn foils
    for foil in &config.particles.foil_rectangles {
        let (x, y) = foil.to_origin_coords();
        handle_command(
            SimCommand::AddFoil {
                width: foil.width,
                height: foil.height,
                x,
                y,
                particle_radius: Species::FoilMetal.radius(),
                current: foil.current,
            },
            &mut sim,
        );
    }

    // Spawn random particles (the Li⁺ ions)
    for entry in &config.particles.random {
        let species = entry.to_species().unwrap_or_else(|e| {
            eprintln!("random species: {}", e);
            std::process::exit(1);
        });
        let body = template_body(species);
        handle_command(
            SimCommand::AddRandom {
                body,
                count: entry.count,
                domain_width: full_w,
                domain_height: full_h,
            },
            &mut sim,
        );
    }

    // Record initial ion IDs
    let initial_ion_ids: Vec<u64> = sim
        .bodies
        .iter()
        .filter(|b| b.species == Species::LithiumIon)
        .map(|b| b.id)
        .collect();

    eprintln!(
        "Loaded {} bodies, {} initial Li⁺ ions, {} foils. Running {} steps...",
        sim.bodies.len(),
        initial_ion_ids.len(),
        sim.foils.len(),
        n_steps,
    );

    // CSV header
    println!("step,sim_time,ion_id,species,x,y,vx,vy,speed,ax,ay,charge,thermo_scale");

    for step in 0..n_steps {
        sim.step();
        let sim_time = sim.time;
        let thermo_scale = *particle_sim::renderer::state::LAST_THERMOSTAT_SCALE.lock();

        // Track ALL bodies that were originally ions, regardless of current species
        for &id in &initial_ion_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == id) {
                let speed = b.vel.mag();
                println!(
                    "{},{:.4},{},{:?},{:.4},{:.4},{:.6},{:.6},{:.6},{:.6},{:.6},{:.4},{:.6}",
                    step,
                    sim_time,
                    id,
                    b.species,
                    b.pos.x,
                    b.pos.y,
                    b.vel.x,
                    b.vel.y,
                    speed,
                    b.acc.x,
                    b.acc.y,
                    b.charge,
                    thermo_scale,
                );
            }
        }
    }

    eprintln!("Done. {} steps completed, sim_time={:.2} fs", n_steps, sim.time);
}
