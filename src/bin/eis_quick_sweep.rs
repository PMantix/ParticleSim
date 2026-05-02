//! eis_quick_sweep — Galvanostatic EIS sweep at proven-working frequencies.
//!
//! Pairs with `tests/eis_validation_runs.rs` (cycle 2). The test sweeps
//! the spec ultra-low frequencies [5e-7, 5e-6] /fs which take 12+ hours.
//! This binary runs the same setup at f ∈ [4e-4, 1.4e-3, 5e-3] /fs — a
//! frequency range historically demonstrated to produce R²(V) > 0.95 in
//! eis_timeseries/eis_ts_025_3.989e-4.csv etc. — finishing in ~30-60 min.
//!
//! Usage:
//!   cargo run --release --bin eis_quick_sweep
//!     [--scenario <path.toml>] [--seed <u64>]
//!     [--amplitude <e/fs>] [--f-min <1/fs>] [--f-max <1/fs>]
//!     [--points-per-decade <float>] [--settle-periods <n>]
//!     [--periods-per-freq <n>]

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::eis::EisMode;
use particle_sim::simulation::Simulation;
use std::sync::mpsc::channel;
use std::time::Instant;
use ultraviolet::Vec2;

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

fn parse_u64_flexible(s: &str) -> Option<u64> {
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

fn main() {
    let mut scenario =
        String::from("measurement_configs/eis_validation_flat_symmetric.toml");
    let mut seed: u64 = 0xC0FFEE;
    let mut amplitude: f32 = 5e-4;
    let mut f_min: f32 = 4e-4;
    let mut f_max: f32 = 5e-3;
    let mut points_per_decade: f32 = 2.0;
    let mut settle_periods: usize = 4;
    let mut periods_per_freq: usize = 4;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut pre_hold_fs: f32 = 10_000.0;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => { i += 1; scenario = args[i].clone(); }
            "--seed" => { i += 1; seed = parse_u64_flexible(&args[i]).unwrap(); }
            "--amplitude" => { i += 1; amplitude = args[i].parse().unwrap(); }
            "--f-min" => { i += 1; f_min = args[i].parse().unwrap(); }
            "--f-max" => { i += 1; f_max = args[i].parse().unwrap(); }
            "--points-per-decade" => { i += 1; points_per_decade = args[i].parse().unwrap(); }
            "--settle-periods" => { i += 1; settle_periods = args[i].parse().unwrap(); }
            "--periods-per-freq" => { i += 1; periods_per_freq = args[i].parse().unwrap(); }
            "--equilibrate-fs" => { i += 1; equilibrate_fs = args[i].parse().unwrap(); }
            "--pre-hold-fs" => { i += 1; pre_hold_fs = args[i].parse().unwrap(); }
            other => {
                eprintln!("Unknown arg: {}", other);
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let (tx, _rx) = channel::<SimCommand>();
    *SIM_COMMAND_SENDER.lock() = Some(tx);
    fastrand::seed(seed);

    let config = InitConfig::load_from_file(&scenario).expect("scenario load failed");
    let (full_w, full_h) = config.simulation.as_ref().unwrap().domain_size();

    let mut sim = Simulation::new();
    sim.domain_width = full_w / 2.0;
    sim.domain_height = full_h / 2.0;
    sim.cell_list.update_domain_size(sim.domain_width, sim.domain_height);

    for rect in &config.particles.metal_rectangles {
        let body = template_body(rect.to_species().unwrap());
        let (x, y) = rect.to_origin_coords();
        handle_command(
            SimCommand::AddRectangle { body, x, y, width: rect.width, height: rect.height },
            &mut sim,
        );
    }
    for foil in &config.particles.foil_rectangles {
        let (x, y) = foil.to_origin_coords();
        handle_command(
            SimCommand::AddFoil {
                width: foil.width,
                height: foil.height,
                x, y,
                particle_radius: Species::FoilMetal.radius(),
                current: foil.current,
            },
            &mut sim,
        );
    }
    for entry in &config.particles.random {
        let body = template_body(entry.to_species().unwrap());
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

    println!("scenario={} seed=0x{:X}", scenario, seed);
    println!("bodies={} foils={}", sim.bodies.len(), sim.foils.len());

    let mut group_a: Vec<u64> = Vec::new();
    let mut group_b: Vec<u64> = Vec::new();
    for foil in &sim.foils {
        let (mut cx, mut cn) = (0.0_f32, 0.0_f32);
        for bid in &foil.body_ids {
            if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) {
                cx += b.pos.x;
                cn += 1.0;
            }
        }
        if cn > 0.0 { cx /= cn; }
        if cx < 0.0 { group_a.push(foil.id); } else { group_b.push(foil.id); }
    }
    handle_command(
        SimCommand::SetFoilGroups { group_a: group_a.clone(), group_b: group_b.clone() },
        &mut sim,
    );

    let dt = sim.dt;
    println!("dt={} fs  group_a={:?}  group_b={:?}", dt, group_a, group_b);

    let n_eq = (equilibrate_fs / dt) as usize;
    println!("equilibrating {} fs ({} steps)", equilibrate_fs, n_eq);
    for _ in 0..n_eq { sim.step(); }

    let n_pre = (pre_hold_fs / dt) as usize;
    println!("zero-current pre-hold {} fs ({} steps)", pre_hold_fs, n_pre);
    for _ in 0..n_pre { sim.step(); }

    handle_command(
        SimCommand::StartEIS {
            amplitude,
            f_min,
            f_max,
            points_per_decade,
            periods_per_freq,
            settle_periods,
            mode: EisMode::Galvanostatic,
            repeats_per_freq: 1,
            voltage_probes: 0,
            c_virtual: 1e-3,
        },
        &mut sim,
    );

    let mut steps = 0usize;
    let t0 = Instant::now();
    loop {
        sim.step();
        steps += 1;
        if sim.eis_state.as_ref().map_or(false, |e| e.finished) { break; }
    }
    let elapsed = t0.elapsed();
    println!(
        "sweep finished in {} steps ({:.1}s wall, sim time {:.2e} fs)",
        steps,
        elapsed.as_secs_f32(),
        steps as f32 * dt
    );

    let eis = sim.eis_state.as_ref().unwrap();
    println!("\n=== {} EisPoints ===", eis.results.len());
    println!(
        "{:>4}  {:>10}  {:>11}  {:>11}  {:>10}  {:>8}  {:>8}  {:>8}  {:>10}  {:>10}",
        "i", "freq", "z_real", "z_imag", "|Z|", "phase", "R²(V)", "R²(I)", "V_amp", "I_amp"
    );
    for (i, p) in eis.results.iter().enumerate() {
        println!(
            "{:>4}  {:>10.3e}  {:>+11.3e}  {:>+11.3e}  {:>10.3e}  {:>+7.1}°  {:>8.4}  {:>8.4}  {:>10.3e}  {:>10.3e}",
            i, p.frequency, p.z_real, p.z_imag, p.magnitude, p.phase_deg,
            p.fit_r2_v, p.fit_r2_i, p.fit_v_amp, p.fit_i_amp
        );
    }
}
