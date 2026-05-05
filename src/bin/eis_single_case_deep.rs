//! eis_single_case_deep — run one EIS frequency in detail.
//!
//! Drops position/charge snapshots at N points across one cycle (default 32),
//! plus a continuous time series of V_cell + I + foil electron counts at every
//! step. Output goes to a per-job directory with snapshots/ and series.csv.
//!
//! Defaults match the doe-027 sweet spot: I=0.02, f=5e-5, settle 4 periods,
//! capture 1 cycle of recording at 32-frame resolution. Wall ~5 min on Mac
//! release.

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::plotting::analysis::calculate_cell_voltage;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::fs::{create_dir_all, File};
use std::io::Write as _;
use std::path::PathBuf;
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

fn snapshot_csv(sim: &Simulation, path: &PathBuf, step: usize, time_fs: f32, v_cell: f32, applied_i: f32) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    writeln!(f, "# step={} time_fs={:.2} v_cell={:.6e} applied_i={:.6e}", step, time_fs, v_cell, applied_i)?;
    writeln!(f, "id,species,x,y,charge,vx,vy")?;
    for b in &sim.bodies {
        writeln!(f, "{},{:?},{:.4},{:.4},{:.6e},{:.4e},{:.4e}",
                 b.id, b.species, b.pos.x, b.pos.y, b.charge, b.vel.x, b.vel.y)?;
    }
    Ok(())
}

