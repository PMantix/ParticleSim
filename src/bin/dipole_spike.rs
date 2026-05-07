//! dipole_spike — Phase 3 spike testing smoking gun #4 (single-charge vs
//! dipole solvent model).
//!
//! See docs/PHYSICS_VALIDATION_PLAN.md.
//!
//! Hypothesis: replacing the simulator's single-charge polar solvent with
//! explicit ±q dipole pairs (two oppositely-charged bonded sub-particles)
//! should increase Li⁺ first-shell coordination above the Phase-2.2
//! baseline of 2.18 EC/Li⁺ — toward the literature target [3, 6].
//!
//! This is a SPIKE: it does not modify the core simulator. Bonded "dipole
//! pairs" are pre-spawned as ordinary bodies (using LithiumIon and
//! ElectrolyteAnion species but with reduced ±0.4 charges) and the
//! harmonic bond force is applied as a per-step velocity correction
//! after `sim.step()`. If the hypothesis is supported here, the proper
//! productionisation is to add a `Bond` mechanism to the core simulator
//! — separate decision.
//!
//! Comparison baselines (from Phase 2):
//!   • li_ec_coordination (single-charge EC): n_coord = 2.18 EC/Li⁺
//!   • nernst_einstein:                       Haven ratio = 11.5

use particle_sim::body::{Body, Species};
use particle_sim::simulation::Simulation;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: dipole_spike \
         [--seed <u64>] [--equilibrate-fs <f>] [--measure-fs <f>] [--sample-every-fs <f>] \
         [--n-li <usize>] [--n-anion <usize>] [--n-dipole <usize>] [--n-dmc <usize>] \
         [--domain-w <f>] [--domain-h <f>] \
         [--dipole-charge <f>] [--dipole-r-eq <f>] [--bond-k <f>] \
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

#[derive(Clone, Copy)]
struct Bond {
    id_pos: u64,
    id_neg: u64,
}

fn apply_bond_forces(sim: &mut Simulation, bonds: &[Bond], r_eq: f32, k_bond: f32, dt: f32) {
    // Build id -> index map once per call (cheap relative to step cost).
    let id_to_idx: HashMap<u64, usize> = sim
        .bodies
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    for bond in bonds {
        let (Some(&ia), Some(&ib)) = (id_to_idx.get(&bond.id_pos), id_to_idx.get(&bond.id_neg))
        else {
            continue;
        };
        let pa = sim.bodies[ia].pos;
        let pb = sim.bodies[ib].pos;
        let ma = sim.bodies[ia].mass;
        let mb = sim.bodies[ib].mass;
        let dr = pb - pa;
        let r = dr.mag();
        if r < 1e-6 {
            continue;
        }
        let displacement = r - r_eq;
        let force_mag = k_bond * displacement; // attractive when r > r_eq
        let force_dir = dr.normalized();
        // Force on `pos` end pulls toward neg if r > r_eq.
        let dv_a = force_dir * (force_mag * dt / ma);
        // Force on `neg` end is equal and opposite.
        let dv_b = force_dir * (-force_mag * dt / mb);
        sim.bodies[ia].vel += dv_a;
        sim.bodies[ib].vel += dv_b;
    }
}

