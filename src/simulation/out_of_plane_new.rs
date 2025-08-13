use crate::body::Species;
use super::Simulation;
use ultraviolet::Vec2;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

// Store previous motion state for frustrated motion detection
static MOTION_HISTORY: Lazy<Mutex<HashMap<u64, MotionState>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

#[derive(Clone, Copy, Debug)]
struct MotionState {
    prev_pos: Vec2,
    prev_vel: Vec2,
    expected_force: Vec2,
}

/// Apply coulombic force-driven out-of-plane displacement using frustrated force.
///
/// When Li+ ions experience strong electric forces but are prevented from moving
/// in the intended direction by collisions/constraints, that frustrated force
/// is redirected out-of-plane to allow "squeezing" behavior. Only Li+ can
/// originate frustrated motion; other particles respond through z-springs.
pub fn apply_out_of_plane_forces(sim: &mut Simulation, dt: f32) {
    let sim_config = crate::config::LJ_CONFIG.lock().clone();
    
    if !sim_config.enable_out_of_plane {
        return;
    }

    let max_z = sim_config.max_z;
    let z_stiffness = sim_config.z_stiffness;
    let z_damping = sim_config.z_damping;
    let z_frustration_strength = sim_config.z_frustration_strength;
    
    // Calculate frustrated motion for Li+ ions only (they can originate z-motion)
    apply_frustrated_motion_to_lithium(&mut sim.bodies, z_frustration_strength, dt);
    
    // Apply basic out-of-plane physics (spring-damper system)
    for i in 0..sim.bodies.len() {
        let body = &mut sim.bodies[i];
        
        // Apply restoring spring force (F = -kz) and damping force (F = -cv)
        let spring_force = -z_stiffness * body.z;
        let damping_force = -z_damping * body.vz;
        
        // Total z-acceleration = (spring + damping) / mass
        let z_acceleration = (spring_force + damping_force) / body.mass;
        
        // Add to existing z-acceleration (from frustrated motion if applicable)
        body.az += z_acceleration;
        
        // Integrate z-motion using semi-implicit Euler
        body.vz += body.az * dt;
        body.z += body.vz * dt;
        
        // Reset z-acceleration for next frame
        body.az = 0.0;
        
        // Enforce z-limits
        body.clamp_z(max_z);
    }
    
    // Inter-particle z-springs (Newton's 3rd law coupling)
    apply_inter_particle_z_springs(&mut sim.bodies, z_stiffness, dt);
}

/// Apply frustrated motion specifically to Li+ ions based on force-motion discrepancy
fn apply_frustrated_motion_to_lithium(bodies: &mut [crate::body::Body], frustration_strength: f32, dt: f32) {
    let mut motion_history = MOTION_HISTORY.lock();
    
    for body in bodies.iter_mut() {
        // Only Li+ can originate frustrated motion
        if body.species != Species::LithiumIon {
            continue;
        }
        
        // Calculate expected electric force on this Li+ ion
        let electric_field = compute_electric_field_at_point(bodies, body.pos, body.id);
        let expected_force = electric_field * body.charge;
        
        // Compare with previous motion state to detect frustration
        if let Some(prev_state) = motion_history.get(&body.id) {
            // Force-based frustration: compare expected vs. achieved force
            let force_discrepancy = expected_force - prev_state.expected_force;
            let actual_motion = (body.pos - prev_state.prev_pos) / dt;
            let expected_motion = prev_state.expected_force * dt / body.mass;
            
            let motion_discrepancy = expected_motion - actual_motion;
            let frustrated_force_magnitude = motion_discrepancy.mag() * body.mass / dt;
            
            // If there's significant frustrated force, redirect portion to z-axis
            if frustrated_force_magnitude > 0.1 {
                let frustrated_z_force = frustrated_force_magnitude * frustration_strength;
                
                // Choose z-direction based on electric field gradient
                let z_direction = if expected_force.x > 0.0 { 1.0 } else { -1.0 };
                
                // Apply frustrated force to z-acceleration
                body.az += frustrated_z_force * z_direction / body.mass;
            }
        }
        
        // Update motion history for this Li+ ion
        motion_history.insert(body.id, MotionState {
            prev_pos: body.pos,
            prev_vel: body.vel,
            expected_force,
        });
    }
}

/// Apply inter-particle z-springs for Newton's 3rd law coupling
fn apply_inter_particle_z_springs(bodies: &mut [crate::body::Body], k: f32, dt: f32) {
    let n = bodies.len();
    let mut z_forces = vec![0.0; n];
    
    // Calculate z-spring forces between all particle pairs
    for i in 0..n {
        for j in (i + 1)..n {
            let r_xy = (bodies[j].pos - bodies[i].pos).mag();
            
            // Only apply z-coupling for nearby particles
            if r_xy < 3.0 {
                let dz = bodies[j].z - bodies[i].z;
                let spring_force = k * dz / (r_xy + 1.0); // Distance-weighted coupling
                
                // Newton's 3rd law: equal and opposite forces
                z_forces[i] += spring_force;
                z_forces[j] -= spring_force;
            }
        }
    }
    
    // Apply calculated z-forces
    for (i, body) in bodies.iter_mut().enumerate() {
        body.az += z_forces[i] / body.mass;
    }
}

fn compute_electric_field_at_point(
    bodies: &[crate::body::Body], 
    pos: Vec2, 
    exclude_id: u64
) -> Vec2 {
    use crate::renderer::draw::compute_field_at_point;
    
    // Use the global config for field calculation
    let sim_config = crate::config::LJ_CONFIG.lock().clone();
    
    // Filter out the excluded particle for self-field calculation
    let filtered_bodies: Vec<_> = bodies.iter()
        .filter(|b| b.id != exclude_id)
        .cloned()
        .collect();
    
    compute_field_at_point(&filtered_bodies, pos, &sim_config)
}

/// Check if a particle is currently experiencing frustrated motion
pub fn is_particle_frustrated(body_id: u64) -> bool {
    MOTION_HISTORY.lock()
        .get(&body_id)
        .map(|state| {
            // Consider a particle frustrated if it has significant expected force
            let force_mag = state.expected_force.mag();
            force_mag > 0.1 // Simple threshold - could be made configurable
        })
        .unwrap_or(false)
}
