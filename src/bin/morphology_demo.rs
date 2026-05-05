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
    self, classify_li_metal_dead, compute_morphology_metrics, extract_metal_frontiers,
    interface_arc_length_with_bin, is_li_metal_accessible, ARC_LENGTH_DEFAULT_Y_BIN_ANGSTROMS,
    DEAD_LI_CUTOFF_FACTOR,
};
use std::fs;
use std::path::PathBuf;
use ultraviolet::Vec2;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: morphology_demo --metric <name> [--out <path>] [--load-state <path>] \
         [--y-bin <float>]\n\
         \n\
         metrics:\n\
           accessible_surface_atoms  (#3, implemented)\n\
           interface_arc_length      (#1, implemented — frontier-trace v1)\n\
           dead_li_fraction          (#2, stub — not yet implemented)\n\
         \n\
         --load-state: load a SavedScenario .bin.gz, evaluate the metric on\n\
                       its bodies, and append it as an extra scenario named\n\
                       'real_sim:<filename>' so the visualizer renders both\n\
                       synthetic and real-snapshot panels in one figure.\n\
         --y-bin:      arc-length only. Bin width in Å (default 5.0).\n\
         \n\
         Default --out: images/morphology_validation/<metric>.json"
    );
    std::process::exit(2);
}

fn parse_args() -> (String, PathBuf, Option<PathBuf>, Option<f32>) {
    let mut metric: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut load_state_path: Option<PathBuf> = None;
    let mut y_bin: Option<f32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--metric" => metric = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            "--load-state" => load_state_path = args.next().map(PathBuf::from),
            "--y-bin" => {
                y_bin = args.next().and_then(|s| s.parse::<f32>().ok());
                if y_bin.is_none() {
                    eprintln!("--y-bin requires a positive float");
                    std::process::exit(2);
                }
            }
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
    (metric, out, load_state_path, y_bin)
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

// ---------------------------------------------------------------------------
// interface_arc_length scenarios + run loop
// ---------------------------------------------------------------------------

/// Build a flat foil column at fixed x with N bodies stacked vertically
/// (matches the test helper convention).
fn flat_foil_column(x: f32, n: usize, dy: f32, y0: f32, species: Species) -> Vec<Body> {
    (0..n)
        .map(|i| make_body(species, Vec2::new(x, y0 + (i as f32) * dy)))
        .collect()
}

fn build_scenarios_arc_length() -> Vec<Scenario> {
    let mut scenarios = Vec::new();

    // (1) Flat 2-foil baseline → expected 1.0.
    let mut s1 = flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal);
    s1.extend(flat_foil_column(150.0, 50, 2.0, -50.0, Species::FoilMetal));
    scenarios.push(Scenario {
        name: "flat_2_foil",
        description: "Two flat foil columns at x=±150. Frontier is purely vertical → ratio = 1.0.",
        expected: 1.0,
        tolerance: 0.02,
        bodies: s1,
    });

    // (2) Sinusoidal perturbation: flat foil with x = -150 + A·sin(2πy/λ).
    // A=3 Å, λ=20 Å. Predicted ratio ≈ 1.05–1.15 depending on bin alignment.
    let mut s2 = Vec::new();
    let lambda = 20.0_f32;
    let amp = 3.0_f32;
    for i in 0..200 {
        let y = -50.0 + (i as f32) * 0.5; // dense particle sampling
        let x = -150.0 + amp * (2.0 * std::f32::consts::PI * y / lambda).sin();
        s2.push(make_body(Species::FoilMetal, Vec2::new(x, y)));
    }
    // Also add a flat right foil for two-sided averaging.
    s2.extend(flat_foil_column(150.0, 50, 2.0, -50.0, Species::FoilMetal));
    scenarios.push(Scenario {
        name: "sinusoidal_perturbation",
        description: "Left foil with sinusoidal x perturbation (A=3 Å, λ=20 Å), flat right foil. \
                      Per-side ratio ≈ 1.07 left + 1.0 right. Average ≈ 1.04.",
        expected: 1.04,
        tolerance: 0.05,
        bodies: s2,
    });

    // (3) Mossy random: flat foil + random ±5 Å bumps every few atoms.
    // Use a deterministic small LCG so the test is reproducible without rand.
    let mut rng_state: u32 = 0xC0FFEEu32;
    let mut next_rand = || -> f32 {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        ((rng_state >> 16) & 0x7fff) as f32 / 32767.0 // [0, 1)
    };
    let mut s3 = Vec::new();
    for i in 0..50 {
        let y = -50.0 + (i as f32) * 2.0;
        let x_perturb = (next_rand() - 0.5) * 10.0; // ±5 Å
        s3.push(make_body(Species::FoilMetal, Vec2::new(-150.0 + x_perturb, y)));
    }
    s3.extend(flat_foil_column(150.0, 50, 2.0, -50.0, Species::FoilMetal));
    scenarios.push(Scenario {
        name: "mossy_random",
        description: "Left foil with random ±5 Å bumps every 2 Å, flat right foil. \
                      The metric's per-bin max smooths out fine-grained random noise: \
                      with 2-3 particles per 5 Å bin, the max-x is biased toward the \
                      population's extreme so adjacent-bin maxes track each other. \
                      Expected modest increase ≈ 1.05.",
        expected: 1.05,
        tolerance: 0.05,
        bodies: s3,
    });

    // (4) Dendritic spike: flat foil + a single 30 Å spike at y=0.
    let mut s4 = flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal);
    for i in 0..6 {
        s4.push(make_body(
            Species::FoilMetal,
            Vec2::new(-145.0 + (i as f32) * 5.0, 0.0),
        ));
    }
    s4.extend(flat_foil_column(150.0, 50, 2.0, -50.0, Species::FoilMetal));
    scenarios.push(Scenario {
        name: "dendritic_spike",
        description: "Flat foils + one 30 Å spike protruding from left foil at y=0. \
                      Per-y-bin frontier jumps inward at one bin. Ratio strongly > 1.",
        expected: 1.6,
        tolerance: 0.5,
        bodies: s4,
    });

    // (5) Empty scenario.
    scenarios.push(Scenario {
        name: "empty",
        description: "No bodies. Degenerate; expected = 0.0 (no foils to measure).",
        expected: 0.0,
        tolerance: 0.0,
        bodies: Vec::new(),
    });

    scenarios
}

