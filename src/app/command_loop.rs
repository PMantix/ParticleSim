use crate::io::{load_state, save_state};
use crate::renderer::state::{SimCommand, PAUSED};
use crate::simulation::Simulation;
use crate::profile_scope;
use std::sync::atomic::Ordering;

use super::spawn;

pub fn handle_command(cmd: SimCommand, simulation: &mut Simulation) {
    profile_scope!("command_handling");
    match cmd {
        SimCommand::ChangeCharge { id, delta } => {
            if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
                if delta > 0.0 {
                    for _ in 0..delta.round() as usize {
                        body.electrons.pop();
                    }
                } else if delta < 0.0 {
                    for _ in 0..(-delta).round() as usize {
                        let angle = fastrand::f32() * std::f32::consts::TAU;
                        let rel_pos = ultraviolet::Vec2::new(angle.cos(), angle.sin())
                            * body.radius
                            * body.species.polar_offset();
                        body.electrons.push(crate::body::Electron {
                            rel_pos,
                            vel: ultraviolet::Vec2::zero(),
                        });
                    }
                }
                body.update_charge_from_electrons();
                println!("Particle {} new charge: {}", id, body.charge);
                println!(
                    "Particle {} new electron count: {}",
                    id,
                    body.electrons.len()
                );
                println!("Particle {} new species: {:?}", id, body.species);
                let was_metal = body.species == crate::body::Species::LithiumMetal;
                let was_ion = body.species == crate::body::Species::LithiumIon;
                body.update_species();
                if was_metal && body.species == crate::body::Species::LithiumIon {
                    println!();
                    println!("Should become ion below...");
                    println!("Particle {} new species: {:?}", id, body.species);
                }
                if was_ion && body.species == crate::body::Species::LithiumMetal {
                    println!();
                    println!("Should become metal below...");
                    println!("Particle {} new species: {:?}", id, body.species);
                }
                println!("Particle {} new charge: {}", id, body.charge);
            }
        }
        SimCommand::AddBody { mut body } => {
            body.electrons.clear();
            if matches!(
                body.species,
                crate::body::Species::LithiumMetal
                    | crate::body::Species::ElectrolyteAnion
                    | crate::body::Species::EC
                    | crate::body::Species::DMC
            ) {
                body.electrons.push(crate::body::Electron {
                    rel_pos: ultraviolet::Vec2::zero(),
                    vel: ultraviolet::Vec2::zero(),
                });
            }
            let temp = crate::config::LJ_CONFIG.lock().temperature;
            body.vel = super::spawn::sample_velocity(body.mass, temp);
            body.update_charge_from_electrons();
            body.update_species();
            simulation.bodies.push(body);
        }
        SimCommand::DeleteAll => {
            simulation.bodies.clear();
            simulation.foils.clear();
            simulation.body_to_foil.clear();
        }
        SimCommand::DeleteSpecies { species } => {
            simulation.bodies.retain(|body| body.species != species);
            if species == crate::body::Species::FoilMetal {
                let remaining_foil_body_ids: std::collections::HashSet<u64> = simulation
                    .bodies
                    .iter()
                    .filter(|body| body.species == crate::body::Species::FoilMetal)
                    .map(|body| body.id)
                    .collect();
                simulation.foils.retain(|foil| {
                    foil.body_ids
                        .iter()
                        .any(|id| remaining_foil_body_ids.contains(id))
                });
                simulation
                    .body_to_foil
                    .retain(|body_id, _| remaining_foil_body_ids.contains(body_id));
            }
        }
        SimCommand::AddCircle { body, x, y, radius } => {
            spawn::add_circle(simulation, body, x, y, radius);
        }
        SimCommand::StepOnce => {
            simulation.step();
            super::simulation_loop::render(simulation);
            #[cfg(feature = "profiling")]
            {
                crate::PROFILER
                    .lock()
                    .print_and_clear(Some(simulation), None);
            }
            PAUSED.store(true, Ordering::Relaxed);
        }
        SimCommand::SaveState { path } => {
            if let Err(e) = save_state(path, simulation) {
                eprintln!("Failed to save state: {}", e);
            }
        }
        SimCommand::LoadState { path } => match load_state(path) {
            Ok(state) => state.apply_to(simulation),
            Err(e) => eprintln!("Failed to load state: {}", e),
        },
        SimCommand::AddRing { body, x, y, radius } => {
            spawn::add_ring(simulation, body, x, y, radius);
        }
        SimCommand::AddRectangle {
            body,
            x,
            y,
            width,
            height,
        } => {
            spawn::add_rectangle(simulation, body, x, y, width, height);
        }
        SimCommand::AddRandom {
            body,
            count,
            domain_width,
            domain_height,
        } => {
            spawn::add_random(simulation, body, count, domain_width, domain_height);
        }
        SimCommand::AddFoil {
            width,
            height,
            x,
            y,
            particle_radius,
            current,
        } => {
            spawn::add_foil(simulation, width, height, x, y, particle_radius, current);
        }
        SimCommand::SetFoilCurrent { foil_id, current } => {
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.dc_current = current;
            }
        }
        SimCommand::SetFoilDCCurrent {
            foil_id,
            dc_current,
        } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.dc_current = dc_current;
            }
            if let Some((link_id, mode)) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.dc_current = match mode {
                        crate::body::foil::LinkMode::Parallel => dc_current,
                        crate::body::foil::LinkMode::Opposite => -dc_current,
                    };
                }
            }
        }
        SimCommand::SetFoilACCurrent {
            foil_id,
            ac_current,
        } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| link_id));
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.ac_current = ac_current;
            }
            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.ac_current = ac_current;
                }
            }
        }
        SimCommand::SetFoilFrequency { foil_id, switch_hz } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| link_id));
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.switch_hz = switch_hz;
            }
            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.switch_hz = switch_hz;
                }
            }
        }
        SimCommand::LinkFoils { a, b, mode } => {
            let a_idx = simulation.foils.iter().position(|f| f.id == a);
            let b_idx = simulation.foils.iter().position(|f| f.id == b);
            if let (Some(a_idx), Some(b_idx)) = (a_idx, b_idx) {
                let (first, second) = if a_idx < b_idx {
                    let (left, right) = simulation.foils.split_at_mut(b_idx);
                    (&mut left[a_idx], &mut right[0])
                } else {
                    let (left, right) = simulation.foils.split_at_mut(a_idx);
                    (&mut right[0], &mut left[b_idx])
                };
                first.link_id = Some(b);
                second.link_id = Some(a);
                first.mode = mode;
                second.mode = mode;
            }
        }
        SimCommand::UnlinkFoils { a, b } => {
            if let Some(foil_a) = simulation
                .foils
                .iter_mut()
                .find(|f| f.id == a && f.link_id == Some(b))
            {
                foil_a.link_id = None;
            }
            if let Some(foil_b) = simulation
                .foils
                .iter_mut()
                .find(|f| f.id == b && f.link_id == Some(a))
            {
                foil_b.link_id = None;
            }
        }
        SimCommand::SetTemperature { temperature } => {
            // Just update the target temperature; thermostat will be applied periodically
            crate::config::LJ_CONFIG.lock().temperature = temperature;
        }
        SimCommand::SetDomainSize { width, height } => {
            let half_width = width / 2.0;
            let half_height = height / 2.0;
            simulation.bodies.retain(|body| {
                body.pos.x >= -half_width
                    && body.pos.x <= half_width
                    && body.pos.y >= -half_height
                    && body.pos.y <= half_height
            });
            // Update rectangular domain dimensions
            simulation.domain_width = half_width;
            simulation.domain_height = half_height;
            simulation.cell_list.update_domain_size(half_width, half_height);
        }
        SimCommand::SetOutOfPlane { enabled, max_z, z_stiffness, z_damping, z_frustration_strength } => {
            let mut cfg = crate::config::LJ_CONFIG.lock();
            cfg.enable_out_of_plane = enabled;
            cfg.max_z = max_z;
            cfg.z_stiffness = z_stiffness;
            cfg.z_damping = z_damping;
            cfg.z_frustration_strength = z_frustration_strength;
            simulation.domain_depth = max_z;
            
            // CRITICAL FIX: Reset z-coordinates when out-of-plane is disabled
            if !enabled {
                simulation.bodies.iter_mut().for_each(|b| b.reset_z());
            } else {
                simulation.bodies.iter_mut().for_each(|b| b.clamp_z(max_z));
            }
        }
        SimCommand::ToggleZVisualization { enabled } => {
            crate::renderer::state::SHOW_Z_VISUALIZATION.store(enabled, std::sync::atomic::Ordering::Relaxed);
        }
        SimCommand::SetZVisualizationStrength { strength } => {
            *crate::renderer::state::Z_VISUALIZATION_STRENGTH.lock() = strength;
        }
    }
}
