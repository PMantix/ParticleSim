// Test to investigate what's happening in the history system over time
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== HISTORY SYSTEM DEEP ANALYSIS ===");
    
    let mut sim = simulation::Simulation::new();
    for i in 0..100 {  // Smaller test for detailed analysis
        let body = body::Body::new(
            ultraviolet::Vec2::new(i as f32, 0.0),
            ultraviolet::Vec2::new(0.1, 0.1),
            1.0, 1.0, 1.0,
            body::Species::LithiumIon,
        );
        sim.bodies.push(body);
    }
    
    println!("Testing with {} particles", sim.bodies.len());
    println!("Measuring step performance and history growth...\n");
    
    for step in 0..100 {
        let start = Instant::now();
        sim.step();
        let duration = start.elapsed();
        
        if step % 10 == 0 {
            let stats = sim.compressed_history.get_memory_stats();
            if let Some((oldest, newest)) = sim.compressed_history.get_frame_range() {
                println!("Step {}: {:.2}ms | Frames: {}-{} ({}) | Keyframes: {} | Deltas: {} | Mem: {:.1}MB", 
                    step, 
                    duration.as_millis(),
                    oldest, newest, newest - oldest + 1,
                    stats.keyframe_count,
                    stats.delta_count,
                    stats.total_memory_bytes as f64 / 1_048_576.0
                );
            }
        }
    }
    
    println!("\nFinal analysis:");
    let stats = sim.compressed_history.get_memory_stats();
    println!("Total keyframes: {}", stats.keyframe_count);
    println!("Total deltas: {}", stats.delta_count);
    println!("Memory usage: {:.1}MB", stats.total_memory_bytes as f64 / 1_048_576.0);
    
    // The issue might be:
    // 1. Too many deltas being created (should compress better)
    // 2. Delta reconstruction becoming expensive
    // 3. Memory fragmentation
    // 4. Inefficient VecDeque operations
}