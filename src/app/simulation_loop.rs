use crate::renderer::state::{SimCommand, BODIES, FOILS, PAUSED, QUADTREE, SPAWN, UPDATE_LOCK};
use crate::simulation::Simulation;
use crate::profile_scope;
use std::sync::atomic::Ordering;

use super::command_loop;

pub fn render(simulation: &mut Simulation) {
    // debug log removed
    let mut lock = UPDATE_LOCK.lock();
    // debug log removed
    
    // debug log removed
    for body in SPAWN.lock().drain(..) {
        simulation.bodies.push(body);
    }
    // debug log removed
    
    {
        // debug log removed
        let mut lock = BODIES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.bodies);
        // debug log removed
    }
    {
        // debug log removed
        let mut lock = QUADTREE.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.quadtree.nodes);
        // debug log removed
    }
    {
        // debug log removed
        let mut lock = FOILS.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.foils);
        // debug log removed
    }
    
    // debug log removed
    *lock |= true;
    // debug log removed
}

pub fn run_simulation_loop(rx: std::sync::mpsc::Receiver<SimCommand>, mut simulation: Simulation) {
    // debug log removed
    loop {
        // debug log removed
        
        // Handle commands
        while let Ok(cmd) = rx.try_recv() {
            // debug log removed
            command_loop::handle_command(cmd, &mut simulation);
        }
        
        if PAUSED.load(Ordering::Relaxed) {
            // debug log removed
            std::thread::yield_now();
        } else {
            // debug log removed
            // Validate simulation state before stepping
            let invalid_count = simulation.bodies.iter()
                .filter(|b| !b.pos.x.is_finite() || !b.pos.y.is_finite() || !b.z.is_finite() ||
                           !b.vel.x.is_finite() || !b.vel.y.is_finite() || !b.vz.is_finite())
                .count();
                
            if invalid_count > 0 {
                eprintln!("[ERROR] Found {} particles with invalid positions/velocities! Resetting...", invalid_count);
                for body in &mut simulation.bodies {
                    if !body.pos.x.is_finite() || !body.pos.y.is_finite() {
                        body.pos = ultraviolet::Vec2::zero();
                    }
                    if !body.vel.x.is_finite() || !body.vel.y.is_finite() {
                        body.vel = ultraviolet::Vec2::zero();
                    }
                    if !body.z.is_finite() {
                        body.z = 0.0;
                    }
                    if !body.vz.is_finite() {
                        body.vz = 0.0;
                    }
                    if !body.az.is_finite() {
                        body.az = 0.0;
                    }
                }
            }
            
            // debug log removed
            {
                profile_scope!("simulation_loop");
                simulation.step();
            }
            // debug log removed
        }
        
        // debug log removed
        render(&mut simulation);
        // debug log removed
        
        // Allow GUI thread to update by yielding CPU time
        // debug log removed
        std::thread::yield_now();
        
        #[cfg(feature = "profiling")]
        {
            crate::PROFILER.lock().print_and_clear_if_running(
                !PAUSED.load(Ordering::Relaxed),
                Some(&simulation),
                None,
            );
        }
        
    // debug log removed
    }
}
