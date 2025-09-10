fn main() {
    // Physical constants
    const ANGSTROM: f64 = 1.0e-10;
    const FEMTOSECOND: f64 = 1.0e-15;
    const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;
    const AMU: f64 = 1.660_539_066_60e-27;
    const COULOMB_CONSTANT_SI: f64 = 8.987_551_792_3e9;

    // OLD calculation (incorrect - double-counting elementary charge)
    let old_coulomb = (COULOMB_CONSTANT_SI * ELEMENTARY_CHARGE * ELEMENTARY_CHARGE * FEMTOSECOND * FEMTOSECOND / (AMU * ANGSTROM * ANGSTROM * ANGSTROM)) as f32;
    
    // NEW calculation (correct - charge unit is already elementary charge)
    let new_coulomb = (COULOMB_CONSTANT_SI * FEMTOSECOND * FEMTOSECOND / (AMU * ANGSTROM * ANGSTROM * ANGSTROM)) as f32;
    
    println!("OLD Coulomb constant: {:.6e}", old_coulomb);
    println!("NEW Coulomb constant: {:.6e}", new_coulomb);
    println!("Ratio NEW/OLD: {:.1e}", new_coulomb / old_coulomb);
    
    // Calculate energy unit for reference
    let energy_joule = AMU * ANGSTROM * ANGSTROM / (FEMTOSECOND * FEMTOSECOND);
    
    // Energy between two unit charges at 1Å with new constant
    let energy_1a_new = new_coulomb * 1.0 * 1.0 / 1.0; // k*q1*q2/r
    let energy_1a_ev = energy_1a_new as f64 * energy_joule / ELEMENTARY_CHARGE;
    
    println!("\nWith NEW constant:");
    println!("Energy between q=+1e and q=+1e at 1Å: {:.3e} sim units", energy_1a_new);
    println!("That equals: {:.3e} eV", energy_1a_ev);
    println!("Expected in vacuum: ~14.4 eV");
    
    // Check if this makes more sense for electrolyte
    println!("\nRealistic comparison:");
    println!("New value {:.1e} × (dielectric ≈ 20) = effective {:.1e} for organic electrolyte", new_coulomb, new_coulomb / 20.0);
}
