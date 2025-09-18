use crate::renderer::state::SimCommand;
use crate::simulation::Simulation;

/// Process a single `SimCommand` by delegating to the main command loop.
pub fn process_command(cmd: SimCommand, simulation: &mut Simulation) {
    crate::app::handle_command(cmd, simulation);
}
