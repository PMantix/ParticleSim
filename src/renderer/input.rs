use super::state::*;
use super::{GuiTab, MeasurementRecord};
use quarkstrom::winit::event::VirtualKeyCode;
use std::f32::consts::{PI, TAU};
use std::sync::atomic::Ordering;
//use crate::body::{Body, Electron};
use super::state::{SimCommand, SIM_COMMAND_SENDER};
use crate::body::Species;
use crate::profile_scope;
use quarkstrom::winit_input_helper::WinitInputHelper;
use ultraviolet::Vec2;

impl super::Renderer {
    pub fn handle_input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        profile_scope!("input_handling");
        if width == 0 || height == 0 {
            return;
        }
        if self.show_splash {
            if input.mouse_pressed(1)  // Only right mouse button
                || input.key_pressed(VirtualKeyCode::Return)
                || input.key_pressed(VirtualKeyCode::Space)
                || input.key_pressed(VirtualKeyCode::Escape)
            {
                self.start_selected_scenario();
            }
            return;
        }
        // Update window dimensions on resize
        if width != self.window_width || height != self.window_height {
            self.window_width = width;
            self.window_height = height;
        }

        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::E);

        if input.key_pressed(VirtualKeyCode::Space) {
            // Check if we're in playback mode
            let playback_status = crate::renderer::state::PLAYBACK_STATUS.lock();
            match playback_status.mode {
                crate::renderer::state::PlaybackModeStatus::HistoryPaused => {
                    // In history mode, send PlaybackPlay command instead of toggling PAUSED
                    drop(playback_status);
                    if let Some(sender) = &*crate::renderer::state::SIM_COMMAND_SENDER.lock() {
                        let _ = sender.send(crate::renderer::state::SimCommand::PlaybackPlay {
                            auto_resume: false,
                        });
                    }
                }
                crate::renderer::state::PlaybackModeStatus::HistoryPlaying => {
                    // In history playing mode, send PlaybackPause command
                    drop(playback_status);
                    if let Some(sender) = &*crate::renderer::state::SIM_COMMAND_SENDER.lock() {
                        let _ = sender.send(crate::renderer::state::SimCommand::PlaybackPause);
                    }
                }
                crate::renderer::state::PlaybackModeStatus::Live => {
                    // In live mode, toggle PAUSED as normal
                    drop(playback_status);
                    let val = PAUSED.load(Ordering::Relaxed);
                    PAUSED.store(!val, Ordering::Relaxed);
                }
            }
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

        // Camera grab (middle mouse or Alt+left for trackpad users)
        let alt_pan =
            input.key_held(VirtualKeyCode::LAlt) || input.key_held(VirtualKeyCode::RAlt);
        if input.mouse_held(2) || (alt_pan && input.mouse_held(0)) {
            let (mdx, mdy) = input.mouse_diff();
            // Mouse diff coordinates are also in logical pixels
            self.pos.x -= mdx / height as f32 * self.scale * 2.0;
            self.pos.y += mdy / height as f32 * self.scale * 2.0;
        }

        // Screen capture removed

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

        // Update hovered species if in Legend tab
        if self.current_tab == GuiTab::Legend {
            let mouse_pos = world_mouse();
            let mut closest_species = None;
            let mut min_dist = f32::MAX;
            
            // Check bodies for hover
            // Optimization: Use quadtree if available, but simple iteration is fine for now
            // since we only do this when Legend tab is open
            for body in &self.bodies {
                let display_pos = self.get_display_position(body);
                let dist = (display_pos - mouse_pos).mag();
                if dist < min_dist && dist < body.radius * 1.5 { // Slightly larger hit area
                    min_dist = dist;
                    closest_species = Some(body.species);
                }
            }
            self.hovered_species = closest_species;
        } else {
            self.hovered_species = None;
        }

        if self.current_tab == GuiTab::Measurement {
            self.measurement_cursor = input.mouse().map(|_| world_mouse());

            if input.mouse_pressed(1) {
                self.current_tab = self.last_non_measurement_tab;
                self.measurement_selecting_start = false;
                self.measurement_selecting_direction = false;
                return;
            }

            if self.measurement_selecting_start {
                if input.mouse_pressed(0) {
                    if let Some(pos) = self.measurement_cursor {
                        self.measurement_start = Some(pos);
                        self.measurement_selecting_start = false;
                        // After choosing start, allow defining direction next if requested
                    }
                }
            } else if self.measurement_selecting_direction {
                // Define direction using a second click: vector from start to clicked point
                if input.mouse_pressed(0) {
                    if let (Some(start), Some(cursor)) =
                        (self.measurement_start, self.measurement_cursor)
                    {
                        let dir = cursor - start;
                        if dir.mag_sq() > 1e-12 {
                            self.measurement_direction = Some(dir.normalized());
                        } else {
                            // Degenerate click: do not set, leave None
                            self.measurement_direction = None;
                        }
                        self.measurement_selecting_direction = false;
                    } else {
                        // No start yet; ignore and disable direction mode
                        self.measurement_selecting_direction = false;
                    }
                }
            } else if input.mouse_pressed(0) {
                if let Some(distance) = self.current_measurement_distance() {
                    let time_fs = *crate::renderer::state::SIM_TIME.lock();
                    // Gather switch charging metadata at this moment (if available)
                    let switch_step_opt = *crate::renderer::state::SWITCH_STEP.lock();
                    let mut switch_mode: Option<String> = None;
                    let mut switch_value: Option<f64> = None;
                    let mut pos_role: Option<String> = None;
                    let mut neg_role: Option<String> = None;
                    if let Some(step) = switch_step_opt {
                        // Roles for this step
                        let (pos, neg) = crate::switch_charging::roles_for_step(step);
                        pos_role = Some(pos.display().to_string());
                        neg_role = Some(neg.display().to_string());

                        // Get the active setpoint from the appropriate config mode
                        if self.switch_ui_state.config.use_active_inactive_setpoints {
                            // Use new step-based active/inactive setpoints (record the active setpoint)
                            if let Some(sai) =
                                self.switch_ui_state.config.step_active_inactive.get(&step)
                            {
                                switch_mode = Some(match sai.active.mode {
                                    crate::switch_charging::Mode::Current => "Current".to_string(),
                                    crate::switch_charging::Mode::Overpotential => {
                                        "Overpotential".to_string()
                                    }
                                });
                                switch_value = Some(sai.active.value);
                            }
                        } else {
                            // Use legacy step setpoints
                            if let Some(sp) = self
                                .switch_ui_state
                                .config
                                .step_setpoints
                                .get(&step)
                                .cloned()
                            {
                                switch_mode = Some(match sp.mode {
                                    crate::switch_charging::Mode::Current => "Current".to_string(),
                                    crate::switch_charging::Mode::Overpotential => {
                                        "Overpotential".to_string()
                                    }
                                });
                                switch_value = Some(sp.value);
                            }
                        }
                    }
                    self.measurement_history.push(MeasurementRecord {
                        step: self.frame,
                        time_fs,
                        distance,
                        switch_step: switch_step_opt,
                        switch_mode,
                        switch_value,
                        pos_role,
                        neg_role,
                    });
                }
            }

            return;
        } else {
            self.measurement_cursor = None;
            self.measurement_selecting_start = false;
            self.measurement_selecting_direction = false;
        }

        if input.mouse_pressed(1) {
            if self.spawn_body.is_none() {
                // If shift is held, select a particle
                if input.key_held(VirtualKeyCode::LShift) || input.key_held(VirtualKeyCode::RShift)
                {
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
                                if let Some(foil) =
                                    self.foils.iter().find(|f| f.body_ids.contains(&id))
                                {
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
                    let body =
                        crate::renderer::gui::make_body_with_species(mouse, Vec2::zero(), spec);
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
