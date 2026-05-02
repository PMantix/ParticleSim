//! find_amplitude_floor — chronoamperometric sweep to identify the minimum
//! overpotential at which current actually flows.
//!
//! Phase 1.2 prerequisite for `docs/EIS_AMPLITUDE_STUDY_PLAN.md`. The result
//! tells us the floor below which an applied potential is too small to drive
//! electron transfer in this simulator — anything smaller produces only thermal
//! noise and is not safe to use as a small-signal EIS perturbation amplitude.
//!
//! Protocol per amplitude Δ:
//!   1. Set group-A foils to target_ratio = 1+Δ (and B to 1−Δ via the existing
//!      complementary-overpotential path in scenario.rs::ConventionalSetOverpotential).
//!   2. Hold for --settle-fs and record the mean foil current over the LAST
//!      half of the hold window (so the transient settles first).
//!   3. Reset to neutral (target_ratio = 1.0) and rest for --rest-fs before the
//!      next amplitude.
//!
//! Output: stdout table + CSV at --output (default
//! doe_results/eis_validation/amplitude_floor.csv).

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
        "Usage: find_amplitude_floor --scenario <path.toml> \
         [--seed <u64>] \
         [--equilibrate-fs <float>] \
         [--settle-fs <float>] \
         [--rest-fs <float>] \
         [--amplitudes <a1,a2,...>] \
         [--output <path.csv>] \
         [--append]\n\
         \n\
         --append: skip the baseline measurement and append new (Δ, I_a, I_b) rows\n\
         to an existing --output CSV. Useful for extending an earlier sweep upward\n\
         (e.g. --amplitudes 0.5,1.0,2.0 --append) without re-running the low-amplitude\n\
         points. Each invocation still re-equilibrates from a fresh sim, so the\n\
         appended rows reflect a separate equilibrated state — supply --seed for\n\
         determinism if cross-run comparison matters."
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

