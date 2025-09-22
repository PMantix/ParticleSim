#![allow(dead_code)] // Public API function that may be used by other systems

use crate::renderer::state::SimCommand;
use crate::simulation::Simulation;
use crate::app::command_loop::handle_command;

/// Process a single `SimCommand` by delegating to the main command loop.
pub fn process_command(cmd: SimCommand, simulation: &mut Simulation) {
    handle_command(cmd, simulation);
}
