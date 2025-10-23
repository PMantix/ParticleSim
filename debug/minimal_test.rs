use particle_sim::*;
use std::time::{Duration, Instant};

fn main() {
    println!("=== Minimal Simulation Test ===");

    // Create a very simple simulation
    let mut sim = simulation::Simulation::new();

    // Add just a few particles to test
    for i in 0..10 {
        let body = body::Body::new(
            ultraviolet::Vec2::new(i as f32 * 5.0, 0.0),
            ultraviolet::Vec2::zero(),
            1.0,
            1.0,
            0.0,
            body::Species::LithiumIon,
        );
        sim.bodies.push(body);
    }

    println!("Created simulation with {} particles", sim.bodies.len());
    println!("Out-of-plane enabled: {}", sim.config.enable_out_of_plane);

    // Run a few simulation steps
    println!("Running simulation steps...");
    for step in 0..10 {
        let start = Instant::now();

        // Check for invalid values before stepping
        let invalid_count = sim
            .bodies
            .iter()
            .filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.z.is_finite())
            .count();

        if invalid_count > 0 {
            println!("Step {}: Found {} invalid particles!", step, invalid_count);
            break;
        }

        sim.step();

        let elapsed = start.elapsed();
        println!(
            "Step {}: Frame {}, took {:.2}ms",
            step,
            sim.frame,
            elapsed.as_millis()
        );

        if elapsed > Duration::from_millis(100) {
            println!("  Warning: Step took longer than expected");
        }
    }

    println!("Test completed successfully!");
}
