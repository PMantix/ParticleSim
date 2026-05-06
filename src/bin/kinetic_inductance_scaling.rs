//! kinetic_inductance_scaling — Phase 2 dimensionless test 2.4.
//!
//! See docs/PHYSICS_VALIDATION_PLAN.md §Test 2.4.
//!
//! Verifies the Phase-0b similitude prediction τ_KI ∝ L²/D by running the
//! same bulk-electrolyte composition at multiple domain sizes (with particle
//! counts scaled to maintain constant density) and checking:
//!
//!   1. D₊ and D₋ are L-independent (bulk-intrinsic; not boundary-dominated).
//!   2. τ_KI = L²/D scales linearly with L² (R² > 0.9 of τ vs L² fit).
//!
//! This is the "is the simulator self-consistent across system sizes" check.
//! Phase-2 tests 2.1–2.3 found the simulator's transport is *qualitatively*
//! distorted relative to real liquids; this test asks whether the distortion
//! at least scales correctly so that EIS results at different L are
//! mutually consistent.
//!
//! Pure measurement — does not modify any physics-parameter constant.

use particle_sim::app::spawn::add_random;
use particle_sim::body::{Body, Species};
use particle_sim::simulation::Simulation;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: kinetic_inductance_scaling \
         [--ls <comma-list of domain L in Å>] [--seed <u64>] \
         [--equilibrate-fs <f>] [--measure-fs <f>] [--sample-every-fs <f>] \
         [--out-dir <path>]"
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

fn linear_fit_f64(samples: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    if samples.len() < 2 {
        return None;
    }
    let n = samples.len() as f64;
    let mean_x: f64 = samples.iter().map(|(x, _)| x).sum::<f64>() / n;
    let mean_y: f64 = samples.iter().map(|(_, y)| y).sum::<f64>() / n;
    let mut ss_xy = 0.0_f64;
    let mut ss_xx = 0.0_f64;
    for (x, y) in samples {
        ss_xy += (x - mean_x) * (y - mean_y);
        ss_xx += (x - mean_x).powi(2);
    }
    if ss_xx <= 0.0 {
        return None;
    }
    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for (x, y) in samples {
        let pred = slope * x + intercept;
        ss_res += (y - pred).powi(2);
        ss_tot += (y - mean_y).powi(2);
    }
    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 1.0 };
    Some((slope, intercept, r2))
}

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
    let com_drift: Vec2 =
        displacements.iter().copied().fold(Vec2::zero(), |a, b| a + b) / n;
    displacements.iter().map(|d| (*d - com_drift).mag_sq()).sum::<f32>() / n
}

#[derive(Debug, Clone)]
struct LRow {
    l: f64,                 // domain side length, Å
    n_li: usize,
    n_anion: usize,
    n_ec: usize,
    n_dmc: usize,
    d_li: f64,              // Å²/fs
    d_anion: f64,
    msd_li_r2: f64,
    msd_anion_r2: f64,
    tau_ki_li: f64,         // fs
    tau_ki_anion: f64,      // fs
}

