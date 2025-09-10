fn main() {
    // Physical constants
    const ANGSTROM: f64 = 1.0e-10;
    const FEMTOSECOND: f64 = 1.0e-15;
    const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;
    const AMU: f64 = 1.660_539_066_60e-27;
    const COULOMB_CONSTANT_SI: f64 = 8.987_551_792_3e9;

    // Calculate energy unit
    let energy_joule = AMU * ANGSTROM * ANGSTROM / (FEMTOSECOND * FEMTOSECOND);
    println!("Energy unit in Joules: {:.6e}", energy_joule);

    // Calculate Coulomb constant in simulation units
    let coulomb_sim = (COULOMB_CONSTANT_SI * ELEMENTARY_CHARGE * ELEMENTARY_CHARGE * FEMTOSECOND * FEMTOSECOND) / (AMU * ANGSTROM * ANGSTROM * ANGSTROM);
    println!("Coulomb constant in sim units: {:.6e}", coulomb_sim);
    println!("Coulomb constant as f32: {:.6}", coulomb_sim as f32);

    // For comparison, what should k*e²/r be for typical distances?
    // At 1 Angstrom separation between two elementary charges:
    let force_1a = coulomb_sim * 1.0 * 1.0 / (1.0 * 1.0);
    println!("Force between two e charges at 1Å: {:.6} sim units", force_1a);

    // Convert to eV/Å for intuition
    let ev_per_angstrom = force_1a * energy_joule * ANGSTROM / ELEMENTARY_CHARGE;
    println!("That is {:.3} eV/Å", ev_per_angstrom);
    println!("Energy at 1Å: {:.3} sim units = {:.3} eV", coulomb_sim, coulomb_sim * energy_joule / ELEMENTARY_CHARGE);
    
    // Check what's actually in the units module
    println!("\nFrom units.rs calculation:");
    let units_coulomb = (8.987_551_792_3e9 * ELEMENTARY_CHARGE * ELEMENTARY_CHARGE * FEMTOSECOND * FEMTOSECOND / (AMU * ANGSTROM * ANGSTROM * ANGSTROM)) as f32;
    println!("units::COULOMB_CONSTANT = {:.6}", units_coulomb);
    
    // Typical values for reference
    println!("\nPhysical reference:");
    println!("Coulomb energy of two e charges at 1Å ≈ 14.4 eV (in vacuum)");
    println!("In water (ε≈80), this would be ≈ 0.18 eV");
    println!("In organic electrolyte (ε≈10-30), this would be ≈ 0.5-1.4 eV");
}
