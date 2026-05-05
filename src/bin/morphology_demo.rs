//! morphology_demo — synthetic-scenario validation harness for
//! `src/simulation/morphology.rs` metrics.
//!
//! Builds a small library of synthetic particle configurations with known
//! expected metric values, evaluates the requested metric on each, and dumps
//! a JSON report (per-particle positions + species + accessibility flag,
//! plus the aggregate metric value and PASS/FAIL judgment) to
//! `images/morphology_validation/<metric>.json`.
//!
//! The Python plot script `scripts/plot_morphology_demo.py` consumes the JSON
//! and renders one panel per scenario.
//!
//! Usage:
//!   morphology_demo --metric accessible_surface_atoms [--out PATH]

use particle_sim::body::{Body, Species};
use particle_sim::io::load_state;
use particle_sim::simulation::morphology::{
    self, compute_morphology_metrics, is_li_metal_accessible,
};
use std::fs;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: morphology_demo --metric <name> [--out <path>] [--load-state <path>]\n\
         \n\
         metrics:\n\
           accessible_surface_atoms  (#3, implemented)\n\
           interface_arc_length      (#1, stub — not yet implemented)\n\
           dead_li_fraction          (#2, stub — not yet implemented)\n\
         \n\
         --load-state: load a SavedScenario .bin.gz, evaluate the metric on\n\
                       its bodies, and append it as an extra scenario named\n\
                       'real_sim:<filename>' so the visualizer renders both\n\
                       synthetic and real-snapshot panels in one figure.\n\
         \n\
         Default --out: images/morphology_validation/<metric>.json"
    );
    std::process::exit(2);
}

fn parse_args() -> (String, PathBuf, Option<PathBuf>) {
    let mut metric: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut load_state_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--metric" => metric = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            "--load-state" => load_state_path = args.next().map(PathBuf::from),
            "-h" | "--help" => print_usage_and_exit(),
            other => {
                eprintln!("unknown arg: {other}");
                print_usage_and_exit();
            }
        }
    }
    let metric = metric.unwrap_or_else(|| print_usage_and_exit());
    let out = out.unwrap_or_else(|| {
        PathBuf::from(format!("images/morphology_validation/{metric}.json"))
    });
    (metric, out, load_state_path)
}

fn make_body(species: Species, pos: Vec2) -> Body {
    Body::new(
        pos,
        Vec2::zero(),
        species.mass(),
        species.radius(),
        0.0,
        species,
    )
}

/// One synthetic test case.
struct Scenario {
    name: &'static str,
    description: &'static str,
    expected: f64,           // expected metric value (numeric, for cross-metric reuse)
    tolerance: f64,          // pass band: |computed - expected| <= tolerance
    bodies: Vec<Body>,
}

/// 5 columns of LithiumMetal at x = -150,-148,-146,-144,-142, 50 atoms vertically each.
fn five_col_li(y0: f32, dy: f32, n: usize) -> Vec<Body> {
    let mut v = Vec::new();
    for k in 0..5 {
        let x = -150.0 + (k as f32) * 2.0;
        for i in 0..n {
            v.push(make_body(
                Species::LithiumMetal,
                Vec2::new(x, y0 + (i as f32) * dy),
            ));
        }
    }
    v
}

fn ec_column(x: f32, y0: f32, dy: f32, n: usize) -> Vec<Body> {
    (0..n)
        .map(|i| make_body(Species::EC, Vec2::new(x, y0 + (i as f32) * dy)))
        .collect()
}