/// Frontier polylines (per side) used for the arc_length JSON dump.
struct ArcLengthSnapshot {
    left: Vec<(f32, f32)>,  // (y, x)
    right: Vec<(f32, f32)>, // (y, x)
}

fn snapshot_frontiers(bodies: &[Body], y_bin: f32) -> ArcLengthSnapshot {
    let (left, right) = extract_metal_frontiers(bodies, y_bin);
    ArcLengthSnapshot {
        left: left.iter().map(|p| (p.y, p.x)).collect(),
        right: right.iter().map(|p| (p.y, p.x)).collect(),
    }
}

fn run_arc_length(scenarios: &[Scenario], out_path: &PathBuf, y_bin: f32) {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"metric\": \"interface_arc_length\",\n");
    json.push_str(&format!("  \"y_bin_angstroms\": {},\n", y_bin));
    json.push_str("  \"scenarios\": [\n");

    for (si, s) in scenarios.iter().enumerate() {
        let computed = interface_arc_length_with_bin(&s.bodies, y_bin) as f64;
        let is_info = s.expected.is_nan() || s.tolerance.is_nan();
        let pass = !is_info && (computed - s.expected).abs() <= s.tolerance;
        let judgment = if is_info {
            "INFO"
        } else if pass {
            "PASS"
        } else {
            "FAIL"
        };

        let snap = snapshot_frontiers(&s.bodies, y_bin);

        let expected_str = if s.expected.is_nan() {
            "n/a".to_string()
        } else {
            format!("{:>6.3}", s.expected)
        };
        println!(
            "[{}/{}] {:30} expected={}  computed={:>6.3}  Bin={:.1} Å  {}",
            si + 1,
            scenarios.len(),
            s.name,
            expected_str,
            computed,
            y_bin,
            judgment
        );

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
        // Per-particle for the visualizer (no accessibility flag for this metric).
        json.push_str("      \"particles\": [\n");
        for (i, b) in s.bodies.iter().enumerate() {
            let sep = if i + 1 == s.bodies.len() { "" } else { "," };
            json.push_str(&format!(
                "        {{\"species\":\"{}\",\"x\":{:.4},\"y\":{:.4},\"r\":{:.4}}}{}\n",
                species_str(b.species),
                b.pos.x,
                b.pos.y,
                b.radius,
                sep
            ));
        }
        json.push_str("      ],\n");
        // Per-side frontier polyline.
        json.push_str("      \"frontiers\": {\n");
        for (label, points) in [("left", &snap.left), ("right", &snap.right)] {
            json.push_str(&format!("        \"{label}\": ["));
            for (i, (y, x)) in points.iter().enumerate() {
                let sep = if i + 1 == points.len() { "" } else { "," };
                json.push_str(&format!("{{\"y\":{:.3},\"x\":{:.3}}}{}", y, x, sep));
            }
            let trailing = if label == "right" { "" } else { "," };
            json.push_str(&format!("]{}\n", trailing));
        }
        json.push_str("      }\n");
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

// ---------------------------------------------------------------------------
// dead_li_fraction scenarios + run loop
// ---------------------------------------------------------------------------

fn build_scenarios_dead_li() -> Vec<Scenario> {
    let mut scenarios = Vec::new();

    // (1) Connected baseline: foil + adjacent Li column → 0.0.
    let mut s1 = flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal);
    s1.extend(flat_foil_column(-148.0, 50, 2.0, -50.0, Species::LithiumMetal));
    scenarios.push(Scenario {
        name: "connected_li_at_foil",
        description: "FoilMetal at x=-150 + LithiumMetal column at x=-148. \
                      All Li atoms connect to the foil through the chain → 0.0.",
        expected: 0.0,
        tolerance: 1e-3,
        bodies: s1,
    });

    // (2) Single isolated Li atom 50 Å from the foil-attached cluster.
    let mut s2 = flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal);
    s2.extend(flat_foil_column(-148.0, 50, 2.0, -50.0, Species::LithiumMetal));
    s2.push(make_body(Species::LithiumMetal, Vec2::new(50.0, 0.0)));
    let expected2 = 1.0 / 51.0;
    scenarios.push(Scenario {
        name: "single_isolated_atom",
        description: "Foil + connected Li (50) + 1 isolated Li atom 50 Å away. \
                      Expected = 1/51 ≈ 0.0196.",
        expected: expected2 as f64,
        tolerance: 1e-3,
        bodies: s2,
    });

    // (3) 10-atom dead island floating in mid-cell.
    let mut s3 = flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal);
    s3.extend(flat_foil_column(-148.0, 50, 2.0, -50.0, Species::LithiumMetal));
    s3.extend(flat_foil_column(0.0, 10, 2.0, -10.0, Species::LithiumMetal));
    let expected3 = 10.0 / 60.0;
    scenarios.push(Scenario {
        name: "dead_10_atom_island",
        description: "Foil + connected Li (50) + 10-atom dead island in mid-cell. \
                      Expected = 10/60 ≈ 0.167.",
        expected: expected3 as f64,
        tolerance: 1e-3,
        bodies: s3,
    });

    // (4) No foil, Li chain only → 1.0.
    let s4 = flat_foil_column(0.0, 10, 2.0, -10.0, Species::LithiumMetal);
    scenarios.push(Scenario {
        name: "no_foil_all_dead",
        description: "10 Li atoms in a chain, no foil anywhere. All dead → 1.0.",
        expected: 1.0,
        tolerance: 1e-3,
        bodies: s4,
    });

    // (5) Two-foil cell with one half disconnected: foil + Li attached to the
    // *right* foil, but one stranded cluster in the middle plus the entire
    // *left* foil's plated layer detached after a stripping event.
    // Mimics late-cycle plating partial detachment.
    let mut s5 = Vec::new();
    // Right foil (intact attached Li).
    s5.extend(flat_foil_column(150.0, 50, 2.0, -50.0, Species::FoilMetal));
    s5.extend(flat_foil_column(148.0, 50, 2.0, -50.0, Species::LithiumMetal));
    // Left foil (still intact) but its plated Li layer is detached.
    s5.extend(flat_foil_column(-150.0, 50, 2.0, -50.0, Species::FoilMetal));
    // Plated Li layer detached: at x=-145 (5 Å gap from foil at -150 → distance 5 > 3.8 cutoff).
    s5.extend(flat_foil_column(-145.0, 30, 2.0, -30.0, Species::LithiumMetal));
    // Mid-cell stranded cluster of 5 atoms.
    s5.extend(flat_foil_column(0.0, 5, 2.0, -5.0, Species::LithiumMetal));
    // Total Li = 50 (right, attached) + 30 (left, detached) + 5 (mid) = 85.
    // Dead = 30 + 5 = 35. Fraction = 35/85 ≈ 0.412.
    let expected5 = 35.0 / 85.0;
    scenarios.push(Scenario {
        name: "partial_stripping_one_side",
        description: "Two foils. Right foil's Li is attached; left foil's Li layer (30 atoms) is \
                      detached by a 5 Å gap; an additional 5-atom stranded cluster sits in mid-cell. \
                      Expected = 35/85 ≈ 0.412.",
        expected: expected5 as f64,
        tolerance: 1e-3,
        bodies: s5,
    });

    scenarios
}

