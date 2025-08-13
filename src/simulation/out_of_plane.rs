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

// Store IDs of particles currently experiencing frustrated z-motion for debug visualization
static FRUSTRATED_PARTICLES: Lazy<Mutex<std::collections::HashSet<u64>>> = Lazy::new(|| {
    Mutex::new(std::collections::HashSet::new())
});

#[derive(Clone, Copy, Debug)]
struct MotionState {
    prev_pos: Vec2,
    prev_vel: Vec2,
    expected_force: Vec2,
}

/// Get the list of particle IDs currently experiencing frustrated z-motion
/// This is used for debug visualization
pub fn get_frustrated_particle_ids() -> std::collections::HashSet<u64> {
    FRUSTRATED_PARTICLES.lock().clone()
}

/// Apply coulombic force-driven out-of-plane displacement using frustrated force.
///
/// When Li+ ions experience strong electric forces but are prevented from moving
/// in the intended direction by collisions/constraints, that frustrated force
/// is redirected out-of-plane to allow "squeezing" behavior. Only Li+ can
/// originate frustrated motion; other particles respond through z-springs.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    if !sim.config.enable_out_of_plane {
        return;
    }

    let dt = sim.dt;
    let k = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;
    let frustration_strength = sim.config.z_frustration_strength;

    // Reset z-accelerations for all particles that can participate
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
            continue;
        }
        sim.bodies[i].az = 0.0;
    }

    let mut motion_history = MOTION_HISTORY.lock();
    let mut frustrated_particles = FRUSTRATED_PARTICLES.lock();
    frustrated_particles.clear(); // Clear previous frame's frustrated particles
    let mut z_forces = Vec::new(); // Store z-forces to apply later

    // First pass: Detect frustrated force for Li+ ions only (originators)
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon) {
            continue; // Only Li+ can originate frustrated motion
        }

        let body = &sim.bodies[i];
        
        // Calculate electric field and expected force
        let electric_field = calculate_electric_field_at_position(&sim.bodies, &sim.foils, body.pos, i);
        let field_magnitude = electric_field.mag();
        
        if field_magnitude < 1e-6 {
            continue;
        }
        
        let coulombic_force = electric_field * body.charge;
        
        // Check if we have previous motion state for this particle
        if let Some(prev_state) = motion_history.get(&body.id) {
            // Calculate actual force from motion change
            let velocity_change = body.vel - prev_state.prev_vel;
            let actual_acceleration = velocity_change / dt;
            let actual_force = actual_acceleration * body.mass;
            
            // Calculate frustrated force in the coulombic force direction
            let force_direction = coulombic_force.normalized();
            let expected_force_magnitude = coulombic_force.mag();
            let actual_force_in_direction = actual_force.dot(force_direction);
            
            // Detect force frustration (when expected force > actual force achieved)
            let frustrated_force_magnitude = (expected_force_magnitude - actual_force_in_direction).max(0.0);
            
            if frustrated_force_magnitude > 1e-6 {
                // Redirect frustrated force out-of-plane
                let z_force = frustrated_force_magnitude * frustration_strength;
                
                // Store z-force to apply later (avoid borrowing conflicts)
                z_forces.push((i, z_force / body.mass));
                
                // Mark this particle as experiencing frustrated motion for debug visualization
                frustrated_particles.insert(body.id);
            }
        }
        
    // Update motion history for next frame
        motion_history.insert(body.id, MotionState {
            prev_pos: body.pos,
            prev_vel: body.vel,
            expected_force: coulombic_force,
        });
    }

    // Apply the calculated z-forces from frustrated motion
    for (i, z_acceleration) in z_forces {
        sim.bodies[i].az += z_acceleration;
    }

    // Second pass: Apply equal and opposite z-forces between overlapping particles
    // Only Li+ and anions participate in inter-particle z-springs to prevent
    // solvents from getting indirect z-motion through Newton's 3rd law
    let mut z_force_pairs = Vec::new();
    
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion) {
            continue; // Only ions participate in inter-particle z-springs
        }

        let search_radius = sim.bodies[i].radius * 2.5;
        let neighbors = sim.quadtree.find_neighbors_within(&sim.bodies, i, search_radius);
        
        for &j in &neighbors {
            if j <= i { continue; } // Avoid double-counting pairs
            
            let other = &sim.bodies[j];
            if !matches!(other.species, Species::LithiumIon | Species::ElectrolyteAnion) {
                continue; // Only ions participate in inter-particle z-springs
            }
            
            let r = other.pos - sim.bodies[i].pos;
            let distance_2d = r.mag();
            let overlap_2d = sim.bodies[i].radius + other.radius - distance_2d;
            
            if overlap_2d > 0.0 {
                // Particles are overlapping in 2D - apply z-spring forces
                let z_separation = sim.bodies[j].z - sim.bodies[i].z;
                let desired_z_separation = overlap_2d; // Push apart in z by the overlap amount
                let z_compression = desired_z_separation - z_separation.abs();
                
                if z_compression > 0.0 {
                    // Apply spring force in z-direction
                    let z_spring_force = k * z_compression;
                    
                    // Determine which particle pushes up and which pushes down
                    let force_sign = if z_separation > 0.0 { -1.0 } else { 1.0 };
                    
                    z_force_pairs.push((i, j, force_sign * z_spring_force));
                }
            }
        }
    }

    // Apply the z-force pairs (equal and opposite forces)
    for (i, j, force) in z_force_pairs {
        // Newton's third law: equal and opposite forces
        sim.bodies[i].az += force / sim.bodies[i].mass;
        sim.bodies[j].az -= force / sim.bodies[j].mass;
    }

    // Integrate z-motion with spring-damper dynamics for ions only
    // Solvents are no longer allowed to participate in z-motion
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion) {
            continue; // Only ions can have z-motion
        }

        let body = &mut sim.bodies[i];
        
        // Spring force (restoring to z=0) and damping force
        let spring_force = -k * body.z;
        let damping_force = -damping * body.vz;
        
        // Total acceleration = coulombic z-force + inter-particle z-forces + spring + damping
        let total_acceleration = body.az + (spring_force + damping_force) / body.mass;
        
        // Integrate velocity and position
        body.vz += total_acceleration * dt;
        body.z += body.vz * dt;
        
        // Clamp to maximum z-displacement
        body.clamp_z(max_z);
    }
}

/// Calculate the electric field at a given position from all sources
fn calculate_electric_field_at_position(
    bodies: &[crate::body::Body], 
    _foils: &[crate::body::foil::Foil], 
    pos: Vec2, 
    _exclude_index: usize
) -> Vec2 {
    use crate::renderer::draw::compute_field_at_point;
    
    // Use the global config for field calculation
    let sim_config = crate::config::LJ_CONFIG.lock().clone();
    compute_field_at_point(bodies, pos, &sim_config)
}
