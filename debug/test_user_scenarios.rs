// Test with full GUI environment and typical user scenarios
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== TESTING TYPICAL USER SCENARIOS ===");
    
    // Test different scenarios that users commonly run
    test_lithium_cation_scenario();
    test_mixed_species_scenario(); 
    test_high_particle_density_scenario();
}

fn test_lithium_cation_scenario() {
    println!("\n--- Testing Lithium Cation Scenario ---");
    let mut sim = simulation::Simulation::new();
    
    // Add typical lithium cation simulation setup
    for i in 0..200 {
        let angle = i as f32 * 0.05;
        let radius = 10.0 + (i as f32 * 0.1) % 5.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(
                (rand::random::<f32>() - 0.5) * 0.2,
                (rand::random::<f32>() - 0.5) * 0.2
            ),
            1.0, 1.0, 1.0,
            body::Species::LithiumCation,
        );
        sim.bodies.push(body);
    }
    
    // Add some PF6 anions for electrochemical behavior
    for i in 0..50 {
        let angle = i as f32 * 0.15;
        let radius = 8.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(
                (rand::random::<f32>() - 0.5) * 0.1,
                (rand::random::<f32>() - 0.5) * 0.1
            ),
            -1.0, 1.0, 1.0,
            body::Species::Pf6Anion,
        );
        sim.bodies.push(body);
    }
    
    sim.initialize_history();
    run_performance_test(&mut sim, "Lithium Cation");
}

fn test_mixed_species_scenario() {
    println!("\n--- Testing Mixed Species Scenario ---");
    let mut sim = simulation::Simulation::new();
    
    // Mix of different species with different charges/masses
    for i in 0..100 {
        let angle = i as f32 * 0.1;
        let radius = 8.0 + (i as f32 * 0.05) % 4.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        let species = match i % 4 {
            0 => body::Species::LithiumCation,
            1 => body::Species::Pf6Anion, 
            2 => body::Species::EC,
            _ => body::Species::DMC,
        };
        
        let charge = match species {
            body::Species::LithiumCation => 1.0,
            body::Species::Pf6Anion => -1.0,
            _ => 0.0,
        };
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(
                (rand::random::<f32>() - 0.5) * 0.3,
                (rand::random::<f32>() - 0.5) * 0.3
            ),
            charge, 1.0, 1.0,
            species,
        );
        sim.bodies.push(body);
    }
    
    sim.initialize_history();
    run_performance_test(&mut sim, "Mixed Species");
}

fn test_high_particle_density_scenario() {
    println!("\n--- Testing High Density Scenario ---");
    let mut sim = simulation::Simulation::new();
    
    // High density with close interactions
    for _i in 0..400 {
        let x = (rand::random::<f32>() - 0.5) * 20.0;
        let y = (rand::random::<f32>() - 0.5) * 20.0;
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(
                (rand::random::<f32>() - 0.5) * 0.4,
                (rand::random::<f32>() - 0.5) * 0.4
            ),
            if rand::random::<bool>() { 1.0 } else { -1.0 },
            1.0, 1.0,
            body::Species::LithiumCation,
        );
        sim.bodies.push(body);
    }
    
    sim.initialize_history();
    run_performance_test(&mut sim, "High Density");
}

fn run_performance_test(sim: &mut simulation::Simulation, scenario_name: &str) {
    println!("Testing {} with {} particles", scenario_name, sim.bodies.len());
    
    let mut step_times = Vec::new();
    let start_time = Instant::now();
    
    for step in 0..150 {  // Longer test to see progressive issues
        let step_start = Instant::now();
        sim.step();
        let step_time = step_start.elapsed();
        step_times.push(step_time.as_micros());
        
        if step % 30 == 0 {
            println!("  Step {}: {:.2}ms", step, step_time.as_millis());
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Analyze different time periods
    let early_avg = step_times[10..20].iter().sum::<u128>() / 10;
    let mid_avg = step_times[50..60].iter().sum::<u128>() / 10; 
    let late_avg = step_times[130..140].iter().sum::<u128>() / 10;
    
    let early_to_mid = mid_avg as f64 / early_avg as f64;
    let early_to_late = late_avg as f64 / early_avg as f64;
    
    println!("  Results for {}:", scenario_name);
    println!("    Early: {:.2}ms, Mid: {:.2}ms, Late: {:.2}ms", 
        early_avg as f64 / 1000.0, mid_avg as f64 / 1000.0, late_avg as f64 / 1000.0);
    println!("    Early→Mid: {:.2}x, Early→Late: {:.2}x", early_to_mid, early_to_late);
    println!("    Total time: {:.2}s", total_time.as_secs_f64());
    
    if let Some((oldest, newest)) = sim.compressed_history.get_frame_range() {
        println!("    History frames: {} to {}", oldest, newest);
    }
    
    if early_to_late > 2.0 {
        println!("    🎯 SLOWDOWN DETECTED in {} scenario!", scenario_name);
    } else if early_to_late > 1.5 {
        println!("    ⚠️  Moderate slowdown in {} scenario", scenario_name);
    } else {
        println!("    ✅ Stable performance in {} scenario", scenario_name);
    }
}