/// Read-and-clear the per-foil `electron_delta_since_measure` accumulator across
/// a group, returning the total electron transfer count for the group since the
/// last call. This is the same quantity the EIS lock-in (eis.rs) uses as
/// `actual_electron_delta` — it counts net Butler-Volmer + PID-injected electron
/// transfers at the foil bodies. At PID steady state in overpotential mode this
/// is non-zero (PID injects electrons at the rate they're consumed) — unlike a
/// raw foil-electron-count snapshot, which tends to zero at steady state.
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
    let mut settle_fs: f32 = 15_000.0;
    let mut rest_fs: f32 = 5_000.0;
    let mut amplitudes: Vec<f32> = vec![0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3];
    let mut output: PathBuf = PathBuf::from("doe_results/eis_validation/amplitude_floor.csv");
    let mut append_mode = false;

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
                        eprintln!("--seed expects a u64 (decimal or 0x-prefixed hex)");
                        print_usage_and_exit();
                    });
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--equilibrate-fs expects a float");
                        print_usage_and_exit();
                    });
            }
            "--settle-fs" => {
                i += 1;
                settle_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--settle-fs expects a float");
                        print_usage_and_exit();
                    });
            }
            "--rest-fs" => {
                i += 1;
                rest_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--rest-fs expects a float");
                        print_usage_and_exit();
                    });
            }
            "--amplitudes" => {
                i += 1;
                amplitudes = args
                    .get(i)
                    .map(|s| {
                        s.split(',')
                            .map(|t| t.parse::<f32>().expect("amplitude must be float"))
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        eprintln!("--amplitudes expects a comma-separated list of floats");
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
            "--append" => {
                append_mode = true;
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

    // Some SimCommand handlers may consult SIM_COMMAND_SENDER to forward; install
    // a dummy sender so they don't panic. Receiver is dropped (commands silently
    // discarded), which is fine because we drive the simulation directly.
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
    let half_w = full_w / 2.0;
    let half_h = full_h / 2.0;
    sim.domain_width = half_w;
    sim.domain_height = half_h;
    sim.cell_list.update_domain_size(half_w, half_h);

    println!("Scenario: {}", scenario_path);
    println!("Seed: 0x{:X}", seed);
    println!("Domain: {} x {}", full_w, full_h);

    // Spawn metal rectangles via the same SimCommand handler the GUI uses.
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

    // Foils.
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

    // Random electrolyte particles.
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

    println!(
        "Total bodies: {}, foils: {}",
        sim.bodies.len(),
        sim.foils.len()
    );

    // Group foils by x: A = left half, B = right half.
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
    println!("Group A foils (x<0): {:?}", group_a);
    println!("Group B foils (x>0): {:?}", group_b);

    let dt = sim.dt;
    let k = sim.config.coulomb_constant;

    // Equilibrate (no charging applied).
    let n_eq = (equilibrate_fs / dt) as usize;
    println!("\nEquilibrating {} fs ({} steps, no charging)...", equilibrate_fs, n_eq);
    for _ in 0..n_eq {
        sim.step();
    }

    let n_settle = (settle_fs / dt) as usize;
    let n_record_start = n_settle / 2;

    // Helper closure: hold for n_settle steps; over the SECOND HALF of the hold,
    // sum each group's electron_delta_since_measure (read-and-clear each step
    // matching the EIS lock-in pattern in eis.rs:1137) and time-average
    // calculate_cell_voltage. Returns (I_a in e/fs, I_b in e/fs, V_cell in V).
    let measure = |sim: &mut Simulation, group_a: &[u64], group_b: &[u64]| -> (f32, f32, f32) {
        let mut delta_a: i64 = 0;
        let mut delta_b: i64 = 0;
        let mut sum_v: f64 = 0.0;
        let mut record_steps = 0u64;
        for i_step in 0..n_settle {
            sim.step();
            // Always clear the accumulator each step so it never grows unbounded;
            // only count toward the recording sum if we're in the recording half.
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

    // Baseline (no charging) — skipped in --append mode, since the existing CSV
    // already contains a baseline row from the original run.
    let mut results: Vec<(f32, f32, f32, f32)> = Vec::new();
    if !append_mode {
        println!("Baseline measurement (no charging applied)...");
        let (base_i_a, base_i_b, base_v) = measure(&mut sim, &group_a, &group_b);
        println!(
            "  baseline I_a = {:+.4e} e/fs  I_b = {:+.4e} e/fs  V_cell = {:+.2} mV",
            base_i_a,
            base_i_b,
            base_v * 1000.0
        );
        results.push((0.0, base_i_a, base_i_b, base_v));
    } else {
        println!("--append mode: skipping baseline measurement.");
    }

    // Sweep amplitudes.
    let n_rest = (rest_fs / dt) as usize;
    for &amp in &amplitudes {
        let target_ratio = 1.0 + amp;
        println!(
            "\nApplying Δ = {:.4} (target_ratio_A = {:.4}, target_ratio_B = {:.4})",
            amp,
            target_ratio,
            2.0 - target_ratio
        );
        handle_command(
            SimCommand::ConventionalSetOverpotential { target_ratio },
            &mut sim,
        );

        let (i_a, i_b, v_cell) = measure(&mut sim, &group_a, &group_b);
        results.push((amp, i_a, i_b, v_cell));
        println!(
            "  Δ = {:.4}  I_a = {:+.4e}  I_b = {:+.4e} e/fs   V_cell = {:+.2} mV",
            amp, i_a, i_b, v_cell * 1000.0
        );

        // Reset to neutral and rest before next amplitude. We also clear
        // electron_delta_since_measure here so the rest period doesn't bleed
        // into the next amplitude's recording window.
        handle_command(
            SimCommand::ConventionalSetOverpotential { target_ratio: 1.0 },
            &mut sim,
        );
        for _ in 0..n_rest {
            sim.step();
            let _ = read_and_clear_group_delta(&mut sim, &group_a);
            let _ = read_and_clear_group_delta(&mut sim, &group_b);
        }
    }

    // Print summary table.
    println!("\n=== Results ===");
    println!(
        "{:>10}  {:>14}  {:>14}  {:>12}",
        "Δ", "I_a (e/fs)", "I_b (e/fs)", "V_cell (mV)"
    );
    println!("{:>10}  {:>14}  {:>14}  {:>12}", "─", "─", "─", "─");
    for (amp, ia, ib, v) in &results {
        let label = if *amp == 0.0 {
            "baseline".to_string()
        } else {
            format!("{:.4}", amp)
        };
        println!(
            "{:>10}  {:>+14.4e}  {:>+14.4e}  {:>+12.2}",
            label,
            ia,
            ib,
            v * 1000.0
        );
    }

    // Write CSV.
    if let Some(parent) = output.parent() {
        create_dir_all(parent).ok();
    }
    let already_exists = output.exists();
    let write_header = !(append_mode && already_exists);
    let mut f = if append_mode && already_exists {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&output)
            .expect("Failed to open output file for append")
    } else {
        File::create(&output).expect("Failed to create output file")
    };
    if write_header {
        writeln!(
            f,
            "# scenario={} seed=0x{:X} equilibrate_fs={} settle_fs={} rest_fs={}",
            scenario_path, seed, equilibrate_fs, settle_fs, rest_fs
        )
        .ok();
        writeln!(f, "# group_a={:?} group_b={:?}", group_a, group_b).ok();
        writeln!(f, "delta,i_a_e_per_fs,i_b_e_per_fs,v_cell_v").ok();
    } else {
        // Appending to existing file — record the new run's parameters as a comment
        // so future readers can tell where the rows came from.
        writeln!(
            f,
            "# appended run: seed=0x{:X} equilibrate_fs={} settle_fs={} rest_fs={}",
            seed, equilibrate_fs, settle_fs, rest_fs
        )
        .ok();
    }
    for (amp, ia, ib, v) in &results {
        writeln!(f, "{},{},{},{}", amp, ia, ib, v).ok();
    }
    println!(
        "\n{} {}",
        if append_mode && already_exists { "Appended to" } else { "Wrote" },
        output.display()
    );
}
