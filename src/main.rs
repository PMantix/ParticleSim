// Entry point for the simulation.
// Sets up threading, simulation, and rendering loop. 
// Handles synchronization between simulation and renderer.

use std::sync::atomic::Ordering;
use crate::renderer::state::{
    PAUSED, UPDATE_LOCK, SPAWN, BODIES, QUADTREE,
};

mod body;
mod partition;
mod quadtree;
mod renderer;
mod simulation;
mod utils;

use renderer::Renderer;
use renderer::state::{SIM_COMMAND_SENDER, SimCommand};
use std::sync::mpsc::channel;
use simulation::core::Simulation;

fn main() {
    // Creates a global thread pool (using rayon) with threads = max(3, total cores - 2). Leaves 1-2 cores free to avoid hogging system resources.
    let threads = std::thread::available_parallelism().unwrap().get().max(3) - 2;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap();

    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(900, 900),
    };

    let (tx, rx) = channel();
    *SIM_COMMAND_SENDER.lock() = Some(tx);

    let mut simulation = Simulation::new();

    std::thread::spawn(move || {
        loop {

            // Handle commands
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    SimCommand::ChangeCharge { id, delta } => {
                        if let Some(body) = simulation.bodies.iter_mut().find(|b| b.id == id) {
                            body.charge += delta;

                            if body.species == body::Species::LithiumMetal && body.charge > 0.0{
                                body.update_species(); // Update species to ion
                                println!("Particle {} new species: {:?}", id, body.species);
                            }

                            if body.species == body::Species::LithiumIon && body.charge < 1.0{
                                body.update_species(); // Update species to ion
                                println!("Particle {} new species: {:?}", id, body.species);
                            }

                            body.set_electron_count(); //
                            println!("Particle {} new charge: {}", id, body.charge);
                        }
                    }
                    //SimCommand::Plate { foil_id, amount } => { /* ... */ }
                    //SimCommand::Strip { foil_id, amount } => { /* ... */ }
                    //SimCommand::AddElectron { pos, vel } => { /* ... */ }
                    //SimCommand::RemoveElectron { id } => { /* ... */ }
                }
            }

            if PAUSED.load(Ordering::Relaxed) {
                std::thread::yield_now();
            } else {
                simulation.step();
            }
            render(&mut simulation);
        }
    });

    quarkstrom::run::<Renderer>(config);
}

fn render(simulation: &mut Simulation) {
    let mut lock = UPDATE_LOCK.lock();
    
    //if new body was created, add to simulation
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
    *lock |= true;
}
