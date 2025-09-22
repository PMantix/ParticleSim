use particle_sim::{body, simulation};
use serde::Serialize;
use std::{env, time::Instant};
use ultraviolet::Vec2;

#[derive(Serialize)]
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

fn parse_arg(args: &[String], flag: &str, default: usize) -> usize {
    for window in args.windows(2) {
        if window[0] == flag {
            return window[1].parse().unwrap_or(default);
        }
    }

    let prefix = format!("{}=", flag);
    for arg in args {
        if let Some(value) = arg.strip_prefix(&prefix) {
            return value.parse().unwrap_or(default);
        }
    }

    default
}

fn build_simulation(bodies: usize) -> simulation::Simulation {
    let mut sim = simulation::Simulation::new();

    for i in 0..bodies {
        let angle = i as f32 * 0.1;
        let radius = (i as f32 * 0.05) % 15.0;
        let x = radius * angle.cos();
        let y = radius * angle.sin();

        let body = body::Body::new(
            Vec2::new(x, y),
            Vec2::new(0.1, 0.1),
            1.0,
            1.0,
            1.0,
            body::Species::LithiumIon,
        );
        sim.bodies.push(body);
    }

    sim
}

fn average(slice: &[u128]) -> f64 {
    if slice.is_empty() {
        0.0
    } else {
        slice.iter().copied().sum::<u128>() as f64 / slice.len() as f64
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let steps = parse_arg(&args, "--steps", 200);
    let bodies = parse_arg(&args, "--bodies", 300);
    let sample_window = parse_arg(&args, "--window", 20).max(1);

    let mut sim = build_simulation(bodies);
    let mut durations = Vec::with_capacity(steps);

    let wall_start = Instant::now();
    for _ in 0..steps {
        let step_start = Instant::now();
        sim.step();
        durations.push(step_start.elapsed().as_micros());
    }
    let wall_time = wall_start.elapsed().as_micros();

    let total_step_micros: u128 = durations.iter().copied().sum();
    let window = sample_window.min(steps).max(1);

    let early_slice = &durations[..window];
    let late_slice = &durations[steps - window..];

    let early_avg = average(early_slice);
    let late_avg = average(late_slice);

    let report = RuntimeReport {
        steps,
        bodies,
        sample_window: window,
        total_step_micros,
        wall_time_micros: wall_time,
        avg_step_micros: total_step_micros as f64 / steps as f64,
        early_avg_micros: early_avg,
        late_avg_micros: late_avg,
        slowdown_factor: if early_avg > 0.0 {
            late_avg / early_avg
        } else {
            0.0
        },
    };

    println!(
        "{}",
        serde_json::to_string(&report).expect("failed to serialize runtime report")
    );
}
