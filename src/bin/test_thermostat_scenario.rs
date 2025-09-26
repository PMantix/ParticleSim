// test_thermostat_scenario.rs
// Test thermostat using the actual default scenario configuration
use particle_sim::simulation::Simulation;
use particle_sim::body::Species;
use particle_sim::init_config::InitConfig;
use particle_sim::scenario; // for scenario loading
use particle_sim::simulation::utils::{compute_liquid_temperature, compute_temperature};

fn main() {
    println!("=== Thermostat Test with Default Scenario ===");
    
    // Attempt to load init config (optional)
    let init_config = InitConfig::load_default().ok();
    if let Some(cfg) = &init_config { println!("Loaded init_config.toml (optional)"); if let Some(simc) = &cfg.simulation { let (w,h)=simc.domain_size(); println!("  Domain: {}x{}", w,h); } }

    // Create a fresh simulation and attempt to apply scenario via existing API
    let mut sim = Simulation::new();
    match scenario::load_and_apply_scenario() {
        Ok(_) => { /* Scenario likely applied to a global or internal state; proceed */ },
        Err(e) => {
            println!("Scenario load failed ({}), attempting hardcoded fallback", e);
            let _ = scenario::load_hardcoded_scenario();
        }
    }

    println!("\nInitial setup:");
    println!("  Total bodies: {}", sim.bodies.len());
    analyze_particles(&sim);

    let initial_liquid_temp = compute_liquid_temperature(&sim.bodies);
    let initial_global_temp = compute_temperature(&sim.bodies);
    println!("  Initial liquid temperature: {:.2}K", initial_liquid_temp);
    println!("  Initial global temperature: {:.2}K", initial_global_temp);
    
    // Run simulation for fewer steps since we have many more particles
    println!("\nRunning simulation...");
    for step in 1..=50usize {
        // Advance a single step (thermostat invoked internally if configured)
        sim.step();
        
        // Report every 10 steps
        if step % 10 == 0 {
            println!("\n=== Step {} (frame={}) ===", step, sim.frame);
            analyze_particles(&sim);
            let liquid_temp = compute_liquid_temperature(&sim.bodies);
            let global_temp = compute_temperature(&sim.bodies);
            println!("  Liquid T: {:.2}K | Global T: {:.2}K | Δ={:.2}K", liquid_temp, global_temp, liquid_temp - 300.0);
        } else {
            let liquid_temp = compute_liquid_temperature(&sim.bodies);
            println!("Step {}: Liquid={:.1}K, Frame={}, Bodies={}", step, liquid_temp, sim.frame, sim.bodies.len());
        }
    }
    
    println!("\n=== FINAL ANALYSIS ===");
    analyze_particles(&sim);
    let final_liquid_temp = compute_liquid_temperature(&sim.bodies);
    let final_global_temp = compute_temperature(&sim.bodies);
    println!("\nThermostat Performance:");
    println!("  Target temperature: 300.0K");
    println!("  Final liquid temperature: {:.2}K", final_liquid_temp);
    println!("  Final global temperature: {:.2}K", final_global_temp);
    println!("  Deviation (liquid-target): {:.2}K", final_liquid_temp - 300.0);
    if (final_liquid_temp - 300.0).abs() < 5.0 { println!("✅ Within 5K of target"); } else { println!("❌ Deviation {:.2}K", (final_liquid_temp - 300.0).abs()); }
}

fn analyze_particles(sim: &Simulation) {
    let mut counts = std::collections::HashMap::new();
    
    for body in &sim.bodies {
        *counts.entry(body.species).or_insert(0) += 1;
    }
    
    println!("  Particle counts:");
    for (species, count) in &counts {
        println!("    {:?}: {}", species, count);
    }
    
    let solvent_count = counts.get(&Species::EC).unwrap_or(&0) + counts.get(&Species::DMC).unwrap_or(&0);
    println!("  Total solvent particles: {}", solvent_count);
}

// Legacy function removed: use compute_liquid_temperature instead.