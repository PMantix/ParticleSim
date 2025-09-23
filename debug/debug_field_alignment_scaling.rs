// debug_field_alignment_scaling.rs
// Test program to verify field alignment bias scales hopping strength correctly

use particle_sim::*;
use ultraviolet::Vec2;

fn main() {
    println!("Testing field alignment bias scaling...");
    
    // Create a simple test scenario: two Li metal particles with different charge states
    let mut bodies = vec![
        body::Body::new_from_species(Vec2::new(-10.0, 0.0), Vec2::zero(), -1.0, body::Species::LithiumMetal),  // Source: will have extra electrons
        body::Body::new_from_species(Vec2::new(10.0, 0.0), Vec2::zero(), 1.0, body::Species::LithiumMetal),   // Destination: will be electron-deficient
    ];
    
    // Add extra electron to source
    bodies[0].electrons.push(body::Electron {
        rel_pos: Vec2::zero(),
        vel: Vec2::zero(),
    });
    
    let mut config = config::SimConfig::default();
    config.hop_alignment_bias = 2.0;  // Test with strong bias
    
    // Test scenario 1: No external field (only local charge field)
    println!("\nScenario 1: Only local charge fields");
    test_hopping_scenario(&bodies, &config, Vec2::zero(), "Local only");
    
    // Test scenario 2: External field aligned with hop direction (left to right)
    println!("\nScenario 2: External field aligned with hopping direction");
    test_hopping_scenario(&bodies, &config, Vec2::new(1.0, 0.0), "External aligned");
    
    // Test scenario 3: External field opposing hop direction (right to left)  
    println!("\nScenario 3: External field opposing hopping direction");
    test_hopping_scenario(&bodies, &config, Vec2::new(-1.0, 0.0), "External opposing");
    
    // Test scenario 4: External field perpendicular to hop direction
    println!("\nScenario 4: External field perpendicular to hopping direction");
    test_hopping_scenario(&bodies, &config, Vec2::new(0.0, 1.0), "External perpendicular");
    
    println!("\nTesting different bias strengths...");
    for bias in [0.0, 0.5, 1.0, 2.0, 5.0] {
        let mut test_config = config.clone();
        test_config.hop_alignment_bias = bias;
        println!("\nBias strength: {}", bias);
        test_hopping_scenario(&bodies, &test_config, Vec2::new(0.5, 0.0), &format!("Bias={}", bias));
    }
}

fn test_hopping_scenario(bodies: &[body::Body], config: &config::SimConfig, external_field: Vec2, label: &str) {
    let mut quadtree = quadtree::Quadtree::new(0.5, 0.1, 10, 100);
    let mut bodies_copy = bodies.to_vec();
    quadtree.build(&mut bodies_copy);
    
    let src_body = &bodies[0];
    let dst_body = &bodies[1];
    let hop_vec = dst_body.pos - src_body.pos;
    let hop_dir = hop_vec.normalized();
    
    // Calculate local field (same as in electron_hopping.rs)
    let local_field = external_field + quadtree.field_at_point(&bodies, src_body.pos, config.coulomb_constant);
    let field_dir = if local_field.mag() > 1e-6 {
        local_field.normalized()
    } else {
        Vec2::zero()
    };
    
    // Calculate alignment (same as in electron_hopping.rs)
    let mut alignment = (-hop_dir.dot(field_dir)).max(0.0);
    if field_dir == Vec2::zero() {
        alignment = 1.0;
    }
    let bias = config.hop_alignment_bias.max(0.0);
    alignment = alignment * bias;
    
    println!("  {}: External field: {:.3}, {:.3}", label, external_field.x, external_field.y);
    println!("  Local field: {:.3}, {:.3} (mag: {:.3})", local_field.x, local_field.y, local_field.mag());
    println!("  Hop direction: {:.3}, {:.3}", hop_dir.x, hop_dir.y);
    println!("  Field direction: {:.3}, {:.3}", field_dir.x, field_dir.y);
    println!("  Dot product: {:.3}", -hop_dir.dot(field_dir));
    println!("  Final alignment: {:.3}", alignment);
}