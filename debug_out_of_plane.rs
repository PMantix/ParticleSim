use particle_sim::*;
use ultraviolet::Vec2;
use std::time::Instant;

fn main() {
    println!("=== Out-of-Plane Debug Test ===");
    
    // Create a simple simulation
    let mut sim = simulation::Simulation::new();
    
    // Add a few test particles
    for i in 0..5 {
        let mut body = body::Body::new(
            Vec2::new(i as f32 * 2.0, 0.0),
            Vec2::new(0.0, 0.0),
            1.0,    // mass
            1.0,    // radius
            0.0,    // charge
            body::Species::LithiumIon,
        );
        body.z = (i as f32 - 2.0); // Start at different z levels
        sim.bodies.push(body);
    }
    
    // Add a metal particle
    let metal = body::Body::new(
        Vec2::new(0.0, 5.0),
        Vec2::new(0.0, 0.0),
        1.0,    // mass
        2.0,    // radius
        0.0,    // charge
        body::Species::LithiumMetal,
    );
    sim.bodies.push(metal);
    
    println!("Created simulation with {} particles", sim.bodies.len());
    
    // Test 1: Out-of-plane disabled (should reset all z)
    println!("\n--- Test 1: Out-of-plane disabled ---");
    sim.config.enable_out_of_plane = false;
    
    println!("Before reset:");
    for (i, body) in sim.bodies.iter().enumerate() {
        println!("  Particle {}: z = {:.3}", i, body.z);
    }
    
    simulation::out_of_plane::apply_out_of_plane(&mut sim);
    
    println!("After reset:");
    for (i, body) in sim.bodies.iter().enumerate() {
        println!("  Particle {}: z = {:.3}", i, body.z);
    }
    
    // Test 2: Out-of-plane enabled 
    println!("\n--- Test 2: Out-of-plane enabled ---");
    sim.config.enable_out_of_plane = true;
    sim.config.max_z = 5.0;
    sim.config.z_stiffness = 1.0;
    sim.config.z_damping = 0.1;
    sim.config.z_frustration_strength = 0.1;
    
    // Set some particles at non-zero z
    sim.bodies[0].z = 2.0;
    sim.bodies[1].z = -1.5;
    sim.bodies[0].acc = Vec2::new(1.0, 0.0); // Add some in-plane acceleration
    
    println!("Before out-of-plane forces:");
    for (i, body) in sim.bodies.iter().enumerate() {
        println!("  Particle {}: z = {:.3}, vz = {:.3}, az = {:.3}", i, body.z, body.vz, body.az);
    }
    
    simulation::out_of_plane::apply_out_of_plane(&mut sim);
    
    println!("After out-of-plane forces:");
    for (i, body) in sim.bodies.iter().enumerate() {
        println!("  Particle {}: z = {:.3}, vz = {:.3}, az = {:.3}", i, body.z, body.vz, body.az);
    }
    
    // Test 3: Performance test
    println!("\n--- Test 3: Performance test ---");
    
    // Add more particles
    for i in 6..100 {
        let body = body::Body::new(
            Vec2::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0),
            Vec2::new(0.0, 0.0),
            1.0, 1.0, 0.0,
            body::Species::LithiumIon,
        );
        sim.bodies.push(body);
    }
    
    println!("Added more particles, total: {}", sim.bodies.len());
    
    let start = Instant::now();
    for i in 0..100 {
        simulation::out_of_plane::apply_out_of_plane(&mut sim);
        if i % 20 == 0 {
            println!("  Iteration {}: {:.2}ms", i, start.elapsed().as_millis());
        }
    }
    let duration = start.elapsed();
    println!("100 iterations took {:.2}ms (avg: {:.2}ms per iteration)", 
        duration.as_millis(), duration.as_millis() as f64 / 100.0);
    
    // Test 4: Safety test with invalid values
    println!("\n--- Test 4: Safety test ---");
    sim.config.z_stiffness = f32::NAN;
    sim.config.max_z = -1.0;
    
    println!("Testing with invalid parameters...");
    simulation::out_of_plane::apply_out_of_plane(&mut sim);
    println!("No crash - safety checks working!");
    
    println!("\n=== Debug test completed successfully ===");
}
