//! li_ec_coordination — Phase 2 dimensionless test 2.2.
//!
//! See docs/PHYSICS_VALIDATION_PLAN.md §Test 2.2.
//!
//! Computes the Li⁺-EC radial distribution function g(r) in a bulk electrolyte
//! and reports the first-shell coordination number n(r_min) — the integrated
//! number of EC molecules surrounding each Li⁺ ion within the first solvation
//! shell.
//!
//! Literature target for liquid carbonate electrolytes: 3-6 EC per Li⁺. A
//! coordination number outside this band suggests the solvation physics is
//! qualitatively wrong — relevant to smoking guns #1 (repulsion), #2 (LJ),
//! and #4 (single-charge vs dipole solvent model).
//!
//! 2D-with-3D-Coulomb caveat: this simulator is 2D, so we use the 2D shell
//! geometry (2π·r·dr) for the ideal-gas reference. The literature target
//! n ∈ [3, 6] comes from 3D liquid carbonates, so a strict numeric match is
//! not expected; the test is most useful as a sensitivity probe for repulsion
//! and LJ tuning.
//!
//! Pure measurement — no physics-parameter changes.

use particle_sim::app::spawn::add_random;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::simulation::Simulation;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: li_ec_coordination [--scenario <toml>] [--seed <u64>] \
         [--equilibrate-fs <f>] [--measure-fs <f>] [--sample-every-fs <f>] \
         [--r-max <f>] [--bin-width <f>] [--out-dir <path>]"
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

