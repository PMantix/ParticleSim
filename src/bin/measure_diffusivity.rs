//! measure_diffusivity — Phase 0a of docs/EIS_AMPLITUDE_STUDY_PLAN.md.
//!
//! Loads a bulk-electrolyte scenario from TOML, builds a Simulation headlessly
//! (no SimCommand channel), equilibrates, then measures the Li+ self-diffusion
//! coefficient via mean-squared displacement of ions whose initial positions lie
//! in the bulk-window (middle fraction of the domain), with center-of-mass drift
//! removed. Reports D in Å²/fs and m²/s along with the linear-fit R².

use particle_sim::app::spawn::add_random;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::simulation::utils::compute_liquid_temperature;
use particle_sim::simulation::Simulation;
use std::collections::{BTreeMap, HashMap};
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: measure_diffusivity --scenario <path.toml> \
         [--seed <u64>] \
         [--equilibrate-fs <float>] \
         [--log-every-fs <float>] \
         [--measure-fs <float>] \
         [--sample-every-fs <float>] \
         [--bulk-window-fraction <float>]"
    );
    std::process::exit(2);
}

/// 2D self-diffusion coefficient: <r²>(t) = 4 D t.
const MSD_DIFFUSION_DIM_FACTOR: f32 = 4.0;

/// 1 Å²/fs in m²/s. (1 Å = 1e-10 m, 1 fs = 1e-15 s.)
const ANGSTROM2_PER_FS_TO_M2_PER_S: f64 = 1e-5;

#[derive(Clone, Copy)]
struct LinearFit {
    slope: f32,
    intercept: f32,
    r2: f32,
}

/// Ordinary least-squares fit of y = slope*x + intercept. Returns r²
/// against the variance of y (1.0 = perfect fit, can be negative if the model
/// is worse than mean-only).
fn linear_fit(samples: &[(f32, f32)]) -> Option<LinearFit> {
    if samples.len() < 2 {
        return None;
    }
    let n = samples.len() as f32;
    let mean_x: f32 = samples.iter().map(|(x, _)| x).sum::<f32>() / n;
    let mean_y: f32 = samples.iter().map(|(_, y)| y).sum::<f32>() / n;
    let ss_xy: f32 = samples.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();
    let ss_xx: f32 = samples.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
    if ss_xx <= 0.0 {
        return None;
    }
    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    let ss_tot: f32 = samples.iter().map(|(_, y)| (y - mean_y).powi(2)).sum();
    let ss_res: f32 = samples
        .iter()
        .map(|(x, y)| {
            let pred = slope * x + intercept;
            (y - pred).powi(2)
        })
        .sum();
    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 1.0 };
    Some(LinearFit { slope, intercept, r2 })
}

/// Parse a u64 from either decimal or `0x`-prefixed hex (e.g. `0xDEADBEEF`).
/// Plain `s.parse::<u64>()` only accepts base-10.
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