fn main() {
    // CLI defaults — easy to override with --arg
    let mut scenario = String::from("measurement_configs/eis_validation_flat_symmetric.toml");
    let mut amplitude: f32 = 0.02;
    let mut freq: f32 = 5e-5;
    let mut settle_periods: usize = 4;
    let mut capture_periods: f32 = 1.0;
    let mut snapshots_per_cycle: usize = 32;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut out_dir: PathBuf = PathBuf::from("doe_results/eis_single_case_deep");
    let mut seed: u64 = 0xC0FFEE;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => { i += 1; scenario = args[i].clone(); }
            "--amplitude" => { i += 1; amplitude = args[i].parse().unwrap(); }
            "--freq" => { i += 1; freq = args[i].parse().unwrap(); }
            "--settle-periods" => { i += 1; settle_periods = args[i].parse().unwrap(); }
            "--capture-periods" => { i += 1; capture_periods = args[i].parse().unwrap(); }
            "--snapshots-per-cycle" => { i += 1; snapshots_per_cycle = args[i].parse().unwrap(); }
            "--equilibrate-fs" => { i += 1; equilibrate_fs = args[i].parse().unwrap(); }
            "--out-dir" => { i += 1; out_dir = PathBuf::from(&args[i]); }
            "--seed" => { i += 1; seed = args[i].parse().unwrap(); }
            other => { eprintln!("unknown arg: {}", other); std::process::exit(2); }
        }
        i += 1;
    }

    let snapshot_dir = out_dir.join("snapshots");
    create_dir_all(&snapshot_dir).unwrap();
    let series_path = out_dir.join("series.csv");
    let summary_path = out_dir.join("summary.txt");

    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);
    fastrand::seed(seed);

    let cfg = InitConfig::load_from_file(&scenario).expect("scenario load");
    let (full_w, full_h) = cfg.simulation.as_ref().unwrap().domain_size();
    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    for rect in &cfg.particles.metal_rectangles {
        let body = template_body(rect.to_species().unwrap());
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

    // Group foils
    let mut group_a: Vec<u64> = Vec::new();
    let mut group_b: Vec<u64> = Vec::new();
    for foil in &sim.foils {
        let (mut cx, mut cn) = (0.0f32, 0.0f32);
        for bid in &foil.body_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) { cx += b.pos.x; cn += 1.0; }
        }
        if cn > 0.0 { cx /= cn; }
        if cx < 0.0 { group_a.push(foil.id); } else { group_b.push(foil.id); }
    }
    handle_command(SimCommand::SetFoilGroups { group_a: group_a.clone(), group_b: group_b.clone() }, &mut sim);

    // Build probe lists (ALL foil bodies — matches voltage_probes=0 in EIS).
    // These are FIXED at the start; they do NOT swap with dc_current sign,
    // unlike calculate_cell_voltage's is_pos heuristic which IS dc_current-sign-driven.
    let probe_a: Vec<u64> = sim.foils.iter().filter(|f| group_a.contains(&f.id))
        .flat_map(|f| f.body_ids.iter().copied()).collect();
    let probe_b: Vec<u64> = sim.foils.iter().filter(|f| group_b.contains(&f.id))
        .flat_map(|f| f.body_ids.iter().copied()).collect();

    let dt = sim.dt;
    let k = sim.config.coulomb_constant;
    let period_fs = 1.0 / freq;
    let n_eq = (equilibrate_fs / dt) as usize;
    let n_settle = (settle_periods as f32 * period_fs / dt) as usize;
    let n_capture = (capture_periods * period_fs / dt) as usize;
    let snapshot_step_interval = (n_capture as f32 / (capture_periods * snapshots_per_cycle as f32)).max(1.0) as usize;

    println!("\n=== eis_single_case_deep ===");
    println!("scenario:        {}", scenario);
    println!("seed:            0x{:X}", seed);
    println!("amplitude (I):   {} e/fs", amplitude);
    println!("freq:            {:.3e} /fs (T = {:.2e} fs = {} steps)", freq, period_fs, (period_fs / dt) as usize);
    println!("settle:          {} periods = {} fs = {} steps", settle_periods, n_settle as f32 * dt, n_settle);
    println!("capture:         {} periods = {} fs = {} steps", capture_periods, n_capture as f32 * dt, n_capture);
    println!("snapshots:       {} (every {} steps)", (capture_periods * snapshots_per_cycle as f32) as usize, snapshot_step_interval);
    println!("equilibrate:     {} steps", n_eq);
    println!("output dir:      {}", out_dir.display());
    println!("bodies={} foils={}", sim.bodies.len(), sim.foils.len());

    // Equilibrate (no current)
    for _ in 0..n_eq { sim.step(); }

    // Apply Galvanostatic AC: foil A = +amp*cos(ωt), B = -amp*cos(ωt).
    // NOTE: deviates from eis.rs (which uses sin) on purpose. The sin form
    // integrates to ΔQ(t) = A·(1-cos(ωt))/ω, which is always ≥ 0 — i.e. the
    // cell accumulates positive charge throughout every cycle and never
    // crosses back through equilibrium. cos integrates to A·sin(ωt)/ω, which
    // swings symmetrically between -A/ω and +A/ω, so the steady-state
    // oscillates around true equilibrium instead of around a positively-biased
    // state. Lock-in fit (re·cos + im·sin + dc) is generic and unaffected.
    let omega = 2.0 * std::f32::consts::PI * freq;
    let t_start = sim.time;
    let n_a = group_a.len() as f32;
    let n_b = group_b.len() as f32;

    // Open series.csv — record BOTH V_cell variants so we can compare
    let mut series = File::create(&series_path).unwrap();
    writeln!(series, "step,time_fs,phase,ac_value,applied_i,v_cell_calc,v_cell_eis,foil_a_de,foil_b_de").unwrap();

    let mut snapshot_idx: usize = 0;
    let mut total_step: usize = 0;

    let total_steps = n_settle + n_capture;
    for s in 0..total_steps {
        // Compute ac_value for this step (matching eis.rs:get_perturbation logic)
        let t_local = sim.time - t_start;
        let ac_value = amplitude * (omega * t_local).cos();
        // Apply to foils
        for &fid in &group_a {
            if let Some(f) = sim.foils.iter_mut().find(|f| f.id == fid) {
                f.dc_current = ac_value / n_a;
            }
        }
        for &fid in &group_b {
            if let Some(f) = sim.foils.iter_mut().find(|f| f.id == fid) {
                f.dc_current = -ac_value / n_b;
            }
        }
        // Step
        sim.step();
        total_step += 1;

        // Sample series + (during capture) snapshots
        let in_capture = s >= n_settle;
        if in_capture {
            let phase = ((omega * t_local) % (2.0 * std::f32::consts::PI) + 2.0 * std::f32::consts::PI) % (2.0 * std::f32::consts::PI);
            // I as the EIS lock-in measures it: -raw_i
            let raw_i_a: f32 = group_a.iter()
                .filter_map(|&fid| sim.foils.iter().find(|f| f.id == fid))
                .map(|f| f.dc_current).sum();
            let applied_i = -raw_i_a;
            let v_cell_calc = calculate_cell_voltage(&sim.bodies, &sim.foils, k);
            let v_cell_eis  = sim.compute_eis_voltage_by_potential(&group_a, &group_b, &probe_a, &probe_b);
            // Foil electron deltas (and clear) — explicit loops avoid nested mutable borrows
            let mut foil_a_de: i32 = 0;
            for fid in &group_a {
                if let Some(f) = sim.foils.iter_mut().find(|f| f.id == *fid) {
                    foil_a_de += f.electron_delta_since_measure;
                    f.electron_delta_since_measure = 0;
                }
            }
            let mut foil_b_de: i32 = 0;
            for fid in &group_b {
                if let Some(f) = sim.foils.iter_mut().find(|f| f.id == *fid) {
                    foil_b_de += f.electron_delta_since_measure;
                    f.electron_delta_since_measure = 0;
                }
            }
            writeln!(series, "{},{:.2},{:.6},{:.6e},{:.6e},{:.6e},{:.6e},{},{}",
                     total_step, sim.time, phase, ac_value, applied_i, v_cell_calc, v_cell_eis, foil_a_de, foil_b_de).unwrap();

            // Snapshot every snapshot_step_interval steps
            let capture_step = s - n_settle;
            if capture_step % snapshot_step_interval == 0 {
                let path = snapshot_dir.join(format!("frame_{:03}.csv", snapshot_idx));
                snapshot_csv(&sim, &path, total_step, sim.time, v_cell_eis, applied_i).unwrap();
                snapshot_idx += 1;
            }
        }
    }

    // Save summary
    let mut summ = File::create(&summary_path).unwrap();
    writeln!(summ, "scenario={}", scenario).ok();
    writeln!(summ, "seed=0x{:X}", seed).ok();
    writeln!(summ, "amplitude={}", amplitude).ok();
    writeln!(summ, "freq={}", freq).ok();
    writeln!(summ, "period_fs={}", period_fs).ok();
    writeln!(summ, "dt_fs={}", dt).ok();
    writeln!(summ, "settle_steps={}", n_settle).ok();
    writeln!(summ, "capture_steps={}", n_capture).ok();
    writeln!(summ, "snapshots_taken={}", snapshot_idx).ok();
    writeln!(summ, "snapshot_step_interval={}", snapshot_step_interval).ok();
    writeln!(summ, "n_bodies={}", sim.bodies.len()).ok();
    writeln!(summ, "group_a={:?}", group_a).ok();
    writeln!(summ, "group_b={:?}", group_b).ok();

    println!("\nWrote {} snapshots, {} series rows, summary",
             snapshot_idx, n_capture);
    println!("output: {}", out_dir.display());
}
