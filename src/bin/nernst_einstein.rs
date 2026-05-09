//! nernst_einstein — Phase 2 dimensionless test 2.3.
//!
//! See docs/PHYSICS_VALIDATION_PLAN.md §Test 2.3.
//!
//! Compares the measured ionic conductivity σ to the Nernst-Einstein
//! prediction:
//!     σ_NE = (1/k_B·T) · Σ qᵢ² · nᵢ · Dᵢ
//!
//! The Haven ratio σ_NE / σ_actual ∈ [1, ~3] for real liquid electrolytes;
//! a value far outside this band (in either direction) indicates the ion
//! correlations or transport mechanism are qualitatively wrong.
//!
//! Two phases:
//!   A) D measurement — thermostat ON, no applied field. MSD of Li⁺ and
//!      anion gives D_+ and D_- via 2D <r²>(t) = 4·D·t (with center-of-mass
//!      drift removed).
//!   B) σ measurement — thermostat ON, small uniform background E-field
//!      applied via FIELD_MAGNITUDE/FIELD_DIRECTION. Steady-state mean
//!      drift velocities of Li⁺ and anion give J = Σ qᵢ·nᵢ·⟨vᵢ⟩, and
//!      σ = J / E.
//!
//! Test value: ratio σ_measured / σ_NE. Pass if in [0.5, 2.0].
//!
//! Pure measurement — does not modify any physics-parameter constant.
//! FIELD_MAGNITUDE is restored to 0 at the end.

use particle_sim::app::spawn::add_random;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::renderer::state::{FIELD_DIRECTION, FIELD_MAGNITUDE};
use particle_sim::simulation::Simulation;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: nernst_einstein \
         [--scenario <toml>] [--seed <u64>] \
         [--equilibrate-fs <f>] \
         [--d-measure-fs <f>] [--d-sample-every-fs <f>] \
         [--sigma-measure-fs <f>] [--sigma-sample-every-fs <f>] \
         [--sigma-equilibrate-fs <f>] \
         [--field-magnitude <f>] [--out-dir <path>]"
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

fn build_sim(scenario_path: &str, full_w: f32, full_h: f32, config: &InitConfig) -> Simulation {
    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);
    if !config.particles.metal_rectangles.is_empty()
        || !config.particles.foil_rectangles.is_empty()
    {
        eprintln!(
            "nernst_einstein: scenario must be bulk-only; got {} metal rect, {} foils",
            config.particles.metal_rectangles.len(),
            config.particles.foil_rectangles.len()
        );
        std::process::exit(3);
    }
    for entry in &config.particles.random {
        let species = entry.to_species().unwrap_or_else(|e| {
            eprintln!("random species: {}", e);
            std::process::exit(3);
        });
        let body = template_body(species);
        add_random(&mut sim, body, entry.count, full_w, full_h);
    }
    let _ = scenario_path; // silence unused (kept for future signature changes)
    sim
}

fn linear_fit(samples: &[(f32, f32)]) -> Option<(f32, f32, f32)> {
    if samples.len() < 2 {
        return None;
    }
    let n = samples.len() as f32;
    let mean_x: f32 = samples.iter().map(|(x, _)| x).sum::<f32>() / n;
    let mean_y: f32 = samples.iter().map(|(_, y)| y).sum::<f32>() / n;
    let mut ss_xy = 0.0_f32;
    let mut ss_xx = 0.0_f32;
    for (x, y) in samples {
        ss_xy += (x - mean_x) * (y - mean_y);
        ss_xx += (x - mean_x).powi(2);
    }
    if ss_xx <= 0.0 {
        return None;
    }
    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    let mut ss_res = 0.0_f32;
    let mut ss_tot = 0.0_f32;
    for (x, y) in samples {
        let pred = slope * x + intercept;
        ss_res += (y - pred).powi(2);
        ss_tot += (y - mean_y).powi(2);
    }
    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 1.0 };
    Some((slope, intercept, r2))
}

/// Compute MSD for a species (relative to initial positions, COM-drift removed).
fn compute_msd(sim: &Simulation, target: Species, initial: &[(u64, Vec2)]) -> f32 {
    let pos_by_id: HashMap<u64, Vec2> = sim
        .bodies
        .iter()
        .filter(|b| b.species == target)
        .map(|b| (b.id, b.pos))
        .collect();
    let displacements: Vec<Vec2> = initial
        .iter()
        .filter_map(|(id, r0)| pos_by_id.get(id).map(|r| *r - *r0))
        .collect();
    if displacements.is_empty() {
        return 0.0;
    }
    let n = displacements.len() as f32;
    let com_drift: Vec2 = displacements.iter().copied().fold(Vec2::zero(), |a, b| a + b) / n;
    displacements.iter().map(|d| (*d - com_drift).mag_sq()).sum::<f32>() / n
}