fn run_dead_li(scenarios: &[Scenario], out_path: &PathBuf) {
    let r_li = Species::LithiumMetal.radius();
    let cutoff = DEAD_LI_CUTOFF_FACTOR * r_li;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"metric\": \"dead_li_fraction\",\n");
    json.push_str(&format!("  \"cutoff_factor\": {},\n", DEAD_LI_CUTOFF_FACTOR));
    json.push_str(&format!("  \"cutoff_angstroms\": {:.3},\n", cutoff));
    json.push_str("  \"scenarios\": [\n");

    for (si, s) in scenarios.iter().enumerate() {
        let m = compute_morphology_metrics(&s.bodies);
        let computed = m.dead_li_fraction as f64;
        let is_info = s.expected.is_nan() || s.tolerance.is_nan();
        let pass = !is_info && (computed - s.expected).abs() <= s.tolerance;
        let judgment = if is_info {
            "INFO"
        } else if pass {
            "PASS"
        } else {
            "FAIL"
        };

        let class = classify_li_metal_dead(&s.bodies);
        let n_li = class.iter().filter(|c| c.is_some()).count();
        let n_dead = class.iter().filter(|c| **c == Some(true)).count();

        let expected_str = if s.expected.is_nan() {
            "n/a".to_string()
        } else {
            format!("{:>6.4}", s.expected)
        };
        println!(
            "[{}/{}] {:30} expected={}  computed={:>6.4}  Li={:>4} Dead={:>4}  {}",
            si + 1,
            scenarios.len(),
            s.name,
            expected_str,
            computed,
            n_li,
            n_dead,
            judgment
        );

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
            // dead_status: "alive" / "dead" / "n/a"
            let status = match class[i] {
                Some(true) => "dead",
                Some(false) => "alive",
                None => "n/a",
            };
            json.push_str(&format!(
                "        {{\"species\":\"{}\",\"x\":{:.4},\"y\":{:.4},\"r\":{:.4},\"dead_status\":\"{}\"}}{}\n",
                species_str(b.species),
                b.pos.x,
                b.pos.y,
                b.radius,
                status,
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

fn main() {
    let (metric, out_path, load_state_path, y_bin_override) = parse_args();
    match metric.as_str() {
        "accessible_surface_atoms" => {
            let mut scenarios = build_scenarios_accessible();
            if let Some(p) = load_state_path {
                append_real_sim_scenario(&mut scenarios, &p);
            }
            run_accessible(&scenarios, &out_path);
        }
        "interface_arc_length" => {
            let mut scenarios = build_scenarios_arc_length();
            if let Some(p) = load_state_path {
                append_real_sim_scenario(&mut scenarios, &p);
            }
            let y_bin = y_bin_override.unwrap_or(ARC_LENGTH_DEFAULT_Y_BIN_ANGSTROMS);
            run_arc_length(&scenarios, &out_path, y_bin);
        }
        "dead_li_fraction" => {
            let mut scenarios = build_scenarios_dead_li();
            if let Some(p) = load_state_path {
                append_real_sim_scenario(&mut scenarios, &p);
            }
            run_dead_li(&scenarios, &out_path);
        }
        other => {
            eprintln!("unknown metric: {other}");
            print_usage_and_exit();
        }
    }
}
