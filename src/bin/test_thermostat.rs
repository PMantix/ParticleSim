// test_thermostat.rs
// Dedicated test binary to analyze thermostat behavior in isolation
use particle_sim::body::{Body, Species};
use particle_sim::config::SimConfig;
use particle_sim::simulation::Simulation;
use rand::prelude::*;
use rand::rng;
use ultraviolet::Vec2;

fn main() {
    println!("=== Thermostat Test Binary ===");

    // Create a minimal simulation with known particles
    let mut sim = create_test_simulation();

    println!("Initial setup:");
    analyze_simulation_state(&sim);

    // Run simulation steps and monitor thermostat for longer duration
    for step in 1..=200 {
        // Manual step without full simulation loop complexity
        sim.step();

        // Report detailed analysis every 10 steps, brief status every step
        if step % 10 == 0 {
            println!("\n=== Step {} ===", step);
            analyze_simulation_state(&sim);
        } else {
            // Brief status
            let solvent_temp = calculate_solvent_temperature(&sim.bodies);
            let gui_temp = particle_sim::simulation::utils::compute_liquid_temperature(&sim.bodies);
            println!(
                "Step {}: Solvent={:.1}K, GUI={:.1}K, Frame={}",
                step, solvent_temp, gui_temp, sim.frame
            );
        }
    }

    println!("\n=== FINAL SUMMARY ===");
    analyze_simulation_state(&sim);

    // Check temperature stability over time
    let final_solvent_temp = calculate_solvent_temperature(&sim.bodies);
    let final_gui_temp = particle_sim::simulation::utils::compute_liquid_temperature(&sim.bodies);

    println!("\nThermostat Performance Analysis:");
    println!("Target temperature: 300.0K");
    println!("Final solvent temperature: {:.2}K", final_solvent_temp);
    println!("Final GUI liquid temperature: {:.2}K", final_gui_temp);
    println!(
        "Solvent deviation from target: {:.2}K",
        final_solvent_temp - 300.0
    );

    if (final_solvent_temp - 300.0).abs() < 1.0 {
        println!("✅ THERMOSTAT WORKING: Solvent temperature maintained within 1K of target");
    } else {
        println!(
            "❌ THERMOSTAT ISSUE: Solvent temperature deviated by {:.2}K",
            (final_solvent_temp - 300.0).abs()
        );
    }

    if final_gui_temp < 50.0 {
        println!(
            "⚠️ GUI DISPLAY ISSUE: Shows {:.1}K but thermostat maintains {:.1}K",
            final_gui_temp, final_solvent_temp
        );
    }
}

fn create_test_simulation() -> Simulation {
    let mut config = SimConfig::default();
    config.temperature = 300.0; // Target 300K
    config.thermostat_interval_fs = 1.0; // Apply every frame

    println!(
        "Config: target_temp={}, interval={}",
        config.temperature, config.thermostat_interval_fs
    );

    let mut sim = Simulation::new();
    sim.config = config;

    // Add known particles manually - bypass command system
    add_test_particles(&mut sim);

    sim
}

fn add_test_particles(sim: &mut Simulation) {
    let mut rng = rng();

    // Add EC particles (should be thermostated)
    for _i in 0..50 {
        let pos = Vec2::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-100.0..100.0),
        );
        let vel = Vec2::new(rng.random_range(-10.0..10.0), rng.random_range(-10.0..10.0));
        let body = Body::new(
            pos,
            vel,
            1.0, // 1 amu mass
            1.0, // radius
            0.0, // zero charge
            Species::EC,
        );
        sim.bodies.push(body);
    }

    // Add DMC particles (should be thermostated)
    for _i in 50..100 {
        let pos = Vec2::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-100.0..100.0),
        );
        let vel = Vec2::new(rng.random_range(-10.0..10.0), rng.random_range(-10.0..10.0));
        let body = Body::new(
            pos,
            vel,
            1.0, // 1 amu mass
            1.0, // radius
            0.0, // zero charge
            Species::DMC,
        );
        sim.bodies.push(body);
    }

    // Add some ions (should NOT be thermostated)
    for _i in 100..110 {
        let pos = Vec2::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-100.0..100.0),
        );
        let vel = Vec2::new(rng.random_range(-10.0..10.0), rng.random_range(-10.0..10.0));
        let body = Body::new(
            pos,
            vel,
            1.0, // 1 amu mass
            1.0, // radius
            1.0, // charged
            Species::LithiumIon,
        );
        sim.bodies.push(body);
    }

    println!("Added {} total particles:", sim.bodies.len());
    println!("  - 50 EC particles");
    println!("  - 50 DMC particles");
    println!("  - 10 LithiumIon particles");
}

