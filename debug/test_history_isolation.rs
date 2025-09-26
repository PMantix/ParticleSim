// Targeted test to isolate history system by temporarily disabling it
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== HISTORY SYSTEM ISOLATION TEST ===");
    
    // Create identical simulations
    let create_test_sim = || {
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
        sim
    };
    
    println!("Testing with 300 particles over 200 steps...");
    println!("This test will measure performance with normal step() method");
    println!("to establish baseline before making any code changes.\n");
    
    let mut sim = create_test_sim();
    let mut step_times = Vec::new();
    
    for step in 0usize..200 {
        let start = Instant::now();
        sim.step();  // Normal step with all systems including history
        let duration = start.elapsed();
        step_times.push(duration.as_micros());
        
        if step % 50 == 0 || step < 5 || step > 195 {
            println!("Step {}: {:.2}ms", step, duration.as_millis());
        }
    }
    
    let early_avg = step_times[10..30].iter().sum::<u128>() / 20;
    let late_avg = step_times[170..190].iter().sum::<u128>() / 20;
    let slowdown_factor = late_avg as f64 / early_avg as f64;
    
    println!("\nRESULTS:");
    println!("Early average (steps 10-30): {:.2}ms", early_avg as f64 / 1000.0);
    println!("Late average (steps 170-190): {:.2}ms", late_avg as f64 / 1000.0);
    println!("Slowdown factor: {:.2}x", slowdown_factor);
    
    if let Some((oldest, newest)) = sim.compressed_history.get_frame_range() {
        let total_frames = newest - oldest + 1;
        println!("History frames accumulated: {}", total_frames);
        let stats = sim.compressed_history.get_memory_stats();
        println!("History memory usage: {:.1}MB", stats.total_memory_bytes as f64 / 1_048_576.0);
        println!("Keyframes: {}", stats.keyframe_count);
        println!("Delta frames: {}", stats.delta_count);
    }
    
    println!("\nNOTE: To confirm history is the cause, we need to temporarily");
    println!("comment out 'self.push_history_snapshot();' in step() and retest.");
}