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
            for _ in 0..new_body.neutral_electron_count() {
                new_body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
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
        .and_then(|foil| foil.link_id.map(|link_id| link_id));

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
