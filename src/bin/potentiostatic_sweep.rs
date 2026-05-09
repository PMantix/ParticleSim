//! Potentiostatic DCR: impose voltage steps via overpotential mode,
//! measure resulting steady-state current at each step.
//! R_DC = eta / I_ss in the linear regime.

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
    Body::new(
        Vec2::zero(),
        Vec2::zero(),
        species.mass(),
        species.radius(),
        charge,
        species,
    )
}

fn build_cell(esize: f32, foil_w: f32, domain_w: f32, domain_h: f32,
              n_li: usize, n_anion: usize, n_ec: usize, n_dmc: usize) -> Simulation {
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let mut sim = Simulation::new();
    sim.domain_width = domain_w / 2.0;
    sim.domain_height = domain_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    let half_sep = domain_w / 4.0;
    let half_e = esize / 2.0;

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
    for (count, species) in [
        (n_li, Species::LithiumIon),
        (n_anion, Species::ElectrolyteAnion),
        (n_ec, Species::EC),
        (n_dmc, Species::DMC),
    ] {
        handle_command(SimCommand::AddRandom {
            body: template_body(species),
            count, domain_width: domain_w, domain_height: domain_h,
        }, &mut sim);
    }

    sim
}

fn main() {
    let esize = 80.0f32;
    let foil_w = 20.0f32;
    let domain_w = esize * 5.0;
    let domain_h = esize * 3.0;
    let electrode_area = esize * esize * 2.0;
    let free_area = domain_w * domain_h - electrode_area;
    let density_factor = free_area / 10000.0;
    let n_li = (15.0 * density_factor).max(5.0) as usize;
    let n_anion = n_li;
    let n_ec = (90.0 * density_factor).max(10.0) as usize;
    let n_dmc = n_ec;

    // Overpotential steps to sweep (target_ratio values)
    // ratio > 1.0 = excess electrons (reduction bias)
    // ratio < 1.0 = deficit (oxidation bias)
    // We apply symmetric: one foil at ratio, other at 2.0 - ratio
    let ratios: Vec<f32> = vec![1.005, 1.01, 1.02, 1.03, 1.05];

    let equilibrate_steps = 1000;  // 5k fs
    let measure_steps = 30000;     // 150k fs = 150 ps
    let log_stride = 20;           // every 100 fs

    println!("target_ratio,sim_time,eta_a,eta_b,eta_avg,i_controller_a,i_controller_b,e_delta_a,e_delta_b");

    for &ratio in &ratios {
        eprintln!("Running ratio={:.3}...", ratio);
        let mut sim = build_cell(esize, foil_w, domain_w, domain_h, n_li, n_anion, n_ec, n_dmc);

        // Equilibrate at zero current
        for _ in 0..equilibrate_steps {
            sim.step();
        }

        // Enable overpotential mode on both foils
        // Foil A: target > 1.0 (excess electrons, deposition)
        // Foil B: target < 1.0 (deficit, stripping)
        if sim.foils.len() >= 2 {
            sim.foils[0].enable_overpotential_mode(ratio);
            sim.foils[1].enable_overpotential_mode(2.0 - ratio);
        }

        // Reset electron delta counters
        for foil in &mut sim.foils {
            foil.electron_delta_since_measure = 0;
        }

        let t0 = sim.time;
        let mut last_delta_a: i32 = 0;
        let mut last_delta_b: i32 = 0;
        let mut last_t = t0;

        for step in 1..=measure_steps {
            sim.step();

            if step % log_stride == 0 {
                let t = sim.time - t0;

                // Electrode-level eta
                let ratio_a = sim.calculate_foil_electron_ratio(&sim.foils[0]);
                let ratio_b = sim.calculate_foil_electron_ratio(&sim.foils[1]);
                let pot = particle_sim::config::POTENTIAL_PER_CHARGE;
                let eta_a = (ratio_a - 1.0) * pot;
                let eta_b = (ratio_b - 1.0) * pot;
                let eta_avg = (eta_a + eta_b) / 2.0;

                // Controller output current
                let i_a = sim.foils[0].overpotential_controller
                    .as_ref().map(|c| c.last_output_current).unwrap_or(0.0);
                let i_b = sim.foils[1].overpotential_controller
                    .as_ref().map(|c| c.last_output_current).unwrap_or(0.0);

                // Actual electron transfers since last sample
                let delta_a = sim.foils[0].electron_delta_since_measure;
                let delta_b = sim.foils[1].electron_delta_since_measure;
                let d_a = delta_a - last_delta_a;
                let d_b = delta_b - last_delta_b;
                last_delta_a = delta_a;
                last_delta_b = delta_b;

                println!("{:.3},{:.1},{:.6},{:.6},{:.6},{:.6},{:.6},{},{}",
                    ratio, t, eta_a, eta_b, eta_avg, i_a, i_b, d_a, d_b);
            }
        }
    }
}
