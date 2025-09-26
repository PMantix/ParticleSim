// Test that reproduces the GUI environment more accurately to find the real bottleneck
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== REPRODUCING GUI ENVIRONMENT SLOWDOWN ===");
    
    // Create test simulation with same setup as GUI
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
    
    // Initialize history system like GUI does
    sim.initialize_history();
    
    println!("Testing with {} particles", sim.bodies.len());
    println!("History capture interval: {} frames", 10); // Hardcoded since we removed the constant
    
    let mut step_times = Vec::new();
    let mut history_ops = 0;
    
    for step in 0..100 {
        let step_start = Instant::now();
        
        // Call the actual step method that includes all GUI interactions
        sim.step();
        
        let step_time = step_start.elapsed();
        step_times.push(step_time.as_micros());
        
        // Count when history operations happen
        if step % 10 == 0 { // Hardcoded since we removed the constant
            history_ops += 1;
        }
        
        if step % 20 == 0 {
            println!("Step {}: {:.2}ms (frame: {})", 
                step, step_time.as_millis(), sim.frame);
        }
    }
    
    // Analysis
    let early_avg = step_times[5..15].iter().sum::<u128>() / 10;
    let late_avg = step_times[85..95].iter().sum::<u128>() / 10;
    let slowdown = late_avg as f64 / early_avg as f64;
    
    println!("\n=== RESULTS ===");
    println!("Early average: {:.2}ms", early_avg as f64 / 1000.0);
    println!("Late average: {:.2}ms", late_avg as f64 / 1000.0);
    println!("Slowdown factor: {:.2}x", slowdown);
    println!("History operations performed: {}", history_ops);
    println!("Total simulation frames: {}", sim.frame);
    
    if let Some((oldest, newest)) = sim.compressed_history.get_frame_range() {
        println!("History range: {} to {}", oldest, newest);
    }
    
    if slowdown > 2.0 {
        println!("🎯 CONFIRMED: Progressive slowdown reproduced!");
        println!("   This matches the user's reported issue.");
        
        // Check memory usage
        let stats = sim.compressed_history.get_memory_stats();
        println!("Memory stats: {} keyframes, {} deltas", stats.keyframe_count, stats.delta_count);
    } else if slowdown > 1.5 {
        println!("⚠️  MODERATE: Some slowdown detected ({:.2}x)", slowdown);
    } else {
        println!("✅ NO SLOWDOWN: Performance remains stable");
    }
}