fn main() {
    let mut scenario =
        "measurement_configs/physics_invariants/li_ec_coordination_dense.toml".to_string();
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 10_000.0;
    let mut measure_fs: f32 = 20_000.0;
    let mut sample_every_fs: f32 = 100.0;
    let mut r_max: f64 = 15.0;
    let mut bin_width: f64 = 0.2;
    let mut out_dir = PathBuf::from("doe_results/physics_validation/li_ec_coordination");

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
            "--r-max" => {
                i += 1;
                r_max = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("--r-max expects a float");
                    print_usage_and_exit();
                });
            }
            "--bin-width" => {
                i += 1;
                bin_width = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("--bin-width expects a float");
                    print_usage_and_exit();
                });
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

    fastrand::seed(seed);
    let config = InitConfig::load_from_file(&scenario).unwrap_or_else(|e| {
        eprintln!("Failed to load scenario {}: {}", scenario, e);
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

    if !config.particles.metal_rectangles.is_empty()
        || !config.particles.foil_rectangles.is_empty()
    {
        eprintln!(
            "li_ec_coordination: scenario must be bulk-only; got {} metal rect, {} foils",
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

    let n_li_initial = sim.bodies.iter().filter(|b| b.species == Species::LithiumIon).count();
    let n_ec_initial = sim.bodies.iter().filter(|b| b.species == Species::EC).count();
    if n_li_initial == 0 || n_ec_initial == 0 {
        eprintln!(
            "li_ec_coordination: scenario must contain both Li⁺ and EC; got {} Li⁺, {} EC",
            n_li_initial, n_ec_initial
        );
        std::process::exit(3);
    }

    let dt = sim.dt;
    let n_eq = (equilibrate_fs / dt) as usize;
    let n_meas = (measure_fs / dt) as usize;
    let sample_stride = (sample_every_fs / dt).max(1.0) as usize;
    let n_bins = (r_max / bin_width).round() as usize;

    println!(
        "li_ec_coordination: scenario={}, seed=0x{:X}, bodies={} ({} Li⁺, {} EC), dt={} fs",
        scenario,
        seed,
        sim.bodies.len(),
        n_li_initial,
        n_ec_initial,
        dt
    );
    println!(
        "  domain = {} × {} Å²,  ρ_EC_initial = {:.4e} Å⁻²",
        full_w,
        full_h,
        n_ec_initial as f64 / (full_w * full_h) as f64
    );
    println!(
        "  equilibrate = {} fs ({} steps),  measure = {} fs ({} steps), sample stride = {} steps ({} fs)",
        equilibrate_fs,
        n_eq,
        measure_fs,
        n_meas,
        sample_stride,
        sample_stride as f32 * dt
    );
    println!(
        "  RDF bins: r_max = {} Å, bin_width = {} Å, n_bins = {}",
        r_max, bin_width, n_bins
    );

    print!("  equilibrating... ");
    use std::io::Write as _;
    std::io::stdout().flush().ok();
    let t_eq = std::time::Instant::now();
    for _ in 0..n_eq {
        sim.step();
    }
    println!("done ({:.1} s)", t_eq.elapsed().as_secs_f32());

    let mut histogram = vec![0_u64; n_bins];
    let mut n_li_total: u64 = 0;
    let mut n_ec_total: u64 = 0;
    let mut n_samples: u64 = 0;

    let collect_pairs = |sim: &Simulation,
                             hist: &mut Vec<u64>,
                             n_li_total: &mut u64,
                             n_ec_total: &mut u64| {
        let li_pos: Vec<Vec2> = sim
            .bodies
            .iter()
            .filter(|b| b.species == Species::LithiumIon)
            .map(|b| b.pos)
            .collect();
        let ec_pos: Vec<Vec2> = sim
            .bodies
            .iter()
            .filter(|b| b.species == Species::EC)
            .map(|b| b.pos)
            .collect();
        for &li in &li_pos {
            for &ec in &ec_pos {
                let r = (li - ec).mag() as f64;
                if r < r_max {
                    let bin = (r / bin_width).floor() as usize;
                    if bin < n_bins {
                        hist[bin] += 1;
                    }
                }
            }
        }
        *n_li_total += li_pos.len() as u64;
        *n_ec_total += ec_pos.len() as u64;
    };

    collect_pairs(&sim, &mut histogram, &mut n_li_total, &mut n_ec_total);
    n_samples += 1;

    let mut steps_since_sample: usize = 0;
    print!("  measuring... ");
    std::io::stdout().flush().ok();
    let t_meas = std::time::Instant::now();
    for step in 1..=n_meas {
        sim.step();
        steps_since_sample += 1;
        if steps_since_sample >= sample_stride || step == n_meas {
            collect_pairs(&sim, &mut histogram, &mut n_li_total, &mut n_ec_total);
            n_samples += 1;
            steps_since_sample = 0;
        }
    }
    println!(
        "done ({:.1} s, {} samples)",
        t_meas.elapsed().as_secs_f32(),
        n_samples
    );

    let n_li_avg = n_li_total as f64 / n_samples as f64;
    let n_ec_avg = n_ec_total as f64 / n_samples as f64;
    let area = (full_w * full_h) as f64;
    let rho_ec = n_ec_avg / area;

    // Build g(r) using 2D shell geometry: shell area = π(r_high² − r_low²).
    let mut g_of_r = vec![0.0_f64; n_bins];
    for i in 0..n_bins {
        let r_low = i as f64 * bin_width;
        let r_high = (i + 1) as f64 * bin_width;
        let shell_area = std::f64::consts::PI * (r_high.powi(2) - r_low.powi(2));
        let expected = rho_ec * shell_area * n_li_avg * n_samples as f64;
        g_of_r[i] = if expected > 0.0 {
            histogram[i] as f64 / expected
        } else {
            0.0
        };
    }

    // Find first peak (g(r) > 1, local maximum, after some initial buildup).
    let mut r_peak_bin: Option<usize> = None;
    for i in 1..(n_bins - 1) {
        if g_of_r[i] > 1.0 && g_of_r[i] > g_of_r[i - 1] && g_of_r[i] > g_of_r[i + 1] {
            r_peak_bin = Some(i);
            break;
        }
    }
    let r_peak_bin = r_peak_bin.unwrap_or(0);
    let r_peak = (r_peak_bin as f64 + 0.5) * bin_width;
    let g_peak = if r_peak_bin < n_bins { g_of_r[r_peak_bin] } else { 0.0 };

    // Find first minimum after the peak (local minimum below the peak height).
    let mut r_min_bin: Option<usize> = None;
    for i in (r_peak_bin + 1)..(n_bins - 1) {
        if g_of_r[i] < g_of_r[i - 1] && g_of_r[i] <= g_of_r[i + 1] {
            r_min_bin = Some(i);
            break;
        }
    }
    let r_min_bin = r_min_bin.unwrap_or(n_bins - 1);
    let r_min = (r_min_bin as f64 + 0.5) * bin_width;
    let g_min = g_of_r[r_min_bin];

    // Coordination number = mean number of EC within r_min around each Li⁺
    // (averaged over Li⁺ and over time).
    let coord_count: u64 = histogram[..=r_min_bin].iter().sum();
    let n_coord = coord_count as f64 / (n_li_avg * n_samples as f64);

    let target_min = 3.0_f64;
    let target_max = 6.0_f64;
    let pass = (target_min..=target_max).contains(&n_coord);

    println!();
    println!("RDF analysis:");
    println!(
        "  ρ_EC      = {:.4e} Å⁻² (avg over samples)   ρ_EC × π × (1Å)² = {:.4e}",
        rho_ec,
        rho_ec * std::f64::consts::PI
    );
    println!(
        "  first peak: r = {:.3} Å,  g(r_peak) = {:.3}",
        r_peak, g_peak
    );
    println!(
        "  first min after peak: r = {:.3} Å,  g(r_min) = {:.3}",
        r_min, g_min
    );
    println!(
        "  coordination number n(r_min) = {:.3}   (target band: [{:.1}, {:.1}])",
        n_coord, target_min, target_max
    );
    println!("  PASS = {}", pass);

    // Write CSV
    let csv_path = out_dir.join("rdf.csv");
    let mut csv = std::io::BufWriter::new(
        fs::File::create(&csv_path).unwrap_or_else(|e| {
            eprintln!("Failed to create CSV: {}", e);
            std::process::exit(1);
        }),
    );
    writeln!(
        csv,
        "bin_idx,r_low,r_high,r_center,count,g_of_r,cumulative_count,cumulative_n_per_li"
    )
    .unwrap();
    let mut cum_count: u64 = 0;
    for i in 0..n_bins {
        cum_count += histogram[i];
        let r_low = i as f64 * bin_width;
        let r_high = (i + 1) as f64 * bin_width;
        let r_center = (r_low + r_high) * 0.5;
        let cum_per_li = cum_count as f64 / (n_li_avg * n_samples as f64);
        writeln!(
            csv,
            "{},{:.4},{:.4},{:.4},{},{:.6e},{},{:.6}",
            i, r_low, r_high, r_center, histogram[i], g_of_r[i], cum_count, cum_per_li
        )
        .unwrap();
    }
    csv.flush().ok();

    let result = json!({
        "test": "li_ec_coordination",
        "value": n_coord,
        "value_label": "li_ec_first_shell_coordination_number",
        "unit": "ec_per_li",
        "tolerance": {
            "kind": "range",
            "min": target_min,
            "max": target_max,
            "literature_source": "liquid carbonate Li⁺ coordination, 3D bulk"
        },
        "pass": pass,
        "details": {
            "scenario": scenario,
            "seed": format!("0x{:X}", seed),
            "equilibrate_fs": equilibrate_fs,
            "measure_fs": measure_fs,
            "sample_every_fs": sample_every_fs,
            "n_samples": n_samples,
            "n_li_avg": n_li_avg,
            "n_ec_avg": n_ec_avg,
            "domain_w": full_w,
            "domain_h": full_h,
            "rho_ec_per_a2": rho_ec,
            "r_max": r_max,
            "bin_width": bin_width,
            "n_bins": n_bins,
            "r_peak_angstrom": r_peak,
            "g_peak": g_peak,
            "r_min_angstrom": r_min,
            "g_min": g_min,
            "n_coord_first_shell": n_coord,
        }
    });
    let result_path = out_dir.join("result.json");
    fs::write(&result_path, serde_json::to_string_pretty(&result).unwrap()).unwrap_or_else(|e| {
        eprintln!("Failed to write result.json: {}", e);
        std::process::exit(1);
    });
    println!();
    println!("CSV:    {}", csv_path.display());
    println!("Result: {}", result_path.display());
    // Always exit 0 — measurement test, not gating.
}
