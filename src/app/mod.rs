use crate::renderer::state::SIM_COMMAND_SENDER;
use crate::renderer::Renderer;
use crate::simulation::Simulation;
use std::sync::mpsc::channel;

mod command_loop;
mod simulation_loop;
mod spawn;

pub const RANDOM_ATTEMPTS: usize = 20;

pub fn run() {
    // Creates a global thread pool (using rayon) with threads = max(3, total cores - 2)
    let threads = std::thread::available_parallelism()
        .unwrap()
        .get()
        .max(crate::config::MIN_THREADS)
        - crate::config::THREADS_LEAVE_FREE;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap();

    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(
            crate::config::WINDOW_WIDTH,
            crate::config::WINDOW_HEIGHT,
        ),
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let simulation = Simulation::new();

    std::thread::spawn(move || {
        simulation_loop::run_simulation_loop(rx, simulation);
    });

    quarkstrom::run::<Renderer>(config);
}
