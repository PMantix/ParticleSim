use rayon::prelude::*;

use super::simulation::Simulation;

/// Apply out-of-plane forces to all bodies.
/// Adds z-direction spring and damping forces and optional frustration
/// redirection based on in-plane acceleration.
pub fn apply_out_of_plane(sim: &mut Simulation) {
    if !sim.config.enable_out_of_plane {
        // If disabled, reset all z-components to keep simulation strictly 2D
        sim.bodies.par_iter_mut().for_each(|b| b.reset_z());
        return;
    }

    let stiffness = sim.config.z_stiffness;
    let damping = sim.config.z_damping;
    let max_z = sim.config.max_z;
    let frustration = sim.config.z_frustration_strength;

    sim.bodies.par_iter_mut().for_each(|body| {
        // Frustration redirects in-plane acceleration into the z-axis
        let frustration_force = body.acc.mag() * frustration;
        body.az += -stiffness * body.z - damping * body.vz + frustration_force;
        body.clamp_z(max_z);
    });
}
