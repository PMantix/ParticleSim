//! E.1 Baseline DCR: rest → pulse → rest protocol on a fresh cell.
//!
//! Protocol:
//!   1. Equilibrate (5 ps, no current, no logging)
//!   2. Pre-rest (20 ps, no current, logged) — baseline reference
//!   3. Pulse (100 ps, overpotential mode at target_ratio, logged)
//!   4. Post-rest (50 ps, overpotential off / ratio=1.0, logged)
//!
//! Logs: phase, sim_time, eta_a, eta_b, e_delta_a, e_delta_b
//! R(t) computed in post-processing from the pulse phase data.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn template_body(species: Species) -> Body {
    let charge = match species {
        Species::LithiumIon => 1.0,
        Species::ElectrolyteAnion => -1.0,
        _ => 0.0,
    };
    Body::new(Vec2::zero(), Vec2::zero(), species.mass(), species.radius(), charge, species)
}

fn build_cell() -> Simulation {
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let esize = 80.0f32;
    let foil_w = 20.0f32;
    let domain_w = 400.0f32;
    let domain_h = 240.0f32;
    let half_sep = domain_w / 4.0;
    let half_e = esize / 2.0;

    let mut sim = Simulation::new();
    sim.domain_width = domain_w / 2.0;
    sim.domain_height = domain_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    // Left electrode
    handle_command(SimCommand::AddRectangle {
        body: template_body(Species::LithiumMetal),
        x: -half_sep - half_e, y: -half_e,
        width: esize, height: esize,
    }, &mut sim);
    handle_command(SimCommand::AddFoil {
        width: foil_w, height: esize,
        x: -half_sep - half_e, y: -half_e,
        particle_radius: Species::FoilMetal.radius(),
        current: 0.0,
    }, &mut sim);

    // Right electrode
    handle_command(SimCommand::AddRectangle {
        body: template_body(Species::LithiumMetal),
        x: half_sep - half_e, y: -half_e,
        width: esize, height: esize,
    }, &mut sim);
    handle_command(SimCommand::AddFoil {
        width: foil_w, height: esize,
        x: half_sep + half_e - foil_w, y: -half_e,
        particle_radius: Species::FoilMetal.radius(),
        current: 0.0,
    }, &mut sim);

    // Electrolyte
    let electrode_area = esize * esize * 2.0;
    let free_area = domain_w * domain_h - electrode_area;
    let density_factor = free_area / 10000.0;
    let n_li = (15.0 * density_factor).max(5.0) as usize;
    let n_ec = (90.0 * density_factor).max(10.0) as usize;

    for (count, species) in [
        (n_li, Species::LithiumIon),
        (n_li, Species::ElectrolyteAnion),
        (n_ec, Species::EC),
        (n_ec, Species::DMC),
    ] {
        handle_command(SimCommand::AddRandom {
            body: template_body(species),
            count, domain_width: domain_w, domain_height: domain_h,
        }, &mut sim);
    }

    sim
}

fn log_sample(sim: &Simulation, phase: &str, t0: f32) {
    let t = sim.time - t0;
    let ratio_a = sim.calculate_foil_electron_ratio(&sim.foils[0]);
    let ratio_b = sim.calculate_foil_electron_ratio(&sim.foils[1]);
    let pot = particle_sim::config::POTENTIAL_PER_CHARGE;
    let eta_a = (ratio_a - 1.0) * pot;
    let eta_b = (ratio_b - 1.0) * pot;
    let ed_a = sim.foils[0].electron_delta_since_measure;
    let ed_b = sim.foils[1].electron_delta_since_measure;
    println!("{},{:.1},{:.6},{:.6},{},{}", phase, t, eta_a, eta_b, ed_a, ed_b);
}

fn main() {
    let target_ratio: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.01);

    let dt = 5.0f32; // fs per step
    let equilibrate_fs = 5000.0;
    let pre_rest_fs = 20000.0;
    let pulse_fs = 100000.0;
    let post_rest_fs = 50000.0;
    let log_every_fs = 100.0;

    let equilibrate_steps = (equilibrate_fs / dt) as usize;
    let pre_rest_steps = (pre_rest_fs / dt) as usize;
    let pulse_steps = (pulse_fs / dt) as usize;
    let post_rest_steps = (post_rest_fs / dt) as usize;
    let log_stride = (log_every_fs / dt) as usize;

    eprintln!("E.1 Baseline DCR: ratio={:.3}, equil={:.0}fs, pre_rest={:.0}fs, pulse={:.0}fs, post_rest={:.0}fs",
        target_ratio, equilibrate_fs, pre_rest_fs, pulse_fs, post_rest_fs);

    let mut sim = build_cell();
    eprintln!("Cell: {} bodies, {} foils", sim.bodies.len(), sim.foils.len());

    // Header
    println!("phase,sim_time,eta_a,eta_b,e_delta_a,e_delta_b");

    // Phase 0: equilibrate (no logging)
    for _ in 0..equilibrate_steps {
        sim.step();
    }
    let t0 = sim.time;
    eprintln!("Equilibrated at t={:.0} fs", t0);

    // Reset delta counters
    for foil in &mut sim.foils {
        foil.electron_delta_since_measure = 0;
    }

    // Phase 1: pre-rest
    for step in 1..=pre_rest_steps {
        sim.step();
        if step % log_stride == 0 {
            log_sample(&sim, "pre_rest", t0);
        }
    }
    eprintln!("Pre-rest complete");

    // Phase 2: pulse — enable overpotential mode
    if sim.foils.len() >= 2 {
        sim.foils[0].enable_overpotential_mode(target_ratio);
        sim.foils[1].enable_overpotential_mode(2.0 - target_ratio);
    }
    // Reset delta counters at pulse onset
    for foil in &mut sim.foils {
        foil.electron_delta_since_measure = 0;
    }
    for step in 1..=pulse_steps {
        sim.step();
        if step % log_stride == 0 {
            log_sample(&sim, "pulse", t0);
        }
    }
    eprintln!("Pulse complete");

    // Phase 3: post-rest — open circuit (current=0, natural relaxation)
    for foil in &mut sim.foils {
        foil.disable_overpotential_mode();
        foil.dc_current = 0.0;
        foil.ac_current = 0.0;
    }
    for foil in &mut sim.foils {
        foil.electron_delta_since_measure = 0;
    }
    for step in 1..=post_rest_steps {
        sim.step();
        if step % log_stride == 0 {
            log_sample(&sim, "post_rest", t0);
        }
    }
    eprintln!("Post-rest complete. Total sim_time={:.0} fs", sim.time);
}
