// Debug program to isolate the actual source of progressive slowdown
use particle_sim::{simulation, body};
use std::time::Instant;

fn test_simulation_performance(name: &str, mut sim: simulation::Simulation, steps: usize) -> f64 {
    println!("\n--- {} ---", name);
    let mut step_times = Vec::new();
    
    for step in 0usize..steps {
        let start = Instant::now();
        sim.step();
        let duration = start.elapsed();
        step_times.push(duration.as_micros());
        
        if step % 50 == 0 {
            let avg_last_10 = if step >= 10 {
                step_times[step.saturating_sub(9)..=step].iter().sum::<u128>() / 10
            } else {
                duration.as_micros()
            };
            println!("Step {}: {:.2}ms (avg last 10: {:.2}ms)", 
                step, duration.as_millis(), avg_last_10 as f64 / 1000.0);
        }
    }
    
    // Calculate slowdown
    let early_avg = step_times[10..30].iter().sum::<u128>() / 20;
    let late_avg = step_times[(steps-30)..(steps-10)].iter().sum::<u128>() / 20;
    let slowdown_factor = late_avg as f64 / early_avg as f64;
    
    println!("Early avg: {:.2}ms", early_avg as f64 / 1000.0);
    println!("Late avg: {:.2}ms", late_avg as f64 / 1000.0);
    println!("Slowdown factor: {:.2}x", slowdown_factor);
    
    slowdown_factor
}

fn main() {
    println!("=== Root Cause Isolation Test ===");
    
    // Create test simulation
    let create_test_sim = || {
        let mut sim = simulation::Simulation::new();
        
        // Add particles to simulate realistic load
        for i in 0..300 {
            let angle = i as f32 * 0.1;
            let radius = (i as f32 * 0.05) % 15.0;
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            
            let body = body::Body::new(
                ultraviolet::Vec2::new(x, y),
                ultraviolet::Vec2::new(0.1, 0.1),
                1.0, 1.0, 1.0,
                body::Species::LithiumIon,
            );
            sim.bodies.push(body);
        }
        sim
    };
    
    println!("Testing with {} particles", 300);
    
    // Test 1: Short run (before slowdown should manifest)  
    println!("=== SHORT RUN TEST (50 steps) ===");
    let sim1 = create_test_sim();
    let short_slowdown = test_simulation_performance("Short run", sim1, 50);
    
    // Test 2: Long run (where slowdown manifests)
    println!("\n=== LONG RUN TEST (200 steps) ===");  
    let sim2 = create_test_sim();
    let long_slowdown = test_simulation_performance("Long run", sim2, 200);
    
    println!("\n=== PARTICLE COUNT TEST ===");
    // Test 3: Few particles, long run
    let mut sim_few = simulation::Simulation::new();
    for i in 0..50 {
        let body = body::Body::new(
            ultraviolet::Vec2::new(i as f32, 0.0),
            ultraviolet::Vec2::zero(),
            1.0, 1.0, 1.0,
            body::Species::LithiumIon,
        );
        sim_few.bodies.push(body);
    }
    let few_particles_slowdown = test_simulation_performance("50 particles, 200 steps", sim_few, 200);
    
    println!("\n=== ANALYSIS ===");
    println!("Short run (50 steps):     {:.2}x slowdown", short_slowdown);
    println!("Long run (200 steps):     {:.2}x slowdown", long_slowdown);
    println!("Few particles (50):       {:.2}x slowdown", few_particles_slowdown);
    
    if long_slowdown > 2.0 && short_slowdown < 1.5 {
        println!("🎯 CONFIRMED: Progressive slowdown occurs over time");
        
        if few_particles_slowdown < 2.0 {
            println!("💡 LIKELY: Slowdown scales with particle count/complexity");
        } else {
            println!("🔍 LIKELY: Slowdown is time-based (history/memory accumulation)");
        }
    } else {
        println!("🤔 No clear pattern detected");
    }
}