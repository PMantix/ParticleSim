fn main() {
    println!("=== FORCE BALANCE ANALYSIS ===\n");
    
    // Physical constants from our simulation
    let k_b_sim = 8.314463e-7; // sim_energy/K from units.rs
    let coulomb_constant = 0.139; // from units.rs, should be 14.4 eV⋅Å/e²
    
    // Typical simulation parameters
    let temperature = 293.15; // K (room temperature)
    let mass_li = 6.94; // amu (lithium)
    let mass_pf6 = 144.96; // amu (PF6 anion)
    let charge_li = 1.0; // +1e
    let charge_pf6 = -1.0; // -1e
    
    println!("=== THERMAL ENERGY ANALYSIS ===");
    let thermal_energy = k_b_sim * temperature;
    println!("Thermal energy (k_B * T): {:.6} sim_energy", thermal_energy);
    println!("Thermal energy: {:.3} eV", thermal_energy * 103.6); // Convert to eV using 1 sim_energy ≈ 103.6 eV
    
    // Thermal velocity magnitudes
    let v_thermal_li = (k_b_sim * temperature / mass_li).sqrt();
    let v_thermal_pf6 = (k_b_sim * temperature / mass_pf6).sqrt();
    
    println!("\nThermal velocities (RMS):");
    println!("Li+: {:.6} Å/fs = {:.1} m/s", v_thermal_li, v_thermal_li * 1e5);
    println!("PF6-: {:.6} Å/fs = {:.1} m/s", v_thermal_pf6, v_thermal_pf6 * 1e5);
    
    println!("\n=== ELECTROSTATIC FORCE ANALYSIS ===");
    
    // Test different separation distances
    let distances = vec![2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0]; // Å
    
    println!("Distance (Å) | Coulomb Force | Coulomb Energy | Thermal Ratio");
    println!("-------------|---------------|----------------|---------------");
    
    for r in distances {
        // Coulomb force: F = k * q1 * q2 / r²
        let coulomb_force = coulomb_constant * charge_li * (-charge_pf6) / (r * r);
        
        // Coulomb potential energy: U = k * q1 * q2 / r  
        let coulomb_energy = coulomb_constant * charge_li * (-charge_pf6) / r;
        
        // Ratio of electrostatic to thermal energy
        let energy_ratio = coulomb_energy.abs() / thermal_energy;
        
        println!("{:8.1}     | {:11.6} | {:12.6} | {:13.1}", 
                r, coulomb_force, coulomb_energy, energy_ratio);
    }
    
    println!("\n=== SOLVATION SHELL ANALYSIS ===");
    
    // For solvation shells to form, electrostatic binding energy should be
    // comparable to or larger than thermal energy
    println!("For stable solvation shells:");
    println!("- Electrostatic energy should be ≥ k_B*T");
    println!("- Energy ratio should be ≥ 1.0");
    println!("- Typical ion pair distances: 2-4 Å");
    
    let typical_distance = 3.0; // Å
    let binding_energy = coulomb_constant * charge_li * (-charge_pf6) / typical_distance;
    let stability_ratio = binding_energy.abs() / thermal_energy;
    
    println!("\nAt typical ion-pair distance ({:.1} Å):", typical_distance);
    println!("Binding energy: {:.6} sim_energy = {:.3} eV", binding_energy, binding_energy * 103.6);
    println!("Thermal energy: {:.6} sim_energy = {:.3} eV", thermal_energy, thermal_energy * 103.6);
    println!("Stability ratio: {:.2}", stability_ratio);
    
    if stability_ratio < 0.5 {
        println!("❌ TOO WEAK: Thermal motion dominates, no solvation shells");
    } else if stability_ratio < 1.0 {
        println!("⚠️  MARGINAL: Weak solvation, easily disrupted");
    } else if stability_ratio < 3.0 {
        println!("✅ GOOD: Stable solvation shells with thermal fluctuations");
    } else {
        println!("🔒 STRONG: Very stable, little thermal motion");
    }
    
    println!("\n=== RECOMMENDED FIXES ===");
    
    if stability_ratio < 1.0 {
        println!("Problem: Thermal energy too high relative to electrostatic forces");
        println!("\nOption 1: Reduce temperature");
        let target_temp = temperature * 0.5;
        println!("  - Try T = {:.1} K (half current temperature)", target_temp);
        
        println!("\nOption 2: Increase Coulomb constant");
        let target_coulomb = coulomb_constant * 2.0;
        println!("  - Try Coulomb = {:.3} (double current value)", target_coulomb);
        
        println!("\nOption 3: Reduce thermal noise in initial velocities");
        println!("  - Scale initial velocities by 0.5-0.7");
        
        println!("\nOption 4: Add artificial damping");
        println!("  - Increase damping_base closer to 1.0");
    }
    
    println!("\n=== CURRENT COULOMB CONSTANT CHECK ===");
    
    // Our Coulomb constant should be 14.4 eV⋅Å/e² in simulation units
    let expected_coulomb_ev_ang = 14.4; // eV⋅Å/e²
    let ev_to_sim = 0.009649; // from our earlier analysis
    let expected_coulomb_sim = expected_coulomb_ev_ang * ev_to_sim;
    
    println!("Expected Coulomb constant: {:.3} sim_units", expected_coulomb_sim);
    println!("Current Coulomb constant:  {:.3} sim_units", coulomb_constant);
    println!("Ratio (current/expected): {:.2}", coulomb_constant / expected_coulomb_sim);
    
    if (coulomb_constant / expected_coulomb_sim - 1.0).abs() > 0.1 {
        println!("⚠️  Coulomb constant may need adjustment");
    } else {
        println!("✅ Coulomb constant looks correct");
    }
}