fn main() {
    let mut scenario: Option<String> = None;
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut log_every_fs: f32 = 5_000.0;
    let mut measure_fs: f32 = 50_000.0;
    let mut sample_every_fs: f32 = 1_000.0;
    let mut bulk_window_fraction: f32 = 0.5;

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
            "--log-every-fs" => {
                i += 1;
                log_every_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--log-every-fs expects a float");
                        print_usage_and_exit();
                    });
            }
            "--measure-fs" => {
                i += 1;
                measure_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--measure-fs expects a float");
                        print_usage_and_exit();
                    });
            }
            "--sample-every-fs" => {
                i += 1;
                sample_every_fs = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--sample-every-fs expects a positive float");
                        print_usage_and_exit();
                    });
            }
            "--bulk-window-fraction" => {
                i += 1;
                bulk_window_fraction = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--bulk-window-fraction expects a float in (0, 1]");
                        print_usage_and_exit();
                    });
                if !(bulk_window_fraction > 0.0 && bulk_window_fraction <= 1.0) {
                    eprintln!("--bulk-window-fraction must be in (0, 1]");
                    std::process::exit(2);
                }
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

    fastrand::seed(seed);

    let config = InitConfig::load_from_file(&scenario_path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}", scenario_path, e);
        std::process::exit(1);
    });
    let (full_width, full_height) = config
        .simulation
        .as_ref()
        .map(|s| s.domain_size())
        .unwrap_or_else(|| {
            eprintln!("scenario must specify [simulation] domain_width/domain_height");
            std::process::exit(1);
        });

    let mut sim = Simulation::new();
    let half_w = full_width / 2.0;
    let half_h = full_height / 2.0;
    sim.domain_width = half_w;
    sim.domain_height = half_h;
    sim.cell_list.update_domain_size(half_w, half_h);

    println!("Scenario: {}", scenario_path);
    println!("Seed: 0x{:X}", seed);
    println!("Domain: {} x {}  (half: {} x {})", full_width, full_height, half_w, half_h);
    println!();

    for entry in &config.particles.random {
        let species = match entry.to_species() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping invalid species: {}", e);
                continue;
            }
        };
        let body = template_body(species);
        // add_random expects FULL domain dims (not half).
        add_random(&mut sim, body, entry.count, full_width, full_height);
    }

    let mut hist: BTreeMap<String, usize> = BTreeMap::new();
    for b in &sim.bodies {
        *hist.entry(format!("{:?}", b.species)).or_insert(0) += 1;
    }
    println!("Total bodies: {}", sim.bodies.len());
    println!("Species histogram:");
    for (species, count) in &hist {
        println!("  {}: {}", species, count);
    }

    let dt = sim.dt;
    let initial_temp = compute_liquid_temperature(&sim.bodies);
    let total_steps = (equilibrate_fs / dt).max(0.0) as usize;
    println!();
    println!(
        "Equilibrating for {} fs (dt={} fs, ~{} steps); log every {} fs",
        equilibrate_fs, dt, total_steps, log_every_fs
    );
    println!(
        "[t={:>7.0} fs] T_liquid = {:>7.2} K  (frame {})",
        0.0, initial_temp, sim.frame
    );

    let mut sim_time_fs: f32 = 0.0;
    let mut next_log_time: f32 = if log_every_fs > 0.0 {
        log_every_fs
    } else {
        f32::INFINITY
    };
    while sim_time_fs + 0.5 * dt < equilibrate_fs {
        sim.step();
        sim_time_fs += dt;
        if sim_time_fs >= next_log_time {
            let t = compute_liquid_temperature(&sim.bodies);
            println!(
                "[t={:>7.0} fs] T_liquid = {:>7.2} K  (frame {})",
                sim_time_fs, t, sim.frame
            );
            next_log_time += log_every_fs;
        }
    }

    let final_temp = compute_liquid_temperature(&sim.bodies);
    println!();
    println!(
        "Equilibration complete: T_liquid {:.2} K -> {:.2} K  ({} steps, {} fs simulated)",
        initial_temp, final_temp, sim.frame, sim_time_fs
    );

    // ---- MSD measurement ----
    let bulk_x = bulk_window_fraction * half_w;
    let bulk_y = bulk_window_fraction * half_h;
    let tracked: Vec<(u64, Vec2)> = sim
        .bodies
        .iter()
        .filter(|b| b.species == Species::LithiumIon)
        .filter(|b| b.pos.x.abs() < bulk_x && b.pos.y.abs() < bulk_y)
        .map(|b| (b.id, b.pos))
        .collect();
    if tracked.is_empty() {
        eprintln!("No Li+ ions found in bulk window ({}× domain). Aborting.", bulk_window_fraction);
        std::process::exit(1);
    }

    println!();
    println!(
        "Measuring D_Li+ for {} fs (sample every {} fs); tracking {} Li+ in bulk window ±{:.1} × ±{:.1} Å",
        measure_fs, sample_every_fs, tracked.len(), bulk_x, bulk_y
    );
    println!("[m_t=     0 fs] <r²> =       0.00 Å²  (T_liquid {:.2} K)", final_temp);

    // (relative time, MSD in Å²) — relative to start of measurement window
    let mut samples: Vec<(f32, f32)> = vec![(0.0, 0.0)];
    let measurement_start_fs = sim_time_fs;
    let mut next_sample_time = measurement_start_fs + sample_every_fs;
    let measurement_end_fs = measurement_start_fs + measure_fs;

    while sim_time_fs + 0.5 * dt < measurement_end_fs {
        sim.step();
        sim_time_fs += dt;
        if sim_time_fs + 0.5 * dt >= next_sample_time {
            // Build id -> current position lookup for this sample
            let pos_by_id: HashMap<u64, Vec2> =
                sim.bodies.iter().map(|b| (b.id, b.pos)).collect();
            // Displacements for surviving tracked ions
            let displacements: Vec<Vec2> = tracked
                .iter()
                .filter_map(|(id, r0)| pos_by_id.get(id).map(|r| *r - *r0))
                .collect();
            if displacements.is_empty() {
                eprintln!("All tracked Li+ disappeared (unexpected). Aborting.");
                std::process::exit(1);
            }
            let n = displacements.len() as f32;
            let com_drift: Vec2 = displacements.iter().copied().fold(Vec2::zero(), |a, b| a + b) / n;
            let msd: f32 = displacements
                .iter()
                .map(|d| (*d - com_drift).mag_sq())
                .sum::<f32>()
                / n;
            let m_t = sim_time_fs - measurement_start_fs;
            let t_liquid = compute_liquid_temperature(&sim.bodies);
            println!(
                "[m_t={:>6.0} fs] <r²> = {:>10.2} Å²  (T_liquid {:.2} K, tracked {})",
                m_t,
                msd,
                t_liquid,
                displacements.len()
            );
            samples.push((m_t, msd));
            next_sample_time += sample_every_fs;
        }
    }

    // ---- Linear fit ----
    println!();
    let fit = match linear_fit(&samples) {
        Some(f) => f,
        None => {
            eprintln!("Insufficient data for linear fit ({} samples)", samples.len());
            std::process::exit(1);
        }
    };
    let d_a2_per_fs = fit.slope / MSD_DIFFUSION_DIM_FACTOR;
    let d_m2_per_s = (d_a2_per_fs as f64) * ANGSTROM2_PER_FS_TO_M2_PER_S;
    println!("Linear fit of <r²>(t):");
    println!("  slope     = {:.4e} Å²/fs", fit.slope);
    println!("  intercept = {:.4e} Å²", fit.intercept);
    println!("  R²        = {:.4}", fit.r2);
    println!();
    println!("D_Li+ = slope / 4 = {:.4e} Å²/fs", d_a2_per_fs);
    println!("       = {:.4e} m²/s", d_m2_per_s);
    println!();
    if fit.r2 < 0.95 {
        println!(
            "Warning: linear-fit R² = {:.3} < 0.95 — MSD may be saturating (boundary clamping?), \
             non-linear (sub-diffusive regime?), or under-sampled. Inspect the trace.",
            fit.r2
        );
    }
}
