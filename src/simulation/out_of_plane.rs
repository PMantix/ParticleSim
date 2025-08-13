use crate::body::{Body, Species};
use super::Simulation;

/// Apply pseudo out-of-plane displacement for ions and anions.
///
/// The effect is limited to local crowding along the background
/// electric-field direction and integrates a damped spring in the
/// abstract Z axis. Long-range electrostatics remain unchanged.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    let field = sim.background_e_field;
    if field.mag_sq() == 0.0 {
        return;
    }
    let normal = field.normalized();
    let dt = sim.dt;
    let k = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;

    for i in 0..sim.bodies.len() {
        if !matches!(sim.bodies[i].species, Species::LithiumIon | Species::ElectrolyteAnion) {
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
            if along <= 0.0 { continue; }
            let perp = r - along * normal;
            let perp_dist = perp.mag();
            let overlap = sim.bodies[i].radius + other.radius - perp_dist;
            if overlap > 0.0 {
                sim.bodies[i].az += k * overlap;
            }
        }
        // integrate vertical motion
        let body: &mut Body = &mut sim.bodies[i];
        body.vz += (body.az - k * body.z - damping * body.vz) * dt;
        body.z += body.vz * dt;
        body.clamp_z(max_z);
    }
}
