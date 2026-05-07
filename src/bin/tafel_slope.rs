//! tafel_slope — Phase 2 dimensionless test.
//!
//! See docs/PHYSICS_VALIDATION_PLAN.md §Test 2.1.
//!
//! Galvanostatic Tafel sweep on a symmetric Li | electrolyte | Li cell.
//! Applies opposing DC currents (±I) at each foil for a series of
//! amplitudes spanning the linear-response regime, measures the steady
//! state mean foil-charge differential, fits ln|I| vs |η| in the Tafel
//! regime, and recovers the Butler-Volmer transfer coefficient α.
//!
//! Recovered α should match the configured BV_TRANSFER_COEFF (default 0.5)
//! to within ~10-20%. A larger discrepancy indicates BV is not the
//! dominant kinetic mechanism in this regime, or that BV implementation
//! has a bug.
//!
//! Pure measurement — does not modify any physics parameter. See
//! memory/feedback_no_unauthorized_retuning.md for the no-retune rule.
//!
//! Usage:
//!   cargo run --release --bin tafel_slope -- \
//!     [--scenario <path.toml>] [--seed <u64>] \
//!     [--amps <comma-list>] [--equilibrate-fs <f>] [--measure-fs <f>] \
//!     [--out-dir <path>]

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: tafel_slope \
         [--scenario <path.toml>] [--seed <u64>] \
         [--amps <comma-list of amps in e/fs>] \
         [--equilibrate-fs <f>] [--measure-fs <f>] \
         [--sample-every-fs <f>] [--out-dir <path>]"
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

#[derive(Debug, Clone)]
struct AmpRow {
    amp_applied: f64,
    mean_q_left: f64,
    mean_q_right: f64,
    mean_q_diff: f64,
    std_q_diff: f64,
    n_left_foil_bodies: usize,
    n_right_foil_bodies: usize,
    eta_proxy_volts: f64,
    mean_i_left: f64,
    mean_i_right: f64,
    mean_i_avg: f64,    // (|i_left| + |i_right|) / 2
    abs_i_measured: f64,
    n_samples: usize,
    accepted_hops: u64,
    candidates_evaluated: u64,
}

/// Simple ordinary-least-squares fit of y = slope*x + intercept.
fn linear_fit(xs: &[f64], ys: &[f64]) -> Option<(f64, f64, f64)> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    let n = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / n;
    let mean_y: f64 = ys.iter().sum::<f64>() / n;
    let mut ss_xy = 0.0_f64;
    let mut ss_xx = 0.0_f64;
    for i in 0..xs.len() {
        ss_xy += (xs[i] - mean_x) * (ys[i] - mean_y);
        ss_xx += (xs[i] - mean_x).powi(2);
    }
    if ss_xx <= 0.0 {
        return None;
    }
    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for i in 0..xs.len() {
        let pred = slope * xs[i] + intercept;
        ss_res += (ys[i] - pred).powi(2);
        ss_tot += (ys[i] - mean_y).powi(2);
    }
    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 1.0 };
    Some((slope, intercept, r2))
}

