//! verify_potential_conversion — empirical check of the simulator's
//! Δratio → mV conversion.
//!
//! Loads a scenario with two foil groups, equilibrates, applies a known
//! potentiostatic Δratio, holds while sampling the mean local_potential of
//! each group's foil bodies, and reports measured vs predicted values.
//!
//! Predicted (from V_local = BASELINE + charge × POTENTIAL_PER_CHARGE,
//! BASELINE = 2.0 V, POTENTIAL_PER_CHARGE = 2.0 V/e):
//!   V_A_mean = 2.0 − 2.0·Δ          (target_ratio_A = 1 + Δ)
//!   V_B_mean = 2.0 + 2.0·Δ          (target_ratio_B = 1 − Δ)
//!   ΔV_cell  = V_A − V_B = −4.0·Δ V
//!
//! If the measurement agrees with prediction, the linear potential model
//! holds and the 4000×Δ conversion is good. If they diverge, we've
//! measured the empirical scaling and can document/use that instead.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::plotting::analysis::calculate_cell_voltage;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: verify_potential_conversion --scenario <path.toml> \
         [--seed <u64>] \
         [--equilibrate-fs <float>] \
         [--hold-fs <float>] \
         [--delta <float>] \
         [--sample-every-fs <float>]"
    );
    std::process::exit(2);
}

