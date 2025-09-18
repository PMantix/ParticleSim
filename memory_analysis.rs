// Memory Usage Analysis for ParticleSim History Feature
//
// This analysis estimates the memory impact of increasing PLAYBACK_HISTORY_FRAMES

use std::mem;

// Estimated sizes (in bytes) for key structures:

// Body structure contains:
// - pos: Vec2 (2 * f32 = 8 bytes)
// - z: f32 (4 bytes)
// - vel: Vec2 (8 bytes)
// - vz: f32 (4 bytes)  
// - acc: Vec2 (8 bytes)
// - az: f32 (4 bytes)
// - mass: f32 (4 bytes)
// - radius: f32 (4 bytes)
// - charge: f32 (4 bytes)
// - id: u64 (8 bytes)
// - species: Species (enum ~1 byte + padding = 4 bytes)
// - electrons: SmallVec<[Electron; 2]> (~24 bytes for SmallVec overhead + 2*Electron size)
// - e_field: Vec2 (8 bytes)
// - surrounded_by_metal: bool (1 byte + padding = 4 bytes)
// - last_surround_pos: Vec2 (8 bytes)
// - last_surround_frame: usize (8 bytes)
//
// Estimated Body size: ~110-130 bytes per body (depending on electrons)

// Foil structure is more complex with PID controllers, history, etc.
// Estimated Foil size: ~200-500 bytes per foil

// SimulationState contains:
// - bodies: Vec<Body>
// - foils: Vec<Foil>  
// - body_to_foil: HashMap<u64, u64>
// - config: SimConfig (~100 bytes)
// - domain_width/height/depth: 3 * f32 (12 bytes)
// - frame: usize (8 bytes)
// - sim_time/dt/last_thermostat_time: 3 * f32 (12 bytes)

// Memory calculation for different scenarios:

fn estimate_memory_usage() {
    const BODY_SIZE: usize = 120; // Conservative estimate
    const FOIL_SIZE: usize = 300; // Conservative estimate
    const SIMSTATE_OVERHEAD: usize = 200; // Config + metadata
    
    // Typical simulation sizes
    let scenarios = [
        ("Small simulation", 1000, 2),
        ("Medium simulation", 5000, 4), 
        ("Large simulation", 20000, 8),
        ("Very large simulation", 50000, 16),
    ];
    
    let history_frame_options = [1000, 2000, 5000, 10000, 20000];
    
    println!("Memory Usage Analysis for ParticleSim History Feature");
    println!("=====================================================\n");
    
    for (name, body_count, foil_count) in scenarios.iter() {
        println!("{}", name);
        println!("Bodies: {}, Foils: {}", body_count, foil_count);
        
        let per_frame_size = (body_count * BODY_SIZE) + (foil_count * FOIL_SIZE) + SIMSTATE_OVERHEAD;
        println!("Size per frame: {:.2} KB", per_frame_size as f64 / 1024.0);
        
        for &frames in history_frame_options.iter() {
            let total_mb = (per_frame_size * frames) as f64 / (1024.0 * 1024.0);
            println!("  {} frames: {:.1} MB", frames, total_mb);
        }
        println!();
    }
    
    println!("Recommendations:");
    println!("- Current setting (2000 frames) is reasonable for most simulations");
    println!("- For large simulations (20k+ particles), consider reducing to 1000 frames");
    println!("- For small simulations (<5k particles), you can safely increase to 5000+ frames");
    println!("- Monitor system RAM usage - keep total below 25% of available RAM");
}

// Optimization ideas:
fn optimization_strategies() {
    println!("\nOptimization Strategies:");
    println!("========================");
    println!("1. Compressed snapshots: Use delta compression between frames");
    println!("2. Adaptive frequency: Store every Nth frame for older history");
    println!("3. Memory-mapped storage: Write older frames to disk");
    println!("4. Selective data: Only store position/velocity, not acceleration/forces");
    println!("5. Runtime configuration: Make history size adjustable via GUI");
}

fn main() {
    estimate_memory_usage();
    optimization_strategies();
}