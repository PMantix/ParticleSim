// Add to Cargo.toml dependencies: toml = "0.8", serde = { version = "1.0", features = ["derive"] }

use serde::Deserialize;
use std::fs;
use std::f32::consts::PI;

#[derive(Deserialize, Debug)]
struct Config {
    simulation: Simulation,
    particles: Particles,
}

#[derive(Deserialize, Debug)]
struct Simulation {
    domain_width: f32,
    domain_height: f32,
    initial_temperature: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct Particles {
    metal_rectangles: Option<Vec<MetalRectangle>>,
    foil_rectangles: Option<Vec<FoilRectangle>>,
    random: Option<Vec<RandomParticles>>,
}

#[derive(Deserialize, Debug)]
struct MetalRectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    species: String,
}

#[derive(Deserialize, Debug)]
struct FoilRectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    current: f32,
}

#[derive(Deserialize, Debug)]
struct RandomParticles {
    count: u32,
    species: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TOML-BASED DENSITY ANALYSIS ===\n");
    
    // Read and parse the config file
    let config_content = fs::read_to_string("init_config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    
    println!("1. CONFIGURATION FROM init_config.toml");
    println!("----------------------------------------");
    
    let domain_width = config.simulation.domain_width;
    let domain_height = config.simulation.domain_height;
    let total_domain_area = domain_width * domain_height;
    
    println!("Domain: {}×{} Å = {:.0} Å²", domain_width, domain_height, total_domain_area);
    
    // Calculate electrode areas
    let mut total_solid_area = 0.0;
    
    if let Some(metal_rects) = &config.particles.metal_rectangles {
        for rect in metal_rects {
            let area = rect.width * rect.height;
            total_solid_area += area;
            println!("Metal electrode: {}×{} Å at ({}, {}) = {:.0} Å²", 
                     rect.width, rect.height, rect.x, rect.y, area);
        }
    }
    
    if let Some(foil_rects) = &config.particles.foil_rectangles {
        for rect in foil_rects {
            let area = rect.width * rect.height;
            total_solid_area += area;
            println!("Foil electrode: {}×{} Å at ({}, {}) = {:.0} Å²", 
                     rect.width, rect.height, rect.x, rect.y, area);
        }
    }
    
    let available_liquid_area = total_domain_area - total_solid_area;
    println!("Total solid area: {:.0} Å²", total_solid_area);
    println!("Available liquid area: {:.0} Å²", available_liquid_area);
    println!("Solid fraction: {:.1}%", (total_solid_area / total_domain_area) * 100.0);
    
    // Count particles by species
    let mut li_count = 0;
    let mut anion_count = 0;
    let mut ec_count = 0;
    let mut dmc_count = 0;
    let mut total_liquid_particles = 0;
    
    if let Some(random_particles) = &config.particles.random {
        for particles in random_particles {
            total_liquid_particles += particles.count;
            match particles.species.as_str() {
                "LithiumIon" => li_count += particles.count,
                "ElectrolyteAnion" => anion_count += particles.count,
                "EC" => ec_count += particles.count,
                "DMC" => dmc_count += particles.count,
                _ => println!("Unknown species: {}", particles.species),
            }
            println!("{}: {}", particles.species, particles.count);
        }
    }
    
    println!("Total liquid particles: {}", total_liquid_particles);
    
    // Step 2: Density calculations
    println!("\n2. DENSITY CALCULATIONS");
    println!("----------------------------------------");
    
    let liquid_density_2d = total_liquid_particles as f32 / available_liquid_area;
    let overall_density_2d = total_liquid_particles as f32 / total_domain_area;
    
    println!("Liquid region density: {:.6} particles/Å²", liquid_density_2d);
    println!("Overall domain density: {:.6} particles/Å²", overall_density_2d);
    
    let area_per_particle_liquid = available_liquid_area / total_liquid_particles as f32;
    let area_per_particle_overall = total_domain_area / total_liquid_particles as f32;
    
    println!("Area per particle (liquid region): {:.1} Å²", area_per_particle_liquid);
    println!("Area per particle (overall domain): {:.1} Å²", area_per_particle_overall);
    
    let typical_spacing_liquid = area_per_particle_liquid.sqrt();
    let typical_spacing_overall = area_per_particle_overall.sqrt();
    
    println!("Typical spacing (liquid region): {:.1} Å", typical_spacing_liquid);
    println!("Typical spacing (overall domain): {:.1} Å", typical_spacing_overall);
    
    // Step 3: Particle size analysis (hardcoded from species.rs)
    println!("\n3. PARTICLE SIZE ANALYSIS");
    println!("----------------------------------------");
    
    let li_radius = 0.6667 / 2.0; // Å
    let anion_radius = 2.0; // Å
    let ec_radius = 3.0; // Å
    let dmc_radius = 2.5; // Å
    
    let li_area = PI * li_radius * li_radius;
    let anion_area = PI * anion_radius * anion_radius;
    let ec_area = PI * ec_radius * ec_radius;
    let dmc_area = PI * dmc_radius * dmc_radius;
    
    println!("Particle radii: Li⁺={:.2} Å, Anion={:.1} Å, EC={:.1} Å, DMC={:.1} Å", 
             li_radius, anion_radius, ec_radius, dmc_radius);
    
    let total_particle_area = li_count as f32 * li_area + anion_count as f32 * anion_area + 
                             ec_count as f32 * ec_area + dmc_count as f32 * dmc_area;
    
    println!("Total particle cross-sectional area: {:.0} Å²", total_particle_area);
    
    let packing_fraction_liquid = total_particle_area / available_liquid_area;
    let packing_fraction_overall = total_particle_area / total_domain_area;
    
    println!("Packing fraction (liquid region): {:.1}%", packing_fraction_liquid * 100.0);
    println!("Packing fraction (overall domain): {:.1}%", packing_fraction_overall * 100.0);
    
    // Step 3.5: Mass density calculations
    println!("\n3.5. MASS DENSITY ANALYSIS");
    println!("----------------------------------------");
    
    // Molecular masses (amu = g/mol)
    let li_mass = 6.94; // amu (g/mol)
    let pf6_mass = 144.96; // amu (g/mol) - assuming PF6⁻ anion
    let ec_mass = 88.06; // amu (g/mol)
    let dmc_mass = 90.08; // amu (g/mol)
    
    println!("Molecular masses: Li⁺={:.1} amu, PF6⁻={:.1} amu, EC={:.1} amu, DMC={:.1} amu", 
             li_mass, pf6_mass, ec_mass, dmc_mass);
    
    // Total mass in simulation (amu)
    let total_mass_amu = li_count as f32 * li_mass + anion_count as f32 * pf6_mass + 
                         ec_count as f32 * ec_mass + dmc_count as f32 * dmc_mass;
    
    // Convert amu to grams: 1 amu = 1.66054e-24 g
    let amu_to_g = 1.66054e-24;
    let total_mass_g = total_mass_amu * amu_to_g;
    
    println!("Total mass in simulation: {:.0} amu = {:.2e} g", total_mass_amu, total_mass_g);
    
    // Calculate 2D mass density (mass per unit area)
    let a2_to_cm2 = 1e-16; // 1 Å² = 1e-16 cm²
    let liquid_area_cm2 = available_liquid_area * a2_to_cm2;
    let overall_area_cm2 = total_domain_area * a2_to_cm2;
    
    let mass_density_2d_liquid = total_mass_g / liquid_area_cm2; // g/cm²
    let mass_density_2d_overall = total_mass_g / overall_area_cm2; // g/cm²
    
    println!("\n2D Mass Densities:");
    println!("Liquid region: {:.2e} g/cm² in {:.2e} cm²", mass_density_2d_liquid, liquid_area_cm2);
    println!("Overall domain: {:.2e} g/cm² in {:.2e} cm²", mass_density_2d_overall, overall_area_cm2);
    
    // Compare with what realistic 3D densities would give as 2D densities
    println!("\nComparison with realistic 3D electrolyte densities:");
    let realistic_3d_densities = [1.20, 1.25, 1.30, 1.35]; // g/cm³
    let reference_thicknesses = [5.0, 10.0, 15.0, 20.0]; // Å - just for reference
    
    println!("If real electrolyte (3D) were compressed to our 2D area:");
    for density_3d in realistic_3d_densities.iter() {
        for thickness in reference_thicknesses.iter() {
            let thickness_cm = thickness * 1e-8; // Å to cm
            let equivalent_2d_density = density_3d * thickness_cm; // g/cm²
            let ratio = mass_density_2d_liquid / equivalent_2d_density;
            
            println!("  {:.2} g/cm³ × {:.0}Å = {:.2e} g/cm² → Our ratio: {:.1}×", 
                     density_3d, thickness, equivalent_2d_density, ratio);
        }
        println!();
    }
    
    // More meaningful: what 3D density would our 2D simulation represent?
    println!("What 3D density would our simulation represent at different thicknesses?");
    for thickness in reference_thicknesses.iter() {
        let thickness_cm = thickness * 1e-8; // Å to cm  
        let implied_3d_density = mass_density_2d_liquid / thickness_cm; // g/cm³
        
        println!("  At {:.0}Å thickness: {:.2} g/cm³", thickness, implied_3d_density);
        
        if implied_3d_density >= 1.20 && implied_3d_density <= 1.35 {
            println!("    ✅ Realistic electrolyte density range");
        } else if implied_3d_density > 1.35 {
            println!("    ⚠ Too dense - more than concentrated electrolyte");
        } else if implied_3d_density < 0.5 {
            println!("    ⚠ Too dilute - less than organic solvents");
        } else {
            println!("    ⚠ Below typical electrolyte range");
        }
    }
    
    // Step 4: Weighted average particle size
    println!("\n4. AVERAGE PARTICLE PROPERTIES");
    println!("----------------------------------------");
    
    let weighted_avg_radius = (li_count as f32 * li_radius + anion_count as f32 * anion_radius + 
                              ec_count as f32 * ec_radius + dmc_count as f32 * dmc_radius) / total_liquid_particles as f32;
    let weighted_avg_diameter = 2.0 * weighted_avg_radius;
    
    println!("Weighted average radius: {:.1} Å", weighted_avg_radius);
    println!("Weighted average diameter: {:.1} Å", weighted_avg_diameter);
    
    let spacing_diameter_ratio_liquid = typical_spacing_liquid / weighted_avg_diameter;
    let spacing_diameter_ratio_overall = typical_spacing_overall / weighted_avg_diameter;
    
    println!("Spacing/diameter ratio (liquid): {:.1}", spacing_diameter_ratio_liquid);
    println!("Spacing/diameter ratio (overall): {:.1}", spacing_diameter_ratio_overall);
    
    // Step 5: Species ratios
    println!("\n5. SPECIES RATIOS");
    println!("----------------------------------------");
    
    println!("Li⁺ count: {}", li_count);
    println!("Anion count: {}", anion_count);
    println!("EC count: {}", ec_count);
    println!("DMC count: {}", dmc_count);
    
    if anion_count > 0 {
        println!("Li⁺:Anion ratio: {:.2}:1", li_count as f32 / anion_count as f32);
    }
    if dmc_count > 0 {
        println!("EC:DMC ratio: {:.2}:1", ec_count as f32 / dmc_count as f32);
    }
    
    let li_fraction = li_count as f32 / total_liquid_particles as f32;
    let anion_fraction = anion_count as f32 / total_liquid_particles as f32;
    let ec_fraction = ec_count as f32 / total_liquid_particles as f32;
    let dmc_fraction = dmc_count as f32 / total_liquid_particles as f32;
    
    println!("Mole fractions: Li⁺={:.3}, Anions={:.3}, EC={:.3}, DMC={:.3}", 
             li_fraction, anion_fraction, ec_fraction, dmc_fraction);
    
    // Step 6: Assessment
    println!("\n6. PHYSICAL ASSESSMENT");
    println!("----------------------------------------");
    
    println!("Liquid region analysis:");
    if spacing_diameter_ratio_liquid < 1.0 {
        println!("❌ OVERLAPPING: Particles would overlap - impossible!");
    } else if spacing_diameter_ratio_liquid < 1.2 {
        println!("⚠ TOO DENSE: Particles barely fit - would jam like a solid");
    } else if spacing_diameter_ratio_liquid < 2.0 {
        println!("✅ DENSE LIQUID: Good interactions, minimal circulation artifacts");
    } else if spacing_diameter_ratio_liquid < 3.0 {
        println!("✅ NORMAL LIQUID: Reasonable fluid behavior");
    } else if spacing_diameter_ratio_liquid < 5.0 {
        println!("⚠ SPARSE: Weak interactions - may cause circulation artifacts");
    } else {
        println!("❌ TOO SPARSE: Gas-like behavior - unrealistic");
    }
    
    // Step 7: Summary and recommendations
    println!("\n7. SUMMARY");
    println!("----------------------------------------");
    println!("Domain: {}×{} Å ({:.0} Å²)", domain_width, domain_height, total_domain_area);
    println!("Liquid particles: {} in {:.0} Å² liquid area", total_liquid_particles, available_liquid_area);
    println!("Liquid density: {:.4} particles/Å² ({:.1}% packing)", liquid_density_2d, packing_fraction_liquid * 100.0);
    println!("Spacing/diameter: {:.1} (liquid)", spacing_diameter_ratio_liquid);
    
    if li_count != anion_count {
        println!("⚠ CHARGE IMBALANCE: Li⁺ ≠ Anions");
    }
    
    if spacing_diameter_ratio_liquid < 1.2 {
        println!("💡 RECOMMENDATION: Reduce particle count or increase domain size");
    } else if spacing_diameter_ratio_liquid > 3.0 {
        println!("💡 RECOMMENDATION: Increase particle count or reduce domain size");
    } else {
        println!("✅ CONFIGURATION LOOKS GOOD");
    }
    
    Ok(())
}
