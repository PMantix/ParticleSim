/// Simple force balance calculation
fn main() {
    // Constants from our simulation (calculated values)
    let k_b_sim = 8.314463e-7; // sim_energy/K
    let coulomb_constant = 0.139; // sim_energy⋅Å/e²
    
    // Test different temperatures
    let temperatures = [50.0, 100.0, 150.0, 200.0, 293.15];
    
    println!("Temperature (K) | Thermal Energy | Binding at 3Å | Ratio E_bind/k_B*T");
    println!("----------------|----------------|----------------|------------------");
    
    for temp in temperatures {
        let thermal_energy = k_b_sim * temp;
        
        // Ion pair binding energy at typical distance
        let distance = 3.0; // Å
        let binding_energy = coulomb_constant * 1.0 * 1.0 / distance; // |q1 * q2| = 1
        
        let ratio = binding_energy / thermal_energy;
        
        println!("{:11.1}     | {:12.6} | {:12.6} | {:16.2}", 
                temp, thermal_energy, binding_energy, ratio);
    }
    
    println!("\nFor solvation shells to form:");
    println!("- Ratio should be > 1.0 (binding stronger than thermal)");
    println!("- Ratio 1-3: stable with thermal fluctuations");
    println!("- Ratio > 3: very stable structures");
    
    println!("\nRecommendations:");
    println!("✅ T = 150K gives ratio = {:.1} (good balance)", 
             (coulomb_constant / 3.0) / (k_b_sim * 150.0));
    println!("⚠️  T = 293K gives ratio = {:.1} (too thermal)", 
             (coulomb_constant / 3.0) / (k_b_sim * 293.15));
}
