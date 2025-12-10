use crate::renderer::state::SIM_COMMAND_SENDER;
use crate::renderer::Renderer;
use crate::simulation::Simulation;
use std::sync::mpsc::channel;

pub mod command_loop;
pub mod simulation_loop;
pub mod spawn;

// Main command handling is now done directly via simulation_loop

pub const RANDOM_ATTEMPTS: usize = 100;

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
        window_mode: quarkstrom::WindowMode::Maximized,
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let (ui_handles, sim_handles) = crate::switch_charging::create_channels();
    crate::switch_charging::install_ui_handles(ui_handles);

    let mut simulation = Simulation::new();
    simulation.set_switch_status_sender(sim_handles.status_tx.clone());

    std::thread::spawn(move || {
        simulation_loop::run_simulation_loop(rx, simulation, sim_handles);
    });

    quarkstrom::run::<Renderer>(config);
}
