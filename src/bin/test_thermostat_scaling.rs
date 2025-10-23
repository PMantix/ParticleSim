// test_thermostat_scaling.rs
// Test to verify thermostat scaling works by starting with wrong temperature
use particle_sim::body::{Body, Species};
use particle_sim::config::SimConfig;
use particle_sim::simulation::Simulation;
use rand::prelude::*;
use rand::rng;
use ultraviolet::Vec2;

fn main() {
    println!("=== Thermostat Scaling Test ===");

    let mut sim = create_hot_simulation();
    println!("Created simulation with artificially high temperature");

    // Check initial state
    let initial_temp = calculate_solvent_temperature(&sim.bodies);
    println!(
        "Initial solvent temperature: {:.2}K (should be ~900K)",
        initial_temp
    );

    // Run a few steps to see scaling in action
    for step in 1..=10 {
        sim.step();

        let solvent_temp = calculate_solvent_temperature(&sim.bodies);
        println!("Step {}: Temperature = {:.2}K", step, solvent_temp);

        if (solvent_temp - 300.0).abs() < 1.0 {
            println!("✅ Thermostat successfully scaled temperature to target!");
            break;
        }
    }
}

fn create_hot_simulation() -> Simulation {
    let mut config = SimConfig::default();
    config.temperature = 300.0; // Target 300K
    config.thermostat_interval_fs = 1.0; // Apply every frame

    let mut sim = Simulation::new();
    sim.config = config;

    let mut rng = rng();

    // Add EC particles with HIGH velocities (should result in ~900K)
    for _i in 0..50 {
        let pos = Vec2::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-100.0..100.0),
        );
        let vel = Vec2::new(rng.random_range(-30.0..30.0), rng.random_range(-30.0..30.0)); // Much higher velocities
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

    // Add DMC particles with HIGH velocities
    for _i in 50..100 {
        let pos = Vec2::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-100.0..100.0),
        );
        let vel = Vec2::new(rng.random_range(-30.0..30.0), rng.random_range(-30.0..30.0)); // Much higher velocities
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

    sim
}

fn calculate_solvent_temperature(bodies: &[particle_sim::body::Body]) -> f32 {
    let mut solvent_ke = 0.0;
    let mut solvent_count = 0;

    for body in bodies {
        if body.species == Species::EC || body.species == Species::DMC {
            let speed_squared = body.vel.mag_sq();
            let ke = 0.5 * body.mass * speed_squared;
            solvent_ke += ke;
            solvent_count += 1;
        }
    }

    if solvent_count > 0 {
        solvent_ke / (solvent_count as f32)
    } else {
        0.0
    }
}