fn build_scenarios_accessible() -> Vec<Scenario> {
    let mut scenarios = Vec::new();

    // (1) flat foil, no electrolyte
    scenarios.push(Scenario {
        name: "flat_no_electrolyte",
        description: "5-column LithiumMetal block, no electrolyte. Expected count = 0.",
        expected: 0.0,
        tolerance: 0.0,
        bodies: five_col_li(-50.0, 2.0, 50),
    });

    // (2) flat foil with EC frontier — only outermost column counts
    let mut s2 = five_col_li(-50.0, 2.0, 50);
    s2.extend(ec_column(-138.0, -50.0, 2.0, 50));
    scenarios.push(Scenario {
        name: "flat_with_electrolyte",
        description: "5-column block + EC frontier 4 Å past outermost column. \
                      Cutoff(Li-EC) ≈ 5.23 Å. Only outermost column counts → 50.",
        expected: 50.0,
        tolerance: 0.0,
        bodies: s2,
    });

    // (3) mossy: same baseline + 10 protruding Li atoms reaching toward EC
    let mut s3 = five_col_li(-50.0, 2.0, 50);
    s3.extend(ec_column(-138.0, -50.0, 2.0, 50));
    for i in 0..10 {
        s3.push(make_body(
            Species::LithiumMetal,
            Vec2::new(-140.0, -25.0 + (i as f32) * 5.0),
        ));
    }
    scenarios.push(Scenario {
        name: "mossy_with_electrolyte",
        description: "Baseline frontier (50) + 10 protruding Li atoms 2 Å closer to EC. \
                      Each protrusion is accessible. Expected = 60.",
        expected: 60.0,
        tolerance: 0.0,
        bodies: s3,
    });

    // (4) buried: 5 columns + EC very far away
    let mut s4 = five_col_li(-50.0, 2.0, 50);
    s4.extend(ec_column(0.0, -50.0, 2.0, 10));
    scenarios.push(Scenario {
        name: "buried_no_reach",
        description: "5-column block, EC located 142 Å away — far past cutoff. Expected = 0.",
        expected: 0.0,
        tolerance: 0.0,
        bodies: s4,
    });

    // (5) dead-Li island: isolated 10-atom Li chain in EC bath
    let mut s5 = Vec::new();
    for i in 0..10 {
        s5.push(make_body(
            Species::LithiumMetal,
            Vec2::new((i as f32) * 4.0, 0.0),
        ));
    }
    for i in 0..10 {
        for dy in [-3.0, 3.0] {
            s5.push(make_body(Species::EC, Vec2::new((i as f32) * 4.0, dy)));
        }
    }
    scenarios.push(Scenario {
        name: "dead_li_island",
        description: "10-atom Li chain sandwiched between EC layers. \
                      All 10 Li atoms are surrounded by electrolyte. Expected = 10.",
        expected: 10.0,
        tolerance: 0.0,
        bodies: s5,
    });

    // (6) realistic_cell: synthetic but dense — two FoilMetal columns +
    // densely packed bulk electrolyte (EC/DMC/Li+/anion) + 30 plated
    // LithiumMetal atoms (15 per side) at the foil frontier. Geometry
    // mimics a partially-cycled validation cell.
    let mut s6 = Vec::new();
    // Foils: 4-column FoilMetal blocks at x=-150..-144 and x=144..150.
    for k in 0..4 {
        for &side in &[-1.0_f32, 1.0_f32] {
            let x = side * (150.0 - (k as f32) * 2.0);
            for i in 0..50 {
                s6.push(make_body(
                    Species::FoilMetal,
                    Vec2::new(x, -50.0 + (i as f32) * 2.0),
                ));
            }
        }
    }
    // Plated LithiumMetal at x=±142 (just outside foils), 15 atoms per side.
    let plated_y_step = 100.0 / 15.0;
    for j in 0..15 {
        let y = -50.0 + (j as f32) * plated_y_step;
        s6.push(make_body(Species::LithiumMetal, Vec2::new(-142.0, y)));
        s6.push(make_body(Species::LithiumMetal, Vec2::new(142.0, y)));
    }
    // Bulk electrolyte: dense 4-Å grid from x=-138 to 138, y=-55 to 55. This
    // puts the nearest electrolyte particle 4 Å from each plated Li (well
    // within Li-EC cutoff of ~5.23 Å, and Li-anion cutoff of ~4.58 Å).
    let mut idx = 0u32;
    let mut x = -138.0;
    while x <= 138.0 {
        let mut y = -55.0;
        while y <= 55.0 {
            let species = match idx % 4 {
                0 => Species::EC,
                1 => Species::DMC,
                2 => Species::LithiumIon,
                _ => Species::ElectrolyteAnion,
            };
            s6.push(make_body(species, Vec2::new(x, y)));
            idx += 1;
            y += 4.0;
        }
        x += 4.0;
    }
    // With the dense 4-Å bulk, plated Li at x=±142 sees grid neighbors at 4 Å
    // distance. Most species (EC/DMC/anion) are in reach but Li+ (cutoff 2.96)
    // is not, so the count is slightly less than 30 — depending on which y
    // values land near a Li+ column position. Expected ~26–30 (4 of 30 plated
    // Li have only Li+ as their nearest species). The point is the metric
    // returns a sensible non-trivial value on a realistic-density body
    // collection — the exact number is geometry-dependent.
    scenarios.push(Scenario {
        name: "realistic_cell",
        description: "Two FoilMetal columns + dense bulk electrolyte (EC/DMC/Li+/anion at 4 Å \
                      grid) + 30 plated LithiumMetal atoms (15 per side) at x=±142. Most plated \
                      Li reaches the bulk; a few hit a Li+-only neighborhood and miss (Li+ has \
                      a smaller cutoff). Expected ≈ 28 ± 4.",
        expected: 28.0,
        tolerance: 4.0,
        bodies: s6,
    });

    scenarios
}

fn species_str(s: Species) -> &'static str {
    match s {
        Species::LithiumIon => "LithiumIon",
        Species::LithiumMetal => "LithiumMetal",
        Species::FoilMetal => "FoilMetal",
        Species::ElectrolyteAnion => "ElectrolyteAnion",
        Species::EC => "EC",
        Species::DMC => "DMC",
        Species::VC => "VC",
        Species::FEC => "FEC",
        Species::EMC => "EMC",
        Species::LLZO => "LLZO",
        Species::LLZT => "LLZT",
        Species::S40B => "S40B",
        Species::SEI => "SEI",
        Species::Graphite => "Graphite",
        Species::HardCarbon => "HardCarbon",
        Species::SiliconOxide => "SiliconOxide",
        Species::LTO => "LTO",
        Species::LFP => "LFP",
        Species::LMFP => "LMFP",
        Species::NMC => "NMC",
        Species::NCA => "NCA",
    }
}

