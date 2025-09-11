use std::f32::consts::PI;

fn main() {
    println!("=== DENSITY AND PHYSICS VERIFICATION ===\n");
    
    // Step 1: Current configuration verification
    println!("1. CURRENT CONFIGURATION");
    println!("----------------------------------------");
    
    let domain_width = 600.0; // Å
    let domain_height = 400.0; // Å
    let domain_area = domain_width * domain_height;
    
    let electrode_width = 30.0; // Å
    let electrode_height = 350.0; // Å
    let foil_width = 5.0; // Å
    let electrode_area = electrode_width * electrode_height;
    let foil_area = foil_width * electrode_height;
    let total_electrode_area = 2.0 * (electrode_area + foil_area);
    let available_liquid_area = domain_area - total_electrode_area;
    
    let current_li = 158.0;
    let current_anion = 158.0;
    let current_ec = 1182.0;
    let current_dmc = 938.0;
    let total_particles = current_li + current_anion + current_ec + current_dmc;
    
    println!("Domain: {}×{} Å = {:.0} Å²", domain_width, domain_height, domain_area);
    println!("Electrode area: {:.0} Å²", total_electrode_area);
    println!("Available liquid area: {:.0} Å²", available_liquid_area);
    println!("Current particles: {:.0}", total_particles);
    println!("Current density: {:.6} particles/Å²", total_particles / available_liquid_area);
    
    // Step 2: Compare with realistic lab densities
    println!("\n2. REALISTIC LAB DENSITY COMPARISON");
    println!("----------------------------------------");
    
    // More detailed realistic density calculation
    let li_concentration = 1.0; // mol/L
    let avogadro = 6.022e23;
    
    // More precise molecular densities for 1M LiPF6 in EC:DMC (1:1 v/v)
    let li_density_3d = li_concentration * avogadro * 1000.0; // 6.02e26 particles/m³
    let pf6_density_3d = li_density_3d; // Electroneutrality
    
    // EC and DMC densities from liquid properties
    let ec_liquid_density = 1321.0; // kg/m³ at 25°C
    let dmc_liquid_density = 1063.0; // kg/m³ at 25°C
    let ec_molar_mass = 88.06; // g/mol
    let dmc_molar_mass = 90.08; // g/mol
    
    // Pure solvent number densities
    let ec_pure_density = (ec_liquid_density * 1000.0 / ec_molar_mass) * avogadro; // particles/m³
    let dmc_pure_density = (dmc_liquid_density * 1000.0 / dmc_molar_mass) * avogadro; // particles/m³
    
    // In 1M LiPF6 solution, solvent is diluted
    let salt_volume_fraction = 0.1; // Approximate for 1M solution
    let solvent_volume_fraction = 1.0 - salt_volume_fraction;
    let ec_solution_density = ec_pure_density * solvent_volume_fraction * 0.5; // 50% of solvent
    let dmc_solution_density = dmc_pure_density * solvent_volume_fraction * 0.5; // 50% of solvent
    
    let total_realistic_density_3d = li_density_3d + pf6_density_3d + ec_solution_density + dmc_solution_density;
    let total_realistic_density_a3 = total_realistic_density_3d * 1e-30; // particles/Å³
    
    println!("Li⁺ density: {:.2e} particles/m³", li_density_3d);
    println!("PF6⁻ density: {:.2e} particles/m³", pf6_density_3d);
    println!("EC density (in solution): {:.2e} particles/m³", ec_solution_density);
    println!("DMC density (in solution): {:.2e} particles/m³", dmc_solution_density);
    println!("Total realistic 3D density: {:.2e} particles/m³", total_realistic_density_3d);
    println!("Total realistic 3D density: {:.6e} particles/Å³", total_realistic_density_3d * 1e-30);
    
    // Step 3: 2D projection analysis
    println!("\n3. 2D PROJECTION ANALYSIS");
    println!("----------------------------------------");
    
    // Different thickness assumptions
    let thicknesses = [0.5,5.0, 10.0, 15.0, 20.0, 30.0]; // Å
    
    for thickness in thicknesses.iter() {
        let realistic_2d_density = total_realistic_density_3d * 1e-30 * thickness;
        let needed_particles = realistic_2d_density * available_liquid_area;
        let scaling_factor = needed_particles / total_particles;
        
        println!("Thickness {:.0} Å:", thickness);
        println!("  Realistic 2D density: {:.6} particles/Å²", realistic_2d_density);
        println!("  Needed particles: {:.0}", needed_particles);
        println!("  Current scaling: {:.2}× realistic", 1.0 / scaling_factor);
        println!("  Should scale up by: {:.1}×", scaling_factor);
    }
    
    // Step 4: Particle spacing analysis  
    println!("\n4. PARTICLE SPACING ANALYSIS");
    println!("----------------------------------------");
    
    let current_2d_density = total_particles / available_liquid_area;
    let avg_area_per_particle: f32 = 1.0 / current_2d_density;
    let avg_spacing = avg_area_per_particle.sqrt();
    
    // Particle radii
    let li_radius = 0.6667 / 2.0;
    let anion_radius = 2.0;
    let ec_radius = 3.0;
    let dmc_radius = 2.5;
    let avg_radius = (current_li * li_radius + current_anion * anion_radius + 
                     current_ec * ec_radius + current_dmc * dmc_radius) / total_particles;
    
    println!("Current 2D density: {:.6} particles/Å²", current_2d_density);
    println!("Average area per particle: {:.1} Å²", avg_area_per_particle);
    println!("Average center-to-center spacing: {:.1} Å", avg_spacing);
    println!("Average particle radius: {:.1} Å", avg_radius);
    println!("Spacing/diameter ratio: {:.1}", avg_spacing / (2.0 * avg_radius));
    
    if avg_spacing / (2.0 * avg_radius) < 1.5 {
        println!("⚠ PARTICLES TOO CLOSE - may cause clumping!");
    } else if avg_spacing / (2.0 * avg_radius) > 5.0 {
        println!("⚠ PARTICLES TOO FAR APART - unrealistic density!");
    } else {
        println!("✓ Particle spacing reasonable");
    }
    
    // Step 5: Boundary effects analysis
    println!("\n5. BOUNDARY EFFECTS ANALYSIS");
    println!("----------------------------------------");
    
    let boundary_length = 2.0 * (domain_width + domain_height); // Total boundary
    let boundary_area_10a = boundary_length * 10.0; // 10 Å boundary region
    let boundary_fraction = boundary_area_10a / available_liquid_area;
    
    println!("Total boundary length: {:.0} Å", boundary_length);
    println!("Boundary region area (10 Å): {:.0} Å²", boundary_area_10a);
    println!("Boundary fraction: {:.1}%", boundary_fraction * 100.0);
    
    if boundary_fraction > 0.3 {
        println!("⚠ HIGH BOUNDARY EFFECTS - domain may be too small!");
    } else {
        println!("✓ Boundary effects manageable");
    }
    
    // Step 6: Recommendations
    println!("\n6. RECOMMENDATIONS");
    println!("----------------------------------------");
    
    // Check current repulsive force settings
    println!("Current repulsive force issues likely due to:");
    println!("1. Particle density too low (large gaps → artificial motion)");
    println!("2. Domain aspect ratio creates circulation patterns");
    println!("3. Boundary effects from domain size");
    
    // Recommend particle count increase
    let recommended_thickness = 15.0; // Å
    let recommended_2d_density = total_realistic_density_3d * 1e-30 * recommended_thickness;
    let recommended_particles = recommended_2d_density * available_liquid_area;
    let scale_up_factor = recommended_particles / total_particles;
    
    println!("\nRecommended particle scaling:");
    println!("Target thickness: {:.0} Å", recommended_thickness);
    println!("Target 2D density: {:.6} particles/Å²", recommended_2d_density);
    println!("Recommended total particles: {:.0}", recommended_particles);
    println!("Scale up current counts by: {:.1}×", scale_up_factor);
    
    let rec_li = (current_li * scale_up_factor).round() as i32;
    let rec_anion = (current_anion * scale_up_factor).round() as i32;
    let rec_ec = (current_ec * scale_up_factor).round() as i32;
    let rec_dmc = (current_dmc * scale_up_factor).round() as i32;
    
    println!("New counts: Li⁺={}, Anions={}, EC={}, DMC={}", rec_li, rec_anion, rec_ec, rec_dmc);
    
    // Check if this fits in domain
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    let new_total_area = (rec_li as f32) * li_area + (rec_anion as f32) * anion_area + (rec_ec as f32) * ec_area + (rec_dmc as f32) * dmc_area;
    let new_packing_fraction = new_total_area / available_liquid_area;
    
    println!("New packing fraction: {:.1}%", new_packing_fraction * 100.0);
    
    if new_packing_fraction > 50.0 {
        println!("⚠ Too dense - need larger domain or fewer particles");
    } else if new_packing_fraction < 15.0 {
        println!("⚠ Still too sparse - circulation effects may persist");
    } else {
        println!("✓ Should reduce clumping and circulation");
    }
}
