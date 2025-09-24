// Drawing routines split into focused modules

mod field;
mod charge;
mod foil_wave;
mod density;

pub use field::compute_field_at_point;

use super::state::*;
use palette::{Hsluv, IntoColor, Srgba};
use ultraviolet::Vec2;
use crate::quadtree::Quadtree;
use crate::body::Species;
use crate::profile_scope;
use rayon::prelude::*;
use std::sync::atomic::Ordering;

impl super::Renderer {
    pub fn draw(&mut self, ctx: &mut quarkstrom::RenderContext, width: u16, height: u16) {
        profile_scope!("draw_particles");
        // Prevent wgpu validation error: skip rendering if window is zero-sized
        if width == 0 || height == 0 {
            return;
        }
        if self.show_splash {
            ctx.clear_circles();
            ctx.clear_lines();
            ctx.clear_rects();
            return;
        }

        {
            let mut lock = UPDATE_LOCK.lock();
            if *lock {
                std::mem::swap(&mut self.bodies, &mut BODIES.lock());
                std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());
                std::mem::swap(&mut self.foils, &mut FOILS.lock());
                self.frame = self.frame.wrapping_add(1);
                self.update_foil_wave_history();

                // Update plotting system with new data
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                self.plotting_system.update_plots(&self.bodies, &self.foils, current_time, self.domain_width, self.domain_height);

                // Update diagnostics
                if let Some(ref mut diagnostic) = self.transference_number_diagnostic {
                    profile_scope!("diagnostics_transference");
                    diagnostic.calculate(&self.bodies);
                }
                if let Some(ref mut diag) = self.foil_electron_fraction_diagnostic {
                    profile_scope!("diagnostics_foil_electron");
                    // Create a temporary quadtree for diagnostic calculation
                    let mut temp_quadtree = crate::quadtree::Quadtree::new(1.0, 2.0, 1, 1024);
                    temp_quadtree.nodes = self.quadtree.clone();
                    
                    // Use time-throttled calculation to avoid performance issues
                    let current_time = *crate::renderer::state::SIM_TIME.lock();
                    diag.calculate_if_needed(&self.bodies, &self.foils, &temp_quadtree, current_time, 1.0);
                }
                if let Some(ref mut diag) = self.solvation_diagnostic {
                    profile_scope!("diagnostics_solvation");
                    // Only run solvation diagnostic every 10th frame to improve performance
                    if self.frame % 10 == 0 {
                        // Use optimized quadtree-based calculation for much better performance
                        let mut temp_quadtree = crate::quadtree::Quadtree::new(1.0, 2.0, 1, 1024);
                        temp_quadtree.nodes = self.quadtree.clone();
                        diag.calculate(&self.bodies, &temp_quadtree);
                    }
                }
            }
            if let Some(body) = self.confirmed_bodies.take() {
                self.bodies.push(body.clone());
                SPAWN.lock().push(body.clone());
            }
            *lock = false;
        }

        // Synchronize domain size from shared state to keep GUI in sync
        self.domain_width = *crate::renderer::state::DOMAIN_WIDTH.lock();
        self.domain_height = *crate::renderer::state::DOMAIN_HEIGHT.lock();

        ctx.clear_circles();
        ctx.clear_lines();
        ctx.clear_rects();
        ctx.set_view_pos(self.pos);
        ctx.set_view_scale(self.scale);

