use crate::io::{load_state, save_state};
use crate::profile_scope;
use crate::renderer::state::{SimCommand, PAUSED};
use crate::simulation::Simulation;
use std::sync::atomic::Ordering;

use super::spawn;

pub fn handle_command(cmd: SimCommand, simulation: &mut Simulation) {
    profile_scope!("command_handling");
    let mut state_changed = false;
    let mut mark_dirty = |sim: &mut Simulation| {
        state_changed = true;
        sim.mark_history_dirty();
    };

    match cmd {
        SimCommand::ChangeCharge { id, delta } => {
            let mut changed = false;
            if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
                if delta > 0.0 {
                    for _ in 0..delta.round() as usize {
                        body.electrons.pop();
                    }
                    changed = true;
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
                    changed = true;
                }
                if changed {
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
            if changed {
                mark_dirty(simulation);
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
            mark_dirty(simulation);
        }
        SimCommand::DeleteAll => {
            eprintln!("[bodies-debug] DeleteAll command clearing {} bodies", simulation.bodies.len());
            simulation.bodies.clear();
            simulation.foils.clear();
            simulation.body_to_foil.clear();
            mark_dirty(simulation);
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
            mark_dirty(simulation);
        }
        SimCommand::AddCircle { body, x, y, radius } => {
            spawn::add_circle(simulation, body, x, y, radius);
            mark_dirty(simulation);
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
            state_changed = true;
            PAUSED.store(true, Ordering::Relaxed);
        }
        SimCommand::SaveState { path } => {
            if let Err(e) = save_state(path, simulation) {
                eprintln!("Failed to save state: {}", e);
            }
        }
        SimCommand::LoadState { path } => match load_state(path) {
            Ok(scenario) => {
                simulation.load_state(scenario);
                PAUSED.store(true, Ordering::Relaxed);
                state_changed = true;
            }
            Err(e) => eprintln!("Failed to load state: {}", e),
        },
        SimCommand::AddRing { body, x, y, radius } => {
            spawn::add_ring(simulation, body, x, y, radius);
            mark_dirty(simulation);
        }
        SimCommand::AddRectangle {
            body,
            x,
            y,
            width,
            height,
        } => {
            spawn::add_rectangle(simulation, body, x, y, width, height);
            mark_dirty(simulation);
        }
        SimCommand::AddRandom {
            body,
            count,
            domain_width,
            domain_height,
        } => {
            spawn::add_random(simulation, body, count, domain_width, domain_height);
            mark_dirty(simulation);
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
            mark_dirty(simulation);
        }
        SimCommand::SetFoilCurrent { foil_id, current } => {
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.dc_current = current;
                mark_dirty(simulation);
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
                mark_dirty(simulation);
            }
            if let Some((link_id, mode)) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.dc_current = match mode {
                        crate::body::foil::LinkMode::Parallel => dc_current,
                        crate::body::foil::LinkMode::Opposite => -dc_current,
                    };
                    mark_dirty(simulation);
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
                mark_dirty(simulation);
            }
            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.ac_current = ac_current;
                    mark_dirty(simulation);
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
                mark_dirty(simulation);
            }
            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.switch_hz = switch_hz;
                    mark_dirty(simulation);
                }
            }
        }
        SimCommand::SetFoilChargingMode { foil_id, mode } => {
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.charging_mode = mode;
                mark_dirty(simulation);
            }
        }
        SimCommand::EnableOverpotentialMode {
            foil_id,
            target_ratio,
        } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.enable_overpotential_mode(target_ratio);
                mark_dirty(simulation);
            }

            if let Some((link_id, mode)) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    let linked_target = match mode {
                        crate::body::foil::LinkMode::Parallel => target_ratio,
                        crate::body::foil::LinkMode::Opposite => 2.0 - target_ratio,
                    };
                    linked_foil.enable_overpotential_mode(linked_target);
                    mark_dirty(simulation);
                }
            }
        }
        SimCommand::DisableOverpotentialMode { foil_id } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| link_id));

            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                foil.disable_overpotential_mode();
                mark_dirty(simulation);
            }

            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    linked_foil.disable_overpotential_mode();
                    mark_dirty(simulation);
                }
            }
        }
        SimCommand::SetFoilOverpotentialTarget {
            foil_id,
            target_ratio,
        } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                if let Some(ref mut controller) = foil.overpotential_controller {
                    controller.target_ratio = target_ratio;
                    mark_dirty(simulation);
                }
            }

            if let Some((link_id, mode)) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    if let Some(ref mut controller) = linked_foil.overpotential_controller {
                        let linked_target = match mode {
                            crate::body::foil::LinkMode::Parallel => target_ratio,
                            crate::body::foil::LinkMode::Opposite => 2.0 - target_ratio,
                        };
                        controller.target_ratio = linked_target;
                        mark_dirty(simulation);
                    }
                }
            }
        }
        SimCommand::SetFoilPIDGains {
            foil_id,
            kp,
            ki,
            kd,
        } => {
            let link_info = simulation
                .foils
                .iter()
                .find(|f| f.id == foil_id)
                .and_then(|foil| foil.link_id.map(|link_id| link_id));

            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                if let Some(ref mut controller) = foil.overpotential_controller {
                    controller.kp = kp;
                    controller.ki = ki;
                    controller.kd = kd;
                    mark_dirty(simulation);
                }
            }

            if let Some(link_id) = link_info {
                if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
                    if let Some(ref mut controller) = linked_foil.overpotential_controller {
                        controller.kp = kp;
                        controller.ki = ki;
                        controller.kd = kd;
                        mark_dirty(simulation);
                    }
                }
            }
        }
        SimCommand::SetFoilGroups { group_a, group_b } => {
            // Update simulation group memberships; ensure no overlap
            simulation.group_a.clear();
            simulation.group_b.clear();
            for id in group_a {
                simulation.group_a.insert(id);
            }
            for id in group_b {
                if simulation.group_a.contains(&id) {
                    // skip overlap, A wins
                    continue;
                }
                simulation.group_b.insert(id);
            }
            mark_dirty(simulation);
        }
        SimCommand::ClearFoilGroups => {
            simulation.group_a.clear();
            simulation.group_b.clear();
            mark_dirty(simulation);
        }
        SimCommand::ConventionalSetCurrent { current } => {
            // Apply to masters of A/B if present; enforce B as opposite DC, same AC/freq
            let master_a = simulation.group_a.iter().min().copied();
            let master_b = simulation.group_b.iter().min().copied();
            if let Some(ma) = master_a {
                if let Some(f) = simulation.foils.iter_mut().find(|f| f.id == ma) {
                    f.charging_mode = crate::body::foil::ChargingMode::Current;
                    f.dc_current = current;
                    mark_dirty(simulation);
                }
            }
            if let Some(mb) = master_b {
                if let Some(f) = simulation.foils.iter_mut().find(|f| f.id == mb) {
                    f.charging_mode = crate::body::foil::ChargingMode::Current;
                    f.dc_current = -current;
                    mark_dirty(simulation);
                }
            }
        }
        SimCommand::ConventionalSetOverpotential { target_ratio } => {
            // Apply to masters of A/B if present; enforce B as complementary (2 - target)
            let master_a = simulation.group_a.iter().min().copied();
            let master_b = simulation.group_b.iter().min().copied();
            if let Some(ma) = master_a {
                if let Some(f) = simulation.foils.iter_mut().find(|f| f.id == ma) {
                    f.enable_overpotential_mode(target_ratio);
                    mark_dirty(simulation);
                }
            }
            if let Some(mb) = master_b {
                if let Some(f) = simulation.foils.iter_mut().find(|f| f.id == mb) {
                    f.enable_overpotential_mode(2.0 - target_ratio);
                    mark_dirty(simulation);
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
                mark_dirty(simulation);
            }
        }
        SimCommand::UnlinkFoils { a, b } => {
            let mut any_changed = false;
            if let Some(foil_a) = simulation
                .foils
                .iter_mut()
                .find(|f| f.id == a && f.link_id == Some(b))
            {
                foil_a.link_id = None;
                any_changed = true;
            }
            if let Some(foil_b) = simulation
                .foils
                .iter_mut()
                .find(|f| f.id == b && f.link_id == Some(a))
            {
                foil_b.link_id = None;
                any_changed = true;
            }
            if any_changed {
                mark_dirty(simulation);
            }
        }
        SimCommand::SetTemperature { temperature } => {
            crate::config::LJ_CONFIG.lock().temperature = temperature;
            mark_dirty(simulation);
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
            simulation.domain_width = half_width;
            simulation.domain_height = half_height;
            simulation
                .cell_list
                .update_domain_size(half_width, half_height);

            // Update shared state so GUI stays in sync
            *crate::renderer::state::DOMAIN_WIDTH.lock() = width;
            *crate::renderer::state::DOMAIN_HEIGHT.lock() = height;
            mark_dirty(simulation);
        }
        SimCommand::SetOutOfPlane {
            enabled,
            max_z,
            z_stiffness,
            z_damping,
        } => {
            let mut cfg = crate::config::LJ_CONFIG.lock();
            cfg.enable_out_of_plane = enabled;
            cfg.max_z = max_z;
            cfg.z_stiffness = z_stiffness;
            cfg.z_damping = z_damping;
            simulation.domain_depth = max_z;

            if !enabled {
                simulation.bodies.iter_mut().for_each(|b| b.reset_z());
            } else {
                simulation.bodies.iter_mut().for_each(|b| b.clamp_z(max_z));
            }
            mark_dirty(simulation);
        }
        SimCommand::ToggleZVisualization { enabled } => {
            crate::renderer::state::SHOW_Z_VISUALIZATION
                .store(enabled, std::sync::atomic::Ordering::Relaxed);
        }
        SimCommand::SetZVisualizationStrength { strength } => {
            *crate::renderer::state::Z_VISUALIZATION_STRENGTH.lock() = strength;
        }
        SimCommand::SetPIDHistorySize {
            foil_id,
            history_size,
        } => {
            if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
                if let Some(ref mut controller) = foil.overpotential_controller {
                    controller.max_history_size = history_size;
                    while controller.history.len() > controller.max_history_size {
                        controller.history.pop_front();
                    }
                    mark_dirty(simulation);
                }
            }
        }
        SimCommand::PlaybackSeek { index } => {
            simulation.seek_history(index);
            state_changed = true;
            PAUSED.store(true, Ordering::Relaxed);
        }
        SimCommand::PlaybackPlay { auto_resume } => {
            simulation.start_playback(auto_resume);
            state_changed = true;
            // Let playback system control PAUSED state via simulation loop
        }
        SimCommand::PlaybackPause => {
            simulation.pause_playback();
            state_changed = true;
            PAUSED.store(true, Ordering::Relaxed);
        }
        SimCommand::PlaybackSetSpeed { speed } => {
            simulation.set_playback_speed(speed);
            state_changed = true;
        }
        SimCommand::PlaybackResumeLive => {
            simulation.go_to_latest();
            state_changed = true;
            PAUSED.store(false, Ordering::Relaxed);
        }
        SimCommand::PlaybackResumeFromCurrent => {
            simulation.resume_live_from_current();
            state_changed = true;
            PAUSED.store(false, Ordering::Relaxed);
        }
        SimCommand::ResetTime => {
            simulation.frame = 0;
            simulation.last_thermostat_time = 0.0;  // Fix: Reset thermostat timer too
            // Update global simulation time
            *crate::renderer::state::SIM_TIME.lock() = 0.0;
            state_changed = true;
        }
        SimCommand::StartManualMeasurement { config } => {
            simulation.start_manual_measurement(config);
        }
        SimCommand::StopManualMeasurement => {
            simulation.stop_manual_measurement();
        }
    }

    if state_changed {
        simulation.flush_history_if_dirty();
    }
}
