use std::f32::consts::PI;

fn main() {
    println!("=== CORRECTED DOMAIN SIZE CALCULATION ===\n");
    println!("Separating liquid electrolyte from solid electrodes\n");
    
    // Step 1: Electrode geometry (solid, not liquid density)
    println!("1. ELECTRODE GEOMETRY");
    println!("----------------------------------------");
    
    let electrode_width = 51.0; // Å
    let electrode_height = 100.0; // Å
    let electrode_area = electrode_width * electrode_height; // per electrode
    let foil_width = 7.0; // Å
    let foil_area = foil_width * electrode_height; // per foil
    
    println!("Metal electrode: {}×{} Å = {} Å² each", electrode_width, electrode_height, electrode_area);
    println!("Foil electrode: {}×{} Å = {} Å² each", foil_width, electrode_height, foil_area);
    println!("Total electrode area: {} Å²", 2.0 * (electrode_area + foil_area));
    
    // Electrodes are at x = ±150 Å, so minimum domain width
    let electrode_separation = 300.0; // Å
    let min_domain_width = electrode_separation + electrode_width; // Need space beyond electrodes
    println!("Electrode separation: {} Å", electrode_separation);
    println!("Minimum domain width: {} Å", min_domain_width);
    
    // Step 2: Liquid electrolyte density (same as before)
    println!("\n2. LIQUID ELECTROLYTE DENSITY");
    println!("----------------------------------------");
    
    let li_concentration = 1.0; // mol/L
    let avogadro = 6.022e23;
    
    // 3D number densities for 1M LiPF6 in EC:DMC
    let li_density_3d = li_concentration * avogadro * 1000.0; // particles/m³
    let pf6_density_3d = li_density_3d;
    let electrolyte_density_kg_m3 = 1300.0; // kg/m³
    
    // Molecular masses
    let li_mass = 6.94;
    let pf6_mass = 144.96;
    let ec_mass = 88.06;
    let dmc_mass = 90.08;
    
    let salt_mass_per_liter = li_concentration * (li_mass + pf6_mass);
    let solvent_mass_per_liter = electrolyte_density_kg_m3 - salt_mass_per_liter;
    let avg_solvent_mass = (ec_mass + dmc_mass) / 2.0;
    let total_solvent_moles = solvent_mass_per_liter / avg_solvent_mass;
    
    let ec_density_3d = total_solvent_moles * 0.5 * avogadro * 1000.0;
    let dmc_density_3d = total_solvent_moles * 0.5 * avogadro * 1000.0;
    
    let total_density_3d = li_density_3d + pf6_density_3d + ec_density_3d + dmc_density_3d;
    let total_density_a3 = total_density_3d * 1e-30; // particles/Å³
    
    println!("Total liquid density: {:.4e} particles/Å³", total_density_a3);
    
    // Step 3: Liquid particle counts and areas
    println!("\n3. LIQUID PARTICLE PROPERTIES");
    println!("----------------------------------------");
    
    let sim_li = 450.0;
    let sim_anion = 450.0;
    let sim_ec = 3370.0;
    let sim_dmc = 2673.0;
    let total_liquid_particles = sim_li + sim_anion + sim_ec + sim_dmc;
    
    // Particle radii
    let li_radius = 0.6667 / 2.0; // Å
    let anion_radius = 2.0; // Å
    let ec_radius = 3.0; // Å
    let dmc_radius = 2.5; // Å
    
    println!("Liquid particles: Li⁺={}, Anions={}, EC={}, DMC={}", sim_li, sim_anion, sim_ec, sim_dmc);
    println!("Total liquid particles: {}", total_liquid_particles);
    
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    let total_particle_area = sim_li * li_area + sim_anion * anion_area + 
                             sim_ec * ec_area + sim_dmc * dmc_area;
    
    println!("Total liquid particle area: {:.0} Å²", total_particle_area);
    
    // Step 4: Domain sizing for liquid region
    println!("\n4. LIQUID REGION SIZING");
    println!("----------------------------------------");
    
    // Only the liquid region needs to follow realistic density
    let effective_thickness = 15.0; // Å (2.5× largest diameter)
    let real_volume_per_particle = 1.0 / total_density_a3;
    let required_liquid_volume = total_liquid_particles * real_volume_per_particle;
    let required_liquid_area = required_liquid_volume / effective_thickness;
    
    println!("Required liquid area (density): {:.0} Å²", required_liquid_area);
    
    // Packing constraint for liquid region
    let target_packing_fraction = 0.3;
    let min_liquid_area = total_particle_area / target_packing_fraction;
    
    println!("Required liquid area (packing): {:.0} Å²", min_liquid_area);
    
    let liquid_area = min_liquid_area.max(required_liquid_area);
    println!("Final liquid area needed: {:.0} Å²", liquid_area);
    
    // Step 5: Total domain calculation
    println!("\n5. TOTAL DOMAIN CALCULATION");
    println!("----------------------------------------");
    
    // The liquid must fit between and around the electrodes
    // Available width for liquid = total_width - electrode_width - some margin
    let electrode_margin = 20.0; // Å buffer around electrodes
    let available_width = min_domain_width - electrode_width - 2.0 * electrode_margin;
    let required_height = liquid_area / available_width;
    
    println!("Available width for liquid: {:.0} Å", available_width);
    println!("Required height for liquid: {:.0} Å", required_height);
    
    // Check if this fits with electrode height
    let min_height = electrode_height.max(required_height);
    let final_width = min_domain_width;
    let final_height = min_height;
    let final_area = final_width * final_height;
    
    println!("\nFinal domain dimensions:");
    println!("Width: {:.0} Å (electrode constraint)", final_width);
    println!("Height: {:.0} Å", final_height);
    println!("Total area: {:.0} Å²", final_area);
    
    // Effective liquid area (excluding electrodes)
    let effective_liquid_area = final_area - 2.0 * (electrode_area + foil_area);
    println!("Effective liquid area: {:.0} Å²", effective_liquid_area);
    
    // Verification
    println!("\n6. VERIFICATION");
    println!("----------------------------------------");
    let liquid_density_2d = total_liquid_particles / effective_liquid_area;
    let implied_3d_density = liquid_density_2d / effective_thickness;
    let density_ratio = implied_3d_density / total_density_a3;
    let packing_fraction = total_particle_area / effective_liquid_area;
    
    println!("Liquid 2D density: {:.4e} particles/Å²", liquid_density_2d);
    println!("Implied 3D density: {:.4e} particles/Å³", implied_3d_density);
    println!("Target 3D density: {:.4e} particles/Å³", total_density_a3);
    println!("Density ratio (sim/real): {:.2}", density_ratio);
    println!("Liquid packing fraction: {:.1}%", packing_fraction * 100.0);
    
    if packing_fraction > 0.9 {
        println!("WARNING: Packing fraction too high!");
    } else if density_ratio < 0.01 {
        println!("WARNING: Density much too low!");
    } else if density_ratio > 10.0 {
        println!("WARNING: Density much too high!");
    } else {
        println!("✓ Domain size appears reasonable");
    }
}
