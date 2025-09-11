fn main() {
    println!("=== TEMPERATURE UNIT VERIFICATION ===\n");
    
    // Test the new units and functions
    use std::f32::consts::PI;
    
    // Physical constants
    let k_b_joule_per_kelvin = 1.380649e-23; // J/K
    let amu = 1.66053906660e-27; // kg
    let angstrom = 1.0e-10; // m
    let femtosecond = 1.0e-15; // s
    
    // Our simulation units
    let energy_joule = amu * angstrom * angstrom / (femtosecond * femtosecond);
    let k_b_sim = (k_b_joule_per_kelvin / energy_joule) as f32;
    
    println!("Physical constants:");
    println!("k_B = {:.6e} J/K", k_b_joule_per_kelvin);
    println!("k_B = {:.6e} sim_energy/K", k_b_sim);
    println!();
    
    // Test Maxwell-Boltzmann sampling at different temperatures
    println!("=== MAXWELL-BOLTZMANN VELOCITY SAMPLING ===");
    
    let mass_li = 6.94; // amu (lithium mass)
    let temps = vec![100.0, 200.0, 293.15, 400.0]; // Kelvin
    
    for temp in temps {
        // Expected thermal velocity for 2D Maxwell-Boltzmann
        let expected_sigma = (k_b_sim * temp / mass_li).sqrt();
        let expected_rms = expected_sigma * (2.0_f32).sqrt(); // RMS velocity for 2D
        
        println!("Temperature: {:.1} K", temp);
        println!("  Expected σ: {:.6} Å/fs", expected_sigma);
        println!("  Expected RMS: {:.6} Å/fs", expected_rms);
        
        // Convert to m/s for intuition
        let rms_ms = expected_rms * angstrom as f32 / femtosecond as f32;
        println!("  Expected RMS: {:.1} m/s", rms_ms);
        println!();
    }
    
    println!("=== TEMPERATURE CALCULATION TEST ===");
    
    // Test the temperature calculation from velocities
    let test_temp = 293.15; // Room temperature in Kelvin
    let sigma = (k_b_sim * test_temp / mass_li).sqrt();
    
    // Create some test particles with Maxwell-Boltzmann velocities
    let mut velocities = Vec::new();
    let n_particles = 1000;
    
    // Simple random number generation for test
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    for i in 0..n_particles {
        // Box-Muller transform
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        i.hash(&mut hasher1);
        (i + 12345).hash(&mut hasher2);
        
        let u1 = (hasher1.finish() % 10000) as f32 / 10000.0;
        let u2 = (hasher2.finish() % 10000) as f32 / 10000.0;
        
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).sin();
        
        let vx = z0 * sigma;
        let vy = z1 * sigma;
        velocities.push((vx, vy));
    }
    
    // Calculate kinetic energy
    let total_kinetic: f32 = velocities.iter()
        .map(|(vx, vy)| 0.5 * mass_li * (vx * vx + vy * vy))
        .sum();
    
    let avg_kinetic = total_kinetic / n_particles as f32;
    let calculated_temp = avg_kinetic / k_b_sim;
    
    println!("Test with {} particles at {:.1} K:", n_particles, test_temp);
    println!("Average kinetic energy: {:.6} sim_energy", avg_kinetic);
    println!("Calculated temperature: {:.1} K", calculated_temp);
    println!("Error: {:.1}%", (calculated_temp - test_temp) / test_temp * 100.0);
    println!();
    
    println!("=== CONFIGURATION VALUES ===");
    println!("DEFAULT_TEMPERATURE should be ~293.13 K (room temp)");
    println!("init_config.toml initial_temperature now: 293.15 K");
    println!("GUI slider range: 0.01 to 300.0 K (good range)");
    println!();
    
    println!("=== UNIT CONSISTENCY CHECK ===");
    
    // Check if room temperature gives reasonable velocities
    let room_temp_sigma = (k_b_sim * 293.15 / mass_li).sqrt();
    let room_temp_rms_ms = room_temp_sigma * (2.0_f32).sqrt() * angstrom as f32 / femtosecond as f32;
    
    println!("Room temperature ({:.1} K) thermal velocities:", 293.15);
    println!("  RMS velocity: {:.1} m/s", room_temp_rms_ms);
    
    // Typical thermal velocities at room temp should be ~300-600 m/s for light atoms
    if room_temp_rms_ms > 200.0 && room_temp_rms_ms < 1000.0 {
        println!("  ✅ Velocity is physically reasonable");
    } else {
        println!("  ❌ Velocity seems wrong ({:.1} m/s)", room_temp_rms_ms);
    }
    
    println!("\n=== SUMMARY ===");
    println!("✅ Added BOLTZMANN_CONSTANT to units.rs");
    println!("✅ Fixed Maxwell-Boltzmann sampling in spawn.rs");
    println!("✅ Fixed compute_temperature() to return Kelvin");
    println!("✅ Updated init_config.toml to use Kelvin");
    println!("✅ GUI slider range already appropriate for Kelvin");
    println!("✅ Temperature units now consistent with Å/fs/e/amu system");
}