        if !self.bodies.is_empty() {
            // --- Ion Classification Overlay (Draw halos BEFORE particles) ---
            if let Some(ref solvation_diag) = self.solvation_diagnostic {
                // Draw CIP pairs with blue for cation, dark blue for anion, light blue for solvents
                if self.show_cip_ions {
                    for &(cation_id, anion_id, ref cation_solvents, ref anion_solvents) in &solvation_diag.cip_pairs {
                        // Draw cation
                        if let Some(body) = self.bodies.iter().find(|b| b.id == cation_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [0, 100, 255, 80]);
                        }
                        // Draw anion
                        if let Some(body) = self.bodies.iter().find(|b| b.id == anion_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [0, 50, 150, 80]);
                        }
                        // Draw cation solvents
                        for &solvent_id in cation_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [100, 150, 255, 60]);
                            }
                        }
                        // Draw anion solvents
                        for &solvent_id in anion_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [50, 100, 200, 60]);
                            }
                        }
                    }
                }

                // Draw SIP pairs with yellow for cation, dark yellow for anion, light yellow for solvents
                if self.show_sip_ions {
                    for &(cation_id, anion_id, ref cation_solvents, ref anion_solvents) in &solvation_diag.sip_pairs {
                        // Draw cation
                        if let Some(body) = self.bodies.iter().find(|b| b.id == cation_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [255, 255, 0, 80]);
                        }
                        // Draw anion
                        if let Some(body) = self.bodies.iter().find(|b| b.id == anion_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [220, 220, 0, 80]);
                        }
                        // Draw cation solvents
                        for &solvent_id in cation_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [255, 255, 120, 60]);
                            }
                        }
                        // Draw anion solvents
                        for &solvent_id in anion_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [240, 240, 80, 60]);
                            }
                        }
                    }
                }

                // Draw S2IP pairs with orange for cation, dark orange for anion, light orange for solvents
                if self.show_s2ip_ions {
                    for &(cation_id, anion_id, ref cation_solvents, ref anion_solvents) in &solvation_diag.s2ip_pairs {
                        // Draw cation
                        if let Some(body) = self.bodies.iter().find(|b| b.id == cation_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [255, 100, 0, 80]);
                        }
                        // Draw anion
                        if let Some(body) = self.bodies.iter().find(|b| b.id == anion_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [230, 80, 0, 80]);
                        }
                        // Draw cation solvents
                        for &solvent_id in cation_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [255, 150, 50, 60]);
                            }
                        }
                        // Draw anion solvents
                        for &solvent_id in anion_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [240, 120, 30, 60]);
                            }
                        }
                    }
                }

                // Draw FD (free/dissociated) cations with red for cation, light red for solvents
                if self.show_fd_ions {
                    for &(cation_id, ref cation_solvents) in &solvation_diag.fd_cations {
                        // Draw cation
                        if let Some(body) = self.bodies.iter().find(|b| b.id == cation_id) {
                            ctx.draw_circle(self.get_display_position(body), body.radius * 2.0, [255, 0, 0, 120]);
                        }
                        // Draw cation solvents
                        for &solvent_id in cation_solvents {
                            if let Some(body) = self.bodies.iter().find(|b| b.id == solvent_id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [255, 80, 80, 90]);
                            }
                        }
                    }
                }
            }

            if self.show_bodies {
                for body in &self.bodies {
                    let mut color = body.species.color();
                    let mut draw_radius = body.radius;

                    // Apply dark mode if enabled
                    if self.species_dark_mode_enabled {
                        match body.species {
                            Species::LithiumMetal | Species::ElectrolyteAnion | Species::EC | Species::DMC => {
                                let darkness_factor = 1.0 - self.species_dark_mode_strength;
                                color[0] = (color[0] as f32 * darkness_factor) as u8;
                                color[1] = (color[1] as f32 * darkness_factor) as u8;
                                color[2] = (color[2] as f32 * darkness_factor) as u8;
                            }
                            _ => {} // Don't apply dark mode to other species
                        }
                    }

                    if SHOW_Z_VISUALIZATION.load(Ordering::Relaxed) {
                        let max_z = self.sim_config.max_z.max(1.0);
                        let z_strength = *crate::renderer::state::Z_VISUALIZATION_STRENGTH.lock();
                        
                        // Normalize z-coordinate to -1.0 to 1.0 range
                        let z_normalized = (body.z / max_z).clamp(-1.0, 1.0);
                        
                        // Apply elegant z-depth effects:
                        // 1. Size scaling: particles closer to camera (positive z) appear larger
                        let size_factor = 1.0 + (z_strength * 0.3 * z_normalized);
                        draw_radius *= size_factor.max(0.3); // Don't let particles disappear
                        
                        // 2. INVERTED Color brightness: particles at higher z are brighter, z=0 is darker for contrast
                        let brightness_factor = 1.0 + (z_strength * 0.6 * z_normalized); // Positive z gets brighter
                        let brightness = brightness_factor.clamp(0.2, 2.0); // Allow overbrightening, but keep minimum visibility
                        
                        // 3. Additional contrast: make z=0 depth darker
                        let z_zero_darkening = if z_normalized.abs() < 0.1 { // Close to z=0
                            1.0 - (z_strength * 0.3) // Darken z=0 particles
                        } else {
                            1.0
                        };
                        let final_brightness = brightness * z_zero_darkening.clamp(0.3, 1.0);
                        
                        // 4. Optional color tinting based on depth
                        if z_strength > 2.0 {
                            if z_normalized > 0.0 {
                                // Closer particles get warmer tint (more red/yellow)
                                let warm_factor = z_normalized * (z_strength - 2.0) * 0.1;
                                color[0] = (color[0] as f32 * (1.0 + warm_factor)).clamp(0.0, 255.0) as u8;
                                color[1] = (color[1] as f32 * (1.0 + warm_factor * 0.5)).clamp(0.0, 255.0) as u8;
                            } else {
                                // Further particles get cooler tint (more blue)
                                let cool_factor = (-z_normalized) * (z_strength - 2.0) * 0.1;
                                color[2] = (color[2] as f32 * (1.0 + cool_factor)).clamp(0.0, 255.0) as u8;
                                color[1] = (color[1] as f32 * (1.0 + cool_factor * 0.5)).clamp(0.0, 255.0) as u8;
                            }
                        }
                        
                        // Apply brightness scaling (can now make particles brighter than original)
                        color[0] = (color[0] as f32 * final_brightness).clamp(0.0, 255.0) as u8;
                        color[1] = (color[1] as f32 * final_brightness).clamp(0.0, 255.0) as u8;
                        color[2] = (color[2] as f32 * final_brightness).clamp(0.0, 255.0) as u8;
                        
                        // 5. Optional alpha transparency for depth
                        if z_strength > 1.5 {
                            let alpha_factor = 1.0 - (z_normalized.abs() * (z_strength - 1.5) * 0.2);
                            color[3] = (color[3] as f32 * alpha_factor.clamp(0.5, 1.0)) as u8;
                        }
                    }
                    
                    if body.species == Species::LithiumIon {
                        if body.surrounded_by_metal {
                            if self.show_metal_electron_deficiency {
                                color = [192, 190, 190, 255];
                            } else {
                                color = [192, 192, 192, 255];
                            }
                            // Force surrounded ions to appear with metal radius
                            draw_radius = Species::LithiumMetal.radius();
                        }
                    }

                    if body.species == Species::FoilMetal {
                        if let Some(foil) = self.foils.iter().find(|f| f.body_ids.contains(&body.id)) {
                            // Selected foil highlight
                            if self.selected_foil_ids.contains(&foil.id) {
                                ctx.draw_circle(self.get_display_position(body), body.radius * 1.1, [255, 255, 0, 32]);
                            }
                            
                            // Switching role halo
                            if self.show_switching_role_halos {
                                // Determine playback mode to decide step precedence.
                                // In playback (HistoryPlaying/HistoryPaused), always use the stored historical step.
                                // In live mode, prefer the UI-reported step and fall back to the cached one if missing.
                                let playback_mode = { crate::renderer::state::PLAYBACK_STATUS.lock().mode };
                                let cached_step = *crate::renderer::state::SWITCH_STEP.lock();
                                let maybe_step = match playback_mode {
                                    crate::renderer::state::PlaybackModeStatus::Live => {
                                        self.switch_ui_state
                                            .last_step_status
                                            .map(|(s, _)| s)
                                            .or(cached_step)
                                    }
                                    _ => cached_step,
                                };
                                if let Some(current_step) = maybe_step {
                                    let (pos_role, neg_role) = crate::switch_charging::roles_for_step(current_step);
                                    
                                    // Check if this foil has a role in the current active step
                                    let pos_foils = self.switch_ui_state.config.foils_for_role(pos_role);
                                    let neg_foils = self.switch_ui_state.config.foils_for_role(neg_role);
                                    
                                    if pos_foils.contains(&foil.id) {
                                        // Positive role - draw green halo
                                        ctx.draw_circle(self.get_display_position(body), body.radius * 1.3, [0, 255, 0, 100]);
                                    } else if neg_foils.contains(&foil.id) {
                                        // Negative role - draw red halo  
                                        ctx.draw_circle(self.get_display_position(body), body.radius * 1.3, [255, 0, 0, 100]);
                                    } else {
                                        // Inactive foil in switching mode - draw gray halo
                                        let has_any_role = crate::switch_charging::Role::ALL.iter()
                                            .any(|role| self.switch_ui_state.config.foils_for_role(*role).contains(&foil.id));
                                        if has_any_role {
                                            ctx.draw_circle(self.get_display_position(body), body.radius * 1.2, [128, 128, 128, 60]);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    ctx.draw_circle(self.get_display_position(body), draw_radius, color);

                    // Visualize electron count for FoilMetal
                    if self.show_foil_electron_deficiency && body.species == Species::FoilMetal {
                        let neutral_electrons = crate::config::FOIL_NEUTRAL_ELECTRONS;
                        let electron_count = body.electrons.len();
                        if electron_count > neutral_electrons {
                            ctx.draw_circle(
                                self.get_display_position(body),
                                body.radius * 0.5,
                                [0, 255, 0, 255],
                            );
                        } else if electron_count < neutral_electrons {
                            ctx.draw_circle(
                                self.get_display_position(body),
                                body.radius * 0.5,
                                [255, 0, 0, 255],
                            );
                        }
                    }

                    // Visualize electron count for LithiumMetal
                    if self.show_metal_electron_deficiency && body.species == Species::LithiumMetal {
                        let neutral_electrons = 1;
                        let electron_count = body.electrons.len();
                        if electron_count > neutral_electrons {
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [0, 255, 0, 255],
                            );
                        } else if electron_count < neutral_electrons {
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [255, 0, 0, 255],
                            );
                        }
                        for electron in &body.electrons {
                            let electron_pos = body.pos + electron.rel_pos;
                            ctx.draw_circle(
                                electron_pos,
                                body.radius * 0.3,
                                [0, 128, 255, 255],
                            );
                        }
                    }
                }
            }

            // --- Velocity Vector Overlay ---
            if self.sim_config.show_velocity_vectors {
                let scale = self.velocity_vector_scale;
                let color = [0, 255, 0, 255];
                for body in &self.bodies {
                    let end = body.pos + body.vel * scale;
                    ctx.draw_line(body.pos, end, color);
                }
            }

            if let Some(body) = &self.confirmed_bodies {
                ctx.draw_circle(self.get_display_position(body), body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            if let Some(body) = &self.spawn_body {
                ctx.draw_circle(self.get_display_position(body), body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            if let Some(id) = self.selected_particle_id {
                if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                    ctx.draw_circle(self.get_display_position(body), body.radius * 1.5, [255, 255, 0, 32]);
                }
            }

            for id in &self.selected_particle_ids {
                if let Some(body) = self.bodies.iter().find(|b| b.id == *id) {
                    ctx.draw_circle(self.get_display_position(body), body.radius * 3.0, [255, 255, 0, 128]);
                }
            }
        }

        if self.show_quadtree && !self.quadtree.is_empty() {
            let mut depth_range = self.depth_range;
            if depth_range.0 >= depth_range.1 {
                let mut stack = Vec::new();
                stack.push((Quadtree::ROOT, 0));

                let mut min_depth = usize::MAX;
                let mut max_depth = 0;
                while let Some((node, depth)) = stack.pop() {
                    let node = &self.quadtree[node];

                    if node.is_leaf() {
                        if depth < min_depth {
                            min_depth = depth;
                        }
                        if depth > max_depth {
                            max_depth = depth;
                        }
                    } else {
                        for i in 0..4 {
                            stack.push((node.children + i, depth + 1));
                        }
                    }
                }

                depth_range = (min_depth, max_depth);
            }
            let (min_depth, max_depth) = depth_range;

            let mut stack = Vec::new();
            stack.push((Quadtree::ROOT, 0));
            while let Some((node, depth)) = stack.pop() {
                let node = &self.quadtree[node];

                if node.is_branch() && depth < max_depth {
                    for i in 0..4 {
                        stack.push((node.children + i, depth + 1));
                    }
                } else if depth >= min_depth {
                    let quad = node.quad;
                    let half = Vec2::new(0.5, 0.5) * quad.size;
                    let min = quad.center - half;
                    let max = quad.center + half;

                    let t = ((depth - min_depth + !node.is_empty() as usize) as f32)
                        / (max_depth - min_depth + 1) as f32;

                    let start_h = -100.0;
                    let end_h = 80.0;
                    let h = start_h + (end_h - start_h) * t;
                    let s = 100.0;
                    let l = t * 100.0;

                    let c = Hsluv::new(h, s, l);
                    let rgba: Srgba = c.into_color();
                    let color = [
                        (rgba.red * 255.0) as u8,
                        (rgba.green * 255.0) as u8,
                        (rgba.blue * 255.0) as u8,
                        (rgba.alpha * 255.0) as u8,
                    ];

                    ctx.draw_rect(min, max, color);
                }
            }
        }

        if self.sim_config.show_charge_density {
            self.draw_charge_density(ctx);
        }

        if self.sim_config.show_2d_domain_density {
            self.draw_2d_domain_density(ctx);
        }

        if self.sim_config.show_field_isolines {
            self.draw_field_isolines(ctx);
        }

        if self.sim_config.show_field_vectors {
            let grid_spacing = 2.0;
            let field_scale = 2.0;
            let color = [255, 0, 0, 255];

            let half_view = Vec2::new(self.scale, self.scale);
            let min = self.pos - half_view;
            let max = self.pos + half_view;

            let nx = ((max.x - min.x) / grid_spacing).ceil() as usize;
            let ny = ((max.y - min.y) / grid_spacing).ceil() as usize;
            let mut lines = vec![(Vec2::zero(), Vec2::zero()); nx * ny];

            lines
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, line)| {
                    let ix = i % nx;
                    let iy = i / nx;
                    let x = min.x + ix as f32 * grid_spacing;
                    let y = min.y + iy as f32 * grid_spacing;
                    let pos = Vec2::new(x, y);
                    let field = compute_field_at_point(&self.bodies, pos, &self.sim_config);
                    *line = (pos, pos + field * field_scale);
                });

            for (start, end) in lines {
                ctx.draw_line(start, end, color);
            }
        }

        if !self.selected_foil_ids.is_empty() {
            self.draw_foil_square_waves(ctx);
        }

        // Draw screen capture region selection
        if self.is_selecting_region {
            if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                // Convert screen coordinates to world coordinates for drawing
                let world_start = self.screen_to_world(start, width, height);
                let world_end = self.screen_to_world(end, width, height);
                
                // Draw red rectangle outline only (no fill)
                let min_x = world_start.x.min(world_end.x);
                let max_x = world_start.x.max(world_end.x);
                let min_y = world_start.y.min(world_end.y);
                let max_y = world_start.y.max(world_end.y);
                
                let top_left = Vec2::new(min_x, max_y);
                let top_right = Vec2::new(max_x, max_y);
                let bottom_right = Vec2::new(max_x, min_y);
                let bottom_left = Vec2::new(min_x, min_y);
                
                // Draw rectangle outline in red (no fill)
                let red = [255, 0, 0, 255];
                ctx.draw_line(top_left, top_right, red);
                ctx.draw_line(top_right, bottom_right, red);
                ctx.draw_line(bottom_right, bottom_left, red);
                ctx.draw_line(bottom_left, top_left, red);
            }
        }

        // Draw the saved capture region if one exists
        if let Some((ratio_start, ratio_end)) = self.capture_region_ratio {
            let start_screen = Vec2::new(ratio_start.x * width as f32, ratio_start.y * height as f32);
            let end_screen = Vec2::new(ratio_end.x * width as f32, ratio_end.y * height as f32);
            let world_start = self.screen_to_world(start_screen, width, height);
            let world_end = self.screen_to_world(end_screen, width, height);

            let min_x = world_start.x.min(world_end.x);
            let max_x = world_start.x.max(world_end.x);
            let min_y = world_start.y.min(world_end.y);
            let max_y = world_start.y.max(world_end.y);
            
            let top_left = Vec2::new(min_x, max_y);
            let top_right = Vec2::new(max_x, max_y);
            let bottom_right = Vec2::new(max_x, min_y);
            let bottom_left = Vec2::new(min_x, min_y);
            
            // Draw rectangle outline in blue with some transparency
            let blue = [0, 128, 255, 128];
            ctx.draw_line(top_left, top_right, blue);
            ctx.draw_line(top_right, bottom_right, blue);
            ctx.draw_line(bottom_right, bottom_left, blue);
            ctx.draw_line(bottom_left, top_left, blue);
        }
    }
}
