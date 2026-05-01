//! measure_diffusivity — Phase 0a stub.
//!
//! Loads a bulk-electrolyte scenario from TOML, builds a Simulation headlessly
//! (no SimCommand channel), and prints the resulting body count + species
//! histogram. Future mini-steps add equilibration, MSD tracking, and a
//! diffusivity fit — see docs/EIS_AMPLITUDE_STUDY_PLAN.md Phase 0a.

use particle_sim::app::spawn::add_random;
use particle_sim::body::{Body, Species};
use particle_sim::init_config::InitConfig;
use particle_sim::simulation::Simulation;
use std::collections::BTreeMap;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: measure_diffusivity --scenario <path.toml> [--seed <u64>]");
    std::process::exit(2);
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
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--seed expects a u64");
                        print_usage_and_exit();
                    });
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
}
