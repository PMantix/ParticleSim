/// Debug tool to verify DOE measurement accuracy
/// Loads a scenario, performs measurements, and prints detailed results
use particle_sim::doe::DoeConfig;
use particle_sim::simulation::Simulation;
use particle_sim::body::Species;

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  DOE Measurement Debug Tool                            ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    // Load DOE config
    let config = match DoeConfig::from_file("switch_charging_study.toml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to load config: {}", e);
            return;
        }
    };
    
    println!("✓ Loaded DOE config: {}", config.study_name);
    println!("  Base scenario: {}", config.base_scenario);
    println!("  Measurement points: {}\n", config.measurements.len());
    
    // Create simulation
    let mut sim = Simulation::new();
    
    // Load scenario
    let scenario_name = &config.base_scenario;
    let extensions = ["bin.gz", "json", "bin"];
    let mut loaded = false;
    
    for ext in &extensions {
        let state_path = format!("saved_state/{}.{}", scenario_name, ext);
        if std::path::Path::new(&state_path).exists() {
            match particle_sim::io::load_state(&state_path) {
                Ok(state) => {
                    sim.load_state(state);
                    println!("✓ Loaded scenario from: {}", state_path);
                    loaded = true;
                    break;
                }
                Err(e) => {
                    println!("⚠️  Failed to load {}: {}", state_path, e);
                }
            }
        }
    }
    
    if !loaded {
        println!("⚠️  Scenario not found, using empty simulation");
        return;
    }
    
    println!("  Total particles: {}", sim.bodies.len());
    println!("  Foils: {}\n", sim.foils.len());
    
    // Count species
    let mut li_metal = 0;
    let mut li_ion = 0;
    let mut foil_metal = 0;
    let mut anion = 0;
    let mut ec = 0;
    let mut dmc = 0;
    
    for body in &sim.bodies {
        match body.species {
            Species::LithiumMetal => li_metal += 1,
            Species::LithiumIon => li_ion += 1,
            Species::FoilMetal => foil_metal += 1,
            Species::ElectrolyteAnion => anion += 1,
            Species::EC => ec += 1,
            Species::DMC => dmc += 1,
        }
    }
    
    println!("Species breakdown:");
    println!("  Li Metal: {}", li_metal);
    println!("  Li Ion: {}", li_ion);
    println!("  Foil Metal: {}", foil_metal);
    println!("  Anion: {}", anion);
    println!("  EC: {}", ec);
    println!("  DMC: {}\n", dmc);
    
    // Analyze each measurement point
    println!("═══════════════════════════════════════════════════════════");
    println!("MEASUREMENT REGION ANALYSIS");
    println!("═══════════════════════════════════════════════════════════\n");
    
    for (idx, point) in config.measurements.iter().enumerate() {
        println!("─── Measurement {}: {} ───", idx + 1, point.label);
        println!("  Position: ({:.1}, {:.1})", point.x, point.y);
        println!("  Direction: {}", point.direction);
        println!("  Width: {} Å", point.width_ang);
        
        // Calculate measurement region bounds
        // Use 50Å vertical height for independent regions
        // Scan horizontally using width_ang as maximum search distance
        let vertical_height = 50.0;  // Independent regions with 50Å height
        let half_height = vertical_height / 2.0;  // ±25Å
        let (x_min, x_max, y_min, y_max) = match point.direction.as_str() {
            "left" => (
                point.x - point.width_ang,  // Scan up to width_ang to the left
                point.x,
                point.y - half_height,      // ±25Å vertically centered at point.y
                point.y + half_height,
            ),
            "right" => (
                point.x,
                point.x + point.width_ang,  // Scan up to width_ang to the right
                point.y - half_height,      // ±25Å vertically centered at point.y
                point.y + half_height,
            ),
            "up" => (
                point.x - half_height,      // ±25Å horizontally centered at point.x
                point.x + half_height,
                point.y,
                point.y + point.width_ang,  // Scan upward
            ),
            "down" => (
                point.x - half_height,      // ±25Å horizontally centered at point.x
                point.x + half_height,
                point.y - point.width_ang,  // Scan downward
                point.y,
            ),
            _ => (point.x - point.width_ang/2.0, point.x + point.width_ang/2.0, point.y - half_height, point.y + half_height),
        };
        
        println!("  Region bounds:");
        println!("    X: [{:.2}, {:.2}]", x_min, x_max);
        println!("    Y: [{:.2}, {:.2}]", y_min, y_max);
        
        // Find particles in region
        let mut li_metal_particles = Vec::new();
        let mut li_ion_particles = Vec::new();
        
        for body in &sim.bodies {
            let pos = body.pos;
            
            if pos.x >= x_min && pos.x <= x_max && pos.y >= y_min && pos.y <= y_max {
                match body.species {
                    Species::LithiumMetal => {
                        li_metal_particles.push((pos.x, pos.y, body.charge));
                    }
                    Species::LithiumIon => {
                        li_ion_particles.push((pos.x, pos.y, body.charge));
                    }
                    _ => {}
                }
            }
        }
        
        println!("\n  Particles found in region:");
        println!("    Li Metal particles: {}", li_metal_particles.len());
        println!("    Li Ion particles: {}", li_ion_particles.len());
        
        if !li_metal_particles.is_empty() {
            // Find edge position
            let edge_position = match point.direction.as_str() {
                "left" => {
                    let min_x = li_metal_particles.iter()
                        .map(|(x, _, _)| *x)
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    println!("\n  Edge detection (leftmost particle):");
                    println!("    Leading edge at X = {:.3} Å", min_x);
                    
                    // Show the 5 leftmost particles
                    let mut sorted = li_metal_particles.clone();
                    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    println!("    5 leftmost Li metal particles:");
                    for (i, (x, y, charge)) in sorted.iter().take(5).enumerate() {
                        println!("      {}: x={:.3}, y={:.3}, charge={}", i+1, x, y, charge);
                    }
                    min_x
                }
                "right" => {
                    let max_x = li_metal_particles.iter()
                        .map(|(x, _, _)| *x)
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    println!("\n  Edge detection (rightmost particle):");
                    println!("    Leading edge at X = {:.3} Å", max_x);
                    
                    let mut sorted = li_metal_particles.clone();
                    sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                    println!("    5 rightmost Li metal particles:");
                    for (i, (x, y, charge)) in sorted.iter().take(5).enumerate() {
                        println!("      {}: x={:.3}, y={:.3}, charge={}", i+1, x, y, charge);
                    }
                    max_x
                }
                "up" => {
                    let max_y = li_metal_particles.iter()
                        .map(|(_, y, _)| *y)
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    println!("\n  Edge detection (topmost particle):");
                    println!("    Leading edge at Y = {:.3} Å", max_y);
                    max_y
                }
                "down" => {
                    let min_y = li_metal_particles.iter()
                        .map(|(_, y, _)| *y)
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    println!("\n  Edge detection (bottommost particle):");
                    println!("    Leading edge at Y = {:.3} Å", min_y);
                    min_y
                }
                _ => point.x,
            };
            
            println!("    Distance from reference point: {:.3} Å", edge_position - point.x);
        } else {
            println!("\n  ⚠️  No Li metal particles found in region!");
            println!("    Edge position will default to reference point (x={:.1})", point.x);
        }
        
        println!();
    }
    
    println!("═══════════════════════════════════════════════════════════");
    println!("\nDEBUG COMPLETE\n");
    println!("To visualize these regions in the GUI:");
    println!("  1. Open the main simulator (cargo run --release)");
    println!("  2. Load the '{}' scenario", config.base_scenario);
    println!("  3. Check these coordinates match your expectations");
    println!("  4. Look for Li metal particles in the defined regions\n");
}
