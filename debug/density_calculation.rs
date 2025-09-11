fn main() {
    // Typical organic electrolyte density calculation
    
    // Example: 1M LiPF6 in EC:DMC (1:1 v/v)
    let li_concentration = 1.0; // M (mol/L)
    let avogadro = 6.022e23;    // particles/mol
    let volume_liter = 1.0;     // L
    
    // EC:DMC mixture density ≈ 1.3 g/cm³ = 1300 kg/m³
    let electrolyte_density = 1300.0; // kg/m³
    
    // Molecular masses (g/mol)
    let ec_mass = 88.0;   // Ethylene carbonate
    let dmc_mass = 90.0;  // Dimethyl carbonate
    let li_mass = 7.0;    // Lithium ion
    let pf6_mass = 145.0; // PF6 anion
    
    // Volume calculations
    let volume_m3 = volume_liter * 1e-3;
    let volume_a3 = volume_m3 * 1e30; // Convert m³ to Å³
    
    println!("Realistic electrolyte density calculation:");
    println!("1M LiPF6 in EC:DMC (1:1 v/v)");
    println!();
    
    // Number density (particles per unit volume)
    let li_per_m3 = li_concentration * avogadro * 1000.0; // particles/m³
    let li_per_a3 = li_per_m3 * 1e-30; // particles/Å³
    
    println!("Li⁺ concentration: {:.3e} particles/Å³", li_per_a3);
    println!("Equal anion concentration: {:.3e} particles/Å³", li_per_a3);
    
    // Estimate solvent molecules
    // For 1:1 EC:DMC by volume, roughly equal molar amounts
    let total_mass_per_liter = electrolyte_density; // g/L since density ≈ 1.3 g/cm³
    let li_mass_per_liter = li_concentration * li_mass;
    let anion_mass_per_liter = li_concentration * pf6_mass;
    let salt_mass = li_mass_per_liter + anion_mass_per_liter;
    let solvent_mass = total_mass_per_liter - salt_mass;
    
    // Assume 50:50 EC:DMC by mass
    let ec_mass_fraction = 0.5;
    let dmc_mass_fraction = 0.5;
    
    let ec_moles = (solvent_mass * ec_mass_fraction) / ec_mass;
    let dmc_moles = (solvent_mass * dmc_mass_fraction) / dmc_mass;
    
    let ec_per_a3 = (ec_moles * avogadro * 1000.0) * 1e-30;
    let dmc_per_a3 = (dmc_moles * avogadro * 1000.0) * 1e-30;
    
    println!("EC concentration: {:.3e} particles/Å³", ec_per_a3);
    println!("DMC concentration: {:.3e} particles/Å³", dmc_per_a3);
    println!("Total density: {:.3e} particles/Å³", li_per_a3 + li_per_a3 + ec_per_a3 + dmc_per_a3);
    
    // For current particle counts, what should domain size be?
    let current_li = 450.0;
    let current_anion = 450.0;
    let current_ec = 3370.0;
    let current_dmc = 2673.0;
    let total_particles = current_li + current_anion + current_ec + current_dmc;
    
    println!("\nCurrent particle counts:");
    println!("Li⁺: {}, Anions: {}, EC: {}, DMC: {}", current_li, current_anion, current_ec, current_dmc);
    println!("Total: {}", total_particles);
    
    // Calculate required area for realistic density
    let target_density_2d = (li_per_a3 + li_per_a3 + ec_per_a3 + dmc_per_a3) * 0.5; // Assume 10 Å thickness
    let required_area = total_particles / target_density_2d;
    let side_length = required_area.sqrt();
    
    println!("\nFor realistic density (assuming 10 Å thickness):");
    println!("Required area: {:.0} Å²", required_area);
    println!("Square domain side: {:.0} Å", side_length);
    
    // Current domain
    let current_area = 800.0 * 500.0;
    let current_density = total_particles / current_area;
    
    println!("\nCurrent domain: 800×500 Å = {:.0} Å²", current_area);
    println!("Current 2D density: {:.4} particles/Å²", current_density);
    println!("Target 2D density: {:.4} particles/Å²", target_density_2d);
    println!("Current domain is {:.1}x too large", current_area / required_area);
}
