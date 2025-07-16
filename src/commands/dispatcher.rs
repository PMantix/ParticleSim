use crate::renderer::state::SimCommand;
use crate::simulation::Simulation;

use super::{particle, foil, state};

/// Process a single `SimCommand` by delegating to the appropriate handler.
pub fn process_command(cmd: SimCommand, simulation: &mut Simulation) {
    match cmd {
        SimCommand::ChangeCharge { id, delta } => {
            particle::handle_change_charge(simulation, id, delta);
        }
        SimCommand::AddBody { mut body } => {
            particle::handle_add_body(simulation, &mut body);
        }
        SimCommand::DeleteAll => {
            particle::handle_delete_all(simulation);
        }
        SimCommand::DeleteSpecies { species } => {
            particle::handle_delete_species(simulation, species);
        }
        SimCommand::AddCircle { body, x, y, radius } => {
            particle::handle_add_circle(simulation, body, x, y, radius);
        }
        SimCommand::AddRing { body, x, y, radius } => {
            particle::handle_add_ring(simulation, body, x, y, radius);
        }
        SimCommand::AddRectangle { body, x, y, width, height } => {
            particle::handle_add_rectangle(simulation, body, x, y, width, height);
        }
        SimCommand::AddRandom { body, count, domain_width, domain_height } => {
            particle::handle_add_random(simulation, body, count, domain_width, domain_height);
        }
        SimCommand::SetDomainSize { width, height } => {
            particle::handle_set_domain_size(simulation, width, height);
        }
        SimCommand::AddFoil { width, height, x, y, particle_radius, current } => {
            foil::handle_add_foil(simulation, width, height, x, y, particle_radius, current);
        }
        SimCommand::SetFoilCurrent { foil_id, current } => {
            foil::handle_set_foil_current(simulation, foil_id, current);
        }
        SimCommand::SetFoilDCCurrent { foil_id, dc_current } => {
            foil::handle_set_foil_dc_current(simulation, foil_id, dc_current);
        }
        SimCommand::SetFoilACCurrent { foil_id, ac_current } => {
            foil::handle_set_foil_ac_current(simulation, foil_id, ac_current);
        }
        SimCommand::SetFoilFrequency { foil_id, switch_hz } => {
            foil::handle_set_foil_frequency(simulation, foil_id, switch_hz);
        }
        SimCommand::LinkFoils { a, b, mode } => {
            foil::handle_link_foils(simulation, a, b, mode);
        }
        SimCommand::UnlinkFoils { a, b } => {
            foil::handle_unlink_foils(simulation, a, b);
        }
        SimCommand::StepOnce => {
            state::handle_step_once(simulation);
        }
        SimCommand::SaveState { path } => {
            state::handle_save_state(simulation, path);
        }
        SimCommand::LoadState { path } => {
            state::handle_load_state(simulation, path);
        }
    }
}