fn run_accessible(scenarios: &[Scenario], out_path: &PathBuf) {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"metric\": \"accessible_surface_atoms\",\n");
    json.push_str(&format!(
        "  \"contact_factor\": {},\n",
        morphology::ACCESSIBLE_CONTACT_FACTOR
    ));
    json.push_str("  \"scenarios\": [\n");

    for (si, s) in scenarios.iter().enumerate() {
        let m = compute_morphology_metrics(&s.bodies);
        let computed = m.accessible_surface_atoms as f64;
        let is_info = s.expected.is_nan() || s.tolerance.is_nan();
        let pass = !is_info && (computed - s.expected).abs() <= s.tolerance;
        let judgment = if is_info {
            "INFO"
        } else if pass {
            "PASS"
        } else {
            "FAIL"
        };

        // Per-particle accessibility flags for the visualizer.
        let flags: Vec<bool> = s
            .bodies
            .iter()
            .map(|b| is_li_metal_accessible(b, &s.bodies))
            .collect();

        let n_li = s.bodies.iter().filter(|b| b.species == Species::LithiumMetal).count();
        let n_electrolyte = s
            .bodies
            .iter()
            .filter(|b| morphology::is_liquid_electrolyte(b.species))
            .count();

        let expected_str = if s.expected.is_nan() {
            "n/a".to_string()
        } else {
            format!("{:>5}", s.expected)
        };
        println!(
            "[{}/{}] {:30} expected={}  computed={:>5}  Li={:>4} Electrolyte={:>4} {}",
            si + 1,
            scenarios.len(),
            s.name,
            expected_str,
            computed,
            n_li,
            n_electrolyte,
            judgment
        );

        // JSON: replace NaN with null (NaN isn't valid JSON).
        let expected_json = if s.expected.is_nan() {
            "null".to_string()
        } else {
            format!("{}", s.expected)
        };
        json.push_str("    {\n");
        json.push_str(&format!("      \"name\": \"{}\",\n", s.name));
        json.push_str(&format!(
            "      \"description\": \"{}\",\n",
            s.description.replace('"', "\\\"")
        ));
        json.push_str(&format!("      \"expected\": {},\n", expected_json));
        json.push_str(&format!("      \"computed\": {},\n", computed));
        json.push_str(&format!("      \"judgment\": \"{}\",\n", judgment));
        json.push_str("      \"particles\": [\n");
        for (i, b) in s.bodies.iter().enumerate() {
            let sep = if i + 1 == s.bodies.len() { "" } else { "," };
            json.push_str(&format!(
                "        {{\"species\":\"{}\",\"x\":{:.4},\"y\":{:.4},\"r\":{:.4},\"accessible\":{}}}{}\n",
                species_str(b.species),
                b.pos.x,
                b.pos.y,
                b.radius,
                flags[i],
                sep
            ));
        }
        json.push_str("      ]\n");
        let comma = if si + 1 == scenarios.len() { "" } else { "," };
        json.push_str(&format!("    }}{comma}\n"));
    }
    json.push_str("  ]\n}\n");

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).expect("create output dir");
    }
    fs::write(out_path, json).expect("write json");
    println!("\nwrote {}", out_path.display());
}

/// Append a scenario constructed from a saved-state file's `current.bodies`.
/// Result is reported with `expected = NaN, tolerance = NaN, judgment = "INFO"`
/// because we don't know the ground-truth value for an arbitrary saved sim;
/// the panel is informational ("here's what the metric says about this run").
fn append_real_sim_scenario(scenarios: &mut Vec<Scenario>, path: &PathBuf) {
    let scenario = match load_state(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to load_state {}: {}", path.display(), e);
            std::process::exit(3);
        }
    };
    let bodies = scenario.current.bodies;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let leaked: &'static str = Box::leak(format!("real_sim:{name}").into_boxed_str());
    let leaked_desc: &'static str = Box::leak(
        format!(
            "Loaded from {}. Bodies: {} total. Reported value is informational \
             (no ground-truth expected count for an arbitrary saved sim).",
            path.display(),
            bodies.len()
        )
        .into_boxed_str(),
    );
    scenarios.push(Scenario {
        name: leaked,
        description: leaked_desc,
        expected: f64::NAN,
        tolerance: f64::NAN,
        bodies,
    });
}

fn main() {
    let (metric, out_path, load_state_path) = parse_args();
    match metric.as_str() {
        "accessible_surface_atoms" => {
            let mut scenarios = build_scenarios_accessible();
            if let Some(p) = load_state_path {
                append_real_sim_scenario(&mut scenarios, &p);
            }
            run_accessible(&scenarios, &out_path);
        }
        "interface_arc_length" | "dead_li_fraction" => {
            eprintln!(
                "metric '{metric}' is not implemented yet. Currently only \
                 'accessible_surface_atoms' has scenarios."
            );
            std::process::exit(2);
        }
        other => {
            eprintln!("unknown metric: {other}");
            print_usage_and_exit();
        }
    }
}
