use std::f32::consts::PI;

fn main() {
    println!("=== PRACTICAL DENSITY COMPROMISE ===\n");
    
    // Current configuration
    let domain_width = 600.0;
    let domain_height = 400.0;
    let available_liquid_area = 215500.0; // From previous calculation
    
    let current_li = 158.0;
    let current_anion = 158.0;
    let current_ec = 1182.0;
    let current_dmc = 938.0;
    
    // Particle areas
    let li_radius = 0.6667 / 2.0;
    let anion_radius = 2.0;
    let ec_radius = 3.0;
    let dmc_radius = 2.5;
    
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    // Target packing fractions to test
    let target_packings = [0.35, 0.40, 0.45, 0.50]; // Higher than before
    
    println!("Testing higher packing fractions to reduce circulation:\n");
    
    for packing in target_packings.iter() {
        println!("=== TARGET PACKING: {:.0}% ===", packing * 100.0);
        
        // Calculate area per "particle set" maintaining ratios
        let total_current = current_li + current_anion + current_ec + current_dmc;
        let li_ratio = current_li / total_current;
        let anion_ratio = current_anion / total_current;
        let ec_ratio = current_ec / total_current;
        let dmc_ratio = current_dmc / total_current;
        
        let area_per_set = li_ratio * li_area + anion_ratio * anion_area + 
                          ec_ratio * ec_area + dmc_ratio * dmc_area;
        
        // Calculate max particles for this packing
        let max_total_particles = (available_liquid_area * packing) / area_per_set;
        
        // Scale up from current counts
        let scale_factor = max_total_particles / total_current;
        
        let new_li = (current_li * scale_factor).round() as i32;
        let new_anion = (current_anion * scale_factor).round() as i32;
        let new_ec = (current_ec * scale_factor).round() as i32;
        let new_dmc = (current_dmc * scale_factor).round() as i32;
        let actual_total = new_li + new_anion + new_ec + new_dmc;
        
        // Calculate spacing
        let new_density = actual_total as f32 / available_liquid_area;
        let avg_spacing = (1.0 / new_density).sqrt();
        let avg_radius = (new_li as f32 * li_radius + new_anion as f32 * anion_radius + 
                         new_ec as f32 * ec_radius + new_dmc as f32 * dmc_radius) / actual_total as f32;
        
        println!("Scale factor: {:.1}×", scale_factor);
        println!("New counts: Li⁺={}, PF6 Anions={}, EC={}, DMC={}", new_li, new_anion, new_ec, new_dmc);
        println!("Total particles: {}", actual_total);
        println!("2D density: {:.4} particles/Å²", new_density);
        println!("Average spacing: {:.1} Å", avg_spacing);
        println!("Spacing/diameter ratio: {:.1}", avg_spacing / (2.0 * avg_radius));
        
        // Check if this reduces circulation
        if avg_spacing / (2.0 * avg_radius) < 2.5 && avg_spacing / (2.0 * avg_radius) > 1.2 {
            println!("✓ Should reduce circulation - particles close enough to interact");
        } else if avg_spacing / (2.0 * avg_radius) < 1.2 {
            println!("⚠ May be too dense - could cause jamming");
        } else {
            println!("⚠ Still too sparse - circulation may persist");
        }
        
        // Compare with realistic density (15 Å thickness)
        let realistic_density_15a = 0.127011; // From previous calculation
        let density_ratio = new_density / realistic_density_15a;
        println!("Density vs realistic (15Å): {:.1}× realistic", density_ratio);
        
        println!();
    }
    
    // Recommend specific configuration
    println!("=== RECOMMENDED CONFIGURATION ===");
    println!("Target: 40-45% packing fraction");
    println!("This should:");
    println!("1. Reduce large gaps that cause circulation");
    println!("2. Keep particles close enough to interact");
    println!("3. Maintain fluid behavior (not jamming)");
    println!("4. Make repulsive forces more effective");
}