fn run_one_l(
    l: f32,
    seed: u64,
    equilibrate_fs: f32,
    measure_fs: f32,
    sample_every_fs: f32,
    out_dir: &PathBuf,
) -> LRow {
    fastrand::seed(seed);

    // Density-scaled counts to match li_ec_coordination_dense.toml
    // (50 Li, 50 anion, 300 EC, 300 DMC in 200×200 = 40000 Å²).
    let area = (l as f64) * (l as f64);
    let area_ratio = area / (200.0 * 200.0);
    let n_li = (50.0 * area_ratio).round().max(4.0) as usize;
    let n_anion = (50.0 * area_ratio).round().max(4.0) as usize;
    let n_ec = (300.0 * area_ratio).round().max(20.0) as usize;
    let n_dmc = (300.0 * area_ratio).round().max(20.0) as usize;

    let mut sim = Simulation::new();
    sim.domain_width = l / 2.0;
    sim.domain_height = l / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    add_random(&mut sim, template_body(Species::LithiumIon), n_li, l, l);
    add_random(
        &mut sim,
        template_body(Species::ElectrolyteAnion),
        n_anion,
        l,
        l,
    );
    add_random(&mut sim, template_body(Species::EC), n_ec, l, l);
    add_random(&mut sim, template_body(Species::DMC), n_dmc, l, l);

    let dt = sim.dt;
    let n_eq = (equilibrate_fs / dt) as usize;
    let n_meas = (measure_fs / dt) as usize;
    let stride = (sample_every_fs / dt).max(1.0) as usize;

    print!("    equilibrating... ");
    use std::io::Write as _;
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for _ in 0..n_eq {
        sim.step();
    }
    println!("done ({:.1} s)", t0.elapsed().as_secs_f32());

    let li_initial: Vec<(u64, Vec2)> = sim
        .bodies
        .iter()
        .filter(|b| b.species == Species::LithiumIon)
        .map(|b| (b.id, b.pos))
        .collect();
    let anion_initial: Vec<(u64, Vec2)> = sim
        .bodies
        .iter()
        .filter(|b| b.species == Species::ElectrolyteAnion)
        .map(|b| (b.id, b.pos))
        .collect();

    let mut samples: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, 0.0)];
    let mut steps_since_sample = 0;
    print!("    sampling MSD... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for step in 1..=n_meas {
        sim.step();
        steps_since_sample += 1;
        if steps_since_sample >= stride || step == n_meas {
            let t_fs = step as f32 * dt;
            let msd_li = compute_msd(&sim, Species::LithiumIon, &li_initial);
            let msd_anion = compute_msd(&sim, Species::ElectrolyteAnion, &anion_initial);
            samples.push((t_fs, msd_li, msd_anion));
            steps_since_sample = 0;
        }
    }
    println!(
        "done ({:.1} s, {} samples)",
        t0.elapsed().as_secs_f32(),
        samples.len()
    );

    let li_pairs: Vec<(f32, f32)> = samples.iter().map(|s| (s.0, s.1)).collect();
    let anion_pairs: Vec<(f32, f32)> = samples.iter().map(|s| (s.0, s.2)).collect();
    let li_fit = linear_fit(&li_pairs);
    let anion_fit = linear_fit(&anion_pairs);
    let (d_li, li_r2) = match li_fit {
        Some((slope, _, r2)) => (slope as f64 / 4.0, r2 as f64),
        None => (f64::NAN, f64::NAN),
    };
    let (d_anion, anion_r2) = match anion_fit {
        Some((slope, _, r2)) => (slope as f64 / 4.0, r2 as f64),
        None => (f64::NAN, f64::NAN),
    };

    // Save per-L MSD CSV
    let l_label = format!("L{:04.0}", l);
    let l_dir = out_dir.join(&l_label);
    fs::create_dir_all(&l_dir).ok();
    let csv_path = l_dir.join("msd.csv");
    let mut csv = std::io::BufWriter::new(fs::File::create(&csv_path).unwrap());
    writeln!(csv, "t_fs,msd_li_a2,msd_anion_a2").unwrap();
    for s in &samples {
        writeln!(csv, "{:.4},{:.6},{:.6}", s.0, s.1, s.2).unwrap();
    }
    csv.flush().ok();

    let l64 = l as f64;
    let tau_li = if d_li > 0.0 { l64 * l64 / d_li } else { f64::NAN };
    let tau_anion = if d_anion > 0.0 {
        l64 * l64 / d_anion
    } else {
        f64::NAN
    };

    LRow {
        l: l64,
        n_li,
        n_anion,
        n_ec,
        n_dmc,
        d_li,
        d_anion,
        msd_li_r2: li_r2,
        msd_anion_r2: anion_r2,
        tau_ki_li: tau_li,
        tau_ki_anion: tau_anion,
    }
}

