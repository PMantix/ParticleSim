use std::f32::consts::PI;

// Reproduce the quadtree force calculation (magnitude only)
fn calculate_force_current(q1: f32, q2: f32, r: f32, k_e: f32, epsilon: f32) -> f32 {
    let r_eff = r;
    let e_sq = epsilon * epsilon;
    let denom = (r_eff * r_eff + e_sq) * r_eff;
    (k_e * q1 * q2 / denom).abs()
}

// Correct Coulomb force (magnitude only)
fn calculate_force_correct(q1: f32, q2: f32, r: f32, k_e: f32, epsilon: f32) -> f32 {
    let e_sq = epsilon * epsilon;
    let denom = (r * r + e_sq).powf(1.5);
    (k_e * q1 * q2 / denom).abs()
}

fn main() {
    let k_e = 0.138935; // From units.rs
    let epsilon = 2.0;  // QUADTREE_EPSILON
    let q1 = 1.0;       // +1e charge
    let q2 = 1.0;       // +1e charge (repulsive)
    
    println!("Coulomb force comparison (magnitude):");
    println!("k_e = {}, epsilon = {}", k_e, epsilon);
    println!();
    
    for r in [0.5, 1.0, 2.0, 5.0] {
        let force_current = calculate_force_current(q1, q2, r, k_e, epsilon);
        let force_correct = calculate_force_correct(q1, q2, r, k_e, epsilon);
        let ratio = force_correct / force_current;
        
        println!("r = {} Å:", r);
        println!("  Current formula: {:.6}", force_current);
        println!("  Correct formula: {:.6}", force_correct);  
        println!("  Ratio (correct/current): {:.3}", ratio);
        println!();
    }
    
    // Check what's happening with the denominator
    println!("Denominator analysis:");
    for r in [1.0, 2.0, 5.0] {
        let current_denom = (r * r + epsilon * epsilon) * r;
        let correct_denom = (r * r + epsilon * epsilon).powf(1.5);
        println!("r = {}: current = {:.3}, correct = {:.3}, ratio = {:.3}", 
                 r, current_denom, correct_denom, correct_denom / current_denom);
    }
    
    // What scaling factor would make current formula match empirical need of ~4000?
    let empirical_k = 4000.0;
    let theoretical_k = 0.138935;
    let empirical_scaling = empirical_k / theoretical_k;
    println!("\nEmpirical scaling factor needed: {:.0}", empirical_scaling);
    
    // Test at typical distance
    let r_test = 2.0;
    let force_theoretical = calculate_force_current(q1, q2, r_test, theoretical_k, epsilon);
    let force_empirical = calculate_force_current(q1, q2, r_test, empirical_k, epsilon);
    println!("At r=2Å with theoretical k: {:.6}", force_theoretical);
    println!("At r=2Å with empirical k: {:.6}", force_empirical);
    println!("Empirical/theoretical ratio: {:.0}", force_empirical / force_theoretical);
}
