#![allow(dead_code)] // Public API functions that may be used by other systems

use crate::simulation::Simulation;
use crate::body::{Species, foil::LinkMode};
use ultraviolet::Vec2;

use crate::body::Electron;

pub fn handle_add_foil(simulation: &mut Simulation, width: f32, height: f32, x: f32, y: f32, particle_radius: f32, current: f32) {
    let origin = Vec2::new(x, y);
    let particle_diameter = 2.0 * particle_radius;
    let cols = (width / particle_diameter).floor() as usize;
    let rows = (height / particle_diameter).floor() as usize;
    let mut body_ids = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let pos = origin
                + Vec2::new(
                    (col as f32 + 0.5) * particle_diameter,
                    (row as f32 + 0.5) * particle_diameter,
                );
            while let Some(idx) = super::particle::overlaps_any(&simulation.bodies, pos, particle_radius) {
                super::particle::remove_body_with_foils(simulation, idx);
            }
            let mut new_body = crate::body::Body::new(
                pos,
                Vec2::zero(),
                Species::FoilMetal.mass(),
                particle_radius,
                0.0,
                Species::FoilMetal,
            );
            new_body.electrons = smallvec::smallvec![Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() }; crate::config::FOIL_NEUTRAL_ELECTRONS];
            new_body.update_charge_from_electrons();
            body_ids.push(new_body.id);
            simulation.bodies.push(new_body);
        }
    }
    let foil = crate::body::foil::Foil::new(
        body_ids.clone(),
        origin,
        width,
        height,
        current,
        0.0,
    );
    for id in &body_ids {
        simulation.body_to_foil.insert(*id, foil.id);
    }
    simulation.foils.push(foil);
}

pub fn handle_set_foil_current(simulation: &mut Simulation, foil_id: u64, current: f32) {
    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.id == foil_id)
    {
        foil.dc_current = current;
    }
}

pub fn handle_set_foil_dc_current(simulation: &mut Simulation, foil_id: u64, dc_current: f32) {
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
        .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.id == foil_id)
    {
        foil.dc_current = dc_current;
    }

    if let Some((link_id, mode)) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            linked_foil.dc_current = match mode {
                LinkMode::Parallel => dc_current,
                LinkMode::Opposite => -dc_current,
            };
        }
    }
}

pub fn handle_set_foil_ac_current(simulation: &mut Simulation, foil_id: u64, ac_current: f32) {
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
        .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.id == foil_id)
    {
        foil.ac_current = ac_current;
    }

    if let Some((link_id, _)) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            linked_foil.ac_current = ac_current;
        }
    }
}

pub fn handle_set_foil_frequency(simulation: &mut Simulation, foil_id: u64, switch_hz: f32) {
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
    .and_then(|foil| foil.link_id);

    if let Some(foil) = simulation
        .foils
        .iter_mut()
        .find(|f| f.id == foil_id)
    {
        foil.switch_hz = switch_hz;
    }

    if let Some(link_id) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            linked_foil.switch_hz = switch_hz;
        }
    }
}

pub fn handle_link_foils(simulation: &mut Simulation, a: u64, b: u64, mode: LinkMode) {
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

pub fn handle_unlink_foils(simulation: &mut Simulation, a: u64, b: u64) {
    if let Some(foil_a) = simulation.foils.iter_mut().find(|f| f.id == a && f.link_id == Some(b)) {
        foil_a.link_id = None;
    }
    if let Some(foil_b) = simulation.foils.iter_mut().find(|f| f.id == b && f.link_id == Some(a)) {
        foil_b.link_id = None;
    }
}

pub fn handle_set_foil_charging_mode(simulation: &mut Simulation, foil_id: u64, mode: crate::body::foil::ChargingMode) {
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        foil.charging_mode = mode;
    }
}

pub fn handle_enable_overpotential_mode(simulation: &mut Simulation, foil_id: u64, target_ratio: f32) {
    // Get link info before mutable borrow
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
    .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

    // Enable overpotential mode on the primary foil (this becomes the master)
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        foil.enable_overpotential_mode(target_ratio);
    }

    // Set up linked foil as SLAVE (NO PID controller)
    if let Some((link_id, _mode)) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            // Slave foil gets NO PID controller - only the master controls both foils
            linked_foil.enable_overpotential_slave_mode(foil_id);
        }
    }
}

pub fn handle_disable_overpotential_mode(simulation: &mut Simulation, foil_id: u64) {
    // Get link info before mutable borrow
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
    .and_then(|foil| foil.link_id);

    // Disable overpotential mode on the primary foil
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        foil.disable_overpotential_mode();
    }

    // Also disable on linked foil if it exists
    if let Some(link_id) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            linked_foil.disable_overpotential_mode();
        }
    }
}

pub fn handle_set_overpotential_target(simulation: &mut Simulation, foil_id: u64, target_ratio: f32) {
    // Get link info before mutable borrow
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
    .and_then(|foil| foil.link_id.map(|link_id| (link_id, foil.mode)));

    // Set target on primary foil
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        if let Some(ref mut controller) = foil.overpotential_controller {
            controller.target_ratio = target_ratio;
        }
    }

    // Apply to linked foil if it exists
    if let Some((link_id, mode)) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            if let Some(ref mut controller) = linked_foil.overpotential_controller {
                let linked_target = match mode {
                    LinkMode::Parallel => target_ratio,
                    LinkMode::Opposite => 2.0 - target_ratio, // Invert around neutral (1.0)
                };
                controller.target_ratio = linked_target;
            }
        }
    }
}

pub fn handle_set_pid_gains(simulation: &mut Simulation, foil_id: u64, kp: f32, ki: f32, kd: f32) {
    // Get link info before mutable borrow
    let link_info = simulation
        .foils
        .iter()
        .find(|f| f.id == foil_id)
    .and_then(|foil| foil.link_id);

    // Set PID gains on primary foil
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        if let Some(ref mut controller) = foil.overpotential_controller {
            controller.kp = kp;
            controller.ki = ki;
            controller.kd = kd;
        }
    }

    // Apply to linked foil if it exists (same PID gains for both)
    if let Some(link_id) = link_info {
        if let Some(linked_foil) = simulation.foils.iter_mut().find(|f| f.id == link_id) {
            if let Some(ref mut controller) = linked_foil.overpotential_controller {
                controller.kp = kp;
                controller.ki = ki;
                controller.kd = kd;
            }
        }
    }
}

pub fn handle_set_pid_history_size(simulation: &mut Simulation, foil_id: u64, history_size: usize) {
    // Set history size on the primary foil
    if let Some(foil) = simulation.foils.iter_mut().find(|f| f.id == foil_id) {
        if let Some(ref mut controller) = foil.overpotential_controller {
            controller.max_history_size = history_size;
            // Trim existing history if it's larger than the new size
            while controller.history.len() > controller.max_history_size {
                controller.history.pop_front();
            }
        }
    }
}