fn run_one_amplitude(
    scenario_path: &str,
    seed: u64,
    amp: f64,
    equilibrate_fs: f32,
    measure_fs: f32,
    sample_every_fs: f32,
    hop_rate_k0_override: Option<f32>,
    bv_exchange_current_override: Option<f32>,
    out_dir: &PathBuf,
) -> AmpRow {
    use particle_sim::simulation::electron_hopping::{read_hop_diag, reset_hop_diag};

    fastrand::seed(seed);

    let config = InitConfig::load_from_file(scenario_path).unwrap_or_else(|e| {
        eprintln!("Failed to load scenario {}: {}", scenario_path, e);
        std::process::exit(3);
    });
    let (full_w, full_h) = config
        .simulation
        .as_ref()
        .map(|s| s.domain_size())
        .unwrap_or_else(|| {
            eprintln!("scenario must specify [simulation] domain_width/domain_height");
            std::process::exit(3);
        });

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    for rect in &config.particles.metal_rectangles {
        let species = rect.to_species().unwrap();
        let body = template_body(species);
        let (x, y) = rect.to_origin_coords();
        handle_command(
            SimCommand::AddRectangle {
                body,
                x,
                y,
                width: rect.width,
                height: rect.height,
            },
            &mut sim,
        );
    }
    for foil in &config.particles.foil_rectangles {
        let (x, y) = foil.to_origin_coords();
        handle_command(
            SimCommand::AddFoil {
                width: foil.width,
                height: foil.height,
                x,
                y,
                particle_radius: Species::FoilMetal.radius(),
                current: foil.current,
            },
            &mut sim,
        );
    }
    for entry in &config.particles.random {
        let species = entry.to_species().unwrap();
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

    if sim.foils.len() != 2 {
        eprintln!("tafel_slope: scenario must define exactly 2 foils");
        std::process::exit(3);
    }

    // Centroid sort -> left/right
    let foil_x_centroids: Vec<(usize, f32)> = sim
        .foils
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (s, n) = f
                .body_ids
                .iter()
                .filter_map(|&bid| sim.bodies.iter().find(|b| b.id == bid).map(|b| b.pos.x))
                .fold((0.0_f32, 0.0_f32), |(s, n), x| (s + x, n + 1.0));
            (i, if n > 0.0 { s / n } else { 0.0 })
        })
        .collect();
    let mut sorted = foil_x_centroids.clone();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let left_idx = sorted[0].0;
    let right_idx = sorted[1].0;
    let n_left = sim.foils[left_idx].body_ids.len();
    let n_right = sim.foils[right_idx].body_ids.len();

    // Apply ±amp drive
    sim.foils[left_idx].dc_current = -(amp as f32);
    sim.foils[right_idx].dc_current = amp as f32;

    // Optional runtime override of HOP_RATE_K0 — used to probe whether BV
    // recovers its α when rate·dt is moved into the linear regime. Pure
    // runtime change; the const in config.rs is unchanged.
    if let Some(k0) = hop_rate_k0_override {
        sim.config.hop_rate_k0 = k0;
    }
    if let Some(i0) = bv_exchange_current_override {
        sim.config.bv_exchange_current = i0;
    }

    let dt = sim.dt;
    let n_eq = (equilibrate_fs / dt) as usize;
    let n_meas = (measure_fs / dt) as usize;
    let sample_stride = (sample_every_fs / dt).max(1.0) as usize;

    // Equilibrate with drive ON
    for _ in 0..n_eq {
        sim.step();
    }

    // Reset deltas
    for f in sim.foils.iter_mut() {
        f.electron_delta_since_measure = 0;
    }
    reset_hop_diag();

    let foil_charge = |sim: &Simulation, idx: usize| -> f64 {
        sim.foils[idx]
            .body_ids
            .iter()
            .filter_map(|&bid| sim.bodies.iter().find(|b| b.id == bid).map(|b| b.charge as f64))
            .sum()
    };

    let label = format!("amp{:+.3e}", amp);
    let amp_dir = out_dir.join(&label);
    fs::create_dir_all(&amp_dir).ok();
    let csv_path = amp_dir.join("timeseries.csv");
    let csv_file = fs::File::create(&csv_path).unwrap_or_else(|e| {
        eprintln!("CSV open failed {}: {}", csv_path.display(), e);
        std::process::exit(1);
    });
    let mut csv = std::io::BufWriter::new(csv_file);
    writeln!(
        csv,
        "step,t_fs,q_left,q_right,q_diff,i_left_e_per_fs,i_right_e_per_fs"
    )
    .unwrap();

    // Per-sample accumulators
    let mut samples_q_left: Vec<f64> = Vec::new();
    let mut samples_q_right: Vec<f64> = Vec::new();
    let mut samples_q_diff: Vec<f64> = Vec::new();
    let mut samples_i_left: Vec<f64> = Vec::new();
    let mut samples_i_right: Vec<f64> = Vec::new();

    let mut i_left_accum: i32 = 0;
    let mut i_right_accum: i32 = 0;
    let mut steps_since_sample: usize = 0;

    for step in 1..=n_meas {
        sim.step();
        steps_since_sample += 1;
        i_left_accum += sim.foils[left_idx].electron_delta_since_measure;
        i_right_accum += sim.foils[right_idx].electron_delta_since_measure;
        sim.foils[left_idx].electron_delta_since_measure = 0;
        sim.foils[right_idx].electron_delta_since_measure = 0;

        if steps_since_sample >= sample_stride || step == n_meas {
            let t_fs = step as f64 * dt as f64;
            let q_l = foil_charge(&sim, left_idx);
            let q_r = foil_charge(&sim, right_idx);
            let q_d = q_l - q_r;
            let win_fs = steps_since_sample as f64 * dt as f64;
            let i_l = i_left_accum as f64 / win_fs;
            let i_r = i_right_accum as f64 / win_fs;
            samples_q_left.push(q_l);
            samples_q_right.push(q_r);
            samples_q_diff.push(q_d);
            samples_i_left.push(i_l);
            samples_i_right.push(i_r);
            writeln!(
                csv,
                "{},{:.3},{:.6},{:.6},{:.6},{:.6e},{:.6e}",
                step, t_fs, q_l, q_r, q_d, i_l, i_r
            )
            .unwrap();
            i_left_accum = 0;
            i_right_accum = 0;
            steps_since_sample = 0;
        }
    }
    csv.flush().ok();

    // Use second half for steady-state stats
    let split = samples_q_diff.len() / 2;
    let tail = |xs: &[f64]| -> Vec<f64> { xs[split..].to_vec() };
    let mean = |xs: &[f64]| -> f64 { xs.iter().sum::<f64>() / xs.len() as f64 };
    let std_dev = |xs: &[f64], m: f64| -> f64 {
        ((xs.iter().map(|x| (x - m).powi(2)).sum::<f64>()) / xs.len() as f64).sqrt()
    };

    let q_l_tail = tail(&samples_q_left);
    let q_r_tail = tail(&samples_q_right);
    let q_d_tail = tail(&samples_q_diff);
    let i_l_tail = tail(&samples_i_left);
    let i_r_tail = tail(&samples_i_right);

    let mean_q_left = mean(&q_l_tail);
    let mean_q_right = mean(&q_r_tail);
    let mean_q_diff = mean(&q_d_tail);
    let std_q_diff = std_dev(&q_d_tail, mean_q_diff);
    let mean_i_left = mean(&i_l_tail);
    let mean_i_right = mean(&i_r_tail);
    // Use the magnitude of right-foil current (positive applied side) as the
    // canonical |I| for Tafel — the cell is symmetric so |i_left| ≈ |i_right|.
    let mean_i_avg = (mean_i_left.abs() + mean_i_right.abs()) * 0.5;
    let abs_i_measured = mean_i_avg;

    // η proxy (volts):
    //   η_per_body_left  = POTENTIAL_PER_CHARGE × mean_q_left  / n_left
    //   η_per_body_right = POTENTIAL_PER_CHARGE × mean_q_right / n_right
    //   η = (η_right − η_left) / 2     (symmetric cell: each side contributes half)
    let pot_per_e = particle_sim::config::POTENTIAL_PER_CHARGE as f64;
    let eta_left_per_body = if n_left > 0 {
        pot_per_e * mean_q_left / n_left as f64
    } else {
        0.0
    };
    let eta_right_per_body = if n_right > 0 {
        pot_per_e * mean_q_right / n_right as f64
    } else {
        0.0
    };
    let eta_proxy_volts = (eta_right_per_body - eta_left_per_body) * 0.5;

    let diag = read_hop_diag();

    AmpRow {
        amp_applied: amp,
        mean_q_left,
        mean_q_right,
        mean_q_diff,
        std_q_diff,
        n_left_foil_bodies: n_left,
        n_right_foil_bodies: n_right,
        eta_proxy_volts,
        mean_i_left,
        mean_i_right,
        mean_i_avg,
        abs_i_measured,
        n_samples: q_d_tail.len(),
        accepted_hops: diag.accepted,
        candidates_evaluated: diag.candidates_reached_predicate,
    }
}

