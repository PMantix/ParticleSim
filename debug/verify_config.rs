use particle_sim::*;

fn main() {
    println!("=== Configuration Verification ===");

    // Create a new simulation to check default values
    let sim = simulation::Simulation::new();

    println!("Default out-of-plane configuration:");
    println!("  enable_out_of_plane: {}", sim.config.enable_out_of_plane);
    println!("  max_z: {}", sim.config.max_z);
    println!("  z_stiffness: {}", sim.config.z_stiffness);
    println!("  z_damping: {}", sim.config.z_damping);
    println!(
        "  li_collision_softness: {}",
        sim.config.li_collision_softness
    );
    println!("  domain_depth: {}", sim.domain_depth);

    // Check if it matches what we expect
    if sim.config.enable_out_of_plane {
        println!("✅ Z-axis (out-of-plane) is ENABLED");
    } else {
        println!("❌ Z-axis (out-of-plane) is DISABLED");
    }

    if sim.config.li_collision_softness == 0.0 {
        println!(
            "✅ Li+ collision softness is default: {}",
            sim.config.li_collision_softness
        );
    } else {
        println!(
            "ℹ Li+ collision softness is: {}",
            sim.config.li_collision_softness
        );
    }

    println!("\n=== Configuration check completed ===");
}
