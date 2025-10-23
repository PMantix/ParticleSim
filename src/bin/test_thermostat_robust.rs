// test_thermostat_robust.rs
// Robust thermostat test using actual default scenario and real-time debug tracking
use particle_sim::body::{Body, Species};
use particle_sim::config::SimConfig;
use particle_sim::init_config::InitConfig;
use particle_sim::simulation::Simulation;
use rand::Rng;
use ultraviolet::Vec2;

fn main() {
    println!("=== ROBUST THERMOSTAT TEST WITH REAL SCENARIO ===");
    println!(
        "This test loads the actual default scenario and tracks thermostat behavior in real-time\n"
    );

    // Load the actual default configuration
    println!("=== LOADING DEFAULT SCENARIO ===");
    let init_config = match InitConfig::load_default() {
        Ok(config) => {
            println!("✅ Successfully loaded init_config.toml");
            config
        }
        Err(e) => {
            println!("❌ Failed to load init_config.toml: {}", e);
            println!("Creating fallback scenario...");
            return;
        }
    };

    // Create simulation with real scenario configuration
    let mut sim = create_realistic_simulation(&init_config);

    println!("\n=== POPULATING SIMULATION WITH REAL PARTICLES ===");
    populate_simulation_from_config(&mut sim, &init_config);

    println!("\n=== INITIAL STATE AFTER SCENARIO LOADING ===");
    analyze_detailed_state(&sim, 0);

    if sim.bodies.is_empty() {
        println!("❌ No particles loaded! This explains the thermostat issue.");
        println!("The simulation has no bodies to thermostat.");
        return;
    }

    let solvent_count = count_solvent_particles(&sim.bodies);
    if solvent_count == 0 {
        println!("❌ No solvent particles (EC/DMC)! Thermostat has nothing to control.");
        return;
    }

    println!("✅ Found {} solvent particles to thermostat", solvent_count);

    println!("\n=== RUNNING SIMULATION ===");

    for step in 1..=20 {
        // Capture state before step
        let temp_before = calculate_detailed_temperature(&sim.bodies);
        let velocities_before = capture_velocity_stats(&sim.bodies);

        // Take simulation step
        sim.step();

        // Capture state after step
        let temp_after = calculate_detailed_temperature(&sim.bodies);
        let velocities_after = capture_velocity_stats(&sim.bodies);

        // Report detailed changes
        println!("\n--- STEP {} ---", step);
        println!(
            "Temperature: {:.3}K -> {:.3}K (Δ{:+.3}K)",
            temp_before.solvent_temp,
            temp_after.solvent_temp,
            temp_after.solvent_temp - temp_before.solvent_temp
        );

        println!("Avg velocity magnitude:");
        println!(
            "  EC:  {:.3} -> {:.3} (Δ{:+.3})",
            velocities_before.ec_avg,
            velocities_after.ec_avg,
            velocities_after.ec_avg - velocities_before.ec_avg
        );
        println!(
            "  DMC: {:.3} -> {:.3} (Δ{:+.3})",
            velocities_before.dmc_avg,
            velocities_after.dmc_avg,
            velocities_after.dmc_avg - velocities_before.dmc_avg
        );
        println!(
            "  Li+: {:.3} -> {:.3} (Δ{:+.3})",
            velocities_before.li_avg,
            velocities_after.li_avg,
            velocities_after.li_avg - velocities_before.li_avg
        );

        // Check if thermostat should have activated
        let thermostat_interval = sim.config.thermostat_interval_fs;
        let current_time = sim.frame as f32 * sim.dt;
        let should_activate = (current_time % thermostat_interval) < sim.dt;

        if should_activate {
            println!(
                "  ⚙️ Thermostat should activate at frame {} (t={:.1}fs)",
                sim.frame, current_time
            );
        } else {
            println!(
                "  ⏸️ Thermostat inactive at frame {} (t={:.1}fs)",
                sim.frame, current_time
            );
        }

        // Detect if velocities were scaled (indicating thermostat action)
        let velocity_change_ec = (velocities_after.ec_avg - velocities_before.ec_avg).abs();
        let velocity_change_dmc = (velocities_after.dmc_avg - velocities_before.dmc_avg).abs();
        let velocity_change_li = (velocities_after.li_avg - velocities_before.li_avg).abs();

        if velocity_change_ec > 0.1 || velocity_change_dmc > 0.1 {
            println!("  🔥 SIGNIFICANT VELOCITY CHANGE DETECTED - Likely thermostat action");
        }

        if velocity_change_li > 0.01 && velocity_change_li < 0.1 {
            println!("  ❌ WARNING: Li+ velocities changed but shouldn't be thermostated!");
        }

        // Temperature trending
        if temp_after.solvent_temp < temp_before.solvent_temp - 1.0 {
            println!("  📉 TEMPERATURE DROPPING - Thermostat may not be working");
        } else if temp_after.solvent_temp > temp_before.solvent_temp + 1.0 {
            println!("  📈 TEMPERATURE RISING");
        }
    }

    println!("\n=== FINAL ANALYSIS ===");
    analyze_detailed_state(&sim, 20);

    // Final verdict
    let final_temp = calculate_detailed_temperature(&sim.bodies);
    let target_temp = sim.config.temperature;
    let deviation = (final_temp.solvent_temp - target_temp).abs();

    println!("\n=== VERDICT ===");
    println!("Target temperature: {:.1}K", target_temp);
    println!("Final solvent temperature: {:.3}K", final_temp.solvent_temp);
    println!("Deviation: {:.3}K", deviation);

    if deviation < 5.0 {
        println!("✅ THERMOSTAT WORKING: Temperature maintained within 5K");
    } else if deviation < 20.0 {
        println!("⚠️ THERMOSTAT WEAK: Significant temperature deviation");
    } else {
        println!("❌ THERMOSTAT FAILED: Large temperature deviation");
    }
}

