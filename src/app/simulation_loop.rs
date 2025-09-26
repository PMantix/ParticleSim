use crate::profile_scope;
use crate::renderer::state::{SimCommand, BODIES, FOILS, PAUSED, QUADTREE, SPAWN, UPDATE_LOCK};
use crate::simulation::{PlaybackProgress, Simulation};
use crate::switch_charging;
use std::sync::atomic::Ordering;
use std::time::Instant;

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

    simulation.publish_playback_status();

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

pub fn run_simulation_loop(
    rx: std::sync::mpsc::Receiver<SimCommand>,
    mut simulation: Simulation,
    switch_handles: switch_charging::SimHandles,
) {
    let switch_charging::SimHandles {
        control_rx: switch_control_rx,
        status_tx: _,
    } = switch_handles;
    // Initialize shared state domain size from simulation
    *crate::renderer::state::DOMAIN_WIDTH.lock() = simulation.domain_width * 2.0; // Convert half-width to full width for GUI
    *crate::renderer::state::DOMAIN_HEIGHT.lock() = simulation.domain_height * 2.0; // Convert half-height to full height for GUI

    // debug log removed
    loop {
        // debug log removed

        // Handle commands
        while let Ok(cmd) = rx.try_recv() {
            eprintln!("[command-recv] Processing command: {:?}", std::mem::discriminant(&cmd));
            command_loop::handle_command(cmd, &mut simulation);
            eprintln!("[command-done] Bodies count now: {}", simulation.bodies.len());
        }

        while let Ok(control) = switch_control_rx.try_recv() {
            simulation.handle_switch_control(control);
        }

        simulation.flush_history_if_dirty();

        let is_paused = PAUSED.load(Ordering::Relaxed);
        let is_viewing_history = simulation.is_viewing_history();

        if is_paused || is_viewing_history {
            // debug log removed
            if let PlaybackProgress::ReachedLive { should_resume_live } =
                simulation.advance_playback(Instant::now())
            {
                if should_resume_live {
                    PAUSED.store(false, Ordering::Relaxed);
                }
            }
            std::thread::yield_now();
        } else {
            // debug log removed
            // Validate simulation state before stepping
            let invalid_count = simulation
                .bodies
                .iter()
                .filter(|b| {
                    !b.pos.x.is_finite()
                        || !b.pos.y.is_finite()
                        || !b.z.is_finite()
                        || !b.vel.x.is_finite()
                        || !b.vel.y.is_finite()
                        || !b.vz.is_finite()
                })
                .count();

            if invalid_count > 0 {
                eprintln!(
                    "[ERROR] Found {} particles with invalid positions/velocities! Resetting...",
                    invalid_count
                );
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
        simulation.publish_playback_status();
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