fn main() {
    // CLI defaults
    let mut scenario =
        "measurement_configs/physics_invariants/driven_symmetric.toml".to_string();
    let mut seed: u64 = 0xC0FFEE;
    let mut amps: Vec<f64> = vec![5e-5, 1e-4, 3e-4, 1e-3, 3e-3, 1e-2, 3e-2];
    let mut equilibrate_fs: f32 = 5_000.0;
    let mut measure_fs: f32 = 5_000.0;
    let mut sample_every_fs: f32 = 50.0;
    let mut out_dir = PathBuf::from("doe_results/physics_validation/tafel_slope");
    let mut hop_rate_k0_override: Option<f32> = None;
    let mut bv_exchange_current_override: Option<f32> = None;

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
            "--amps" => {
                i += 1;
                amps = args[i]
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .collect();
                if amps.is_empty() {
                    eprintln!("--amps got an empty list");
                    print_usage_and_exit();
                }
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("--equilibrate-fs expects a float");
                    print_usage_and_exit();
                });
            }
            "--measure-fs" => {
                i += 1;
                measure_fs = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("--measure-fs expects a float");
                    print_usage_and_exit();
                });
            }
            "--sample-every-fs" => {
                i += 1;
                sample_every_fs = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("--sample-every-fs expects a float");
                    print_usage_and_exit();
                });
            }
            "--out-dir" => {
                i += 1;
                out_dir = PathBuf::from(args[i].clone());
            }
            "--hop-rate-k0" => {
                i += 1;
                hop_rate_k0_override = Some(args[i].parse::<f32>().unwrap_or_else(|_| {
                    eprintln!("--hop-rate-k0 expects a float");
                    print_usage_and_exit();
                }));
            }
            "--bv-exchange-current" => {
                i += 1;
                bv_exchange_current_override = Some(args[i].parse::<f32>().unwrap_or_else(|_| {
                    eprintln!("--bv-exchange-current expects a float");
                    print_usage_and_exit();
                }));
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

    // Set up the SimCommand sink so handle_command works
    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let configured_alpha = particle_sim::config::BV_TRANSFER_COEFF as f64;
    let configured_scale = particle_sim::config::BV_OVERPOTENTIAL_SCALE as f64;

    println!("tafel_slope sweep");
    println!("  scenario      = {}", scenario);
    println!("  seed          = 0x{:X}", seed);
    println!(
        "  amplitudes    = {:?} e/fs",
        amps.iter().map(|x| format!("{:.3e}", x)).collect::<Vec<_>>()
    );
    println!("  equilibrate   = {} fs", equilibrate_fs);
    println!("  measure       = {} fs", measure_fs);
    println!("  configured α  = {}", configured_alpha);
    println!("  configured BV_OVERPOTENTIAL_SCALE = {} (V or sim eV-equiv)", configured_scale);
    if let Some(k0) = hop_rate_k0_override {
        println!("  HOP_RATE_K0 OVERRIDE: {} fs⁻¹ (default: {})",
                 k0, particle_sim::config::HOP_RATE_K0);
    } else {
        println!("  HOP_RATE_K0 = {} fs⁻¹ (default)", particle_sim::config::HOP_RATE_K0);
    }
    if let Some(i0) = bv_exchange_current_override {
        println!("  BV_EXCHANGE_CURRENT OVERRIDE: {} (default: {})",
                 i0, particle_sim::config::BV_EXCHANGE_CURRENT);
    } else {
        println!("  BV_EXCHANGE_CURRENT = {} (default)", particle_sim::config::BV_EXCHANGE_CURRENT);
    }
    println!();

    // Summary CSV header
    let summary_path = out_dir.join("sweep_summary.csv");
    let mut summary = std::io::BufWriter::new(
        fs::File::create(&summary_path).unwrap_or_else(|e| {
            eprintln!("Failed to create summary CSV: {}", e);
            std::process::exit(1);
        }),
    );
    writeln!(
        summary,
        "amp_applied,mean_q_left,mean_q_right,mean_q_diff,std_q_diff,\
         n_left_foil,n_right_foil,eta_proxy_volts,\
         mean_i_left,mean_i_right,mean_i_avg,abs_i_measured,\
         n_samples,accepted_hops,candidates_evaluated"
    )
    .unwrap();

    let mut rows: Vec<AmpRow> = Vec::new();
    let total = amps.len();
    for (idx, &amp) in amps.iter().enumerate() {
        println!(
            "[{}/{}] amp = {:.3e} e/fs ...",
            idx + 1,
            total,
            amp
        );
        let t0 = std::time::Instant::now();
        let row = run_one_amplitude(
            &scenario,
            seed,
            amp,
            equilibrate_fs,
            measure_fs,
            sample_every_fs,
            hop_rate_k0_override,
            bv_exchange_current_override,
            &out_dir,
        );
        let dt = t0.elapsed().as_secs_f32();
        println!(
            "        I_meas={:+.3e}, η_proxy={:+.4} V, accepted={}, candidates={} ({:.1} s)",
            row.abs_i_measured, row.eta_proxy_volts, row.accepted_hops,
            row.candidates_evaluated, dt
        );
        writeln!(
            summary,
            "{:.6e},{:.6},{:.6},{:.6},{:.6},{},{},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{},{},{}",
            row.amp_applied,
            row.mean_q_left,
            row.mean_q_right,
            row.mean_q_diff,
            row.std_q_diff,
            row.n_left_foil_bodies,
            row.n_right_foil_bodies,
            row.eta_proxy_volts,
            row.mean_i_left,
            row.mean_i_right,
            row.mean_i_avg,
            row.abs_i_measured,
            row.n_samples,
            row.accepted_hops,
            row.candidates_evaluated,
        )
        .unwrap();
        rows.push(row);
    }
    summary.flush().ok();

    // ---- Tafel fit ----
    // Use rows where (a) accepted hops > 0 (kinetics engaged), (b) measured
    // current is within 50% of applied (linear regime, not saturated), and
    // (c) η is non-trivially nonzero.
    let mut linear_rows: Vec<&AmpRow> = Vec::new();
    for row in &rows {
        if row.accepted_hops == 0 {
            continue;
        }
        if row.eta_proxy_volts.abs() < 1e-6 {
            continue;
        }
        let frac_delivered = row.abs_i_measured / row.amp_applied.abs();
        if !(0.5..=1.5).contains(&frac_delivered) {
            continue;
        }
        linear_rows.push(row);
    }
    let xs: Vec<f64> = linear_rows
        .iter()
        .map(|r| r.eta_proxy_volts.abs())
        .collect();
    let ys: Vec<f64> = linear_rows
        .iter()
        .map(|r| r.abs_i_measured.ln())
        .collect();

    let (slope_per_volt, intercept, r2) = match linear_fit(&xs, &ys) {
        Some(t) => t,
        None => {
            eprintln!("\nNot enough points in linear regime to fit Tafel slope.");
            (f64::NAN, f64::NAN, f64::NAN)
        }
    };
    let recovered_alpha = slope_per_volt * configured_scale;
    let alpha_rel_err = if configured_alpha != 0.0 {
        (recovered_alpha - configured_alpha).abs() / configured_alpha
    } else {
        f64::NAN
    };

    println!();
    println!("Tafel fit on {} points in the linear regime:", linear_rows.len());
    if !linear_rows.is_empty() {
        println!(
            "  amps used:    {:?}",
            linear_rows
                .iter()
                .map(|r| format!("{:.2e}", r.amp_applied))
                .collect::<Vec<_>>()
        );
    }
    println!("  slope (per V) = {:.4e}", slope_per_volt);
    println!("  intercept     = {:.4e}", intercept);
    println!("  R²            = {:.4}", r2);
    println!(
        "  recovered α   = slope × BV_OVERPOTENTIAL_SCALE = {:.4} (configured: {:.4}, rel_err: {:.3})",
        recovered_alpha, configured_alpha, alpha_rel_err
    );

    // Tolerance: 20% relative error on recovered α
    let tolerance: f64 = 0.20;
    let pass = alpha_rel_err.is_finite() && alpha_rel_err <= tolerance;
    println!("  PASS = {} (tolerance: |α_recovered − α_config|/α_config ≤ {:.2})", pass, tolerance);

    let amps_json: Vec<f64> = rows.iter().map(|r| r.amp_applied).collect();
    let i_meas_json: Vec<f64> = rows.iter().map(|r| r.abs_i_measured).collect();
    let etas_json: Vec<f64> = rows.iter().map(|r| r.eta_proxy_volts).collect();

    let result_path = out_dir.join("result.json");
    let result = json!({
        "test": "tafel_slope",
        "value": recovered_alpha,
        "value_label": "recovered_butler_volmer_alpha",
        "unit": "dimensionless",
        "tolerance": {
            "kind": "relative_to_configured",
            "value": tolerance,
            "configured_alpha": configured_alpha
        },
        "pass": pass,
        "details": {
            "configured_alpha": configured_alpha,
            "configured_bv_overpotential_scale": configured_scale,
            "tafel_slope_per_volt": slope_per_volt,
            "tafel_intercept": intercept,
            "tafel_fit_r2": r2,
            "alpha_relative_error": alpha_rel_err,
            "n_amps_total": rows.len(),
            "n_amps_in_linear_regime": linear_rows.len(),
            "linear_regime_amps": linear_rows.iter().map(|r| r.amp_applied).collect::<Vec<_>>(),
            "amplitudes_e_per_fs": amps_json,
            "abs_i_measured_e_per_fs": i_meas_json,
            "eta_proxy_volts": etas_json,
            "scenario": scenario,
            "seed": format!("0x{:X}", seed),
            "equilibrate_fs": equilibrate_fs,
            "measure_fs": measure_fs,
            "hop_rate_k0_override": hop_rate_k0_override,
            "default_hop_rate_k0": particle_sim::config::HOP_RATE_K0,
            "bv_exchange_current_override": bv_exchange_current_override,
            "default_bv_exchange_current": particle_sim::config::BV_EXCHANGE_CURRENT,
        }
    });
    let pretty = serde_json::to_string_pretty(&result).unwrap();
    fs::write(&result_path, pretty).unwrap_or_else(|e| {
        eprintln!("Failed to write result.json: {}", e);
        std::process::exit(1);
    });
    println!();
    println!("Summary CSV: {}", summary_path.display());
    println!("Result JSON: {}", result_path.display());
    // Always exit 0 — this is a measurement test, not a gating invariant.
    // Pass/fail is recorded in result.json for downstream consumers.
}
