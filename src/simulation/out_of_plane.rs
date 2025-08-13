use crate::body::{Body, Species};
use super::Simulation;
use ultraviolet::Vec2;

/// Apply pseudo out-of-plane displacement for ions and anions.
///
/// The effect is limited to local crowding along the background
/// electric-field direction and integrates a damped spring in the
/// abstract Z axis. Long-range electrostatics remain unchanged.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    let field = sim.background_e_field;
    // Fallback direction if no external field is applied so the mechanism still activates.
    let normal: Vec2 = if field.mag_sq() == 0.0 {
        Vec2::new(1.0, 0.0) // arbitrary stable direction
    } else {
        field.normalized()
    };
    let dt = sim.dt;
    let k = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;

    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion | Species::EC | Species::DMC) {
            continue;
        }
        sim.bodies[i].az = 0.0;
        let cutoff = sim.bodies[i].radius * 2.0;
        let neighbors = sim.quadtree.find_neighbors_within(&sim.bodies, i, cutoff);
        for j in neighbors {
            if j == i { continue; }
            let other = &sim.bodies[j];
            let r = other.pos - sim.bodies[i].pos;
            let along = r.dot(normal);
            // Treat both forward and backward crowding; forward gets full weight, backward half to preserve original bias when field present.
            let perp = r - along * normal;
            let perp_dist = perp.mag();
            let overlap = sim.bodies[i].radius + other.radius - perp_dist;
            if overlap > 0.0 {
                let weight = if along >= 0.0 { 1.0 } else { 0.5 }; // maintain slight directionality
                sim.bodies[i].az += k * overlap * weight;
            }
        }
        // integrate vertical motion
        let body: &mut Body = &mut sim.bodies[i];
    // Solvents get reduced vertical stiffness to mimic greater flexibility
    let species_scale = match body.species { Species::EC | Species::DMC => 0.5, _ => 1.0 };
    body.vz += (body.az - (k * species_scale) * body.z - damping * body.vz) * dt;
        body.z += body.vz * dt;
        body.clamp_z(max_z);
    }
}
