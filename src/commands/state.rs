#![allow(dead_code)] // Public API functions that may be used by other systems

use crate::io::{load_state, save_state};
use crate::profile_scope;
use crate::renderer::state::PAUSED;
use crate::simulation::Simulation;

#[cfg(feature = "profiling")]
use crate::PROFILER;

/// Manually step the simulation once and render.
pub fn handle_step_once(simulation: &mut Simulation) {
    simulation.step();
    crate::renderer_utils::render(simulation);
    #[cfg(feature = "profiling")]
    {
        PROFILER.lock().print_and_clear(Some(simulation), None);
    }
    PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Save the simulation state to disk.
pub fn handle_save_state(simulation: &Simulation, path: String) {
    if let Err(e) = save_state(path, simulation) {
        eprintln!("Failed to save state: {}", e);
    }
}

/// Load the simulation state from disk.
pub fn handle_load_state(simulation: &mut Simulation, path: String) {
    match load_state(path) {
        Ok(state) => {
            simulation.load_state(state);
            PAUSED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Err(e) => eprintln!("Failed to load state: {}", e),
    }
}

/// Configure pseudo out-of-plane motion parameters at runtime.
pub fn handle_set_out_of_plane(
    simulation: &mut Simulation,
    enabled: bool,
    max_z: f32,
    z_stiffness: f32,
    z_damping: f32,
) {
    profile_scope!("config_update");
    let mut cfg = crate::config::LJ_CONFIG.lock();
    cfg.enable_out_of_plane = enabled;
    cfg.max_z = max_z;
    cfg.z_stiffness = z_stiffness;
    cfg.z_damping = z_damping;

    // Update simulation domain depth
    simulation.domain_depth = max_z;

    if !enabled {
        simulation.bodies.iter_mut().for_each(|b| b.reset_z());
    } else {
        simulation.bodies.iter_mut().for_each(|b| b.clamp_z(max_z));
    }
}