fn create_realistic_simulation(init_config: &InitConfig) -> Simulation {
    let mut config = SimConfig::default();
    config.temperature = init_config
        .simulation
        .as_ref()
        .and_then(|s| s.initial_temperature)
        .unwrap_or(300.0);
    config.thermostat_interval_fs = 5.0; // Match actual simulation

    println!("Realistic thermostat config:");
    println!("  Target temperature: {}K", config.temperature);
    println!("  Thermostat interval: {}fs", config.thermostat_interval_fs);
    println!("  Timestep: {}fs", particle_sim::config::DEFAULT_DT_FS);

    let mut sim = Simulation::new();
    sim.config = config;

    // Set domain size to match config
    if let Some(sim_config) = &init_config.simulation {
        let (width, height) = sim_config.domain_size();
        println!("  Domain size: {}×{}", width, height);
    }

    sim
}

fn populate_simulation_from_config(sim: &mut Simulation, init_config: &InitConfig) {
    let mut rng = rand::rng();
    let mut total_particles = 0;

    // Add circles from config
    for circle_config in &init_config.particles.circles {
        if let Ok(species) = circle_config.to_species() {
            let body_template = get_body_template_for_species(species);
            let radius = circle_config.radius;
            let center = Vec2::new(circle_config.x, circle_config.y);

            // Calculate number of particles to fill the circle area (scaled down for testing)
            let area = std::f32::consts::PI * radius * radius;
            let particle_area = 100.0; // Much larger area per particle for testing
            let particle_count = (area / particle_area).max(10.0) as usize;

            println!(
                "Adding {} {} particles in circle at ({}, {}) radius {}",
                particle_count,
                format!("{:?}", species),
                circle_config.x,
                circle_config.y,
                radius
            );

            for _ in 0..particle_count {
                // Random position within circle
                let angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
                let r = rng.random::<f32>().sqrt() * radius;
                let pos = center + Vec2::new(angle.cos() * r, angle.sin() * r);

                // Initialize with thermal velocity for the target temperature
                let vel = thermal_velocity(&mut rng, body_template.mass, sim.config.temperature);

                let mut body = body_template.clone();
                body.pos = pos;
                body.vel = vel;

                sim.bodies.push(body);
                total_particles += 1;
            }
        }
    }

    // Add metal rectangles from config
    for rect_config in &init_config.particles.metal_rectangles {
        if let Ok(species) = rect_config.to_species() {
            let body_template = get_body_template_for_species(species);
            let (origin_x, origin_y) = rect_config.to_origin_coords();

            // Calculate particles to fill rectangle (scaled down for testing)
            let particle_density = 0.1; // much lower density for testing
            let particle_count =
                (rect_config.width * rect_config.height * particle_density) as usize;

            println!(
                "Adding {} {} particles in rectangle {}×{} at ({}, {})",
                particle_count,
                format!("{:?}", species),
                rect_config.width,
                rect_config.height,
                rect_config.x,
                rect_config.y
            );

            for _ in 0..particle_count {
                let pos = Vec2::new(
                    origin_x + rng.random::<f32>() * rect_config.width,
                    origin_y + rng.random::<f32>() * rect_config.height,
                );

                let vel = thermal_velocity(&mut rng, body_template.mass, sim.config.temperature);

                let mut body = body_template.clone();
                body.pos = pos;
                body.vel = vel;

                sim.bodies.push(body);
                total_particles += 1;
            }
        }
    }

    // Add random particles from config
    for random_config in &init_config.particles.random {
        if let Ok(species) = random_config.to_species() {
            let body_template = get_body_template_for_species(species);

            println!(
                "Adding {} random {} particles",
                random_config.count,
                format!("{:?}", species)
            );

            for _ in 0..random_config.count {
                let width = random_config.domain_width.unwrap_or(400.0);
                let height = random_config.domain_height.unwrap_or(300.0);
                let pos = Vec2::new(
                    rng.random::<f32>() * width - width / 2.0,
                    rng.random::<f32>() * height - height / 2.0,
                );

                let vel = thermal_velocity(&mut rng, body_template.mass, sim.config.temperature);

                let mut body = body_template.clone();
                body.pos = pos;
                body.vel = vel;

                sim.bodies.push(body);
                total_particles += 1;
            }
        }
    }

    println!("✅ Total particles created: {}", total_particles);
}

