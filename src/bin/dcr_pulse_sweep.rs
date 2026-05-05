//! dcr_pulse_sweep — Galvanostatic square-wave (DCR) pulse probe.
//!
//! Phase 1 of `docs/EIS_DCR_PULSE_PLAN.md`. Time-domain analog of the
//! EIS frequency-domain probe: apply a square pulse of amplitude `I_amp`
//! for `duration_on_fs`, then rest for `duration_rest_fs`, then repeat.
//! Log dense V(t), I(t) so an external Python script can fit a 2-RC
//! ECM (R₀, R₁‖C₁ fast arc, R₂‖C₂ slow arc).
//!
//! Mirrors `eis_quick_sweep`'s scenario+equilibrate setup. Skips the EIS
//! lock-in pipeline entirely; foil currents are driven directly by this
//! binary between `sim.step()` calls.
//!
//! Output (under `--out-dir`, default `doe_results/dcr_pulse_sweep/<run_id>/`):
//!   - `dense_series.csv` — t_fs, step, phase, i_applied, v_cell
//!   - `pulse_summary.csv` — pulse_idx, v_pre, v_pulse_end, v_relax_end, i_amp, r0_apparent
//!   - `summary.txt` — run params + counts
//!
//! Usage:
//!   cargo run --release --bin dcr_pulse_sweep -- \
//!     [--scenario <path.toml>] [--seed <u64>] \
//!     --amplitude <e/fs> \
//!     [--duration-on-fs <float>] [--duration-rest-fs <float>] \
//!     [--num-pulses <n>] [--log-stride <n>] \
//!     [--equilibrate-fs <float>] [--pre-hold-fs <float>] \
//!     [--out-dir <path>] [--run-id <name>]

