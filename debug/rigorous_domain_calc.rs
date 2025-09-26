use std::f32::consts::PI;

fn main() {
    println!("=== RIGOROUS DOMAIN SIZE CALCULATION ===\n");
    
    // Step 1: Realistic 3D electrolyte density
    println!("1. REALISTIC ELECTROLYTE DENSITY");
    println!("----------------------------------------");
    
    // 1M LiPF6 in EC:DMC (1:1 v/v) - typical Li-ion electrolyte
    let li_concentration = 1.0; // mol/L
    let avogadro = 6.022e23;
    let electrolyte_density_kg_m3 = 1300.0; // kg/m³ for EC:DMC mixture
    
    // Molecular masses (g/mol)
    let li_mass = 6.94;
    let pf6_mass = 144.96;
    let ec_mass = 88.06;
    let dmc_mass = 90.08;
    
    // Calculate number densities (particles/m³)
    let li_density_3d = li_concentration * avogadro * 1000.0; // particles/m³
    let pf6_density_3d = li_density_3d; // Electroneutrality
    
    println!("Li⁺ concentration: {:.2e} particles/m³", li_density_3d);
    println!("PF6⁻ concentration: {:.2e} particles/m³", pf6_density_3d);
    
    // Estimate solvent density from remaining mass
    let salt_mass_per_liter = li_concentration * (li_mass + pf6_mass); // g/L
    let solvent_mass_per_liter = electrolyte_density_kg_m3 - salt_mass_per_liter; // g/L
    
    // Assume 50:50 EC:DMC by moles (approximately by volume)
    let avg_solvent_mass = (ec_mass + dmc_mass) / 2.0;
    let total_solvent_moles = solvent_mass_per_liter / avg_solvent_mass;
    let ec_moles = total_solvent_moles * 0.5;
    let dmc_moles = total_solvent_moles * 0.5;
    
    let ec_density_3d = ec_moles * avogadro * 1000.0;
    let dmc_density_3d = dmc_moles * avogadro * 1000.0;
    
    println!("EC concentration: {:.2e} particles/m³", ec_density_3d);
    println!("DMC concentration: {:.2e} particles/m³", dmc_density_3d);
    
    let total_density_3d = li_density_3d + pf6_density_3d + ec_density_3d + dmc_density_3d;
    println!("Total 3D density: {:.2e} particles/m³", total_density_3d);
    
    // Convert to particles/Å³
    let total_density_a3 = total_density_3d * 1e-30;
    println!("Total 3D density: {:.4e} particles/Å³", total_density_a3);
    
    // Step 2: Simulation particle counts and sizes
    println!("\n2. SIMULATION PARTICLE PROPERTIES");
    println!("----------------------------------------");
    
    let sim_li = 450.0;
    let sim_anion = 450.0;
    let sim_ec = 3370.0;
    let sim_dmc = 2673.0;
    let total_sim_particles = sim_li + sim_anion + sim_ec + sim_dmc;
    
    // Particle radii from species.rs
    let li_radius = 0.6667 / 2.0; // Å
    let anion_radius = 2.0; // Å (approximate for PF6⁻)
    let ec_radius = 3.0; // Å
    let dmc_radius = 2.5; // Å
    
    println!("Particle counts: Li⁺={}, PF6 Anions={}, EC={}, DMC={}", sim_li, sim_anion, sim_ec, sim_dmc);
    println!("Total particles: {}", total_sim_particles);
    println!("Particle radii: Li⁺={:.2} Å, PF6 Anion={:.1} Å, EC={:.1} Å, DMC={:.1} Å", 
             li_radius, anion_radius, ec_radius, dmc_radius);
    
    // Step 3: Calculate effective volume per particle
    println!("\n3. VOLUME SCALING ANALYSIS");
    println!("----------------------------------------");
    
    // Volume per particle in real electrolyte
    let real_volume_per_particle = 1.0 / total_density_a3; // Å³/particle
    println!("Real volume per particle: {:.0} Å³", real_volume_per_particle);
    
    // Total volume needed for simulation particles at realistic density
    let required_3d_volume = total_sim_particles * real_volume_per_particle;
    println!("Required 3D volume: {:.0} Å³", required_3d_volume);
    
    // Step 4: 2D projection - choose effective thickness
    println!("\n4. 2D PROJECTION CALCULATION");
    println!("----------------------------------------");
    
    // The thickness should be related to the particle sizes
    // A reasonable choice is 2-3x the largest particle diameter
    let max_diameter = 2.0 * ec_radius; // EC is largest
    let effective_thickness = 2.5 * max_diameter;
    println!("Largest particle diameter: {:.1} Å (EC)", max_diameter);
    println!("Chosen effective thickness: {:.1} Å", effective_thickness);
    
    let required_2d_area = required_3d_volume / effective_thickness;
    println!("Required 2D area: {:.0} Å²", required_2d_area);
    
    // Step 5: Packing efficiency constraint
    println!("\n5. PACKING EFFICIENCY CONSTRAINT");
    println!("----------------------------------------");
    
    // Calculate average particle cross-sectional area
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    let total_particle_area = sim_li * li_area + sim_anion * anion_area + 
                             sim_ec * ec_area + sim_dmc * dmc_area;
    
    println!("Total particle cross-sectional area: {:.0} Å²", total_particle_area);
    
    // Maximum theoretical packing for circles is π/(2√3) ≈ 0.9069
    // But we want much lower density for realistic fluid behavior
    let target_packing_fraction = 0.3; // 30% - reasonable for liquid
    let min_domain_area = total_particle_area / target_packing_fraction;
    
    println!("Target packing fraction: {:.1}%", target_packing_fraction * 100.0);
    println!("Minimum domain area (packing): {:.0} Å²", min_domain_area);
    
    // Step 6: Final domain size recommendation
    println!("\n6. DOMAIN SIZE RECOMMENDATION");
    println!("----------------------------------------");
    
    // Take the larger of density-based and packing-based requirements
    let final_area = min_domain_area.max(required_2d_area);
    let side_length = final_area.sqrt();
    
    println!("Required area (density): {:.0} Å²", required_2d_area);
    println!("Required area (packing): {:.0} Å²", min_domain_area);
    println!("Final required area: {:.0} Å²", final_area);
    println!("Square domain side: {:.0} Å", side_length);
    
    // Suggest rectangular domain with reasonable aspect ratio
    let aspect_ratio = 1.5; // Width:height
    let width = (final_area * aspect_ratio).sqrt();
    let height = final_area / width;
    
    println!("\nRecommended rectangular domain:");
    println!("Width: {:.0} Å", width);
    println!("Height: {:.0} Å", height);
    println!("Area: {:.0} Å²", width * height);
    
    // Verify densities
    let final_2d_density = total_sim_particles / final_area;
    let implied_3d_density = final_2d_density / effective_thickness;
    let density_ratio = implied_3d_density / total_density_a3;
    
    println!("\n7. VERIFICATION");
    println!("----------------------------------------");
    println!("Final 2D density: {:.4e} particles/Å²", final_2d_density);
    println!("Implied 3D density: {:.4e} particles/Å³", implied_3d_density);
    println!("Target 3D density: {:.4e} particles/Å³", total_density_a3);
    println!("Density ratio (sim/real): {:.2}", density_ratio);
    println!("Final packing fraction: {:.1}%", (total_particle_area / final_area) * 100.0);
}
