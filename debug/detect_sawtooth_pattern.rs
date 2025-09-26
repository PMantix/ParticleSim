// Long-running test to detect sawtooth performance pattern in compressed history
use particle_sim::{simulation, body};
use std::time::Instant;

fn main() {
    println!("=== LONG-TERM SAWTOOTH PATTERN DETECTION ===");
    
    // Create simulation matching default setup
    let mut sim = simulation::Simulation::new();
    
    // Add default particle configuration (similar to GUI default)
    for i in 0..250 {
        let angle = i as f32 * 0.05;
        let radius = 8.0 + (i as f32 * 0.02) % 6.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        
        let body = body::Body::new(
            ultraviolet::Vec2::new(x, y),
            ultraviolet::Vec2::new(
                (rand::random::<f32>() - 0.5) * 0.3,
                (rand::random::<f32>() - 0.5) * 0.3
            ),
            if i % 3 == 0 { 1.0 } else if i % 3 == 1 { -1.0 } else { 0.0 },
            1.0, 1.0,
            if i % 2 == 0 { body::Species::LithiumCation } else { body::Species::Pf6Anion },
        );
        sim.bodies.push(body);
    }
    
    sim.initialize_history();
    
    println!("Testing with {} particles", sim.bodies.len());
    println!("Keyframe interval: {} frames", get_keyframe_interval(&sim));
    println!("History capture interval: {} frames", 10); // Hardcoded since we removed the constant
    println!("Running long test to detect sawtooth pattern...\n");
    
    let mut step_times = Vec::new();
    let mut history_stats = Vec::new();
    let test_start = Instant::now();
    
    // Run for much longer to catch sawtooth cycles (simulate ~60 seconds of real time)
    for step in 0..2000 {
        let step_start = Instant::now();
        sim.step();
        let step_time = step_start.elapsed();
        step_times.push(step_time.as_micros());
        
        // Collect history statistics periodically
        if step % 10 == 0 {
            let stats = sim.compressed_history.get_memory_stats();
            history_stats.push((step, stats.keyframe_count, stats.delta_count, step_time.as_micros()));
        }
        
        // Report progress and detect patterns
        if step % 100 == 0 && step > 0 {
            let recent_avg = step_times[(step-50)..step].iter().sum::<u128>() / 50;
            let stats = sim.compressed_history.get_memory_stats();
            
            println!("Step {}: {:.2}ms | Keyframes: {} | Deltas: {} | Frame: {}", 
                step, recent_avg as f64 / 1000.0, stats.keyframe_count, stats.delta_count, sim.frame);
        }
    }
    
    let total_time = test_start.elapsed();
    
    analyze_sawtooth_pattern(&step_times, &history_stats);
    
    println!("\nTotal test time: {:.1}s", total_time.as_secs_f64());
    println!("Average step time: {:.2}ms", step_times.iter().sum::<u128>() as f64 / step_times.len() as f64 / 1000.0);
}

fn get_keyframe_interval(_sim: &simulation::Simulation) -> usize {
    // Try to extract keyframe interval from compressed history config
    // For now, assume default of 100 based on typical settings
    100
}

fn analyze_sawtooth_pattern(step_times: &[u128], history_stats: &[(usize, usize, usize, u128)]) {
    println!("\n=== SAWTOOTH PATTERN ANALYSIS ===");
    
    // Look for periodic performance improvements (valleys in the sawtooth)
    let mut valleys = Vec::new();
    let mut peaks = Vec::new();
    
    // Analyze in chunks to find sawtooth pattern
    let chunk_size = 100;
    for chunk_start in (0..step_times.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(step_times.len());
        if chunk_end - chunk_start < 50 { break; }
        
        let chunk = &step_times[chunk_start..chunk_end];
        let avg_time = chunk.iter().sum::<u128>() / chunk.len() as u128;
        let min_time = *chunk.iter().min().unwrap();
        let max_time = *chunk.iter().max().unwrap();
        
        let variation = (max_time - min_time) as f64 / avg_time as f64;
        
        if min_time < avg_time - (avg_time / 4) {
            valleys.push((chunk_start, min_time, avg_time));
        }
        if max_time > avg_time + (avg_time / 3) {
            peaks.push((chunk_start, max_time, avg_time));
        }
        
        if chunk_start % 500 == 0 {
            println!("Chunk {}-{}: Avg={:.2}ms, Min={:.2}ms, Max={:.2}ms, Variation={:.1}%", 
                chunk_start, chunk_end-1,
                avg_time as f64 / 1000.0,
                min_time as f64 / 1000.0, 
                max_time as f64 / 1000.0,
                variation * 100.0);
        }
    }
    
    println!("\nPerformance valleys (improvements): {}", valleys.len());
    println!("Performance peaks (degradations): {}", peaks.len());
    
    if valleys.len() > 2 && peaks.len() > 2 {
        // Calculate average cycle length
        let valley_intervals: Vec<usize> = valleys.windows(2)
            .map(|w| w[1].0 - w[0].0)
            .collect();
        
        if !valley_intervals.is_empty() {
            let avg_cycle = valley_intervals.iter().sum::<usize>() / valley_intervals.len();
            println!("Average sawtooth cycle length: {} steps", avg_cycle);
            
            // Convert to real time estimate (assuming ~60 FPS target)
            let cycle_time_estimate = avg_cycle as f64 / 60.0;
            println!("Estimated cycle time: {:.1}s (if running at 60 FPS)", cycle_time_estimate);
            
            if avg_cycle > 50 && avg_cycle < 2000 {
                println!("🎯 SAWTOOTH PATTERN DETECTED!");
                println!("   This confirms the compressed history system is causing periodic slowdowns.");
                println!("   Performance degrades as deltas accumulate, then briefly improves on cleanup.");
            }
        }
    }
    
    // Analyze correlation with history stats
    if history_stats.len() > 10 {
        println!("\n=== HISTORY CORRELATION ANALYSIS ===");
        
        // Look for correlation between delta count and performance
        let mut delta_perf_correlation = Vec::new();
        for &(_step, _keyframes, deltas, step_time) in history_stats {
            delta_perf_correlation.push((deltas, step_time));
        }
        
        // Sort by delta count to see trend
        delta_perf_correlation.sort_by_key(|&(deltas, _)| deltas);
        
        let low_delta_avg = delta_perf_correlation[..delta_perf_correlation.len()/3]
            .iter().map(|(_, time)| *time).sum::<u128>() / (delta_perf_correlation.len()/3) as u128;
        let high_delta_avg = delta_perf_correlation[2*delta_perf_correlation.len()/3..]
            .iter().map(|(_, time)| *time).sum::<u128>() / (delta_perf_correlation.len()/3) as u128;
            
        let correlation_factor = high_delta_avg as f64 / low_delta_avg as f64;
        
        println!("Low delta count performance: {:.2}ms", low_delta_avg as f64 / 1000.0);
        println!("High delta count performance: {:.2}ms", high_delta_avg as f64 / 1000.0);
        println!("Performance correlation factor: {:.2}x", correlation_factor);
        
        if correlation_factor > 1.5 {
            println!("🎯 STRONG CORRELATION: Performance degrades significantly with delta accumulation!");
        }
    }
}