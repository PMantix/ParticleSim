use std::f32::consts::PI;

fn main() {
    println!("=== CURRENT CONFIGURATION DENSITY ANALYSIS ===\n");
    
    // Current configuration from init_config.toml
    println!("1. CURRENT CONFIGURATION");
    println!("----------------------------------------");
    
    let domain_width = 600.0; // Å
    let domain_height = 400.0; // Å
    let total_domain_area = domain_width * domain_height;
    
    // Electrode geometry
    let electrode_width = 30.0; // Å
    let electrode_height = 350.0; // Å
    let foil_width = 5.0; // Å
    let foil_height = 350.0; // Å
    
    let electrode_area = electrode_width * electrode_height; // per electrode
    let foil_area = foil_width * foil_height; // per foil
    let total_solid_area = 2.0 * (electrode_area + foil_area); // 2 electrodes + 2 foils
    let available_liquid_area = total_domain_area - total_solid_area;
    
    println!("Domain: {}×{} Å = {:.0} Å²", domain_width, domain_height, total_domain_area);
    println!("Each electrode: {}×{} Å = {:.0} Å²", electrode_width, electrode_height, electrode_area);
    println!("Each foil: {}×{} Å = {:.0} Å²", foil_width, foil_height, foil_area);
    println!("Total solid area: {:.0} Å²", total_solid_area);
    println!("Available liquid area: {:.0} Å²", available_liquid_area);
    println!("Solid fraction: {:.1}%", (total_solid_area / total_domain_area) * 100.0);
    
    // Particle counts
    let li_count = 158.0;
    let anion_count = 158.0;
    let ec_count = 1182.0;
    let dmc_count = 938.0;
    let total_liquid_particles = li_count + anion_count + ec_count + dmc_count;
    
    println!("\nParticle counts:");
    println!("Li⁺: {}", li_count);
    println!("PF6 Anions: {}", anion_count);
    println!("EC: {}", ec_count);
    println!("DMC: {}", dmc_count);
    println!("Total liquid particles: {}", total_liquid_particles);
    
    // Step 2: Density calculations
    println!("\n2. DENSITY CALCULATIONS");
    println!("----------------------------------------");
    
    let liquid_density_2d = total_liquid_particles / available_liquid_area;
    let overall_density_2d = total_liquid_particles / total_domain_area; // Including solid regions
    
    println!("Liquid region density: {:.6} particles/Å²", liquid_density_2d);
    println!("Overall domain density: {:.6} particles/Å²", overall_density_2d);
    
    // Area per particle
    let area_per_particle_liquid: f32 = available_liquid_area / total_liquid_particles;
    let area_per_particle_overall: f32 = total_domain_area / total_liquid_particles;
    
    println!("Area per particle (liquid region): {:.1} Å²", area_per_particle_liquid);
    println!("Area per particle (overall domain): {:.1} Å²", area_per_particle_overall);
    
    // Spacing analysis
    let typical_spacing_liquid = area_per_particle_liquid.sqrt();
    let typical_spacing_overall = area_per_particle_overall.sqrt();
    
    println!("Typical spacing (liquid region): {:.1} Å", typical_spacing_liquid);
    println!("Typical spacing (overall domain): {:.1} Å", typical_spacing_overall);
    
    // Step 3: Particle size analysis
    println!("\n3. PARTICLE SIZE ANALYSIS");
    println!("----------------------------------------");
    
    // Particle radii from species.rs
    let li_radius = 0.6667 / 2.0; // Å
    let anion_radius = 2.0; // Å (approximate for PF6⁻)
    let ec_radius = 3.0; // Å
    let dmc_radius = 2.5; // Å
    
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    println!("Particle radii: Li⁺={:.2} Å, PF6 Anion={:.1} Å, EC={:.1} Å, DMC={:.1} Å", 
             li_radius, anion_radius, ec_radius, dmc_radius);
    println!("Particle diameters: Li⁺={:.2} Å, PF6 Anion={:.1} Å, EC={:.1} Å, DMC={:.1} Å", 
             2.0*li_radius, 2.0*anion_radius, 2.0*ec_radius, 2.0*dmc_radius);
    
    // Total particle area
    let total_particle_area = li_count * li_area + anion_count * anion_area + 
                             ec_count * ec_area + dmc_count * dmc_area;
    
    println!("Total particle cross-sectional area: {:.0} Å²", total_particle_area);
    
    // Packing fractions
    let packing_fraction_liquid = total_particle_area / available_liquid_area;
    let packing_fraction_overall = total_particle_area / total_domain_area;
    
    println!("Packing fraction (liquid region): {:.1}%", packing_fraction_liquid * 100.0);
    println!("Packing fraction (overall domain): {:.1}%", packing_fraction_overall * 100.0);
    
    // Step 4: Weighted average particle size
    println!("\n4. AVERAGE PARTICLE PROPERTIES");
    println!("----------------------------------------");
    
    let weighted_avg_radius = (li_count * li_radius + anion_count * anion_radius + 
                              ec_count * ec_radius + dmc_count * dmc_radius) / total_liquid_particles;
    let weighted_avg_diameter = 2.0 * weighted_avg_radius;
    
    println!("Weighted average radius: {:.1} Å", weighted_avg_radius);
    println!("Weighted average diameter: {:.1} Å", weighted_avg_diameter);
    
    // Spacing to size ratios
    let spacing_diameter_ratio_liquid = typical_spacing_liquid / weighted_avg_diameter;
    let spacing_diameter_ratio_overall = typical_spacing_overall / weighted_avg_diameter;
    
    println!("Spacing/diameter ratio (liquid): {:.1}", spacing_diameter_ratio_liquid);
    println!("Spacing/diameter ratio (overall): {:.1}", spacing_diameter_ratio_overall);
    
    // Step 5: Physical interpretation
    println!("\n5. PHYSICAL INTERPRETATION");
    println!("----------------------------------------");
    
    println!("Liquid region analysis:");
    if spacing_diameter_ratio_liquid < 1.0 {
        println!("⚠ OVERLAPPING: Particles would overlap - impossible!");
    } else if spacing_diameter_ratio_liquid < 1.5 {
        println!("⚠ TOO DENSE: Particles barely fit - would jam like a solid");
    } else if spacing_diameter_ratio_liquid < 3.0 {
        println!("✓ LIQUID-LIKE: Good for fluid behavior with interactions");
    } else if spacing_diameter_ratio_liquid < 5.0 {
        println!("⚠ SPARSE: Weak interactions - may cause circulation artifacts");
    } else {
        println!("⚠ TOO SPARSE: Essentially non-interacting gas - unrealistic");
    }
    
    println!("\nOverall domain analysis:");
    if spacing_diameter_ratio_overall < 3.0 {
        println!("✓ REASONABLE: Particles have good interaction potential");
    } else if spacing_diameter_ratio_overall < 5.0 {
        println!("⚠ SOMEWHAT SPARSE: May show some circulation effects");
    } else {
        println!("⚠ TOO SPARSE: Strong circulation artifacts expected");
    }
    
    // Step 6: Summary
    println!("\n6. SUMMARY");
    println!("----------------------------------------");
    println!("Domain: {}×{} Å ({:.0} Å²)", domain_width, domain_height, total_domain_area);
    println!("Liquid particles: {:.0} in {:.0} Å² liquid area", total_liquid_particles, available_liquid_area);
    println!("Liquid density: {:.4} particles/Å² ({:.1}% packing)", liquid_density_2d, packing_fraction_liquid * 100.0);
    println!("Overall density: {:.4} particles/Å² ({:.1}% packing)", overall_density_2d, packing_fraction_overall * 100.0);
    println!("Spacing/diameter: {:.1} (liquid), {:.1} (overall)", spacing_diameter_ratio_liquid, spacing_diameter_ratio_overall);
    
    if packing_fraction_liquid > 40.0 {
        println!("❌ TOO DENSE - reduce particle count");
    } else if packing_fraction_liquid < 15.0 {
        println!("❌ TOO SPARSE - increase particle count or domain size");
    } else if spacing_diameter_ratio_liquid > 3.0 {
        println!("⚠ CIRCULATION RISK - particles may not interact enough");
    } else {
        println!("✅ DENSITY LOOKS REASONABLE");
    }
}
