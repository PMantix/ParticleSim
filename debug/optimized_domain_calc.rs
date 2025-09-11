use std::f32::consts::PI;

fn main() {
    println!("=== OPTIMIZED DOMAIN SIZE CALCULATION ===\n");
    println!("Adjusting electrode geometry and particle counts for reasonable domain\n");
    
    // Step 1: Current particle ratios (must maintain these)
    println!("1. CURRENT PARTICLE RATIOS");
    println!("----------------------------------------");
    
    let current_li = 450.0;
    let current_anion = 450.0;
    let current_ec = 3370.0;
    let current_dmc = 2673.0;
    let current_total = current_li + current_anion + current_ec + current_dmc;
    
    // Calculate ratios
    let li_ratio = current_li / current_total;
    let anion_ratio = current_anion / current_total;
    let ec_ratio = current_ec / current_total;
    let dmc_ratio = current_dmc / current_total;
    
    println!("Current counts: Li⁺={}, Anions={}, EC={}, DMC={}", current_li, current_anion, current_ec, current_dmc);
    println!("Total particles: {}", current_total);
    println!("Ratios: Li⁺={:.3}, Anions={:.3}, EC={:.3}, DMC={:.3}", li_ratio, anion_ratio, ec_ratio, dmc_ratio);
    println!("Li:Anion = 1:1 (electroneutrality)");
    println!("EC:DMC = {:.2}:1", current_ec / current_dmc);
    
    // Step 2: Target domain size (reasonable for GUI)
    println!("\n2. TARGET DOMAIN SIZE");
    println!("----------------------------------------");
    
    let target_width = 600.0; // Å - reasonable for viewing
    let target_height = 400.0; // Å - reasonable aspect ratio
    let target_area = target_width * target_height;
    
    println!("Target domain: {}×{} Å", target_width, target_height);
    println!("Target area: {} Å²", target_area);
    
    // Step 3: Optimized electrode geometry
    println!("\n3. OPTIMIZED ELECTRODE GEOMETRY");
    println!("----------------------------------------");
    
    // Make electrodes taller and thinner, closer together
    let new_electrode_width = 30.0; // Å (was 51)
    let new_electrode_height = 350.0; // Å (was 100) - most of domain height
    let new_foil_width = 5.0; // Å (was 7)
    let electrode_separation = 200.0; // Å (was 300) - closer together
    
    let electrode_area = new_electrode_width * new_electrode_height;
    let foil_area = new_foil_width * new_electrode_height;
    let total_electrode_area = 2.0 * (electrode_area + foil_area);
    
    println!("New electrode: {}×{} Å = {} Å² each", new_electrode_width, new_electrode_height, electrode_area);
    println!("New foil: {}×{} Å = {} Å² each", new_foil_width, new_electrode_height, foil_area);
    println!("Electrode separation: {} Å", electrode_separation);
    println!("Total electrode area: {} Å²", total_electrode_area);
    
    // Check if electrodes fit in target domain
    let electrode_positions = electrode_separation / 2.0; // ±100 Å
    let margin = 20.0; // Å
    let required_width = electrode_separation + new_electrode_width + 2.0 * margin;
    let required_height = new_electrode_height + 2.0 * margin;
    
    println!("Required width: {} Å", required_width);
    println!("Required height: {} Å", required_height);
    println!("Fits in target? Width: {}, Height: {}", 
             target_width >= required_width, target_height >= required_height);
    
    // Step 4: Available liquid area
    println!("\n4. AVAILABLE LIQUID AREA");
    println!("----------------------------------------");
    
    let available_liquid_area = target_area - total_electrode_area;
    println!("Available liquid area: {} Å²", available_liquid_area);
    
    // Step 5: Calculate optimal particle count
    println!("\n5. OPTIMAL PARTICLE COUNT");
    println!("----------------------------------------");
    
    // Use same density and packing calculations as before
    let total_density_a3 = 8.9667e-3; // particles/Å³ from previous calculation
    let effective_thickness = 15.0; // Å
    let target_packing_fraction = 0.25; // 25% - slightly lower for more space
    
    // Particle radii
    let li_radius = 0.6667 / 2.0;
    let anion_radius = 2.0;
    let ec_radius = 3.0;
    let dmc_radius = 2.5;
    
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    // Calculate particle area per unit count (maintaining ratios)
    let area_per_particle_set = li_ratio * li_area + anion_ratio * anion_area + 
                               ec_ratio * ec_area + dmc_ratio * dmc_area;
    
    println!("Area per particle (weighted by ratios): {:.2} Å²", area_per_particle_set);
    
    // From packing constraint
    let max_particles_packing = available_liquid_area * target_packing_fraction / area_per_particle_set;
    
    // From density constraint
    let real_volume_per_particle = 1.0 / total_density_a3;
    let available_liquid_volume = available_liquid_area * effective_thickness;
    let max_particles_density = available_liquid_volume / real_volume_per_particle;
    
    println!("Max particles (packing): {:.0}", max_particles_packing);
    println!("Max particles (density): {:.0}", max_particles_density);
    
    let optimal_total = max_particles_packing.min(max_particles_density);
    println!("Optimal total particles: {:.0}", optimal_total);
    
    // Step 6: New particle counts
    println!("\n6. NEW PARTICLE COUNTS");
    println!("----------------------------------------");
    
    let new_li = (optimal_total * li_ratio).round();
    let new_anion = (optimal_total * anion_ratio).round();
    let new_ec = (optimal_total * ec_ratio).round();
    let new_dmc = (optimal_total * dmc_ratio).round();
    let actual_total = new_li + new_anion + new_ec + new_dmc;
    
    println!("New counts: Li⁺={}, Anions={}, EC={}, DMC={}", new_li, new_anion, new_ec, new_dmc);
    println!("Actual total: {}", actual_total);
    println!("Scaling factor: {:.2}× (current → new)", actual_total / current_total);
    
    // Verify ratios maintained
    println!("New ratios: Li⁺={:.3}, Anions={:.3}, EC={:.3}, DMC={:.3}", 
             new_li/actual_total, new_anion/actual_total, new_ec/actual_total, new_dmc/actual_total);
    
    // Step 7: Final verification
    println!("\n7. FINAL VERIFICATION");
    println!("----------------------------------------");
    
    let actual_particle_area = new_li * li_area + new_anion * anion_area + 
                              new_ec * ec_area + new_dmc * dmc_area;
    let actual_packing = actual_particle_area / available_liquid_area;
    let actual_2d_density = actual_total / available_liquid_area;
    let actual_3d_density = actual_2d_density / effective_thickness;
    let density_ratio = actual_3d_density / total_density_a3;
    
    println!("Final domain: {}×{} Å", target_width, target_height);
    println!("Electrode positions: ±{} Å", electrode_positions);
    println!("Electrode size: {}×{} Å", new_electrode_width, new_electrode_height);
    println!("Foil size: {}×{} Å", new_foil_width, new_electrode_height);
    println!("Liquid packing fraction: {:.1}%", actual_packing * 100.0);
    println!("Density ratio (sim/real): {:.2}", density_ratio);
    
    if actual_packing < 0.4 && density_ratio > 0.05 && density_ratio < 2.0 {
        println!("✓ Configuration looks excellent!");
    } else {
        println!("⚠ May need further adjustment");
    }
    
    // Step 8: Configuration output
    println!("\n8. RECOMMENDED CONFIGURATION");
    println!("----------------------------------------");
    println!("Domain: width={}, height={}", target_width, target_height);
    println!("Electrode positions: x = ±{}", electrode_positions);
    println!("Metal electrode: {}×{}", new_electrode_width, new_electrode_height);
    println!("Foil electrode: {}×{}", new_foil_width, new_electrode_height);
    println!("Particle counts: Li⁺={}, Anions={}, EC={}, DMC={}", new_li, new_anion, new_ec, new_dmc);
}
