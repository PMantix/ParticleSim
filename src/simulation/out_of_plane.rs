use crate::body::Species;
use super::Simulation;

/// Apply pseudo out-of-plane displacement for ions and anions using proper physics.
///
/// Uses mass-based acceleration (F=ma) with full-distance overlap detection
/// to allow lighter Li+ ions to escape crowded solvent shells more easily
/// than heavier anions and solvents.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    if !sim.config.enable_out_of_plane {
        return;
    }

    let dt = sim.dt;
    let k = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;

    // Reset accelerations
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
            continue;
        }
        sim.bodies[i].az = 0.0;
    }

    // Calculate crowding forces using full-distance overlap detection
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
            continue;
        }

        let cutoff = sim.bodies[i].radius * 4.0; // Larger search radius for better neighbor detection
        let neighbors = sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff);
        
        for j in neighbors {
            if j == i { continue; }
            
            let other = &sim.bodies[j];
            let r = other.pos - sim.bodies[i].pos;
            let distance = r.mag();
            let overlap = sim.bodies[i].radius + other.radius - distance;
            
            if overlap > 0.0 {
                // Apply force proportional to overlap
                let force = k * overlap;
                
                // F = ma, so acceleration = force / mass
                // Lighter particles (Li+) accelerate more for same force
                sim.bodies[i].az += force / sim.bodies[i].mass;
            }
        }
    }

    // Integrate z-motion with spring-damper dynamics
    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
            continue;
        }

        let body = &mut sim.bodies[i];
        
        // Spring force (restoring to z=0) and damping force
        let spring_force = -k * body.z;
        let damping_force = -damping * body.vz;
        
        // Total acceleration = crowding + spring + damping (all divided by mass for proper physics)
        let total_acceleration = body.az + (spring_force + damping_force) / body.mass;
        
        // Integrate velocity and position
        body.vz += total_acceleration * dt;
        body.z += body.vz * dt;
        
        // Clamp to maximum z-displacement
        body.clamp_z(max_z);
    }
}