fn main() {
    // Defaults
    let mut seed: u64 = 0xC0FFEE;
    let mut equilibrate_fs: f32 = 10_000.0;
    let mut measure_fs: f32 = 10_000.0;
    let mut sample_every_fs: f32 = 200.0;
    let mut n_li_free: usize = 50;
    let mut n_anion_free: usize = 50;
    let mut n_dipole: usize = 300;
    let mut n_dmc: usize = 300;
    let mut domain_w: f32 = 200.0;
    let mut domain_h: f32 = 200.0;
    let mut dipole_charge: f32 = 0.4; // matches existing EC polar_charge magnitude
    let mut r_eq: f32 = 2.5; // dipole length (Å); real EC dipole 1 e·Å with q=0.4 → 2.5 Å
    let mut bond_k: f32 = 0.5; // bond spring (sim force/length units)
    let mut r_max: f64 = 15.0;
    let mut bin_width: f64 = 0.2;
    let mut out_dir = PathBuf::from("doe_results/physics_validation/dipole_spike");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
            "--n-li" => {
                i += 1;
                n_li_free = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--n-anion" => {
                i += 1;
                n_anion_free = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--n-dipole" => {
                i += 1;
                n_dipole = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--n-dmc" => {
                i += 1;
                n_dmc = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--domain-w" => {
                i += 1;
                domain_w = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--domain-h" => {
                i += 1;
                domain_h = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--dipole-charge" => {
                i += 1;
                dipole_charge = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--dipole-r-eq" => {
                i += 1;
                r_eq = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--bond-k" => {
                i += 1;
                bond_k = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--r-max" => {
                i += 1;
                r_max = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
            }
            "--bin-width" => {
                i += 1;
                bin_width = args[i].parse().unwrap_or_else(|_| print_usage_and_exit());
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
    let mut sim = Simulation::new();
    sim.domain_width = domain_w / 2.0;
    sim.domain_height = domain_h / 2.0;
    sim.cell_list
        .update_domain_size(sim.domain_width, sim.domain_height);

    // Spawn free Li⁺ (charge=+1)
    for _ in 0..n_li_free {
        let pos = Vec2::new(
            (fastrand::f32() - 0.5) * domain_w,
            (fastrand::f32() - 0.5) * domain_h,
        );
        sim.bodies.push(Body::new(
            pos,
            Vec2::zero(),
            Species::LithiumIon.mass(),
            Species::LithiumIon.radius(),
            1.0,
            Species::LithiumIon,
        ));
    }
    // Spawn free anion (charge=-1)
    for _ in 0..n_anion_free {
        let pos = Vec2::new(
            (fastrand::f32() - 0.5) * domain_w,
            (fastrand::f32() - 0.5) * domain_h,
        );
        sim.bodies.push(Body::new(
            pos,
            Vec2::zero(),
            Species::ElectrolyteAnion.mass(),
            Species::ElectrolyteAnion.radius(),
            -1.0,
            Species::ElectrolyteAnion,
        ));
    }
    // Spawn dipole pairs — these use LithiumIon/ElectrolyteAnion species
    // (which have repulsion_cutoff = 2.0 Å, no LJ) but with reduced ±dipole_charge
    // values. They get tagged via the bonds list so we can distinguish them.
    let mut bonds: Vec<Bond> = Vec::new();
    for _ in 0..n_dipole {
        let center = Vec2::new(
            (fastrand::f32() - 0.5) * domain_w,
            (fastrand::f32() - 0.5) * domain_h,
        );
        let theta = fastrand::f32() * std::f32::consts::TAU;
        let half_off = Vec2::new(theta.cos(), theta.sin()) * (r_eq * 0.5);
        // Positive end
        let mut bp = Body::new(
            center + half_off,
            Vec2::zero(),
            Species::LithiumIon.mass(),
            Species::LithiumIon.radius(),
            dipole_charge,
            Species::LithiumIon,
        );
        bp.charge = dipole_charge; // re-assert (Body::new sanitises)
        sim.bodies.push(bp);
        let id_pos = sim.bodies.last().unwrap().id;
        // Negative end
        let mut bn = Body::new(
            center - half_off,
            Vec2::zero(),
            Species::ElectrolyteAnion.mass(),
            Species::ElectrolyteAnion.radius(),
            -dipole_charge,
            Species::ElectrolyteAnion,
        );
        bn.charge = -dipole_charge;
        sim.bodies.push(bn);
        let id_neg = sim.bodies.last().unwrap().id;
        bonds.push(Bond { id_pos, id_neg });
    }
    let bonded_pos_ids: HashSet<u64> = bonds.iter().map(|b| b.id_pos).collect();
    let bonded_neg_ids: HashSet<u64> = bonds.iter().map(|b| b.id_neg).collect();
    let bonded_any: HashSet<u64> = bonded_pos_ids
        .union(&bonded_neg_ids)
        .copied()
        .collect();

    // Spawn DMC bulk (passive, no charge — just to fill volume similar to Phase-2.2 scenario)
    for _ in 0..n_dmc {
        let pos = Vec2::new(
            (fastrand::f32() - 0.5) * domain_w,
            (fastrand::f32() - 0.5) * domain_h,
        );
        sim.bodies.push(Body::new(
            pos,
            Vec2::zero(),
            Species::DMC.mass(),
            Species::DMC.radius(),
            0.0,
            Species::DMC,
        ));
    }

    let dt = sim.dt;
    let n_eq = (equilibrate_fs / dt) as usize;
    let n_meas = (measure_fs / dt) as usize;
    let stride = (sample_every_fs / dt).max(1.0) as usize;
    let n_bins = (r_max / bin_width).round() as usize;

    let n_li_total = n_li_free; // free Li⁺ count
    let area = (domain_w * domain_h) as f64;
    let rho_dipole_neg = n_dipole as f64 / area; // dipole-neg density

    println!("dipole_spike");
    println!(
        "  domain {}×{} Å²,  bodies = {} ({} free Li⁺, {} free anion, {} dipole pairs, {} DMC)",
        domain_w,
        domain_h,
        sim.bodies.len(),
        n_li_free,
        n_anion_free,
        n_dipole,
        n_dmc
    );
    println!(
        "  dipole pair: ±{} e at separation r_eq = {} Å,  bond k = {}",
        dipole_charge, r_eq, bond_k
    );
    println!(
        "  equilibrate {} fs, measure {} fs (sample {} fs)",
        equilibrate_fs, measure_fs, sample_every_fs
    );

    print!("  equilibrating with bond forces... ");
    use std::io::Write as _;
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for _ in 0..n_eq {
        sim.step();
        apply_bond_forces(&mut sim, &bonds, r_eq, bond_k, dt);
    }
    println!("done ({:.1} s)", t0.elapsed().as_secs_f32());

    // Diagnostic: average bond length post-equilibration
    let id_to_idx: HashMap<u64, usize> = sim
        .bodies
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut bond_lengths: Vec<f64> = Vec::with_capacity(bonds.len());
    for bond in &bonds {
        if let (Some(&ia), Some(&ib)) =
            (id_to_idx.get(&bond.id_pos), id_to_idx.get(&bond.id_neg))
        {
            bond_lengths.push((sim.bodies[ib].pos - sim.bodies[ia].pos).mag() as f64);
        }
    }
    let mean_bond = bond_lengths.iter().sum::<f64>() / bond_lengths.len() as f64;
    let std_bond = ((bond_lengths
        .iter()
        .map(|x| (x - mean_bond).powi(2))
        .sum::<f64>())
        / bond_lengths.len() as f64)
        .sqrt();
    println!(
        "  bond length post-eq:   mean = {:.3} Å,  std = {:.3} Å  (target r_eq = {} Å)",
        mean_bond, std_bond, r_eq
    );

    // RDF measurement: free Li⁺ → dipole-neg ends.
    let mut histogram = vec![0_u64; n_bins];
    let mut n_samples: u64 = 0;

    let collect = |sim: &Simulation, hist: &mut Vec<u64>| {
        let li_pos: Vec<Vec2> = sim
            .bodies
            .iter()
            .filter(|b| b.species == Species::LithiumIon && !bonded_any.contains(&b.id))
            .map(|b| b.pos)
            .collect();
        let dipole_neg_pos: Vec<Vec2> = sim
            .bodies
            .iter()
            .filter(|b| bonded_neg_ids.contains(&b.id))
            .map(|b| b.pos)
            .collect();
        for &li in &li_pos {
            for &dn in &dipole_neg_pos {
                let r = (li - dn).mag() as f64;
                if r < r_max {
                    let bin = (r / bin_width).floor() as usize;
                    if bin < n_bins {
                        hist[bin] += 1;
                    }
                }
            }
        }
    };

    collect(&sim, &mut histogram);
    n_samples += 1;

    let mut steps_since_sample: usize = 0;
    print!("  measuring RDF... ");
    std::io::stdout().flush().ok();
    let t0 = std::time::Instant::now();
    for step in 1..=n_meas {
        sim.step();
        apply_bond_forces(&mut sim, &bonds, r_eq, bond_k, dt);
        steps_since_sample += 1;
        if steps_since_sample >= stride || step == n_meas {
            collect(&sim, &mut histogram);
            n_samples += 1;
            steps_since_sample = 0;
        }
    }
    println!(
        "done ({:.1} s, {} samples)",
        t0.elapsed().as_secs_f32(),
        n_samples
    );

    // Final bond-length stats
    let id_to_idx: HashMap<u64, usize> = sim
        .bodies
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut bond_lengths_final: Vec<f64> = Vec::with_capacity(bonds.len());
    for bond in &bonds {
        if let (Some(&ia), Some(&ib)) =
            (id_to_idx.get(&bond.id_pos), id_to_idx.get(&bond.id_neg))
        {
            bond_lengths_final.push((sim.bodies[ib].pos - sim.bodies[ia].pos).mag() as f64);
        }
    }
    let mean_bond_final = bond_lengths_final.iter().sum::<f64>() / bond_lengths_final.len() as f64;
    let std_bond_final = ((bond_lengths_final
        .iter()
        .map(|x| (x - mean_bond_final).powi(2))
        .sum::<f64>())
        / bond_lengths_final.len() as f64)
        .sqrt();

    // g(r) and coordination number — same convention as li_ec_coordination
    let mut g_of_r = vec![0.0_f64; n_bins];
    for i in 0..n_bins {
        let r_low = i as f64 * bin_width;
        let r_high = (i + 1) as f64 * bin_width;
        let shell_area = std::f64::consts::PI * (r_high.powi(2) - r_low.powi(2));
        let expected = rho_dipole_neg * shell_area * n_li_total as f64 * n_samples as f64;
        g_of_r[i] = if expected > 0.0 {
            histogram[i] as f64 / expected
        } else {
            0.0
        };
    }

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

    let coord_count: u64 = histogram[..=r_min_bin].iter().sum();
    let n_coord = coord_count as f64 / (n_li_total as f64 * n_samples as f64);

    let baseline_2_2 = 2.18_f64; // single-charge EC coordination from Phase 2.2
    let target_lo = 3.0_f64;
    let target_hi = 6.0_f64;
    let pass = (target_lo..=target_hi).contains(&n_coord);

    println!();
    println!("=== Dipole-spike RDF result ===");
    println!("  free Li⁺           = {}", n_li_total);
    println!("  dipole pairs       = {}", n_dipole);
    println!(
        "  bond length pre-meas:  mean = {:.3} ± {:.3} Å",
        mean_bond, std_bond
    );
    println!(
        "  bond length post-meas: mean = {:.3} ± {:.3} Å",
        mean_bond_final, std_bond_final
    );
    println!("  first peak: r = {:.3} Å, g(r) = {:.2}", r_peak, g_peak);
    println!("  first min:  r = {:.3} Å, g(r) = {:.3}", r_min, g_min);
    println!(
        "  coordination n(r_min) = {:.3}   (target [{:.1}, {:.1}], 2.2-baseline = {})",
        n_coord, target_lo, target_hi, baseline_2_2
    );
    println!(
        "  Δ vs Phase-2.2 baseline = {:+.3}   ({:+.1}%)",
        n_coord - baseline_2_2,
        100.0 * (n_coord - baseline_2_2) / baseline_2_2
    );
    println!("  PASS = {}", pass);

    let csv_path = out_dir.join("rdf.csv");
    let mut csv = std::io::BufWriter::new(fs::File::create(&csv_path).unwrap());
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
        let cum_per_li = cum_count as f64 / (n_li_total as f64 * n_samples as f64);
        writeln!(
            csv,
            "{},{:.4},{:.4},{:.4},{},{:.6e},{},{:.6}",
            i, r_low, r_high, r_center, histogram[i], g_of_r[i], cum_count, cum_per_li
        )
        .unwrap();
    }
    csv.flush().ok();

    let bond_csv = out_dir.join("bond_lengths.csv");
    let mut bw = std::io::BufWriter::new(fs::File::create(&bond_csv).unwrap());
    writeln!(bw, "bond_idx,bond_length_pre_meas,bond_length_post_meas").unwrap();
    for (i, (&pre, &post)) in bond_lengths
        .iter()
        .zip(bond_lengths_final.iter())
        .enumerate()
    {
        writeln!(bw, "{},{:.4},{:.4}", i, pre, post).unwrap();
    }
    bw.flush().ok();

    let result = json!({
        "test": "dipole_spike",
        "value": n_coord,
        "value_label": "li_dipole_neg_first_shell_coordination_number",
        "unit": "dipole_neg_per_li",
        "tolerance": {
            "kind": "range",
            "min": target_lo,
            "max": target_hi,
            "literature_source": "liquid carbonate Li⁺ coordination, 3D bulk"
        },
        "pass": pass,
        "details": {
            "seed": format!("0x{:X}", seed),
            "domain_w": domain_w,
            "domain_h": domain_h,
            "n_free_li": n_li_free,
            "n_free_anion": n_anion_free,
            "n_dipole_pairs": n_dipole,
            "n_dmc": n_dmc,
            "dipole_charge": dipole_charge,
            "r_eq": r_eq,
            "bond_k": bond_k,
            "equilibrate_fs": equilibrate_fs,
            "measure_fs": measure_fs,
            "sample_every_fs": sample_every_fs,
            "n_samples": n_samples,
            "bond_length_pre_mean": mean_bond,
            "bond_length_pre_std": std_bond,
            "bond_length_post_mean": mean_bond_final,
            "bond_length_post_std": std_bond_final,
            "r_peak_angstrom": r_peak,
            "g_peak": g_peak,
            "r_min_angstrom": r_min,
            "g_min": g_min,
            "n_coord_first_shell": n_coord,
            "phase_2_2_baseline": baseline_2_2,
            "delta_from_baseline": n_coord - baseline_2_2,
            "rho_dipole_neg_per_a2": rho_dipole_neg,
        }
    });
    fs::write(
        out_dir.join("result.json"),
        serde_json::to_string_pretty(&result).unwrap(),
    )
    .unwrap();
    println!();
    println!("RDF CSV:   {}", csv_path.display());
    println!("Bond CSV:  {}", bond_csv.display());
    println!("Result:    {}", out_dir.join("result.json").display());
}
