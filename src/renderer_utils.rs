// renderer_utils.rs
// Utility functions for rendering and simulation state management

use crate::renderer::state::{UPDATE_LOCK, SPAWN, BODIES, QUADTREE, FOILS};
use crate::simulation::Simulation;

/// Update the renderer state with current simulation data
pub fn render(simulation: &mut Simulation) {
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
    {
        let mut lock = FOILS.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.foils);
    }
    *lock |= true;
}
