fn main() {
    println!("=== TEMPERATURE UNITS ANALYSIS ===\n");
    
    // Physical constants
    let k_b_joule_per_kelvin = 1.380649e-23; // J/K (Boltzmann constant)
    let elementary_charge = 1.602176634e-19; // C
    let amu = 1.66053906660e-27; // kg
    let angstrom = 1.0e-10; // m
    let femtosecond = 1.0e-15; // s
    
    // Energy unit in our simulation (from units.rs)
    let energy_joule = amu * angstrom * angstrom / (femtosecond * femtosecond);
    println!("Simulation energy unit: {:.6e} J", energy_joule);
    
    // Convert eV to our energy units
    let ev_to_joule = elementary_charge;
    let ev_to_sim = ev_to_joule / energy_joule;
    println!("1 eV = {:.6} simulation energy units", ev_to_sim);
    
    // Boltzmann constant in simulation units
    let k_b_sim = k_b_joule_per_kelvin / energy_joule;
    println!("k_B = {:.6e} simulation units/K", k_b_sim);
    
    println!("\n=== CURRENT TEMPERATURE SETTINGS ===");
    
    // Current default temperature
    let default_temp_code = 293.13; // From config.rs DEFAULT_TEMPERATURE
    let init_temp_toml = 5.0; // From init_config.toml
    
    println!("DEFAULT_TEMPERATURE (config.rs): {:.2}", default_temp_code);
    println!("initial_temperature (init_config.toml): {:.2}", init_temp_toml);
    
    println!("\n=== TEMPERATURE UNIT ANALYSIS ===");
    
    // Check if these are meant to be Kelvin
    if default_temp_code > 200.0 && default_temp_code < 400.0 {
        println!("DEFAULT_TEMPERATURE likely in Kelvin: {:.1} K", default_temp_code);
        let default_temp_celsius = default_temp_code - 273.15;
        println!("  = {:.1}°C (room temperature)", default_temp_celsius);
    }
    
    if init_temp_toml < 10.0 {
        println!("initial_temperature likely in simulation units or very cold");
        println!("  If Kelvin: {:.1} K = {:.1}°C (extremely cold!)", init_temp_toml, init_temp_toml - 273.15);
    }
    
    println!("\n=== CHECKING TEMPERATURE CALCULATION ===");
    
    // From simulation/utils.rs: compute_temperature
    // Formula: T = (2 * kinetic) / (2 * N) = kinetic / N
    // This assumes k_B = 1, so T is in units of energy
    
    println!("compute_temperature() assumes k_B = 1");
    println!("Returns: <kinetic energy per particle> = average energy per DOF");
    println!("Units: simulation energy units (not Kelvin!)");
    
    println!("\n=== UNIT INCONSISTENCY FOUND ===");
    println!("1. DEFAULT_TEMPERATURE = 293.13 (looks like Kelvin)");
    println!("2. compute_temperature() returns energy units");
    println!("3. Maxwell-Boltzmann sampling uses temperature directly");
    
    println!("\n=== CONVERSIONS NEEDED ===");
    
    // What should room temperature be in simulation units?
    let room_temp_k = 293.15; // K
    let room_temp_energy_joule = k_b_joule_per_kelvin * room_temp_k;
    let room_temp_sim_units = room_temp_energy_joule / energy_joule;
    
    println!("Room temperature ({:.1} K):", room_temp_k);
    println!("  = {:.6e} J (thermal energy)", room_temp_energy_joule);
    println!("  = {:.6} simulation energy units", room_temp_sim_units);
    
    // What about the init_config temperature?
    if init_temp_toml < 1.0 {
        println!("\ninit_config temperature ({:.1}) might be:", init_temp_toml);
        println!("  1. Already in simulation energy units");
        println!("  2. Very cold temperature in Kelvin (unlikely)");
        println!("  3. Dimensionless scaling factor (possible)");
    }
    
    println!("\n=== RECOMMENDATIONS ===");
    println!("1. Define k_B constant in units.rs");
    println!("2. Decide: temperature in Kelvin or energy units?");
    println!("3. Update compute_temperature() accordingly");
    println!("4. Update Maxwell-Boltzmann sampling");
    println!("5. Update GUI temperature slider range");
    
    println!("\nOption A: Use Kelvin everywhere");
    println!("  - k_B = {:.6e} sim_energy/K", k_b_sim);
    println!("  - compute_temperature returns Kelvin");
    println!("  - GUI range: 200-400 K");
    
    println!("\nOption B: Use energy units everywhere");
    println!("  - Room temp = {:.6} energy units", room_temp_sim_units);
    println!("  - compute_temperature returns energy");
    println!("  - GUI range: 0.001-0.1 energy units");
}