use particle_sim::app::command_loop::handle_command;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use particle_sim::simulation::Simulation;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Instant;
use ultraviolet::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DcrMode {
    Galvanostatic,
    Potentiostatic,
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

fn parse_u64_flexible(s: &str) -> Option<u64> {
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

fn main() {
    // --- Defaults -------------------------------------------------------
    let mut scenario =
        String::from("measurement_configs/eis_validation_flat_symmetric.toml");
    let mut seed: u64 = 0xC0FFEE;
    let mut amplitude: f32 = 0.02;
    // Pulse timing — sim time scale.
    let mut duration_on_fs: f32 = 1000.0;
    let mut duration_rest_fs: f32 = 5000.0;
    let mut num_pulses: usize = 5;
    let mut log_stride: usize = 1;
    let mut equilibrate_fs: f32 = 50_000.0;
    let mut pre_hold_fs: f32 = 10_000.0;
    let mut out_dir_arg: Option<String> = None;
    let mut run_id_arg: Option<String> = None;
    let mut mode: DcrMode = DcrMode::Galvanostatic;

    // --- Args -----------------------------------------------------------
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
                seed = parse_u64_flexible(&args[i]).expect("--seed");
            }
            "--amplitude" => {
                i += 1;
                amplitude = args[i].parse().expect("--amplitude");
            }
            "--duration-on-fs" => {
                i += 1;
                duration_on_fs = args[i].parse().expect("--duration-on-fs");
            }
            "--duration-rest-fs" => {
                i += 1;
                duration_rest_fs = args[i].parse().expect("--duration-rest-fs");
            }
            "--num-pulses" => {
                i += 1;
                num_pulses = args[i].parse().expect("--num-pulses");
            }
            "--log-stride" => {
                i += 1;
                log_stride = args[i].parse().expect("--log-stride");
            }
            "--equilibrate-fs" => {
                i += 1;
                equilibrate_fs = args[i].parse().expect("--equilibrate-fs");
            }
            "--pre-hold-fs" => {
                i += 1;
                pre_hold_fs = args[i].parse().expect("--pre-hold-fs");
            }
            "--out-dir" => {
                i += 1;
                out_dir_arg = Some(args[i].clone());
            }
            "--run-id" => {
                i += 1;
                run_id_arg = Some(args[i].clone());
            }
            "--mode" => {
                i += 1;
                mode = match args[i].as_str() {
                    "galvanostatic" | "galvano" | "g" => DcrMode::Galvanostatic,
                    "potentiostatic" | "potentio" | "p" => DcrMode::Potentiostatic,
                    other => {
                        eprintln!("--mode: expected galvanostatic|potentiostatic, got {other}");
                        std::process::exit(2);
                    }
                };
            }
            other => {
                eprintln!("Unknown arg: {}", other);
                std::process::exit(2);
            }
        }
        i += 1;
    }

    // --- Set up simulation (mirrors eis_quick_sweep) --------------------
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

    // --- Identify left/right foil groups (same convention as EIS) -------
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
    let n_a = group_a.len().max(1) as f32;
    let n_b = group_b.len().max(1) as f32;

    // Probe IDs = all foil-body IDs in each group (matches EIS `voltage_probes=0`).
    let collect_foil_bodies = |foil_ids: &[u64]| -> Vec<u64> {
        let mut ids = Vec::new();
        for &fid in foil_ids {
            if let Some(foil) = sim.foils.iter().find(|f| f.id == fid) {
                ids.extend(foil.body_ids.iter().copied());
            }
        }
        ids
    };
    let probe_a: Vec<u64> = collect_foil_bodies(&group_a);
    let probe_b: Vec<u64> = collect_foil_bodies(&group_b);
    println!("metal probe bodies: A={} B={}", probe_a.len(), probe_b.len());

    // Bulk-electrolyte probes: pick electrolyte-species bodies in slabs near
    // each foil's bulk-facing side. Slab is 10–50 Å past the foil's centroid
    // x toward the cell center. Captures the ionic potential gradient that
    // develops due to ion redistribution under polarization.
    let foil_centroid_x = |foil_ids: &[u64]| -> f32 {
        let mut sx = 0.0_f32;
        let mut n = 0.0_f32;
        for &fid in foil_ids {
            if let Some(foil) = sim.foils.iter().find(|f| f.id == fid) {
                for bid in &foil.body_ids {
                    if let Some(b) = sim.bodies.iter().find(|b| b.id == *bid) {
                        sx += b.pos.x;
                        n += 1.0;
                    }
                }
            }
        }
        if n > 0.0 { sx / n } else { 0.0 }
    };
    let foil_a_x = foil_centroid_x(&group_a);
    let foil_b_x = foil_centroid_x(&group_b);
    let bulk_slab = |foil_x: f32, toward_center_sign: f32| -> Vec<u64> {
        // Slab spans foil_x + sign*10 to foil_x + sign*50 (10 to 50 Å into
        // the bulk, on the cell-center side of the foil).
        let x_min = foil_x + toward_center_sign * 10.0;
        let x_max = foil_x + toward_center_sign * 50.0;
        let (lo, hi) = if x_min < x_max { (x_min, x_max) } else { (x_max, x_min) };
        sim.bodies
            .iter()
            .filter(|b| {
                matches!(
                    b.species,
                    Species::LithiumIon
                        | Species::ElectrolyteAnion
                        | Species::EC
                        | Species::DMC
                        | Species::VC
                        | Species::FEC
                        | Species::EMC
                ) && b.pos.x >= lo
                    && b.pos.x <= hi
            })
            .map(|b| b.id)
            .collect()
    };
    // For left foil (x < 0): "toward center" is +x, so sign = +1.
    // For right foil (x > 0): "toward center" is -x, so sign = -1.
    let probe_bulk_a: Vec<u64> = bulk_slab(foil_a_x, if foil_a_x < 0.0 { 1.0 } else { -1.0 });
    let probe_bulk_b: Vec<u64> = bulk_slab(foil_b_x, if foil_b_x < 0.0 { 1.0 } else { -1.0 });
    println!(
        "bulk probe bodies: A={} (slab near foil_a_x={:.1}), B={} (slab near foil_b_x={:.1})",
        probe_bulk_a.len(),
        foil_a_x,
        probe_bulk_b.len(),
        foil_b_x
    );

    // Pre-build exclude sets for each probe group:
    // - metal probes exclude their own foil's bodies (avoid self-interaction)
    // - bulk probes exclude the probe IDs themselves (avoid divergent r=0)
    use std::collections::HashSet;
    let exclude_metal_a: HashSet<u64> = probe_a.iter().copied().collect();
    let exclude_metal_b: HashSet<u64> = probe_b.iter().copied().collect();
    let exclude_bulk_a: HashSet<u64> = probe_bulk_a.iter().copied().collect();
    let exclude_bulk_b: HashSet<u64> = probe_bulk_b.iter().copied().collect();

    let dt = sim.dt;
    println!(
        "dt={} fs  group_a={:?}  group_b={:?}  mode={:?}",
        dt, group_a, group_b, mode
    );

    // Potentiostatic: enable overpotential controllers on group foils with
    // neutral target (1.0). The pulse injects perturbations on top of this.
    if mode == DcrMode::Potentiostatic {
        for foil in sim.foils.iter_mut() {
            if group_a.contains(&foil.id) || group_b.contains(&foil.id) {
                foil.enable_overpotential_mode(1.0);
            }
        }
    }

    // --- Equilibrate (no logging — thermalization only) -----------------
    let n_eq = (equilibrate_fs / dt) as usize;
    println!("equilibrating {} fs ({} steps)", equilibrate_fs, n_eq);
    for _ in 0..n_eq {
        sim.step();
    }

    // --- Output paths (open before pre-hold so we can log it) ----------
    let run_id = run_id_arg.unwrap_or_else(|| {
        format!(
            "amp{:.3e}_ton{:.0}fs_trest{:.0}fs_n{}",
            amplitude, duration_on_fs, duration_rest_fs, num_pulses
        )
    });
    let out_dir = PathBuf::from(
        out_dir_arg.unwrap_or_else(|| format!("doe_results/dcr_pulse_sweep/{}", run_id)),
    );
    fs::create_dir_all(&out_dir).expect("create out dir");
    let series_path = out_dir.join("dense_series.csv");
    let summary_path = out_dir.join("pulse_summary.csv");
    let info_path = out_dir.join("summary.txt");
    let mut series_f = fs::File::create(&series_path).expect("series.csv");
    let mut summary_f = fs::File::create(&summary_path).expect("summary.csv");
    writeln!(
        series_f,
        "t_fs,step,phase,i_applied,v_cell,v_metal_a,v_metal_b,v_bulk_a,v_bulk_b"
    )
    .unwrap();

    // We need step counters declared before pre-hold logging.
    let mut step_global = 0usize;
    let mut sim_time_fs = 0.0_f32;

    // Helper closures (defined before pre-hold so they can be used during it).
    let read_v = |sim: &Simulation| -> f32 {
        sim.compute_eis_voltage_by_potential(&group_a, &group_b, &probe_a, &probe_b)
    };

    // Returns (V_metal_a, V_metal_b, V_bulk_a, V_bulk_b).
    // Each potential is averaged over its probe positions, with appropriate
    // self/foil exclusion.
    let read_v_components = |sim: &Simulation| -> (f32, f32, f32, f32) {
        let v_ma = sim.compute_potential_at_probes(&probe_a, &exclude_metal_a);
        let v_mb = sim.compute_potential_at_probes(&probe_b, &exclude_metal_b);
        let v_ba = sim.compute_potential_at_probes(&probe_bulk_a, &exclude_bulk_a);
        let v_bb = sim.compute_potential_at_probes(&probe_bulk_b, &exclude_bulk_b);
        (v_ma, v_mb, v_ba, v_bb)
    };

    let read_applied_current = |sim: &Simulation| -> f32 {
        match mode {
            DcrMode::Galvanostatic => {
                if let Some(foil) = sim.foils.iter().find(|f| group_a.contains(&f.id)) {
                    foil.dc_current
                } else {
                    0.0
                }
            }
            DcrMode::Potentiostatic => {
                let mut tot = 0.0_f32;
                let mut n = 0;
                for foil in &sim.foils {
                    if group_a.contains(&foil.id) {
                        if let Some(ctrl) = foil.overpotential_controller.as_ref() {
                            tot += ctrl.last_output_current;
                            n += 1;
                        }
                    }
                }
                if n > 0 { tot / n as f32 } else { 0.0 }
            }
        }
    };

    let apply_perturbation = |sim: &mut Simulation, amp_a: f32, amp_b: f32| {
        for foil in sim.foils.iter_mut() {
            let in_a = group_a.contains(&foil.id);
            let in_b = group_b.contains(&foil.id);
            if !in_a && !in_b {
                continue;
            }
            match mode {
                DcrMode::Galvanostatic => {
                    foil.dc_current = if in_a { amp_a } else { amp_b };
                }
                DcrMode::Potentiostatic => {
                    if let Some(ctrl) = foil.overpotential_controller.as_mut() {
                        ctrl.target_ratio = 1.0 + if in_a { amp_a } else { amp_b };
                    }
                }
            }
        }
    };

    // Helper to write a single dense_series row with all 4 V components.
    fn log_row(
        f: &mut fs::File,
        t_fs: f32,
        step: usize,
        phase: &str,
        i: f32,
        v_cell: f32,
        v_metal_a: f32,
        v_metal_b: f32,
        v_bulk_a: f32,
        v_bulk_b: f32,
    ) {
        let _ = writeln!(
            f,
            "{:.3},{},{},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e}",
            t_fs, step, phase, i, v_cell, v_metal_a, v_metal_b, v_bulk_a, v_bulk_b
        );
    }

    // --- Pre-hold (no perturbation, LOGGED so the rest period is visible) ---
    let n_pre = (pre_hold_fs / dt) as usize;
    println!("pre-hold {} fs ({} steps)", pre_hold_fs, n_pre);
    // Log the pre-hold start sample.
    let v_initial = read_v(&sim);
    let (vma, vmb, vba, vbb) = read_v_components(&sim);
    log_row(
        &mut series_f,
        sim_time_fs,
        step_global,
        "pre_hold",
        0.0,
        v_initial,
        vma,
        vmb,
        vba,
        vbb,
    );
    for s in 0..n_pre {
        sim.step();
        step_global += 1;
        sim_time_fs += dt;
        let last = s + 1 == n_pre;
        if (s + 1) % log_stride == 0 || last {
            let v = read_v(&sim);
            let (vma, vmb, vba, vbb) = read_v_components(&sim);
            let i = read_applied_current(&sim);
            log_row(
                &mut series_f,
                sim_time_fs,
                step_global,
                "pre_hold",
                i,
                v,
                vma,
                vmb,
                vba,
                vbb,
            );
        }
    }
    writeln!(
        summary_f,
        "pulse_idx,v_pre,v_post_onset,v_pulse_end,v_relax_end,i_amp,r0_apparent"
    )
    .unwrap();

    // --- Pulse loop -----------------------------------------------------
    let n_on = (duration_on_fs / dt) as usize;
    let n_rest = (duration_rest_fs / dt) as usize;
    println!(
        "pulses={} duration_on={} fs ({} steps)  duration_rest={} fs ({} steps)",
        num_pulses, duration_on_fs, n_on, duration_rest_fs, n_rest
    );

    // (apply_perturbation and read_applied_current closures already defined
    // above before pre-hold logging.)

    let t0 = Instant::now();
    // step_global and sim_time_fs already declared above for pre-hold logging.

    for pulse_idx in 0..num_pulses {
        // ---- V_pre (read at zero current, just before turn-on) ------
        let v_pre = read_v(&sim);
        let (vma, vmb, vba, vbb) = read_v_components(&sim);
        if step_global % log_stride == 0 {
            log_row(&mut series_f, sim_time_fs, step_global, "pre", 0.0, v_pre, vma, vmb, vba, vbb);
        }

        // ---- Pulse-on phase ---------------------------------------
        // Per-foil amplitude split: A gets +amp/n_a, B gets -amp/n_b.
        // In potentiostatic mode the "amplitude" is a target_ratio offset.
        let amp_a = amplitude / n_a;
        let amp_b = -amplitude / n_b;
        apply_perturbation(&mut sim, amp_a, amp_b);

        // First step: capture V_post_onset for R0 estimation. Always log
        // — phase-transition samples are too important to skip with a stride.
        sim.step();
        step_global += 1;
        sim_time_fs += dt;
        let v_post_onset = read_v(&sim);
        let (vma, vmb, vba, vbb) = read_v_components(&sim);
        let i_post_onset = read_applied_current(&sim);
        log_row(
            &mut series_f, sim_time_fs, step_global, "on", i_post_onset,
            v_post_onset, vma, vmb, vba, vbb,
        );

        for s in 1..n_on {
            sim.step();
            step_global += 1;
            sim_time_fs += dt;
            let last_on_step = s + 1 == n_on;
            if (s + 1) % log_stride == 0 || last_on_step {
                let v = read_v(&sim);
                let (vma, vmb, vba, vbb) = read_v_components(&sim);
                let i = read_applied_current(&sim);
                log_row(&mut series_f, sim_time_fs, step_global, "on", i, v, vma, vmb, vba, vbb);
            }
        }
        let v_pulse_end = read_v(&sim);

        // ---- Rest phase -------------------------------------------
        // Galvanostatic: zero current. Potentiostatic: target_ratio back to 1.0.
        apply_perturbation(&mut sim, 0.0, 0.0);
        // First rest step: always log (phase-transition sample).
        if n_rest > 0 {
            sim.step();
            step_global += 1;
            sim_time_fs += dt;
            let v = read_v(&sim);
            let (vma, vmb, vba, vbb) = read_v_components(&sim);
            let i = read_applied_current(&sim);
            log_row(&mut series_f, sim_time_fs, step_global, "rest", i, v, vma, vmb, vba, vbb);
        }
        for s in 1..n_rest {
            sim.step();
            step_global += 1;
            sim_time_fs += dt;
            let last_rest_step = s + 1 == n_rest;
            if (s + 1) % log_stride == 0 || last_rest_step {
                let v = read_v(&sim);
                let (vma, vmb, vba, vbb) = read_v_components(&sim);
                let i = read_applied_current(&sim);
                log_row(&mut series_f, sim_time_fs, step_global, "rest", i, v, vma, vmb, vba, vbb);
            }
        }
        let v_relax_end = read_v(&sim);

        // ---- Per-pulse summary ------------------------------------
        let r0_apparent = (v_post_onset - v_pre) / amplitude;
        writeln!(
            summary_f,
            "{},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e},{:.6e}",
            pulse_idx, v_pre, v_post_onset, v_pulse_end, v_relax_end, amplitude, r0_apparent
        )
        .unwrap();
        println!(
            "pulse {}: V_pre={:.4} V_post_onset={:.4} V_pulse_end={:.4} V_relax_end={:.4} R0={:.3e}",
            pulse_idx, v_pre, v_post_onset, v_pulse_end, v_relax_end, r0_apparent
        );
    }

    let elapsed = t0.elapsed();
    println!(
        "\nDCR pulse sweep finished: {} pulses, {} steps, {:.1}s wall, sim time {:.0} fs",
        num_pulses,
        step_global,
        elapsed.as_secs_f32(),
        sim_time_fs
    );

    let mut info_f = fs::File::create(&info_path).expect("summary.txt");
    writeln!(info_f, "scenario={}", scenario).unwrap();
    writeln!(info_f, "seed=0x{:X}", seed).unwrap();
    writeln!(info_f, "amplitude={}", amplitude).unwrap();
    writeln!(info_f, "duration_on_fs={}", duration_on_fs).unwrap();
    writeln!(info_f, "duration_rest_fs={}", duration_rest_fs).unwrap();
    writeln!(info_f, "num_pulses={}", num_pulses).unwrap();
    writeln!(info_f, "log_stride={}", log_stride).unwrap();
    writeln!(info_f, "equilibrate_fs={}", equilibrate_fs).unwrap();
    writeln!(info_f, "pre_hold_fs={}", pre_hold_fs).unwrap();
    writeln!(info_f, "dt_fs={}", dt).unwrap();
    writeln!(info_f, "n_steps={}", step_global).unwrap();
    writeln!(info_f, "wall_seconds={:.1}", elapsed.as_secs_f32()).unwrap();
    println!("wrote {}", series_path.display());
    println!("wrote {}", summary_path.display());
    println!("wrote {}", info_path.display());
}