fn main() {
    let mut scenario =
        "measurement_configs/physics_invariants/li_ec_coordination_dense.toml".to_string();
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 10_000.0;
    let mut d_measure_fs: f32 = 30_000.0;
    let mut d_sample_every_fs: f32 = 500.0;
    let mut sigma_measure_fs: f32 = 20_000.0;
    let mut sigma_sample_every_fs: f32 = 200.0;
    let mut sigma_equilibrate_fs: f32 = 20_000.0; // additional equilibration with field on
    // Field magnitude in sim units. 1e-5 ≈ 1e-3 V/Å — small enough that
    // drift is in the linear-response regime (verified empirically: at 1e-4
    // the system overshoots and shows transient counter-drift before
    // settling).
    let mut field_magnitude: f32 = 1.0e-5;
    let mut out_dir = PathBuf::from("doe_results/physics_validation/nernst_einstein");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => {
                i += 1;
                scenario = args[i].clone();
            }
            "--seed" => {
                i += 1;
                seed = parse_u64_flexible(&args[i]).unwrap_or_else(|| {
                    eprintln!("--seed expects a u64");
                    print_usage_and_exit();
                });
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--d-measure-fs" => {
                i += 1;
                d_measure_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--d-sample-every-fs" => {
                i += 1;
                d_sample_every_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--sigma-measure-fs" => {
                i += 1;
                sigma_measure_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--sigma-sample-every-fs" => {
                i += 1;
                sigma_sample_every_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--sigma-equilibrate-fs" => {
                i += 1;
                sigma_equilibrate_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--field-magnitude" => {
                i += 1;
                field_magnitude = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--out-dir" => {
                i += 1;
                out_dir = PathBuf::from(args[i].clone());
            }
            "--help" | "-h" => print_usage_and_exit(),
            other => {
                eprintln!("Unknown arg: {}", other);
                print_usage_and_exit();
            }
        }
        i += 1;
    }
    fs::create_dir_all(&out_dir).ok();

    // Ensure no stale field at start
    *FIELD_MAGNITUDE.lock() = 0.0;
    *FIELD_DIRECTION.lock() = 0.0;

    let kbt_simenergy =
        particle_sim::units::BOLTZMANN_CONSTANT as f64 * 300.0; // amu·Å²/fs²
    let kbt_volts = particle_sim::config::BV_OVERPOTENTIAL_SCALE as f64;

    // ============================================================
    // PHASE A — D₊ and D₋ from MSD
    // ============================================================
    println!("=========================================");
    println!("Phase A — D measurement (no field, thermostat ON)");
    println!("=========================================");
    fastrand::seed(seed);
    let config = InitConfig::load_from_file(&scenario).unwrap_or_else(|e| {
        eprintln!("Failed to load scenario {}: {}", scenario, e);
        std::process::exit(3);
    });
    let (full_w, full_h) = config
        .simulation
        .as_ref()
        .map(|s| s.domain_size())
        .unwrap();
    let area = (full_w * full_h) as f64;
    let mut sim_a = build_sim(&scenario, full_w, full_h, &config);
    let dt = sim_a.dt;

    let n_eq = (equilibrate_fs / dt) as usize;
    println!(
        "  scenario={}, seed=0x{:X}, bodies={}, domain {}×{} Å²",
        scenario,
        seed,
        sim_a.bodies.len(),
        full_w,
        full_h
    );
    println!(
        "  equilibrate {} fs ({} steps), measure {} fs (sample {} fs)",
        equilibrate_fs, n_eq, d_measure_fs, d_sample_every_fs
    );
    print!("  equilibrating... ");
    use std::io::Write as _;
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for _ in 0..n_eq {
        sim_a.step();
    }
    println!("done ({:.1} s)", t0.elapsed().as_secs_f32());

    let li_initial: Vec<(u64, Vec2)> = sim_a
        .bodies
        .iter()
        .filter(|b| b.species == Species::LithiumIon)
        .map(|b| (b.id, b.pos))
        .collect();
    let anion_initial: Vec<(u64, Vec2)> = sim_a
        .bodies
        .iter()
        .filter(|b| b.species == Species::ElectrolyteAnion)
        .map(|b| (b.id, b.pos))
        .collect();
    let n_li = li_initial.len();
    let n_anion = anion_initial.len();
    if n_li == 0 || n_anion == 0 {
        eprintln!(
            "scenario must contain Li⁺ and anion; got {} Li, {} anion",
            n_li, n_anion
        );
        std::process::exit(3);
    }
    let n_li_density = n_li as f64 / area;
    let n_anion_density = n_anion as f64 / area;

    let n_d_meas = (d_measure_fs / dt) as usize;
    let d_stride = (d_sample_every_fs / dt).max(1.0) as usize;

    let mut samples_msd: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, 0.0)];
    let mut steps_since_sample: usize = 0;
    print!("  sampling MSD... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for step in 1..=n_d_meas {
        sim_a.step();
        steps_since_sample += 1;
        if steps_since_sample >= d_stride || step == n_d_meas {
            let t_fs = step as f32 * dt;
            let msd_li = compute_msd(&sim_a, Species::LithiumIon, &li_initial);
            let msd_anion = compute_msd(&sim_a, Species::ElectrolyteAnion, &anion_initial);
            samples_msd.push((t_fs, msd_li, msd_anion));
            steps_since_sample = 0;
        }
    }
    println!(
        "done ({:.1} s, {} samples)",
        t0.elapsed().as_secs_f32(),
        samples_msd.len()
    );

    let li_pairs: Vec<(f32, f32)> = samples_msd.iter().map(|s| (s.0, s.1)).collect();
    let anion_pairs: Vec<(f32, f32)> = samples_msd.iter().map(|s| (s.0, s.2)).collect();
    let li_fit = linear_fit(&li_pairs);
    let anion_fit = linear_fit(&anion_pairs);

    let (d_li, li_r2) = match li_fit {
        Some((slope, _intercept, r2)) => (slope as f64 / 4.0, r2 as f64),
        None => {
            eprintln!("D_Li fit failed");
            std::process::exit(1);
        }
    };
    let (d_anion, anion_r2) = match anion_fit {
        Some((slope, _intercept, r2)) => (slope as f64 / 4.0, r2 as f64),
        None => {
            eprintln!("D_anion fit failed");
            std::process::exit(1);
        }
    };

    println!();
    println!(
        "  D_Li⁺   = {:.4e} Å²/fs   (R² = {:.3})",
        d_li, li_r2
    );
    println!(
        "  D_anion = {:.4e} Å²/fs   (R² = {:.3})",
        d_anion, anion_r2
    );
    let d_li_m2_per_s = d_li * 1.0e-5; // Å²/fs to m²/s
    let d_anion_m2_per_s = d_anion * 1.0e-5;
    println!(
        "          (= {:.3e} m²/s and {:.3e} m²/s)",
        d_li_m2_per_s, d_anion_m2_per_s
    );

    // Write D-phase CSV
    let d_csv = out_dir.join("msd.csv");
    let mut wtr = std::io::BufWriter::new(fs::File::create(&d_csv).unwrap());
    writeln!(wtr, "t_fs,msd_li_a2,msd_anion_a2").unwrap();
    for s in &samples_msd {
        writeln!(wtr, "{:.4},{:.6},{:.6}", s.0, s.1, s.2).unwrap();
    }
    wtr.flush().ok();

    // ============================================================
    // PHASE B — σ from drift velocity under applied E-field
    // ============================================================
    println!();
    println!("=========================================");
    println!("Phase B — σ measurement (uniform field, thermostat ON)");
    println!("=========================================");
    println!(
        "  E-field magnitude = {:.3e} sim units (~{:.3e} V/Å)",
        field_magnitude,
        field_magnitude as f64 / particle_sim::units::EV_TO_SIM
    );
    println!(
        "  field equilibration = {} fs, measurement = {} fs (sample {} fs)",
        sigma_equilibrate_fs, sigma_measure_fs, sigma_sample_every_fs
    );

    fastrand::seed(seed.wrapping_add(1));
    let mut sim_b = build_sim(&scenario, full_w, full_h, &config);

    print!("  equilibrating (no field, thermostat ON)... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for _ in 0..n_eq {
        sim_b.step();
    }
    println!("done ({:.1} s)", t0.elapsed().as_secs_f32());

    // Disable thermostat so it doesn't damp drift velocity. Without the
    // thermostat the system will heat/cool from non-Hamiltonian forces (per
    // nve_energy_drift findings), but over the modest measurement window the
    // drift signal should outgrow the thermal noise and remain interpretable.
    sim_b.config.temperature = 0.0;
    println!("  thermostat disabled for σ measurement");

    // Apply field and let drift develop
    *FIELD_MAGNITUDE.lock() = field_magnitude;
    *FIELD_DIRECTION.lock() = 0.0;
    let n_sig_eq = (sigma_equilibrate_fs / dt) as usize;
    print!("  equilibrating with field... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for _ in 0..n_sig_eq {
        sim_b.step();
    }
    println!("done ({:.1} s)", t0.elapsed().as_secs_f32());

    let n_sig_meas = (sigma_measure_fs / dt) as usize;
    let sig_stride = (sigma_sample_every_fs / dt).max(1.0) as usize;
    let mut samples_drift: Vec<(f32, f64, f64)> = Vec::new();
    let mut steps_since_sample: usize = 0;

    print!("  sampling drift... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for step in 1..=n_sig_meas {
        sim_b.step();
        steps_since_sample += 1;
        if steps_since_sample >= sig_stride || step == n_sig_meas {
            let t_fs = step as f32 * dt;
            let mut sum_v_li_x = 0.0_f64;
            let mut count_li = 0u64;
            let mut sum_v_an_x = 0.0_f64;
            let mut count_an = 0u64;
            let mut sum_v_all_x = 0.0_f64;
            let mut sum_m_all = 0.0_f64;
            for b in &sim_b.bodies {
                sum_v_all_x += b.vel.x as f64 * b.mass as f64;
                sum_m_all += b.mass as f64;
                if b.species == Species::LithiumIon {
                    sum_v_li_x += b.vel.x as f64;
                    count_li += 1;
                } else if b.species == Species::ElectrolyteAnion {
                    sum_v_an_x += b.vel.x as f64;
                    count_an += 1;
                }
            }
            let v_com_x = if sum_m_all > 0.0 { sum_v_all_x / sum_m_all } else { 0.0 };
            let v_li_x = if count_li > 0 {
                sum_v_li_x / count_li as f64 - v_com_x
            } else {
                0.0
            };
            let v_an_x = if count_an > 0 {
                sum_v_an_x / count_an as f64 - v_com_x
            } else {
                0.0
            };
            samples_drift.push((t_fs, v_li_x, v_an_x));
            steps_since_sample = 0;
        }
    }
    println!(
        "done ({:.1} s, {} samples)",
        t0.elapsed().as_secs_f32(),
        samples_drift.len()
    );

    // Restore field
    *FIELD_MAGNITUDE.lock() = 0.0;

    // Stats over second half
    let split = samples_drift.len() / 2;
    let tail = &samples_drift[split..];
    let mean = |xs: &[f64]| xs.iter().sum::<f64>() / xs.len() as f64;
    let std_dev = |xs: &[f64], m: f64| {
        ((xs.iter().map(|x| (x - m).powi(2)).sum::<f64>()) / xs.len() as f64).sqrt()
    };
    let li_drifts: Vec<f64> = tail.iter().map(|s| s.1).collect();
    let an_drifts: Vec<f64> = tail.iter().map(|s| s.2).collect();
    let mean_v_li = mean(&li_drifts);
    let std_v_li = std_dev(&li_drifts, mean_v_li);
    let mean_v_an = mean(&an_drifts);
    let std_v_an = std_dev(&an_drifts, mean_v_an);

    println!();
    println!(
        "  ⟨v_Li_x⟩    = {:+.4e} ± {:.4e} Å/fs",
        mean_v_li, std_v_li
    );
    println!(
        "  ⟨v_anion_x⟩ = {:+.4e} ± {:.4e} Å/fs",
        mean_v_an, std_v_an
    );

    // J_x = q_Li · n_Li · v_Li + q_anion · n_anion · v_anion
    // Units: e · Å⁻² · Å/fs = e/(Å·fs)
    let j_x = 1.0 * n_li_density * mean_v_li + (-1.0) * n_anion_density * mean_v_an;
    let sigma_measured = j_x / field_magnitude as f64; // e/(Å·fs · sim_field) = e²/(sim_energy·fs)

    // σ_NE = (q²·n·D)/kT, both species
    // Units: e² · Å⁻² · Å²/fs / sim_energy = e²/(sim_energy·fs)
    let sigma_ne_li = 1.0 * n_li_density * d_li / kbt_simenergy;
    let sigma_ne_an = 1.0 * n_anion_density * d_anion / kbt_simenergy;
    let sigma_ne = sigma_ne_li + sigma_ne_an;

    let ratio = if sigma_ne != 0.0 {
        sigma_measured / sigma_ne
    } else {
        f64::NAN
    };

    let target_lo = 0.5_f64;
    let target_hi = 2.0_f64;
    let pass = ratio.is_finite() && (target_lo..=target_hi).contains(&ratio);

    println!();
    println!("--- Conductivity comparison ---");
    println!("  J_x (drift)        = {:+.4e}  e/(Å·fs)", j_x);
    println!("  E_x                = {:.4e}  sim_field", field_magnitude);
    println!(
        "  σ_measured         = {:+.4e}  e²/(sim_energy·fs)",
        sigma_measured
    );
    println!(
        "  σ_NE = q²·n·D/kT   = {:+.4e}  e²/(sim_energy·fs)",
        sigma_ne
    );
    println!(
        "    contributions: σ_NE_Li = {:.4e}, σ_NE_anion = {:.4e}",
        sigma_ne_li, sigma_ne_an
    );
    println!(
        "  ratio σ_meas/σ_NE  = {:.4}      (target band [{:.1}, {:.1}])",
        ratio, target_lo, target_hi
    );
    println!("  PASS = {}", pass);

    // Write drift CSV
    let drift_csv = out_dir.join("drift.csv");
    let mut w2 = std::io::BufWriter::new(fs::File::create(&drift_csv).unwrap());
    writeln!(w2, "t_fs,v_li_x,v_anion_x").unwrap();
    for s in &samples_drift {
        writeln!(w2, "{:.4},{:.6e},{:.6e}", s.0, s.1, s.2).unwrap();
    }
    w2.flush().ok();

    let result = json!({
        "test": "nernst_einstein",
        "value": ratio,
        "value_label": "sigma_measured_over_sigma_nernst_einstein",
        "unit": "dimensionless_ratio",
        "tolerance": {
            "kind": "range",
            "min": target_lo,
            "max": target_hi
        },
        "pass": pass,
        "details": {
            "scenario": scenario,
            "seed": format!("0x{:X}", seed),
            "equilibrate_fs": equilibrate_fs,
            "d_measure_fs": d_measure_fs,
            "d_sample_every_fs": d_sample_every_fs,
            "sigma_equilibrate_fs": sigma_equilibrate_fs,
            "sigma_measure_fs": sigma_measure_fs,
            "sigma_sample_every_fs": sigma_sample_every_fs,
            "field_magnitude_simunits": field_magnitude,
            "n_li": n_li,
            "n_anion": n_anion,
            "n_li_density_per_a2": n_li_density,
            "n_anion_density_per_a2": n_anion_density,
            "domain_w": full_w,
            "domain_h": full_h,
            "kbt_simenergy": kbt_simenergy,
            "kbt_volts_proxy": kbt_volts,
            "d_li_a2_per_fs": d_li,
            "d_anion_a2_per_fs": d_anion,
            "d_li_m2_per_s": d_li_m2_per_s,
            "d_anion_m2_per_s": d_anion_m2_per_s,
            "msd_li_r2": li_r2,
            "msd_anion_r2": anion_r2,
            "mean_v_li_x_a_per_fs": mean_v_li,
            "std_v_li_x_a_per_fs": std_v_li,
            "mean_v_anion_x_a_per_fs": mean_v_an,
            "std_v_anion_x_a_per_fs": std_v_an,
            "j_x": j_x,
            "sigma_measured": sigma_measured,
            "sigma_ne_li": sigma_ne_li,
            "sigma_ne_anion": sigma_ne_an,
            "sigma_ne_total": sigma_ne,
            "ratio_sigma_meas_over_sigma_ne": ratio,
        }
    });
    let result_path = out_dir.join("result.json");
    fs::write(&result_path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
    println!();
    println!("MSD CSV:    {}", d_csv.display());
    println!("Drift CSV:  {}", drift_csv.display());
    println!("Result:     {}", result_path.display());
    // Always exit 0 — measurement test, not gating.
}