fn get_body_template_for_species(species: Species) -> Body {
    match species {
        Species::EC => Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::EC),
        Species::DMC => Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 0.0, Species::DMC),
        Species::LithiumIon => Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            1.0,
            Species::LithiumIon,
        ),
        Species::ElectrolyteAnion => Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            -1.0,
            Species::ElectrolyteAnion,
        ),
        Species::LithiumMetal => Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::LithiumMetal,
        ),
        Species::FoilMetal => Body::new(
            Vec2::zero(),
            Vec2::zero(),
            1.0,
            1.0,
            0.0,
            Species::FoilMetal,
        ),
    }
}

fn thermal_velocity(rng: &mut impl Rng, mass: f32, temperature: f32) -> Vec2 {
    // Maxwell-Boltzmann velocity distribution
    // In simulation units where k_B = 1, thermal velocity scale is sqrt(T/m)
    let scale = (temperature / mass).sqrt();

    Vec2::new(
        rng.random::<f32>() * scale - scale / 2.0,
        rng.random::<f32>() * scale - scale / 2.0,
    )
}

fn count_solvent_particles(bodies: &[Body]) -> usize {
    bodies
        .iter()
        .filter(|b| matches!(b.species, Species::EC | Species::DMC))
        .count()
}

#[derive(Debug)]
struct DetailedTemp {
    total_temp: f32,
    solvent_temp: f32,
    ec_temp: f32,
    dmc_temp: f32,
    li_temp: f32,
    solvent_count: usize,
}

