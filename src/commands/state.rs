use crate::simulation::Simulation;
use crate::renderer::state::PAUSED;
use crate::io::{save_state, load_state};

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
        Ok(state) => state.apply_to(simulation),
        Err(e) => eprintln!("Failed to load state: {}", e),
    }
}
