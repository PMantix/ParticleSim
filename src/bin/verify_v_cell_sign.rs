//! verify_v_cell_sign — apply known DC current and compare both V_cell functions.
//!
//! `calculate_cell_voltage` (plotting/analysis.rs) and
//! `compute_eis_voltage_by_potential` (simulation.rs) are both used as "V_cell"
//! in different parts of the code. They have different exclusion rules
//! (calculate_cell_voltage includes foil bodies in its potential sum,
//! compute_eis_voltage_by_potential excludes the local foil's own bodies
//! from each probe's sum). They could disagree on sign of V_cell vs applied
//! current. If they do, that's a likely root cause of the negative-Re(Z)
//! reading in the EIS lock-in.
//!
//! Protocol: load validation scenario, equilibrate, apply +I DC for some
//! settle window, time-average each V_cell, print signs side by side.
//! Repeat at -I to confirm sign-flip behavior.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::plotting::analysis::calculate_cell_voltage;
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

fn main() {
    let scenario = "measurement_configs/eis_validation_flat_symmetric.toml";
    let i_test: f32 = 1e-3; // small but measurable

    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);
    fastrand::seed(0xC0FFEE);

    let cfg = InitConfig::load_from_file(scenario).expect("scenario");
    let (full_w, full_h) = cfg.simulation.as_ref().unwrap().domain_size();

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    for rect in &cfg.particles.metal_rectangles {
        let species = rect.to_species().unwrap();
        let body = template_body(species);
        let (x, y) = rect.to_origin_coords();
        handle_command(SimCommand::AddRectangle { body, x, y, width: rect.width, height: rect.height }, &mut sim);
    }
    for foil in &cfg.particles.foil_rectangles {
        let (x, y) = foil.to_origin_coords();
        handle_command(SimCommand::AddFoil {
            width: foil.width, height: foil.height, x, y,
            particle_radius: Species::FoilMetal.radius(), current: foil.current,
        }, &mut sim);
    }
    for entry in &cfg.particles.random {
        let body = template_body(entry.to_species().unwrap());
        handle_command(SimCommand::AddRandom {
            body, count: entry.count,
            domain_width: full_w, domain_height: full_h,
        }, &mut sim);
    }

    // Group foils by x-coord
    let mut group_a: Vec<u64> = Vec::new();
    let mut group_b: Vec<u64> = Vec::new();
    for foil in &sim.foils {
        let mut cx = 0.0f32; let mut cn = 0.0f32;
        for bid in &foil.body_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) { cx += b.pos.x; cn += 1.0; }
        }
        if cn > 0.0 { cx /= cn; }
        if cx < 0.0 { group_a.push(foil.id); } else { group_b.push(foil.id); }
    }
    handle_command(SimCommand::SetFoilGroups { group_a: group_a.clone(), group_b: group_b.clone() }, &mut sim);

    // Probe IDs (every body in the foils — same as voltage_probes=0 in EIS)
    let probe_a: Vec<u64> = sim.foils.iter()
        .filter(|f| group_a.contains(&f.id))
        .flat_map(|f| f.body_ids.iter().copied()).collect();
    let probe_b: Vec<u64> = sim.foils.iter()
        .filter(|f| group_b.contains(&f.id))
        .flat_map(|f| f.body_ids.iter().copied()).collect();

    let dt = sim.dt;
    let k = sim.config.coulomb_constant;

    // Equilibrate
    let n_eq = (50_000.0 / dt) as usize;
    println!("Equilibrating {} steps...", n_eq);
    for _ in 0..n_eq { sim.step(); }

    let measure_window = (10_000.0 / dt) as usize; // 10 ks averaging window

    let trial = |sim: &mut Simulation, label: &str, i_amp: f32, group_a: &[u64], group_b: &[u64], probe_a: &[u64], probe_b: &[u64]| {
        handle_command(SimCommand::ConventionalSetCurrent { current: i_amp }, sim);
        // Hold + settle
        let n_settle = (15_000.0 / sim.dt) as usize;
        for _ in 0..n_settle { sim.step(); }
        // Average over measure_window
        let mut sum_calc = 0.0f64;
        let mut sum_eis = 0.0f64;
        for _ in 0..measure_window {
            sim.step();
            sum_calc += calculate_cell_voltage(&sim.bodies, &sim.foils, k) as f64;
            sum_eis += sim.compute_eis_voltage_by_potential(group_a, group_b, probe_a, probe_b) as f64;
        }
        let avg_calc = (sum_calc / measure_window as f64) as f32;
        let avg_eis  = (sum_eis  / measure_window as f64) as f32;
        println!("  {:<22}  calc_cell_voltage = {:>+10.4} V    compute_eis_voltage = {:>+10.4} V    sign agree: {}",
                 label, avg_calc, avg_eis,
                 (avg_calc.signum() == avg_eis.signum()));
        // Reset current and rest
        handle_command(SimCommand::ConventionalSetCurrent { current: 0.0 }, sim);
        let n_rest = (5_000.0 / sim.dt) as usize;
        for _ in 0..n_rest { sim.step(); }
    };

    println!("\nApplying I = +{} e/fs to group A:", i_test);
    trial(&mut sim, "I_A=+1e-3", i_test, &group_a, &group_b, &probe_a, &probe_b);

    println!("\nApplying I = -{} e/fs to group A:", i_test);
    trial(&mut sim, "I_A=-1e-3", -i_test, &group_a, &group_b, &probe_a, &probe_b);
}