fn analyze_simulation_state(sim: &Simulation) {
    println!("Frame: {}, Bodies: {}", sim.frame, sim.bodies.len());

    // Count particles by species
    let mut ec_count = 0;
    let mut dmc_count = 0;
    let mut ion_count = 0;
    let mut other_count = 0;

    for body in &sim.bodies {
        match body.species {
            Species::EC => ec_count += 1,
            Species::DMC => dmc_count += 1,
            Species::LithiumIon => ion_count += 1,
            _ => other_count += 1,
        }
    }

    println!(
        "Particle counts: EC={}, DMC={}, Li+={}, Other={}",
        ec_count, dmc_count, ion_count, other_count
    );

    // Calculate temperatures manually using both methods
    let total_temp = calculate_total_temperature(&sim.bodies);
    let liquid_temp = calculate_liquid_temperature(&sim.bodies);
    let solvent_temp = calculate_solvent_temperature(&sim.bodies);

    println!("Temperature analysis:");
    println!("  Total temperature: {:.2}K", total_temp);
    println!("  Liquid temperature: {:.2}K", liquid_temp);
    println!("  Solvent (EC+DMC) temperature: {:.2}K", solvent_temp);

    // Check what the GUI would show
    let gui_liquid_temp = particle_sim::simulation::utils::compute_liquid_temperature(&sim.bodies);
    println!("  GUI liquid temp: {:.2}K", gui_liquid_temp);

    // Analyze velocities
    analyze_velocities(&sim.bodies);
}

fn calculate_total_temperature(bodies: &[Body]) -> f32 {
    if bodies.is_empty() {
        return 0.0;
    }

    let total_ke: f32 = bodies.iter().map(|b| 0.5 * b.mass * b.vel.mag_sq()).sum();

    total_ke / bodies.len() as f32
}

fn calculate_liquid_temperature(bodies: &[Body]) -> f32 {
    let liquid_bodies: Vec<&Body> = bodies
        .iter()
        .filter(|b| {
            matches!(
                b.species,
                Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC
            )
        })
        .collect();

    if liquid_bodies.is_empty() {
        return 0.0;
    }

    let total_ke: f32 = liquid_bodies
        .iter()
        .map(|b| 0.5 * b.mass * b.vel.mag_sq())
        .sum();

    total_ke / liquid_bodies.len() as f32
}

fn calculate_solvent_temperature(bodies: &[Body]) -> f32 {
    let solvent_bodies: Vec<&Body> = bodies
        .iter()
        .filter(|b| matches!(b.species, Species::EC | Species::DMC))
        .collect();

    if solvent_bodies.is_empty() {
        return 0.0;
    }

    let total_ke: f32 = solvent_bodies
        .iter()
        .map(|b| 0.5 * b.mass * b.vel.mag_sq())
        .sum();

    total_ke / solvent_bodies.len() as f32
}

fn analyze_velocities(bodies: &[Body]) {
    if bodies.is_empty() {
        return;
    }

    let velocities: Vec<f32> = bodies.iter().map(|b| b.vel.mag()).collect();

    let avg_vel = velocities.iter().sum::<f32>() / velocities.len() as f32;
    let max_vel = velocities.iter().fold(0.0f32, |a, &b| a.max(b));
    let min_vel = velocities.iter().fold(f32::INFINITY, |a, &b| a.min(b));

    println!(
        "Velocity analysis: avg={:.3}, min={:.3}, max={:.3}",
        avg_vel, min_vel, max_vel
    );

    // Check for zero velocities (problematic)
    let zero_count = velocities.iter().filter(|&&v| v < 1e-6).count();
    if zero_count > 0 {
        println!(
            "WARNING: {} particles have near-zero velocities!",
            zero_count
        );
    }
}
