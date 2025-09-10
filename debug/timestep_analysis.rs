fn main() {
    let old_dt = 0.015;  // fs
    let new_dt = 1.0;    // fs
    let dt_ratio = new_dt / old_dt;
    
    println!("Timestep scaling analysis:");
    println!("Old timestep: {} fs", old_dt);
    println!("New timestep: {} fs", new_dt);
    println!("Ratio (new/old): {:.1}x", dt_ratio);
    
    // With Verlet integration: x += v*dt, v += a*dt
    // For same motion per real-time, acceleration needs to scale by dt_ratio²
    let motion_scaling = dt_ratio * dt_ratio;
    println!("Motion scaling factor: {:.1}x", motion_scaling);
    
    // Your empirical scaling was ~28,800x
    let empirical_scaling = 28800.0;
    let new_expected_scaling = empirical_scaling / motion_scaling;
    
    println!("\nCoulomb constant scaling:");
    println!("Previous empirical scaling: {:.0}x", empirical_scaling);
    println!("Expected new scaling: {:.0}x", new_expected_scaling);
    println!("Theoretical value: 0.139");
    println!("Expected working value: {:.1}", 0.139 * new_expected_scaling);
}
