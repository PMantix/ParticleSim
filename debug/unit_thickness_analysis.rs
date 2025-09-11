fn main() {
    println!("=== UNIT THICKNESS DENSITY CALCULATION ===\n");
    
    // Current simulation values
    let available_liquid_area = 215500.0; // Å²
    let total_particles = 6943.0;
    
    // Molecular masses (amu)
    let li_mass = 6.94;
    let pf6_mass = 144.96;
    let ec_mass = 88.06;
    let dmc_mass = 90.08;
    
    // Particle counts
    let li_count = 450.0;
    let anion_count = 450.0;
    let ec_count = 3370.0;
    let dmc_count = 2673.0;
    
    // Total mass in amu
    let total_mass_amu = li_count * li_mass + anion_count * pf6_mass + 
                         ec_count * ec_mass + dmc_count * dmc_mass;
    
    println!("CURRENT SIMULATION:");
    println!("Domain liquid area: {:.0} Å²", available_liquid_area);
    println!("Total mass: {:.0} amu", total_mass_amu);
    
    // Convert to g/cm² using unit thickness approach
    // 1 amu = 1.66054e-24 g
    // 1 Å² = 1e-16 cm²
    let amu_to_g = 1.66054e-24;
    let a2_to_cm2 = 1e-16;
    
    let total_mass_g = total_mass_amu * amu_to_g;
    let area_cm2 = available_liquid_area * a2_to_cm2;
    let current_density_g_cm2 = total_mass_g / area_cm2;
    
    println!("Current 2D mass density: {:.6e} g/cm²", current_density_g_cm2);
    
    // Target density using unit thickness
    let target_density_3d = 1.25; // g/cm³
    let unit_thickness_cm = 1.0;  // 1 cm
    let target_density_2d = target_density_3d * unit_thickness_cm; // g/cm²
    
    println!("\nTARGET (using unit thickness):");
    println!("Target 3D density: {:.2} g/cm³", target_density_3d);
    println!("Unit thickness: {:.0} cm", unit_thickness_cm);
    println!("Target 2D density: {:.2} g/cm²", target_density_2d);
    
    // What scaling factor do we need?
    let scaling_factor = target_density_2d / current_density_g_cm2;
    
    println!("\nSCALING ANALYSIS:");
    println!("Current: {:.6e} g/cm²", current_density_g_cm2);
    println!("Target:  {:.6e} g/cm²", target_density_2d);
    println!("Need to scale mass by: {:.2e}×", scaling_factor);
    
    // This is equivalent to scaling particle count
    let target_particles = total_particles * scaling_factor;
    
    println!("Current particles: {:.0}", total_particles);
    println!("Target particles: {:.2e}", target_particles);
    
    // Check if this is reasonable
    if target_particles > 1e6 {
        println!("⚠ WAY too many particles - not computationally feasible");
    } else if target_particles > 50000.0 {
        println!("⚠ Very high particle count - may be slow");
    } else if target_particles < 100.0 {
        println!("⚠ Too few particles - poor statistics");
    } else {
        println!("✅ Reasonable particle count");
    }
    
    // Alternative: what if we use a more reasonable thickness?
    println!("\nALTERNATIVE: More reasonable 'unit' thickness");
    let reasonable_thicknesses = [1e-8, 1e-7, 1e-6, 1e-5]; // cm
    
    for thickness in reasonable_thicknesses.iter() {
        let alt_target_2d = target_density_3d * thickness;
        let alt_scaling = alt_target_2d / current_density_g_cm2;
        let alt_particles = total_particles * alt_scaling;
        
        println!("Thickness {:.0e} cm:", thickness);
        println!("  Target 2D density: {:.2e} g/cm²", alt_target_2d);
        println!("  Required particles: {:.0}", alt_particles);
        
        if alt_particles >= 1000.0 && alt_particles <= 20000.0 {
            println!("  ✅ Reasonable range");
        } else if alt_particles < 1000.0 {
            println!("  ⚠ Too few particles");
        } else {
            println!("  ⚠ Too many particles");
        }
    }
    
    // What about the packing constraint?
    println!("\nPACKING CONSTRAINT CHECK:");
    let current_packing = 0.713; // 71.3%
    let target_packing = 0.30;   // 30%
    let packing_scaling = target_packing / current_packing;
    let packing_based_particles = total_particles * packing_scaling;
    
    println!("Current packing: {:.1}%", current_packing * 100.0);
    println!("Target packing: {:.1}%", target_packing * 100.0);
    println!("Packing-based particles: {:.0}", packing_based_particles);
    
    // Find thickness that reconciles both constraints
    println!("\nRECONCILING BOTH CONSTRAINTS:");
    for thickness in reasonable_thicknesses.iter() {
        let density_target_particles = total_particles * (target_density_3d * thickness) / current_density_g_cm2;
        let ratio = density_target_particles / packing_based_particles;
        
        println!("Thickness {:.0e} cm: density wants {:.0}, packing wants {:.0}, ratio = {:.2}", 
                 thickness, density_target_particles, packing_based_particles, ratio);
        
        if ratio >= 0.8 && ratio <= 1.2 {
            println!("  ✅ Good match! Both constraints satisfied");
        }
    }
}