fn main() {
    let mut ls: Vec<f32> = vec![100.0, 150.0, 200.0, 300.0];
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 5_000.0;
    let mut measure_fs: f32 = 20_000.0;
    let mut sample_every_fs: f32 = 500.0;
    let mut out_dir = PathBuf::from("doe_results/physics_validation/kinetic_inductance_scaling");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ls" => {
                i += 1;
                ls = args[i]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect();
                if ls.is_empty() {
                    print_usage_and_exit();
                }
            }
            "--seed" => {
                i += 1;
                seed = parse_u64_flexible(&args[i]).unwrap_or_else(|| print_usage_and_exit());
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--measure-fs" => {
                i += 1;
                measure_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--sample-every-fs" => {
                i += 1;
                sample_every_fs = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
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

    println!("kinetic_inductance_scaling sweep");
    println!("  domain L values: {:?} Å", ls);
    println!("  seed = 0x{:X}", seed);
    println!(
        "  equilibrate = {} fs, measure = {} fs (sample {} fs)",
        equilibrate_fs, measure_fs, sample_every_fs
    );
    println!();

    let mut rows: Vec<LRow> = Vec::new();
    let total = ls.len();
    for (idx, &l) in ls.iter().enumerate() {
        println!("[{}/{}] L = {} Å", idx + 1, total, l);
        let row = run_one_l(l, seed, equilibrate_fs, measure_fs, sample_every_fs, &out_dir);
        println!(
            "  bodies: {} Li, {} anion, {} EC, {} DMC ({} total)",
            row.n_li,
            row.n_anion,
            row.n_ec,
            row.n_dmc,
            row.n_li + row.n_anion + row.n_ec + row.n_dmc
        );
        println!(
            "  D_Li⁺   = {:.4e} Å²/fs (R² = {:.3})  τ_KI_Li⁺   = {:.4e} fs",
            row.d_li, row.msd_li_r2, row.tau_ki_li
        );
        println!(
            "  D_anion = {:.4e} Å²/fs (R² = {:.3})  τ_KI_anion = {:.4e} fs",
            row.d_anion, row.msd_anion_r2, row.tau_ki_anion
        );
        println!();
        rows.push(row);
    }

    // ---- Sweep summary CSV ----
    let csv_path = out_dir.join("sweep_summary.csv");
    let mut csv = std::io::BufWriter::new(fs::File::create(&csv_path).unwrap());
    writeln!(
        csv,
        "L_angstrom,L_squared,n_li,n_anion,n_ec,n_dmc,d_li,d_anion,msd_li_r2,msd_anion_r2,tau_ki_li,tau_ki_anion"
    )
    .unwrap();
    for r in &rows {
        writeln!(
            csv,
            "{},{},{},{},{},{},{:.6e},{:.6e},{:.4},{:.4},{:.6e},{:.6e}",
            r.l,
            r.l * r.l,
            r.n_li,
            r.n_anion,
            r.n_ec,
            r.n_dmc,
            r.d_li,
            r.d_anion,
            r.msd_li_r2,
            r.msd_anion_r2,
            r.tau_ki_li,
            r.tau_ki_anion,
        )
        .unwrap();
    }
    csv.flush().ok();

    // ---- Test 1: D L-independence ----
    let d_li_values: Vec<f64> = rows.iter().map(|r| r.d_li).collect();
    let d_an_values: Vec<f64> = rows.iter().map(|r| r.d_anion).collect();
    let mean = |xs: &[f64]| xs.iter().sum::<f64>() / xs.len() as f64;
    let std = |xs: &[f64], m: f64| {
        ((xs.iter().map(|x| (x - m).powi(2)).sum::<f64>()) / xs.len() as f64).sqrt()
    };
    let d_li_mean = mean(&d_li_values);
    let d_li_std = std(&d_li_values, d_li_mean);
    let d_li_cv = if d_li_mean.abs() > 0.0 {
        d_li_std / d_li_mean.abs()
    } else {
        f64::INFINITY
    };
    let d_an_mean = mean(&d_an_values);
    let d_an_std = std(&d_an_values, d_an_mean);
    let d_an_cv = if d_an_mean.abs() > 0.0 {
        d_an_std / d_an_mean.abs()
    } else {
        f64::INFINITY
    };

    // ---- Test 2: τ_KI vs L² fit ----
    let l2_li_pairs: Vec<(f64, f64)> = rows.iter().map(|r| (r.l * r.l, r.tau_ki_li)).collect();
    let l2_an_pairs: Vec<(f64, f64)> = rows.iter().map(|r| (r.l * r.l, r.tau_ki_anion)).collect();
    let li_fit = linear_fit_f64(&l2_li_pairs);
    let an_fit = linear_fit_f64(&l2_an_pairs);

    let cv_tolerance = 0.30_f64;
    let r2_tolerance = 0.90_f64;
    let li_r2 = li_fit.map(|(_, _, r)| r).unwrap_or(f64::NAN);
    let an_r2 = an_fit.map(|(_, _, r)| r).unwrap_or(f64::NAN);
    let pass_d_const = d_li_cv < cv_tolerance && d_an_cv < cv_tolerance;
    let pass_tau_linear = li_r2 > r2_tolerance && an_r2 > r2_tolerance;
    let pass = pass_d_const && pass_tau_linear;

    println!("================================================");
    println!("Sweep summary across {} L values:", rows.len());
    println!(
        "  D_Li⁺:    mean={:.3e} ± std={:.3e}   CV={:.3}",
        d_li_mean, d_li_std, d_li_cv
    );
    println!(
        "  D_anion:  mean={:.3e} ± std={:.3e}   CV={:.3}",
        d_an_mean, d_an_std, d_an_cv
    );
    if let Some((slope, intercept, r2)) = li_fit {
        println!(
            "  τ_KI Li⁺   vs L²:  slope = {:+.4e}, intercept = {:+.4e}, R² = {:.4}",
            slope, intercept, r2
        );
    }
    if let Some((slope, intercept, r2)) = an_fit {
        println!(
            "  τ_KI anion vs L²:  slope = {:+.4e}, intercept = {:+.4e}, R² = {:.4}",
            slope, intercept, r2
        );
    }
    println!();
    println!("Pass criteria:");
    println!(
        "  [{}] D L-independence: CV(D_Li) {:.3} & CV(D_anion) {:.3} both < {:.2}",
        if pass_d_const { "✓" } else { "✗" },
        d_li_cv,
        d_an_cv,
        cv_tolerance
    );
    println!(
        "  [{}] τ_KI ∝ L²:        R²(Li) {:.3} & R²(anion) {:.3} both > {:.2}",
        if pass_tau_linear { "✓" } else { "✗" },
        li_r2,
        an_r2,
        r2_tolerance
    );
    println!("  PASS = {}", pass);

    let result = json!({
        "test": "kinetic_inductance_scaling",
        "value": (d_li_cv + d_an_cv) / 2.0,
        "value_label": "mean_d_coefficient_of_variation_across_L",
        "unit": "dimensionless",
        "tolerance": {
            "kind": "compound",
            "d_cv_max": cv_tolerance,
            "tau_l2_r2_min": r2_tolerance
        },
        "pass": pass,
        "details": {
            "ls_angstrom": ls.iter().map(|x| *x as f64).collect::<Vec<_>>(),
            "rows": rows.iter().map(|r| json!({
                "l": r.l,
                "l_squared": r.l * r.l,
                "n_li": r.n_li,
                "n_anion": r.n_anion,
                "n_ec": r.n_ec,
                "n_dmc": r.n_dmc,
                "d_li": r.d_li,
                "d_anion": r.d_anion,
                "msd_li_r2": r.msd_li_r2,
                "msd_anion_r2": r.msd_anion_r2,
                "tau_ki_li": r.tau_ki_li,
                "tau_ki_anion": r.tau_ki_anion,
            })).collect::<Vec<_>>(),
            "d_li_mean": d_li_mean,
            "d_li_std": d_li_std,
            "d_li_cv": d_li_cv,
            "d_anion_mean": d_an_mean,
            "d_anion_std": d_an_std,
            "d_anion_cv": d_an_cv,
            "tau_li_vs_l2_slope": li_fit.map(|(s, _, _)| s),
            "tau_li_vs_l2_intercept": li_fit.map(|(_, i, _)| i),
            "tau_li_vs_l2_r2": li_r2,
            "tau_anion_vs_l2_slope": an_fit.map(|(s, _, _)| s),
            "tau_anion_vs_l2_intercept": an_fit.map(|(_, i, _)| i),
            "tau_anion_vs_l2_r2": an_r2,
            "pass_d_const": pass_d_const,
            "pass_tau_linear": pass_tau_linear,
            "seed": format!("0x{:X}", seed),
            "equilibrate_fs": equilibrate_fs,
            "measure_fs": measure_fs,
            "sample_every_fs": sample_every_fs,
        }
    });
    let result_path = out_dir.join("result.json");
    fs::write(&result_path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
    println!();
    println!("Summary CSV: {}", csv_path.display());
    println!("Result:      {}", result_path.display());
    // Always exit 0.
}
