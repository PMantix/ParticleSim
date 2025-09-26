// Precise diagnostic to identify exactly where in the history system the slowdown occurs
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== PINPOINT HISTORY SYSTEM BOTTLENECK ===");
    
    // Create test simulation
    let mut sim = simulation::Simulation::new();
    for i in 0..300 {
        let angle = i as f32 * 0.1;
        let radius = (i as f32 * 0.05) % 15.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(0.1, 0.1),
            1.0, 1.0, 1.0,
            body::Species::LithiumCation,
        );
        sim.bodies.push(body);
    }
    
    println!("Testing with {} particles", sim.bodies.len());
    println!("Measuring individual components to isolate the bottleneck...\n");
    
    let mut step_times = Vec::new();
    let mut history_times = Vec::new();
    
    for step in 0..100 {
        // Time the full step
        let step_start = Instant::now();
        
        // Manual step breakdown to isolate history cost
        sim.dt = *particle_sim::renderer::state::TIMESTEP.lock();
        
        // Reset accelerations
        for body in &mut sim.bodies {
            body.acc = ultraviolet::Vec2::zero();
            body.az = 0.0;
        }
        
        // Force calculations (main physics)
        let forces_start = Instant::now();
        particle_sim::simulation::forces::attract(&mut sim);
        particle_sim::simulation::forces::apply_polar_forces(&mut sim);
        particle_sim::simulation::forces::apply_lj_forces(&mut sim);
        particle_sim::simulation::forces::apply_repulsive_forces(&mut sim);
        let forces_time = forces_start.elapsed();
        
        // Physics integration
        sim.iterate();
        
        // Collision detection
        let num_passes = *particle_sim::renderer::state::COLLISION_PASSES.lock();
        for _ in 1..num_passes {
            particle_sim::simulation::collision::collide(&mut sim);
        }
        
        sim.frame += 1;
        
        // Time JUST the history capture
        let history_start = Instant::now();
        if step % 10 == 0 { // Hardcoded since we removed the constant
            sim.push_history_snapshot();
        }
        let history_time = history_start.elapsed();
        
        let total_step_time = step_start.elapsed();
        
        step_times.push(total_step_time.as_micros());
        history_times.push(history_time.as_micros());
        
        if step % 20 == 0 {
            println!("Step {}: Total={:.2}ms | Forces={:.2}ms | History={:.2}ms | History%={:.1}%", 
                step,
                total_step_time.as_millis(),
                forces_time.as_millis(),
                history_time.as_millis(),
                (history_time.as_micros() as f64 / total_step_time.as_micros() as f64) * 100.0
            );
        }
    }
    
    // Analysis
    let early_total = step_times[5..15].iter().sum::<u128>() / 10;
    let late_total = step_times[85..95].iter().sum::<u128>() / 10;
    let total_slowdown = late_total as f64 / early_total as f64;
    
    let early_history = history_times[5..15].iter().sum::<u128>() / 10;
    let late_history = history_times[85..95].iter().sum::<u128>() / 10;
    let history_slowdown = if early_history > 0 { late_history as f64 / early_history as f64 } else { 1.0 };
    
    println!("\n=== BOTTLENECK ANALYSIS ===");
    println!("Total step slowdown: {:.2}x ({:.2}ms → {:.2}ms)", 
        total_slowdown, early_total as f64 / 1000.0, late_total as f64 / 1000.0);
    println!("History operation slowdown: {:.2}x ({:.2}ms → {:.2}ms)", 
        history_slowdown, early_history as f64 / 1000.0, late_history as f64 / 1000.0);
    
    let avg_history_percent = (history_times.iter().sum::<u128>() as f64 / step_times.iter().sum::<u128>() as f64) * 100.0;
    println!("History operations consume {:.1}% of total step time on average", avg_history_percent);
    
    if history_slowdown > 2.0 {
        println!("🎯 CONFIRMED: History operations are getting progressively slower!");
        println!("   The compressed history system has an internal performance bug.");
    } else if total_slowdown > 2.0 {
        println!("🔍 History operations are not the bottleneck - look elsewhere.");
    } else {
        println!("✅ No significant progressive slowdown detected in this test.");
    }
}