fn main() {
    println!("=== PARTICLE SIMULATION CALIBRATION ANALYSIS ===\n");
    
    // 1. DOMAIN SIZE ANALYSIS
    println!("1. DOMAIN SIZE OPTIMIZATION");
    println!("----------------------------------------");
    
    // Current setup
    let current_width = 800.0;  // Å
    let current_height = 500.0; // Å
    let current_area = current_width * current_height;
    
    let li_count = 450.0;
    let anion_count = 450.0; 
    let ec_count = 3370.0;
    let dmc_count = 2673.0;
    let total_particles = li_count + anion_count + ec_count + dmc_count;
    
    println!("Current domain: {}×{} Å = {:.0} Å²", current_width, current_height, current_area);
    println!("Particle counts: Li⁺={}, Anions={}, EC={}, DMC={}", li_count, anion_count, ec_count, dmc_count);
    println!("Total particles: {}", total_particles);
    println!("Current 2D density: {:.4} particles/Å²", total_particles / current_area);
    
    // Realistic electrolyte density (1M LiPF6 in EC:DMC)
    // From literature: ~0.001 particles/Å³ for 1M electrolyte
    let realistic_3d_density = 0.001; // particles/Å³
    let assumed_thickness = 10.0; // Å (quasi-2D approximation)
    let target_2d_density = realistic_3d_density * assumed_thickness;
    
    println!("\nTarget density (1M electrolyte, 10Å thick): {:.4} particles/Å²", target_2d_density);
    
    let optimal_area: f32 = total_particles / target_2d_density;
    let optimal_side = optimal_area.sqrt();
    
    println!("Optimal area: {:.0} Å²", optimal_area);
    println!("Optimal square domain: {:.0}×{:.0} Å", optimal_side, optimal_side);
    println!("Current domain is {:.1}x too large!", current_area / optimal_area);
    
    // Recommended new domain size
    let recommended_width = 300.0;
    let recommended_height = 200.0;
    let recommended_area = recommended_width * recommended_height;
    let recommended_density = total_particles / recommended_area;
    
    println!("\nRecommended domain: {}×{} Å = {:.0} Å²", recommended_width, recommended_height, recommended_area);
    println!("Resulting density: {:.4} particles/Å² ({:.1}x realistic)", recommended_density, recommended_density / target_2d_density);
    
    println!("\n============================================================\n");
    
    // 2. REPULSIVE FORCE ANALYSIS
    println!("2. REPULSIVE FORCE CALIBRATION");
    println!("----------------------------------------");
    
    // Current repulsion parameters
    let current_strength = 100.0;
    let current_cutoff = 11.0; // Å
    
    println!("Current EC/DMC repulsion:");
    println!("  Strength: {}", current_strength);
    println!("  Cutoff: {} Å", current_cutoff);
    
    // Analyze the repulsion formula: F = k * (1 - r/r0) / r
    println!("\nRepulsion formula: F = k * (1 - r/r0) / r");
    println!("where k = strength, r0 = cutoff, r = distance");
    
    // Calculate forces at different distances
    println!("\nForce vs distance:");
    for r in [1.0, 2.0, 3.0, 5.0, 8.0, 10.0] {
        if r < current_cutoff {
            let force_mag = current_strength * (1.0 - r / current_cutoff) / r;
            println!("  r = {:.1} Å: F = {:.2}", r, force_mag);
        } else {
            println!("  r = {:.1} Å: F = 0.00 (beyond cutoff)", r);
        }
    }
    
    // Physical justification for repulsion (pressure forces)
    println!("\nPhysical basis for repulsion:");
    println!("- Models osmotic pressure in concentrated electrolyte");
    println!("- Represents missing many-body interactions");
    println!("- Prevents unrealistic clustering");
    
    // Osmotic pressure calculation
    let concentration = 1.0; // M
    let R = 8.314; // J/(mol·K) 
    let T = 300.0; // K
    let osmotic_pressure = concentration * R * T; // Pa
    let osmotic_pressure_atm = osmotic_pressure / 101325.0;
    
    println!("\nOsmotic pressure of 1M solution:");
    println!("  P = CRT = {:.0} Pa = {:.2} atm", osmotic_pressure, osmotic_pressure_atm);
    
    // Convert to force per particle over molecular dimensions
    let molecular_volume = 4.0 * 3.14159 * 2.0_f64.powi(3) / 3.0; // ~33 Å³ for 2Å radius
    let force_per_molecule = osmotic_pressure * molecular_volume * 1e-30; // N
    let force_sim_units = force_per_molecule / (1.66e-27 * 1e-20 / 1e-30); // Convert to sim units
    
    println!("Force per molecule (~2Å radius): {:.2e} N", force_per_molecule);
    println!("In simulation units: {:.2}", force_sim_units);
    
    // Recommended repulsion parameters
    let recommended_strength = 5.0; // Much weaker than current 100.0
    let recommended_cutoff = 6.0;   // Shorter range than current 11.0
    
    println!("\nRecommended repulsion parameters:");
    println!("  Strength: {} (vs current {})", recommended_strength, current_strength);
    println!("  Cutoff: {} Å (vs current {} Å)", recommended_cutoff, current_cutoff);
    println!("  Reduction factor: {:.1}x weaker", current_strength / recommended_strength);
    
    println!("\nRecommended force vs distance:");
    for r in [1.0, 2.0, 3.0, 4.0, 5.0] {
        if r < recommended_cutoff {
            let force_mag = recommended_strength * (1.0 - r / recommended_cutoff) / r;
            println!("  r = {:.1} Å: F = {:.2}", r, force_mag);
        } else {
            println!("  r = {:.1} Å: F = 0.00 (beyond cutoff)", r);
        }
    }
}
