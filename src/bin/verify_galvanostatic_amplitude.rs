//! verify_galvanostatic_amplitude — Phase 1.2 cycle 2 prerequisite.
//!
//! For Galvanostatic EIS we apply a sinusoidal current to group-A foils
//! (and -current to group-B). To pick the AC amplitude, we need to know
//! the cell's I→V relationship in current-control mode. This binary does
//! a DC sweep of imposed currents and reports the time-averaged V_cell
//! response at steady state, mirroring find_amplitude_floor.rs's protocol
//! but switched to current control.
//!
//! Output: stdout table + CSV at --output.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::plotting::analysis::calculate_cell_voltage;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::fs::{create_dir_all, File};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: verify_galvanostatic_amplitude --scenario <path.toml> \
         [--seed <u64>] \
         [--equilibrate-fs <float>] \
         [--settle-fs <float>] \
         [--rest-fs <float>] \
         [--currents <c1,c2,...>] \
         [--output <path.csv>]"
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

fn read_and_clear_group_delta(sim: &mut Simulation, group_ids: &[u64]) -> i64 {
    let mut total: i64 = 0;
    for foil_id in group_ids {
        if let Some(foil) = sim.foils.iter_mut().find(|f| f.id == *foil_id) {
            total += foil.electron_delta_since_measure as i64;
            foil.electron_delta_since_measure = 0;
        }
    }
    total
}

