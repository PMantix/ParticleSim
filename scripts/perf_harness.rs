use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const LTO_OPTIONS: &[(&str, &str)] = &[("false", "off"), ("thin", "thin"), ("fat", "fat")];
const CODEGEN_UNITS: &[u32] = &[1, 4, 8, 16];

#[derive(Debug)]
struct BuildMeasurement {
    lto: String,
    codegen_units: u32,
    build_seconds: f64,
    timing_wall_seconds: Option<f64>,
    binary_size_bytes: u64,
}

#[derive(Debug)]
struct RuntimeMeasurement {
    lto: String,
    codegen_units: u32,
    wall_time_seconds: f64,
    steps: usize,
    bodies: usize,
    sample_window: usize,
    total_step_micros: u128,
    avg_step_micros: f64,
    early_avg_micros: f64,
    late_avg_micros: f64,
    slowdown_factor: f64,
}

#[derive(Deserialize)]
struct RuntimeReport {
    steps: usize,
    bodies: usize,
    sample_window: usize,
    total_step_micros: u128,
    wall_time_micros: u128,
    avg_step_micros: f64,
    early_avg_micros: f64,
    late_avg_micros: f64,
    slowdown_factor: f64,
}

fn main() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("docs/perf")?;
    fs::create_dir_all("target/perf")?;

    let mut build_metrics = Vec::new();
    let mut runtime_metrics = Vec::new();

    for &(label, rustc_lto) in LTO_OPTIONS {
        for &codegen_units in CODEGEN_UNITS {
            println!(
                "\n=== Measuring build: lto={} | codegen-units={} ===",
                label, codegen_units
            );
            let target_dir =
                PathBuf::from("target/perf").join(format!("lto-{}-cgu-{}", label, codegen_units));
            if target_dir.exists() {
                fs::remove_dir_all(&target_dir)?;
            }

            let rustflags = format!("-C lto={} -C codegen-units={}", rustc_lto, codegen_units);
            let build = measure_build(&target_dir, label, &rustflags, codegen_units)?;
            build_metrics.push(build);

            println!(
                "--- Capturing runtime: lto={} | codegen-units={} ---",
                label, codegen_units
            );
            let runtime = measure_runtime(&target_dir, label, &rustflags, codegen_units)?;
            runtime_metrics.push(runtime);
        }
    }

    write_build_csv(&build_metrics)?;
    write_runtime_csv(&runtime_metrics)?;

    println!("\nBuild metrics written to docs/perf/lto-build-matrix.csv");
    println!("Runtime metrics written to docs/perf/runtime-benchmarks.csv");

    Ok(())
}

fn measure_build(
    target_dir: &Path,
    lto_label: &str,
    rustflags: &str,
    codegen_units: u32,
) -> Result<BuildMeasurement, Box<dyn Error>> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .arg("--bin")
        .arg("particle_sim")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--timings=json");
    cmd.env("RUSTFLAGS", rustflags);

    let start = Instant::now();
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!(
            "cargo build failed for lto={} codegen-units={}",
            lto_label, codegen_units
        )
        .into());
    }
    let elapsed = start.elapsed().as_secs_f64();

    let binary_path = target_dir.join("release").join(binary_name("particle_sim"));
    let binary_size = fs::metadata(&binary_path)?.len();

    let timing_wall_seconds = read_latest_timing(target_dir)?;

    Ok(BuildMeasurement {
        lto: lto_label.to_string(),
        codegen_units,
        build_seconds: elapsed,
        timing_wall_seconds,
        binary_size_bytes: binary_size,
    })
}

fn measure_runtime(
    target_dir: &Path,
    lto_label: &str,
    rustflags: &str,
    codegen_units: u32,
) -> Result<RuntimeMeasurement, Box<dyn Error>> {
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut build_cmd = Command::new("cargo");
    build_cmd
        .arg("build")
        .arg("--release")
        .arg("--bin")
        .arg("runtime_probe")
        .arg("--target-dir")
        .arg(target_dir);
    build_cmd.env("RUSTFLAGS", rustflags);

    let status = build_cmd.status()?;
    if !status.success() {
        return Err(format!(
            "cargo build (runtime probe) failed for lto={} codegen-units={}",
            lto_label, codegen_units
        )
        .into());
    }

    let probe_path = target_dir
        .join("release")
        .join(binary_name("runtime_probe"));

    let runtime_start = Instant::now();
    let output = Command::new(&probe_path)
        .arg("--steps")
        .arg("200")
        .arg("--bodies")
        .arg("300")
        .arg("--window")
        .arg("20")
        .output()?;
    let wall_time_seconds = runtime_start.elapsed().as_secs_f64();

    if !output.status.success() {
        return Err(format!(
            "runtime probe failed for lto={} codegen-units={}",
            lto_label, codegen_units
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let report: RuntimeReport = serde_json::from_str(stdout.trim())?;

    Ok(RuntimeMeasurement {
        lto: lto_label.to_string(),
        codegen_units,
        wall_time_seconds,
        steps: report.steps,
        bodies: report.bodies,
        sample_window: report.sample_window,
        total_step_micros: report.total_step_micros,
        avg_step_micros: report.avg_step_micros,
        early_avg_micros: report.early_avg_micros,
        late_avg_micros: report.late_avg_micros,
        slowdown_factor: report.slowdown_factor,
    })
}

fn write_build_csv(records: &[BuildMeasurement]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create("docs/perf/lto-build-matrix.csv")?;
    writeln!(
        file,
        "lto,codegen_units,build_seconds,timing_wall_seconds,binary_size_bytes"
    )?;

    for record in records {
        let timing = record.timing_wall_seconds.unwrap_or(record.build_seconds);
        writeln!(
            file,
            "{},{},{:.3},{:.3},{}",
            record.lto,
            record.codegen_units,
            record.build_seconds,
            timing,
            record.binary_size_bytes
        )?;
    }

    Ok(())
}

fn write_runtime_csv(records: &[RuntimeMeasurement]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create("docs/perf/runtime-benchmarks.csv")?;
    writeln!(
        file,
        "lto,codegen_units,steps,bodies,sample_window,total_step_micros,avg_step_micros,early_avg_micros,late_avg_micros,slowdown_factor,wall_time_seconds"
    )?;

    for record in records {
        writeln!(
            file,
            "{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.4},{:.3}",
            record.lto,
            record.codegen_units,
            record.steps,
            record.bodies,
            record.sample_window,
            record.total_step_micros,
            record.avg_step_micros,
            record.early_avg_micros,
            record.late_avg_micros,
            record.slowdown_factor,
            record.wall_time_seconds
        )?;
    }

    Ok(())
}

fn read_latest_timing(target_dir: &Path) -> Result<Option<f64>, Box<dyn Error>> {
    let timing_dir = target_dir.join("cargo-timings");
    if !timing_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(&timing_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified = metadata.modified()?;
        match &mut latest {
            Some((stored_time, stored_path)) => {
                if modified > *stored_time {
                    *stored_time = modified;
                    *stored_path = path;
                }
            }
            None => {
                latest = Some((modified, path));
            }
        }
    }

    let (_, path) = match latest {
        Some(pair) => pair,
        None => return Ok(None),
    };

    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;
    let wall = json
        .get("wall_time")
        .and_then(Value::as_f64)
        .or_else(|| json.get("wall-time").and_then(Value::as_f64));

    Ok(wall)
}

fn binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}