fn parse_u64_flexible(s: &str) -> Option<u64> {
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

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

// Voltage measurement: `calculate_cell_voltage(bodies, foils, k)` from
// plotting/analysis.rs. It computes V_pos_centroid − V_neg_centroid where
// each V is a Coulomb sum over all bodies and pos/neg is determined by the
// foil's overpotential controller (target_ratio ≥ 1 → positive). This is
// what the GUI plots as "CellVoltage" — same number EIS sweeps would see at
// DC. Replaces the earlier per-body-local_potential averaging, which was
// not the cell voltage.

fn main() {
    let mut scenario: Option<String> = None;
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut hold_fs: f32 = 20_000.0;
    let mut delta: f32 = 0.01;
    let mut sample_every_fs: f32 = 1_000.0;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => { i += 1; scenario = args.get(i).cloned(); }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| parse_u64_flexible(s)).unwrap_or_else(|| {
                    eprintln!("--seed expects a u64 (decimal or 0x-prefixed hex)");
                    print_usage_and_exit();
                });
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--equilibrate-fs expects a float"); print_usage_and_exit();
                });
            }
            "--hold-fs" => {
                i += 1;
                hold_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--hold-fs expects a float"); print_usage_and_exit();
                });
            }
            "--delta" => {
                i += 1;
                delta = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--delta expects a float"); print_usage_and_exit();
                });
            }
            "--sample-every-fs" => {
                i += 1;
                sample_every_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--sample-every-fs expects a float"); print_usage_and_exit();
                });
            }
            "--help" | "-h" => print_usage_and_exit(),
            other => { eprintln!("Unknown argument: {}", other); print_usage_and_exit(); }
        }
        i += 1;
    }
    let scenario_path = scenario.unwrap_or_else(|| {
        eprintln!("--scenario is required"); print_usage_and_exit();
    });

    // Dummy SIM_COMMAND_SENDER so handlers that consult it don't panic.
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    fastrand::seed(seed);

    // --- load + build sim via the same code path the GUI uses ---
    let config = InitConfig::load_from_file(&scenario_path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}", scenario_path, e); std::process::exit(1);
    });
    let (full_w, full_h) = config.simulation.as_ref()
        .map(|s| s.domain_size())
        .unwrap_or_else(|| {
            eprintln!("scenario must specify [simulation] domain"); std::process::exit(1);
        });

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list.update_domain_size(full_w / 2.0, full_h / 2.0);

    println!("Scenario: {}", scenario_path);
    println!("Seed: 0x{:X}, Δ = {}", seed, delta);
    println!("Domain: {} x {}", full_w, full_h);

    for rect in &config.particles.metal_rectangles {
        let species = rect.to_species().expect("invalid metal_rectangles species");
        let body = template_body(species);
        let (origin_x, origin_y) = rect.to_origin_coords();
        handle_command(SimCommand::AddRectangle {
            body, x: origin_x, y: origin_y, width: rect.width, height: rect.height,
        }, &mut sim);
    }
    for foil in &config.particles.foil_rectangles {
        let (origin_x, origin_y) = foil.to_origin_coords();
        handle_command(SimCommand::AddFoil {
            width: foil.width, height: foil.height, x: origin_x, y: origin_y,
            particle_radius: Species::FoilMetal.radius(), current: foil.current,
        }, &mut sim);
    }
    for entry in &config.particles.random {
        let species = entry.to_species().expect("invalid random species");
        let body = template_body(species);
        handle_command(SimCommand::AddRandom {
            body, count: entry.count, domain_width: full_w, domain_height: full_h,
        }, &mut sim);
    }

    // Group foils by x-position.
    let mut group_a: Vec<u64> = Vec::new();
    let mut group_b: Vec<u64> = Vec::new();
    for foil in &sim.foils {
        let mut cx = 0.0f32; let mut cn = 0.0f32;
        for bid in &foil.body_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) {
                cx += b.pos.x; cn += 1.0;
            }
        }
        if cn > 0.0 { cx /= cn; }
        if cx < 0.0 { group_a.push(foil.id); } else { group_b.push(foil.id); }
    }
    handle_command(SimCommand::SetFoilGroups {
        group_a: group_a.clone(), group_b: group_b.clone(),
    }, &mut sim);

    let dt = sim.dt;
    let k = sim.config.coulomb_constant;

    // --- equilibrate (no charging) ---
    let n_eq = (equilibrate_fs / dt) as usize;
    println!("\nEquilibrating {} fs ({} steps, no charging)...", equilibrate_fs, n_eq);
    for _ in 0..n_eq {
        sim.step();
    }

    // Pre-charge baseline: ΔV_cell with both foils in current mode (no controller
    // yet, so calculate_cell_voltage may return 0 since pos/neg classification
    // requires Overpotential mode). Sample after applying Δ instead.
    println!("\nApplying target_ratio_A = {:.4}, target_ratio_B = {:.4}...",
        1.0 + delta, 1.0 - delta);
    handle_command(SimCommand::ConventionalSetOverpotential { target_ratio: 1.0 + delta }, &mut sim);

    // --- hold + sample every step; time-average the second half of the window ---
    let n_hold = (hold_fs / dt) as usize;
    let n_avg_start = n_hold / 2;
    let sample_print_interval = ((sample_every_fs / dt) as usize).max(1);
    println!("\nHolding for {} fs ({} steps); sampling cell voltage every step,", hold_fs, n_hold);
    println!("printing every {} fs, time-averaging over the second half ({} steps).",
        sample_every_fs, n_hold - n_avg_start);
    println!("\n{:>8} {:>10} {:>10}", "t (fs)", "V_cell (V)", "V_cell (mV)");
    println!("{:>8} {:>10} {:>10}", "─", "─", "─");

    let mut sum_v: f64 = 0.0;
    let mut n_samples: u64 = 0;
    let mut last_v: f32 = 0.0;
    for step_i in 0..n_hold {
        sim.step();
        let v = calculate_cell_voltage(&sim.bodies, &sim.foils, k);
        last_v = v;
        if step_i >= n_avg_start {
            sum_v += v as f64;
            n_samples += 1;
        }
        if step_i % sample_print_interval == 0 || step_i == n_hold - 1 {
            let t = (step_i + 1) as f32 * dt;
            println!("{:>8.0} {:>10.4} {:>10.2}", t, v, v * 1000.0);
        }
    }
    let avg_v = if n_samples > 0 { (sum_v / n_samples as f64) as f32 } else { last_v };

    // Predicted: ΔV_cell = V_pos − V_neg. With target_A = 1+Δ ≥ 1 → A is "pos",
    // target_B = 1−Δ < 1 → B is "neg". calculate_cell_voltage returns
    // V_A_centroid − V_B_centroid. From the per-body model:
    //   foil mean charge ≈ −Δ (positive side has Δ excess electrons per body avg)
    //   foil mean potential ≈ BASELINE − Δ × POTENTIAL_PER_CHARGE (positive side)
    //                       ≈ BASELINE + Δ × POTENTIAL_PER_CHARGE (negative side)
    //   ΔV_cell (V_pos − V_neg) ≈ −2 × Δ × POTENTIAL_PER_CHARGE = −4·Δ V at POT_PER_CHARGE=2
    // BUT: calculate_cell_voltage does a *spatial* Coulomb sum at centroids over all
    // bodies (not just foil bodies), so the actual measured value reflects screening
    // by the surrounding electrolyte — likely smaller magnitude.
    let predicted_dv = -4.0 * delta;

    println!("\n=== Predicted vs time-averaged measured ===");
    println!("                            Predicted        Measured (avg)        Final sample");
    println!("ΔV_cell (V)             {:>9.4}            {:>9.4}            {:>9.4}",
        predicted_dv, avg_v, last_v);
    println!("ΔV_cell (mV)            {:>9.2}            {:>9.2}            {:>9.2}",
        predicted_dv * 1000.0, avg_v * 1000.0, last_v * 1000.0);

    let measured_factor_v_per_delta: f32 = if delta != 0.0 { avg_v / delta } else { 0.0 };
    let predicted_factor_v_per_delta: f32 = -4.0;
    println!("\n=== Conversion factor ΔV_cell / Δ ===");
    println!("  Predicted (linear, no screening): {:>+6.3} V/Δ  ({:>+5} mV/Δ)",
        predicted_factor_v_per_delta,
        (predicted_factor_v_per_delta * 1000.0) as i32);
    println!("  Measured (Coulomb sum, screened): {:>+6.3} V/Δ  ({:>+5.0} mV/Δ)",
        measured_factor_v_per_delta,
        measured_factor_v_per_delta * 1000.0);

    let ratio = if predicted_factor_v_per_delta != 0.0 {
        measured_factor_v_per_delta / predicted_factor_v_per_delta
    } else {
        0.0
    };
    println!("\n  Measured / predicted ratio: {:.3}", ratio);
    if (ratio - 1.0).abs() < 0.10 {
        println!("  ✓ Within 10% — linear unscreened model holds at this Δ");
    } else if ratio > 0.0 && ratio < 1.0 {
        println!("  → Screening reduces |ΔV_cell| to {:.0}% of unscreened value;", ratio * 100.0);
        println!("    use measured factor (~{:.0} mV/Δ) for amplitude-floor work.",
            measured_factor_v_per_delta.abs() * 1000.0);
    } else {
        println!("  ✗ Sign or magnitude unexpected — needs investigation.");
    }
}
