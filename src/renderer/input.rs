use super::state::*;
use quarkstrom::winit::event::VirtualKeyCode;
use std::sync::atomic::Ordering;
use std::f32::consts::{PI, TAU};
use crate::body::{Body, Species, Electron};
use ultraviolet::Vec2;
use quarkstrom::winit_input_helper::WinitInputHelper;
use super::state::{SIM_COMMAND_SENDER, SimCommand};

impl super::Renderer {
    pub fn handle_input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::E);

        if input.key_pressed(VirtualKeyCode::Space) {
            let val = PAUSED.load(Ordering::Relaxed);
            PAUSED.store(!val, Ordering::Relaxed)
        }

        if input.key_pressed(VirtualKeyCode::Back) {
            self.selected_particle_id = None;
        }

        // Camera zoom and pan
        if let Some((mx, my)) = input.mouse() {
            // Scroll steps to double/halve the scale
            let steps = 5.0;
            // Modify input
            let zoom = (-input.scroll_diff() / steps).exp2();
            // Screen space -> view space
            let target =
                Vec2::new(mx * 2.0 - width as f32, height as f32 - my * 2.0) / height as f32;
            // Move view position based on target
            self.pos += target * self.scale * (1.0 - zoom);
            self.scale *= zoom;
        }

        // Edit charge of selected particle by id
        if let Some(id) = self.selected_particle_id {
            let mut delta = 0.0;
            if input.key_pressed(VirtualKeyCode::Minus) {
                delta = -1.0;
            }
            if input.key_pressed(VirtualKeyCode::Equals) {
                delta = 1.0;
            }
            if delta != 0.0 {
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::ChangeCharge { id, delta });
                }
            }
        }

        // Camera grab
        if input.mouse_held(2) {
            let (mdx, mdy) = input.mouse_diff();
            self.pos.x -= mdx / height as f32 * self.scale * 2.0;
            self.pos.y += mdy / height as f32 * self.scale * 2.0;
        }

        // Mouse to world conversion
        let world_mouse = || -> Vec2 {
            let (mx, my) = input.mouse().unwrap_or_default();
            let mut mouse = Vec2::new(mx, my);
            mouse *= 2.0 / height as f32;
            mouse.y -= 1.0;
            mouse.y *= -1.0;
            mouse.x -= width as f32 / height as f32;
            mouse * self.scale + self.pos
        };

        if input.mouse_pressed(1) {
            if self.spawn_body.is_none() {

                // If shift is held, select a particle
                if input.key_held(VirtualKeyCode::LShift) || input.key_held(VirtualKeyCode::RShift) {
                    let mouse_pos = world_mouse();
                    let mut closest = None;
                    let mut min_dist = f32::MAX;
                    for body in &self.bodies {
                        let dist = (body.pos - mouse_pos).mag();
                        if dist < min_dist && dist < body.radius * 2.0 {
                            min_dist = dist;
                            closest = Some(body.id);
                        }
                    }
                    self.selected_particle_id = closest;

                    if let Some(id) = closest {
                        if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                            println!(
                                "Selected Body: id={}, pos={:?}, vel={:?}, acc={:?}, charge={}, electrons={}, species={:?}",
                                body.id, body.pos, body.vel, body.acc, body.charge, body.electrons.len(), body.species
                            );
                                                        // If you accumulate per-source forces, print them here:
                            {
                                println!("LJ force: {:?}", body.lj_force);
                                println!("Coulomb force: {:?}", body.coulomb_force);
                                // etc.
                            }
                        }
                    }

                    println!()

                // If shift is not held, spawn a new body    
                } else {
                    // Spawning logic (no shift)
                    let mouse = world_mouse();
                    let mut body = Body::new(mouse, Vec2::zero(), 1.0, 1.0, 0.0, Species::LithiumMetal);
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                    body.update_charge_from_electrons();
                    self.spawn_body = Some(body);
                    self.angle = None;
                    self.total = Some(0.0);
                }
            }
            // If we are already spawning a body, set the mass
        } else if input.mouse_held(1) {
            if let Some(body) = &mut self.spawn_body {
                let mouse = world_mouse();
                if let Some(angle) = self.angle {
                    let d = mouse - body.pos;
                    let angle2 = d.y.atan2(d.x);
                    let a = angle2 - angle;
                    let a = (a + PI).rem_euclid(TAU) - PI;
                    let total = self.total.unwrap() - a;
                    body.mass = (total / TAU).exp2();
                    self.angle = Some(angle2);
                    self.total = Some(total);
                    // Update the velocity based on the angle
                } else {
                    let d = mouse - body.pos;
                    let angle = d.y.atan2(d.x);
                    self.angle = Some(angle);
                }
                body.radius = body.mass.cbrt();
                body.vel = mouse - body.pos;
            }
        // If the mouse is released, confirm the body
        } else if input.mouse_released(1) {
            if let Some(body) = self.spawn_body.take() {
                if let Some(sender) = SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = sender.send(SimCommand::AddBody { body });
                }
            }
        }
    }
}