fn main() {
    let mut scenario: Option<String> = None;
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut settle_fs: f32 = 30_000.0;
    let mut rest_fs: f32 = 5_000.0;
    let mut currents: Vec<f32> = vec![1e-5, 3e-5, 1e-4, 3e-4, 1e-3];
    let mut output: PathBuf =
        PathBuf::from("doe_results/eis_validation/galvanostatic_amplitude.csv");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                scenario = args.get(i).cloned();
            }
            "--seed" => {
                i += 1;
                seed = args
                    .get(i)
                    .and_then(|s| parse_u64_flexible(s))
                    .unwrap_or_else(|| {
                        eprintln!("--seed expects a u64");
                        print_usage_and_exit();
                    });
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(
                    || {
                        eprintln!("--equilibrate-fs expects a float");
                        print_usage_and_exit();
                    },
                );
            }
            "--settle-fs" => {
                i += 1;
                settle_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--settle-fs expects a float");
                    print_usage_and_exit();
                });
            }
            "--rest-fs" => {
                i += 1;
                rest_fs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--rest-fs expects a float");
                    print_usage_and_exit();
                });
            }
            "--currents" => {
                i += 1;
                currents = args
                    .get(i)
                    .map(|s| {
                        s.split(',')
                            .map(|t| t.parse::<f32>().expect("current must be float"))
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        eprintln!("--currents expects a comma-separated list of floats");
                        print_usage_and_exit();
                    });
            }
            "--output" => {
                i += 1;
                output = PathBuf::from(args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("--output expects a path");
                    print_usage_and_exit();
                }));
            }
            "--help" | "-h" => print_usage_and_exit(),
            other => {
                eprintln!("Unknown argument: {}", other);
                print_usage_and_exit();
            }
        }
        i += 1;
    }
    let scenario_path = scenario.unwrap_or_else(|| {
        eprintln!("--scenario is required");
        print_usage_and_exit();
    });

    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    fastrand::seed(seed);

    let config = InitConfig::load_from_file(&scenario_path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}", scenario_path, e);
        std::process::exit(1);
    });
    let (full_w, full_h) = config
        .simulation
        .as_ref()
        .map(|s| s.domain_size())
        .unwrap_or_else(|| {
            eprintln!("scenario must specify [simulation] domain_width/domain_height");
            std::process::exit(1);
        });

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    println!("Scenario: {}", scenario_path);
    println!("Seed: 0x{:X}", seed);

    for rect in &config.particles.metal_rectangles {
        let species = rect.to_species().expect("invalid species in metal_rectangles");
        let body = template_body(species);
        let (origin_x, origin_y) = rect.to_origin_coords();
        handle_command(
            SimCommand::AddRectangle {
                body,
                x: origin_x,
                y: origin_y,
                width: rect.width,
                height: rect.height,
            },
            &mut sim,
        );
    }
    for foil in &config.particles.foil_rectangles {
        let (origin_x, origin_y) = foil.to_origin_coords();
        handle_command(
            SimCommand::AddFoil {
                width: foil.width,
                height: foil.height,
                x: origin_x,
                y: origin_y,
                particle_radius: Species::FoilMetal.radius(),
                current: foil.current,
            },
            &mut sim,
        );
    }
    for entry in &config.particles.random {
        let species = entry.to_species().expect("invalid species in random");
        let body = template_body(species);
        handle_command(
            SimCommand::AddRandom {
                body,
                count: entry.count,
                domain_width: full_w,
                domain_height: full_h,
            },
            &mut sim,
        );
    }

    println!("Total bodies: {}, foils: {}", sim.bodies.len(), sim.foils.len());

    let mut group_a: Vec<u64> = Vec::new();
    let mut group_b: Vec<u64> = Vec::new();
    for foil in &sim.foils {
        let mut cx = 0.0f32;
        let mut cn = 0.0f32;
        for bid in &foil.body_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) {
                cx += b.pos.x;
                cn += 1.0;
            }
        }
        if cn > 0.0 {
            cx /= cn;
        }
        if cx < 0.0 {
            group_a.push(foil.id);
        } else {
            group_b.push(foil.id);
        }
    }
    handle_command(
        SimCommand::SetFoilGroups {
            group_a: group_a.clone(),
            group_b: group_b.clone(),
        },
        &mut sim,
    );
    println!("Group A (x<0): {:?}, Group B (x>0): {:?}", group_a, group_b);

    let dt = sim.dt;
    let k = sim.config.coulomb_constant;

    let n_eq = (equilibrate_fs / dt) as usize;
    println!("\nEquilibrating {} fs ({} steps)...", equilibrate_fs, n_eq);
    for _ in 0..n_eq {
        sim.step();
    }

    let n_settle = (settle_fs / dt) as usize;
    let n_record_start = n_settle / 2;

    let measure = |sim: &mut Simulation,
                   group_a: &[u64],
                   group_b: &[u64]|
     -> (f32, f32, f32) {
        let mut delta_a: i64 = 0;
        let mut delta_b: i64 = 0;
        let mut sum_v: f64 = 0.0;
        let mut record_steps = 0u64;
        for i_step in 0..n_settle {
            sim.step();
            let step_a = read_and_clear_group_delta(sim, group_a);
            let step_b = read_and_clear_group_delta(sim, group_b);
            if i_step >= n_record_start {
                delta_a += step_a;
                delta_b += step_b;
                sum_v += calculate_cell_voltage(&sim.bodies, &sim.foils, k) as f64;
                record_steps += 1;
            }
        }
        let window_fs = record_steps as f32 * dt;
        let i_a = delta_a as f32 / window_fs;
        let i_b = delta_b as f32 / window_fs;
        let v_cell = if record_steps > 0 {
            (sum_v / record_steps as f64) as f32
        } else {
            0.0
        };
        (i_a, i_b, v_cell)
    };

    let mut results: Vec<(f32, f32, f32, f32)> = Vec::new();

    println!("Baseline (zero current)...");
    let (b_ia, b_ib, b_v) = measure(&mut sim, &group_a, &group_b);
    println!(
        "  baseline I_a={:+.4e} e/fs  I_b={:+.4e} e/fs  V_cell={:+.2} mV",
        b_ia, b_ib, b_v * 1000.0
    );
    results.push((0.0, b_ia, b_ib, b_v));

    let n_rest = (rest_fs / dt) as usize;
    for &i_amp in &currents {
        println!(
            "\nApplying I = {:+.4e} e/fs (group A = +I, group B = -I)",
            i_amp
        );
        handle_command(
            SimCommand::ConventionalSetCurrent { current: i_amp },
            &mut sim,
        );

        let (i_a, i_b, v_cell) = measure(&mut sim, &group_a, &group_b);
        results.push((i_amp, i_a, i_b, v_cell));
        println!(
            "  I_a_actual={:+.4e}  I_b_actual={:+.4e} e/fs   V_cell={:+.2} mV",
            i_a,
            i_b,
            v_cell * 1000.0
        );

        // Reset and rest before next current.
        handle_command(SimCommand::ConventionalSetCurrent { current: 0.0 }, &mut sim);
        for _ in 0..n_rest {
            sim.step();
            let _ = read_and_clear_group_delta(&mut sim, &group_a);
            let _ = read_and_clear_group_delta(&mut sim, &group_b);
        }
    }

    println!("\n=== Results ===");
    println!(
        "{:>13}  {:>14}  {:>14}  {:>12}  {:>15}",
        "I_set (e/fs)", "I_a actual", "I_b actual", "V_cell (mV)", "|Z|_DC (V*fs/e)"
    );
    println!(
        "{:>13}  {:>14}  {:>14}  {:>12}  {:>15}",
        "─", "─", "─", "─", "─"
    );
    for (i_set, ia, ib, v) in &results {
        let label = if *i_set == 0.0 {
            "baseline".to_string()
        } else {
            format!("{:+.3e}", i_set)
        };
        let z_dc = if i_set.abs() > 0.0 {
            format!("{:.2e}", v.abs() / i_set.abs())
        } else {
            "—".to_string()
        };
        println!(
            "{:>13}  {:>+14.4e}  {:>+14.4e}  {:>+12.2}  {:>15}",
            label,
            ia,
            ib,
            v * 1000.0,
            z_dc
        );
    }

    if let Some(parent) = output.parent() {
        create_dir_all(parent).ok();
    }
    let mut f = File::create(&output).expect("Failed to create output file");
    writeln!(
        f,
        "# scenario={} seed=0x{:X} equilibrate_fs={} settle_fs={} rest_fs={}",
        scenario_path, seed, equilibrate_fs, settle_fs, rest_fs
    )
    .ok();
    writeln!(f, "# group_a={:?} group_b={:?}", group_a, group_b).ok();
    writeln!(f, "i_set_e_per_fs,i_a_actual,i_b_actual,v_cell_v").ok();
    for (i_set, ia, ib, v) in &results {
        writeln!(f, "{},{},{},{}", i_set, ia, ib, v).ok();
    }
    println!("\nWrote {}", output.display());
}
