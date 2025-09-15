use particle_sim::*;
use ultraviolet::Vec2;

fn main() {
    println!("=== Frustration-Based Soft Repulsion Test ===");
    
    // Create a simulation with some test particles
    let mut sim = simulation::Simulation::new();
    
    // Add particles that will be in close proximity (collision scenario)
    for i in 0..4 {
        let x = (i as f32 - 1.5) * 1.0; // Spread particles horizontally
        let mut body = body::Body::new(
            Vec2::new(x, 0.0),
            Vec2::new(0.0, 0.0),
            1.0,    // mass
            0.6,    // radius (will overlap)
            if i % 2 == 0 { 1.0 } else { -1.0 }, // alternating charges
            body::Species::LithiumIon,
        );
        
        // Give some particles high forces (simulating external field)
        if i < 2 {
            body.acc = Vec2::new(5.0, 0.0); // High acceleration - should trigger frustration
        }
        
        sim.bodies.push(body);
    }
    
    println!("Created {} particles", sim.bodies.len());
    println!("Initial frustration stats: {:?}", sim.frustration_tracker.get_frustration_stats());
    
    // Run simulation for several steps to build up frustration
    for step in 0..20 {
        // Update frustration tracking
        sim.frustration_tracker.update(&sim.bodies);
        
        let (frustrated_count, avg_duration) = sim.frustration_tracker.get_frustration_stats();
        
        if step % 5 == 0 {
            println!("\nStep {}: {} frustrated particles (avg duration: {:.1})", 
                    step, frustrated_count, avg_duration);
            
            for (i, body) in sim.bodies.iter().enumerate() {
                let is_frustrated = sim.frustration_tracker.is_frustrated(i);
                let repulsion_factor = sim.frustration_tracker.get_repulsion_factor(i);
                println!("  Particle {}: pos=({:.2},{:.2}) acc=({:.2},{:.2}) frustrated={} factor={:.2}",
                        i, body.pos.x, body.pos.y, body.acc.x, body.acc.y, is_frustrated, repulsion_factor);
            }
        }
    }
    
    // Final stats
    let (frustrated_count, avg_duration) = sim.frustration_tracker.get_frustration_stats();
    println!("\nFinal: {} frustrated particles (avg duration: {:.1})", 
            frustrated_count, avg_duration);
    
    // Test collision softening
    println!("\n=== Testing Collision Softening ===");
    let normal_force = Vec2::new(1.0, 0.0);
    
    for i in 0..sim.bodies.len() {
        for j in (i+1)..sim.bodies.len() {
            let softened_force = simulation::frustration::apply_frustration_softening(
                &sim.frustration_tracker,
                i,
                j,
                normal_force
            );
            
            let reduction = (normal_force.mag() - softened_force.mag()) / normal_force.mag() * 100.0;
            
            if reduction > 0.1 {
                println!("Collision {}-{}: force reduced by {:.1}% ({:.2} -> {:.2})",
                        i, j, reduction, normal_force.mag(), softened_force.mag());
            }
        }
    }
    
    println!("\nTest completed successfully!");
}