fn calculate_detailed_temperature(bodies: &[Body]) -> DetailedTemp {
    let mut ec_ke = 0.0;
    let mut dmc_ke = 0.0;
    let mut li_ke = 0.0;
    let mut total_ke = 0.0;

    let mut ec_count = 0;
    let mut dmc_count = 0;
    let mut li_count = 0;
    let mut total_count = 0;

    for body in bodies {
        let ke = 0.5 * body.mass * body.vel.mag_sq();
        total_ke += ke;
        total_count += 1;

        match body.species {
            Species::EC => {
                ec_ke += ke;
                ec_count += 1;
            }
            Species::DMC => {
                dmc_ke += ke;
                dmc_count += 1;
            }
            Species::LithiumIon => {
                li_ke += ke;
                li_count += 1;
            }
            _ => {}
        }
    }

    let solvent_ke = ec_ke + dmc_ke;
    let solvent_count = ec_count + dmc_count;

    DetailedTemp {
        total_temp: if total_count > 0 {
            total_ke / total_count as f32
        } else {
            0.0
        },
        solvent_temp: if solvent_count > 0 {
            solvent_ke / solvent_count as f32
        } else {
            0.0
        },
        ec_temp: if ec_count > 0 {
            ec_ke / ec_count as f32
        } else {
            0.0
        },
        dmc_temp: if dmc_count > 0 {
            dmc_ke / dmc_count as f32
        } else {
            0.0
        },
        li_temp: if li_count > 0 {
            li_ke / li_count as f32
        } else {
            0.0
        },
        solvent_count,
    }
}

#[derive(Debug)]
struct VelocityStats {
    ec_avg: f32,
    dmc_avg: f32,
    li_avg: f32,
}

fn capture_velocity_stats(bodies: &[Body]) -> VelocityStats {
    let mut ec_vel_sum = 0.0;
    let mut dmc_vel_sum = 0.0;
    let mut li_vel_sum = 0.0;

    let mut ec_count = 0;
    let mut dmc_count = 0;
    let mut li_count = 0;

    for body in bodies {
        let vel_mag = body.vel.mag();
        match body.species {
            Species::EC => {
                ec_vel_sum += vel_mag;
                ec_count += 1;
            }
            Species::DMC => {
                dmc_vel_sum += vel_mag;
                dmc_count += 1;
            }
            Species::LithiumIon => {
                li_vel_sum += vel_mag;
                li_count += 1;
            }
            _ => {}
        }
    }

    VelocityStats {
        ec_avg: if ec_count > 0 {
            ec_vel_sum / ec_count as f32
        } else {
            0.0
        },
        dmc_avg: if dmc_count > 0 {
            dmc_vel_sum / dmc_count as f32
        } else {
            0.0
        },
        li_avg: if li_count > 0 {
            li_vel_sum / li_count as f32
        } else {
            0.0
        },
    }
}

fn analyze_detailed_state(sim: &Simulation, step: u32) {
    let temp = calculate_detailed_temperature(&sim.bodies);
    let vel_stats = capture_velocity_stats(&sim.bodies);

    println!("Step {} Analysis:", step);
    println!("  Bodies: {} total", sim.bodies.len());
    println!(
        "  Frame: {}, Time: {:.1}fs",
        sim.frame,
        sim.frame as f32 * sim.dt
    );
    println!("  Temperatures:");
    println!("    Total: {:.3}K", temp.total_temp);
    println!(
        "    Solvent (EC+DMC): {:.3}K ({} particles)",
        temp.solvent_temp, temp.solvent_count
    );
    println!("    EC only: {:.3}K", temp.ec_temp);
    println!("    DMC only: {:.3}K", temp.dmc_temp);
    println!("    Li+ only: {:.3}K", temp.li_temp);
    println!("  Average velocity magnitudes:");
    println!("    EC: {:.3}", vel_stats.ec_avg);
    println!("    DMC: {:.3}", vel_stats.dmc_avg);
    println!("    Li+: {:.3}", vel_stats.li_avg);
}
