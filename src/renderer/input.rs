use super::state::*;
use quarkstrom::winit::event::VirtualKeyCode;
use std::sync::atomic::Ordering;
use std::f32::consts::{PI, TAU};
//use crate::body::{Body, Electron};
use ultraviolet::Vec2;
use quarkstrom::winit_input_helper::WinitInputHelper;
use super::state::{SIM_COMMAND_SENDER, SimCommand};
use crate::body::Species;
use crate::profile_scope;

impl super::Renderer {
    pub fn handle_input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        profile_scope!("input_handling");
        // Check if window dimensions changed and verify capture region if needed
        if width != self.window_width || height != self.window_height {
            self.verify_capture_region_after_resize(width, height);
        }
        
        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::E);

        if input.key_pressed(VirtualKeyCode::Space) {
            let val = PAUSED.load(Ordering::Relaxed);
            PAUSED.store(!val, Ordering::Relaxed)
        }

        if input.key_pressed(VirtualKeyCode::Back) {
            self.selected_particle_id = None;
            self.selected_foil_ids.clear();
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

        // Screen capture region selection handling
        if self.is_selecting_region {
            if let Some((mx, my)) = input.mouse() {
                let screen_pos = Vec2::new(mx, my);
                
                if input.mouse_pressed(0) {
                    // Start selection on mouse press
                    self.selection_start = Some(screen_pos);
                    self.selection_end = Some(screen_pos);
                    println!("Region selection: Started at ({:.0}, {:.0})", screen_pos.x, screen_pos.y);
                } else if input.mouse_held(0) && self.selection_start.is_some() {
                    // Update selection continuously while dragging
                    self.selection_end = Some(screen_pos);
                } else if input.mouse_released(0) && self.selection_start.is_some() {
                    // Finish selection on mouse release
                    self.selection_end = Some(screen_pos);
                    println!("Region selection: Finished at ({:.0}, {:.0})", screen_pos.x, screen_pos.y);
                    self.finish_region_selection(width, height);
                }
                
                // Cancel selection with right click or escape
                if input.mouse_pressed(1) || input.key_pressed(VirtualKeyCode::Escape) {
                    self.cancel_region_selection();
                    println!("Region selection cancelled");
                }
            }
        } else {
            // Normal input handling when not selecting region
            // Camera grab
            if input.mouse_held(2) {
                let (mdx, mdy) = input.mouse_diff();
                // Mouse diff coordinates are also in logical pixels
                self.pos.x -= mdx / height as f32 * self.scale * 2.0;
                self.pos.y += mdy / height as f32 * self.scale * 2.0;
            }
        }

        // Handle screen capture timing
        let current_time = *crate::renderer::state::SIM_TIME.lock();
        self.handle_screen_capture(current_time, width, height);

        // Mouse to world conversion
        let world_mouse = || -> Vec2 {
            let (mx, my) = input.mouse().unwrap_or_default();
            // Mouse coordinates are already in logical pixels, don't scale by DPI
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
                        let display_pos = self.get_display_position(body);
                        let dist = (display_pos - mouse_pos).mag();
                        if dist < min_dist && dist < body.radius * 2.0 {
                            min_dist = dist;
                            closest = Some(body.id);
                        }
                    }
                    self.selected_particle_id = closest;
                    if let Some(id) = closest {
                        if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                            if body.species == Species::FoilMetal {
                                if let Some(foil) = self.foils.iter().find(|f| f.body_ids.contains(&id)) {
                                    if !self.selected_foil_ids.contains(&foil.id) {
                                        if self.selected_foil_ids.len() == 2 {
                                            self.selected_foil_ids.remove(0);
                                        }
                                        self.selected_foil_ids.push(foil.id);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(id) = closest {
                        if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                            println!(
                                "Selected Body: id={}, pos={:?}, vel={:?}, acc={:?}, charge={}, electrons={}, species={:?}",
                                body.id, body.pos, body.vel, body.acc, body.charge, body.electrons.len(), body.species
                            );
                        }
                    }

                    println!()

                // If shift is not held, spawn a new body    
                } else {
                    // Spawning logic (no shift)
                    let mouse = world_mouse();
                    let spec = self.scenario_species;
                    let body = crate::renderer::gui::make_body_with_species(
                        mouse,
                        Vec2::zero(),
                        spec,
                    );
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
                // Note: Radius is never overridden during mouse spawning
                // Particles always use their species-defined radius from species.rs
                // If radius customization is needed, it should be done explicitly via GUI controls
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

    /// Transform body coordinates based on view mode
    /// Returns Vec2 with the appropriate coordinates for display
    pub fn get_display_position(&self, body: &crate::body::Body) -> ultraviolet::Vec2 {
        if self.side_view_mode {
            // Side view: X-Z coordinates (show X vs Z)
            ultraviolet::Vec2::new(body.pos.x, body.z)
        } else {
            // Top-down view: X-Y coordinates (default)
            body.pos
        }
    }
}