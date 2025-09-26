// test_realistic_thermostat.rs
// Focused test using actual configuration but scaled down
use particle_sim::simulation::Simulation;
use particle_sim::body::{Body, Species};
use particle_sim::config::SimConfig;
use particle_sim::init_config::InitConfig;
use ultraviolet::Vec2;
use rand::Rng;

fn main() {
    println!("=== REALISTIC THERMOSTAT TEST ===");
    println!("Using actual default configuration but with scaled particle counts\n");
    
    // Load the real configuration
    let init_config = match InitConfig::load_default() {
        Ok(config) => {
            println!("✅ Loaded init_config.toml");
            config
        }
        Err(e) => {
            println!("❌ Failed to load config: {}", e);
            return;
        }
    };
    
    // Create simulation with realistic settings
    let mut sim = Simulation::new();
    sim.config.temperature = init_config.simulation.as_ref()
        .and_then(|s| s.initial_temperature)
        .unwrap_or(300.0);
    sim.config.thermostat_interval_fs = 5.0;
    
    println!("Configuration:");
    println!("  Target temperature: {}K", sim.config.temperature);
    println!("  Thermostat interval: {}fs", sim.config.thermostat_interval_fs);
    
    // Add a representative sample of the real particles
    add_representative_particles(&mut sim, &init_config);
    
    println!("\n=== INITIAL STATE ===");
    analyze_state(&sim, 0);
    
    if sim.bodies.is_empty() {
        println!("❌ No particles - this would explain thermostat issues!");
        return;
    }
    
    let solvent_count = sim.bodies.iter().filter(|b| matches!(b.species, Species::EC | Species::DMC)).count();
    println!("Solvent particles for thermostat: {}", solvent_count);
    
    if solvent_count == 0 {
        println!("❌ No solvent particles - thermostat has nothing to control!");
        return;
    }
    
    println!("\n=== SIMULATION STEPS ===");
    for step in 1..=10 {
        // Capture before
        let temp_before = calculate_solvent_temperature(&sim.bodies);
        
        sim.step();
        
        // Capture after  
        let temp_after = calculate_solvent_temperature(&sim.bodies);
        
        println!("Step {}: {:.3}K -> {:.3}K (Δ{:+.3}K)", 
            step, temp_before, temp_after, temp_after - temp_before);
        
        if temp_after < temp_before - 5.0 {
            println!("  📉 WARNING: Temperature dropping significantly!");
        }
    }
    
    let final_temp = calculate_solvent_temperature(&sim.bodies);
    let target = sim.config.temperature;
    let deviation = (final_temp - target).abs();
    
    println!("\n=== RESULT ===");
    println!("Target: {:.1}K, Final: {:.3}K, Deviation: {:.3}K", target, final_temp, deviation);
    
    if deviation < 10.0 {
        println!("✅ Thermostat working within 10K");
    } else {
        println!("❌ Thermostat not maintaining temperature");
    }
}

fn add_representative_particles(sim: &mut Simulation, init_config: &InitConfig) {
    let mut rng = rand::rng();
    let mut total = 0;
    
    // Add some metal particles (should not be thermostated)
    if !init_config.particles.metal_rectangles.is_empty() {
        for _ in 0..50 {
            let pos = Vec2::new(rng.random::<f32>() * 100.0 - 50.0, rng.random::<f32>() * 100.0 - 50.0);
            let vel = thermal_velocity(&mut rng, 1.0, sim.config.temperature);
            let body = Body::new(pos, vel, 1.0, 1.0, 0.0, Species::LithiumMetal);
            sim.bodies.push(body);
            total += 1;
        }
        println!("Added 50 LithiumMetal particles");
    }
    
    // Add solvent particles from random config (these get thermostated)
    for random_config in &init_config.particles.random {
        if let Ok(species) = random_config.to_species() {
            if matches!(species, Species::EC | Species::DMC) {
                let count = (random_config.count / 10).max(20); // Scale down but ensure minimum
                
                for _ in 0..count {
                    let pos = Vec2::new(rng.random::<f32>() * 200.0 - 100.0, rng.random::<f32>() * 150.0 - 75.0);
                    let vel = thermal_velocity(&mut rng, 1.0, sim.config.temperature);
                    let body = Body::new(pos, vel, 1.0, 1.0, 0.0, species);
                    sim.bodies.push(body);
                    total += 1;
                }
                println!("Added {} {} particles", count, format!("{:?}", species));
            }
        }
    }
    
    // Add some ions 
    for random_config in &init_config.particles.random {
        if let Ok(species) = random_config.to_species() {
            if matches!(species, Species::LithiumIon | Species::ElectrolyteAnion) {
                let count = (random_config.count / 10).max(10);
                
                for _ in 0..count {
                    let pos = Vec2::new(rng.random::<f32>() * 200.0 - 100.0, rng.random::<f32>() * 150.0 - 75.0);
                    let vel = thermal_velocity(&mut rng, 1.0, sim.config.temperature);
                    let charge = match species {
                        Species::LithiumIon => 1.0,
                        Species::ElectrolyteAnion => -1.0,
                        _ => 0.0,
                    };
                    let body = Body::new(pos, vel, 1.0, 1.0, charge, species);
                    sim.bodies.push(body);
                    total += 1;
                }
                println!("Added {} {} particles", count, format!("{:?}", species));
            }
        }
    }
    
    println!("Total particles: {}", total);
}

fn thermal_velocity(rng: &mut impl Rng, mass: f32, temperature: f32) -> Vec2 {
    let scale = (temperature / mass).sqrt();
    Vec2::new(
        (rng.random::<f32>() - 0.5) * scale,
        (rng.random::<f32>() - 0.5) * scale,
    )
}

fn calculate_solvent_temperature(bodies: &[Body]) -> f32 {
    let mut ke = 0.0;
    let mut count = 0;
    
    for body in bodies {
        if matches!(body.species, Species::EC | Species::DMC) {
            ke += 0.5 * body.mass * body.vel.mag_sq();
            count += 1;
        }
    }
    
    if count > 0 { ke / count as f32 } else { 0.0 }
}

fn analyze_state(sim: &Simulation, step: u32) {
    let solvent_temp = calculate_solvent_temperature(&sim.bodies);
    let ec_count = sim.bodies.iter().filter(|b| matches!(b.species, Species::EC)).count();
    let dmc_count = sim.bodies.iter().filter(|b| matches!(b.species, Species::DMC)).count();
    let li_count = sim.bodies.iter().filter(|b| matches!(b.species, Species::LithiumIon)).count();
    let metal_count = sim.bodies.iter().filter(|b| matches!(b.species, Species::LithiumMetal)).count();
    
    println!("Step {} - Total: {}, EC: {}, DMC: {}, Li+: {}, Metal: {}", 
        step, sim.bodies.len(), ec_count, dmc_count, li_count, metal_count);
    println!("  Solvent temperature: {:.3}K", solvent_temp);
    println!("  Frame: {}, Time: {:.1}fs", sim.frame, sim.frame as f32 * sim.dt);
}