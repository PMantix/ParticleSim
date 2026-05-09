//! Headless sweep to find electrode size + current combinations that reach
//! quasi-steady-state η under applied DC current.
//!
//! For each (electrode_size, current) pair: builds a symmetric Li|electrolyte|Li
//! cell, equilibrates, applies current, and tracks η over time. Reports whether
//! η stabilized (drift rate < threshold) and the final η value.

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

struct CellConfig {
    electrode_size: f32,
    foil_width: f32,
    domain_width: f32,
    domain_height: f32,
    n_li: usize,
    n_anion: usize,
    n_ec: usize,
    n_dmc: usize,
}

fn build_cell(cfg: &CellConfig, current: f32) -> Simulation {
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let mut sim = Simulation::new();
    sim.domain_width = cfg.domain_width / 2.0;
    sim.domain_height = cfg.domain_height / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    let half_sep = cfg.domain_width / 4.0;
    let half_e = cfg.electrode_size / 2.0;

    // Coordinates are center-based; convert to bottom-left origin for
    // AddRectangle/AddFoil which expect origin coords.
    let left_cx = -half_sep;
    let right_cx = half_sep;

    // Left electrode (deposition side, +current)
    // Metal centered at left_cx, foil embedded at its left edge
    handle_command(
        SimCommand::AddRectangle {
            body: template_body(Species::LithiumMetal),
            x: left_cx - half_e,
            y: -half_e,
            width: cfg.electrode_size,
            height: cfg.electrode_size,
        },
        &mut sim,
    );
    handle_command(
        SimCommand::AddFoil {
            width: cfg.foil_width,
            height: cfg.electrode_size,
            x: left_cx - half_e,
            y: -half_e,
            particle_radius: Species::FoilMetal.radius(),
            current,
        },
        &mut sim,
    );

    // Right electrode (stripping side, -current)
    // Metal centered at right_cx, foil embedded at its right edge
    handle_command(
        SimCommand::AddRectangle {
            body: template_body(Species::LithiumMetal),
            x: right_cx - half_e,
            y: -half_e,
            width: cfg.electrode_size,
            height: cfg.electrode_size,
        },
        &mut sim,
    );
    handle_command(
        SimCommand::AddFoil {
            width: cfg.foil_width,
            height: cfg.electrode_size,
            x: right_cx + half_e - cfg.foil_width,
            y: -half_e,
            particle_radius: Species::FoilMetal.radius(),
            current: -current,
        },
        &mut sim,
    );

    // Electrolyte
    for (count, species) in [
        (cfg.n_li, Species::LithiumIon),
        (cfg.n_anion, Species::ElectrolyteAnion),
        (cfg.n_ec, Species::EC),
        (cfg.n_dmc, Species::DMC),
    ] {
        handle_command(
            SimCommand::AddRandom {
                body: template_body(species),
                count,
                domain_width: cfg.domain_width,
                domain_height: cfg.domain_height,
            },
            &mut sim,
        );
    }

    sim
}

/// Electrode-level overpotential from the electron ratio of the full
/// connected cluster (foil + all attached Li metal). ratio > 1 means
/// excess electrons (deposition-ready), < 1 means deficit (stripping).
/// η = (ratio - 1.0) × POTENTIAL_PER_CHARGE gives a voltage proxy.
fn electrode_eta(sim: &Simulation) -> (f32, f32) {
    if sim.foils.len() < 2 {
        return (0.0, 0.0);
    }
    let ratio_a = sim.calculate_foil_electron_ratio(&sim.foils[0]);
    let ratio_b = sim.calculate_foil_electron_ratio(&sim.foils[1]);
    let pot = particle_sim::config::POTENTIAL_PER_CHARGE;
    let ea = (ratio_a - 1.0) * pot;
    let eb = (ratio_b - 1.0) * pot;
    (ea, eb)
}

fn main() {
    let electrode_sizes: Vec<f32> = vec![50.0, 65.0, 80.0];
    let currents: Vec<f32> = vec![0.00005, 0.00007, 0.00009, 0.00011];

    let equilibrate_steps = 2000;  // 10k fs at dt=5
    let measure_steps = 30000;    // 150k fs = 150 ps
    let log_stride = 20;          // log every 100 fs

    println!("electrode_size,current,n_bodies,sim_time,eta_a,eta_b,eta_avg");

    for &esize in &electrode_sizes {
        let foil_w = (esize / 4.0).max(5.0);
        let domain_w = esize * 5.0;
        let domain_h = esize * 3.0;
        let electrode_area = esize * esize * 2.0;
        let free_area = domain_w * domain_h - electrode_area;
        let density_factor = free_area / 10000.0;
        let n_li = (15.0 * density_factor).max(5.0) as usize;
        let n_anion = n_li;
        let n_ec = (90.0 * density_factor).max(10.0) as usize;
        let n_dmc = n_ec;

        let cfg = CellConfig {
            electrode_size: esize,
            foil_width: foil_w,
            domain_width: domain_w,
            domain_height: domain_h,
            n_li,
            n_anion,
            n_ec,
            n_dmc,
        };

        for &current in &currents {
            eprintln!("Running esize={:.0} I={:.1e}...", esize, current);
            let mut sim = build_cell(&cfg, current);
            let n_bodies = sim.bodies.len();

            for _ in 0..equilibrate_steps {
                sim.step();
            }

            for step in 1..=measure_steps {
                sim.step();
                if step % log_stride == 0 {
                    let (ea, eb) = electrode_eta(&sim);
                    let avg = (ea + eb) / 2.0;
                    println!(
                        "{},{},{},{:.1},{:.6},{:.6},{:.6}",
                        esize, current, n_bodies, sim.time, ea, eb, avg
                    );
                }
            }
        }
    }
}
