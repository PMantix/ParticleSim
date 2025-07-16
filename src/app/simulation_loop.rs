use crate::renderer::state::{SimCommand, BODIES, FOILS, PAUSED, QUADTREE, SPAWN, UPDATE_LOCK};
use crate::simulation::Simulation;
use std::sync::atomic::Ordering;

use super::command_loop;

pub fn render(simulation: &mut Simulation) {
    let mut lock = UPDATE_LOCK.lock();
    for body in SPAWN.lock().drain(..) {
        simulation.bodies.push(body);
    }
    {
        let mut lock = BODIES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.bodies);
    }
    {
        let mut lock = QUADTREE.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.quadtree.nodes);
    }
    {
        let mut lock = FOILS.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.foils);
    }
    *lock |= true;
}

pub fn run_simulation_loop(rx: std::sync::mpsc::Receiver<SimCommand>, mut simulation: Simulation) {
    loop {
        while let Ok(cmd) = rx.try_recv() {
            command_loop::handle_command(cmd, &mut simulation);
        }
        if PAUSED.load(Ordering::Relaxed) {
            std::thread::yield_now();
        } else {
            simulation.step();
        }
        render(&mut simulation);
        #[cfg(feature = "profiling")]
        {
            crate::PROFILER.lock().print_and_clear_if_running(
                !PAUSED.load(Ordering::Relaxed),
                Some(&simulation),
                None,
            );
        }
    }
}
