use super::state::*;
use palette::{Hsluv, IntoColor, Srgba};
use ultraviolet::Vec2;
use crate::quadtree::Quadtree;
use crate::body::{Species, Body};

impl super::Renderer {
    pub fn draw(&mut self, ctx: &mut quarkstrom::RenderContext, width: u16, height: u16) {
        // Prevent wgpu validation error: skip rendering if window is zero-sized
        if width == 0 || height == 0 {
            return;
        }

        {
            let mut lock = UPDATE_LOCK.lock();
            if *lock {
                std::mem::swap(&mut self.bodies, &mut BODIES.lock());
                std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());
                std::mem::swap(&mut self.foils, &mut FOILS.lock());
            }
            if let Some(body) = self.confirmed_bodies.take() {
                self.bodies.push(body.clone());
                SPAWN.lock().push(body.clone());
            }
            *lock = false;
        }

        ctx.clear_circles();
        ctx.clear_lines();
        ctx.clear_rects();
        ctx.set_view_pos(self.pos);
        ctx.set_view_scale(self.scale);

        if !self.bodies.is_empty() {
			if self.show_bodies {
				for body in &self.bodies {
					// Map charge to RGB color: red for positive, blue for negative, white for 0
					/*let charge = body.charge;
					let max_charge = 1.0; // adjust if needed

					let norm = (charge / max_charge).clamp(-1.0, 1.0);
					let r = norm.max(0.0);
					let g = 1.0 - norm.abs();
					let b = (-norm).max(0.0);

					// Convert to [u8; 4] RGBA
					let color = [
						(r * 255.0) as u8,
						(g * 255.0) as u8,
						(b * 255.0) as u8,
						255,
					];*/

                    let color = match body.species {
                        Species::LithiumIon => [255, 255, 0, 255],      // Yellow
                        Species::LithiumMetal => [192, 192, 192, 255],  // Silverish
                        Species::FoilMetal => [128, 64, 0, 255],        // Brownish (example)
                        Species::ElectrolyteAnion => [0, 128, 255, 255], // Blueish for anion
                    };

                    ctx.draw_circle(body.pos, body.radius, color);

                    if body.species == Species::FoilMetal {
                        if let Some(foil) = self.foils.iter().find(|f| f.body_ids.contains(&body.id)) {
                            if self.selected_foil_ids.contains(&foil.id) {
                                ctx.draw_circle(body.pos, body.radius * 1.5, [255, 255, 0, 128]);
                            }
                        }
                    }

                    // Visualize electron count for FoilMetal
                    if body.species == Species::FoilMetal {
                        let neutral_electrons = crate::config::FOIL_NEUTRAL_ELECTRONS;
                        let electron_count = body.electrons.len();
                        if electron_count > neutral_electrons {
                            // More electrons: draw a green circle (smaller, centered)
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [0, 255, 0, 255], // green
                            );
                        } else if electron_count < neutral_electrons {
                            // Fewer electrons: draw a red circle (smaller, centered)
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [255, 0, 0, 255], // red
                            );
                        }
                    }

                    // Visualize electron count for LithiumMetal
                    if body.species == Species::LithiumMetal {
                        let neutral_electrons = 1; // adjust if your neutral is different
                        let electron_count = body.electrons.len();
                        if electron_count > neutral_electrons {
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [0, 255, 0, 255], // green
                            );
                        } else if electron_count < neutral_electrons {
                            ctx.draw_circle(
                                body.pos,
                                body.radius * 0.5,
                                [255, 0, 0, 255], // red
                            );
                        }
                        for electron in &body.electrons {
                            let electron_pos = body.pos + electron.rel_pos;
                            ctx.draw_circle(
                                electron_pos,
                                body.radius * 0.3,
                                [0, 128, 255, 255],
                            ); // blue
                        }
                    }
                                }
                        }

            // --- Velocity Vector Overlay ---
            if self.sim_config.show_velocity_vectors {
                let scale = self.velocity_vector_scale;
                let color = [0, 255, 0, 255]; // green
                for body in &self.bodies {
                    let end = body.pos + body.vel * scale;
                    ctx.draw_line(body.pos, end, color);
                }
            }


            if let Some(body) = &self.confirmed_bodies {
                ctx.draw_circle(body.pos, body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            if let Some(body) = &self.spawn_body {
                ctx.draw_circle(body.pos, body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            for id in &self.selected_particle_ids {
                if let Some(body) = self.bodies.iter().find(|b| b.id == *id) {
                    // Draw a larger, semi-transparent circle as a halo
                    ctx.draw_circle(body.pos, body.radius * 2.0, [255, 255, 0, 128]);
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

        // --- CHARGE DENSITY VISUALIZATION ---
        if self.sim_config.show_charge_density {
            self.draw_charge_density(ctx);
        }

        // --- FIELD ISOLINE VISUALIZATION ---
        if self.sim_config.show_field_isolines {
            self.draw_field_isolines(ctx);
        }

        // --- FIELD VECTOR VISUALIZATION ---
        if self.sim_config.show_field_vectors {
            let grid_spacing = 2.0; // simulation units
            let field_scale = 2.0;   // much larger for debug
            let color = [255, 0, 0, 255]; // opaque red for debug

            // Compute visible bounds in world coordinates
            let half_view = Vec2::new(self.scale, self.scale);
            let min = self.pos - half_view;
            let max = self.pos + half_view;

            let mut x = min.x;
            while x < max.x {
                let mut y = min.y;
                while y < max.y {
                    let pos = Vec2::new(x, y);
                    let field = compute_field_at_point(&self.bodies, pos, &self.sim_config);
                    let end = pos + field * field_scale;
                    ctx.draw_line(pos, end, color);
                    y += grid_spacing;
                }
                x += grid_spacing;
            }
        }
    }
}

impl super::Renderer {
    /// Draw electric field isolines using a simple marching squares algorithm.
    pub fn draw_field_isolines(&mut self, ctx: &mut quarkstrom::RenderContext) {
        // Dynamic grid spacing based on zoom
        let min_spacing = 8.0;
        let max_spacing = 60.0;
        let grid_spacing = (self.scale / 10.0).clamp(min_spacing, max_spacing);

        // Expand the view area to ensure isolines cover the full window, even in fullscreen
        let half_view = Vec2::new(self.scale, self.scale) * 1.2;
        let min = self.pos - half_view;
        let max = self.pos + half_view;

        let nx = ((max.x - min.x) / grid_spacing).ceil() as usize + 1;
        let ny = ((max.y - min.y) / grid_spacing).ceil() as usize + 1;
        let stride_x = (nx as f32 / 200.0).ceil().max(1.0) as usize;
        let stride_y = (ny as f32 / 200.0).ceil().max(1.0) as usize;

        let mut samples = Vec::with_capacity((nx/stride_x+1)*(ny/stride_y+1));
        let mut min_val = f32::INFINITY;
        let mut max_val = 0.0f32;
        for ix in (0..nx).step_by(stride_x) {
            for iy in (0..ny).step_by(stride_y) {
                let x = min.x + ix as f32 * grid_spacing;
                let y = min.y + iy as f32 * grid_spacing;
                let pos = Vec2::new(x, y);
                let v = compute_potential_at_point(&self.bodies, pos, &self.sim_config);
                min_val = min_val.min(v);
                max_val = max_val.max(v);
                samples.push(v);
            }
        }

        let abs_max = min_val.abs().max(max_val.abs());
        if abs_max < 1e-3 {
            return;
        }

        // Percentile-based isoline levels
        let num_levels = 11;
        let mut sorted_samples = samples.clone();
        sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut iso_values = Vec::with_capacity(num_levels);
        for i in 0..num_levels {
            let p = i as f32 / (num_levels - 1) as f32;
            let idx = ((sorted_samples.len() - 1) as f32 * p).round() as usize;
            iso_values.push(sorted_samples[idx]);
        }
        iso_values.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        // Now do the full grid for isoline drawing
        let mut field_grid = vec![0.0f32; nx * ny];
        for ix in 0..nx {
            for iy in 0..ny {
                let x = min.x + ix as f32 * grid_spacing;
                let y = min.y + iy as f32 * grid_spacing;
                let pos = Vec2::new(x, y);
                let v = compute_potential_at_point(&self.bodies, pos, &self.sim_config);
                field_grid[iy * nx + ix] = v;
            }
        }

        for &iso in &iso_values {
            // Color: blue for negative, white for zero, red for positive
            let t = (iso / abs_max).clamp(-1.0, 1.0);
            let color = if t < 0.0 {
                let f = t.abs();
                [
                    (255.0 * (1.0 - f) + 0.0 * f) as u8,
                    (255.0 * (1.0 - f) + 128.0 * f) as u8,
                    255u8,
                    255u8,
                ]
            } else if t > 0.0 {
                let f = t;
                [
                    255u8,
                    (255.0 * (1.0 - f) + 64.0 * f) as u8,
                    (255.0 * (1.0 - f) + 64.0 * f) as u8,
                    255u8,
                ]
            } else {
                [255, 255, 255, 255]
            };

            for ix in 0..nx - 1 {
                for iy in 0..ny - 1 {
                    let i00 = iy * nx + ix;
                    let i10 = iy * nx + ix + 1;
                    let i01 = (iy + 1) * nx + ix;
                    let i11 = (iy + 1) * nx + ix + 1;

                    let v00 = field_grid[i00];
                    let v10 = field_grid[i10];
                    let v01 = field_grid[i01];
                    let v11 = field_grid[i11];

                    let p00 = Vec2::new(min.x + ix as f32 * grid_spacing, min.y + iy as f32 * grid_spacing);
                    let p10 = Vec2::new(min.x + (ix + 1) as f32 * grid_spacing, min.y + iy as f32 * grid_spacing);
                    let p01 = Vec2::new(min.x + ix as f32 * grid_spacing, min.y + (iy + 1) as f32 * grid_spacing);
                    let p11 = Vec2::new(min.x + (ix + 1) as f32 * grid_spacing, min.y + (iy + 1) as f32 * grid_spacing);

                    let mut pts = Vec::new();
                    if (v00 - iso) * (v10 - iso) < 0.0 {
                        let t = (iso - v00) / (v10 - v00);
                        pts.push(lerp(p00, p10, t));
                    }
                    if (v10 - iso) * (v11 - iso) < 0.0 {
                        let t = (iso - v10) / (v11 - v10);
                        pts.push(lerp(p10, p11, t));
                    }
                    if (v11 - iso) * (v01 - iso) < 0.0 {
                        let t = (iso - v11) / (v01 - v11);
                        pts.push(lerp(p11, p01, t));
                    }
                    if (v01 - iso) * (v00 - iso) < 0.0 {
                        let t = (iso - v01) / (v00 - v01);
                        pts.push(lerp(p01, p00, t));
                    }
                    if pts.len() == 2 {
                        ctx.draw_line(pts[0], pts[1], color);
                    }
                }
            }
        }
    }

    /// Draw a simple charge density heatmap.
    pub fn draw_charge_density(&self, ctx: &mut quarkstrom::RenderContext) {
        let grid_spacing = 5.0;
        let smoothing = 5.0;

        let half_view = Vec2::new(self.scale, self.scale);
        let min = self.pos - half_view;
        let max = self.pos + half_view;

        let nx = ((max.x - min.x) / grid_spacing).ceil() as usize + 1;
        let ny = ((max.y - min.y) / grid_spacing).ceil() as usize + 1;

        let mut samples = vec![0.0f32; nx * ny];
        let mut max_abs = 0.0f32;

        for ix in 0..nx {
            for iy in 0..ny {
                let x = min.x + (ix as f32 + 0.5) * grid_spacing;
                let y = min.y + (iy as f32 + 0.5) * grid_spacing;
                let pos = Vec2::new(x, y);
                let mut density = 0.0f32;
                for body in &self.bodies {
                    let r = pos - body.pos;
                    let dist2 = r.mag_sq();
                    let weight = (-dist2 / (smoothing * smoothing)).exp();
                    density += body.charge * weight;
                }
                max_abs = max_abs.max(density.abs());
                samples[iy * nx + ix] = density;
            }
        }

        max_abs = max_abs.max(1e-6);

        for ix in 0..nx - 1 {
            for iy in 0..ny - 1 {
                let density = samples[iy * nx + ix];
                let norm = (density / max_abs).clamp(-1.0, 1.0);
                let r = norm.max(0.0);
                let b = (-norm).max(0.0);
                let color = [
                    (r * 255.0) as u8,
                    0,
                    (b * 255.0) as u8,
                    80,
                ];

                let rect_min = Vec2::new(
                    min.x + (ix as f32 + 0.5) * grid_spacing,
                    min.y + (iy as f32 + 0.5) * grid_spacing,
                );
                let rect_max = rect_min + Vec2::new(grid_spacing, grid_spacing);
                ctx.draw_rect(rect_min, rect_max, color);
            }
        }
    }
}

// Helper function to compute the electric field at a point
pub fn compute_field_at_point(
    bodies: &[Body],
    pos: Vec2,
    config: &crate::config::SimConfig,
) -> Vec2 {
    let mut field = Vec2::zero();

    if config.isoline_field_mode != crate::config::IsolineFieldMode::ExternalOnly {
        for body in bodies {
            let r = pos - body.pos;
            let dist2 = r.mag_sq().max(1e-4); // avoid div by zero
            let e = body.charge * r / (dist2 * dist2.sqrt()); // Coulomb's law (unitless K)
            field += e;
        }
    }

    if config.isoline_field_mode != crate::config::IsolineFieldMode::BodyOnly {
        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        let background = Vec2::new(theta.cos(), theta.sin()) * mag;
        field += background;
    }

    field
}

fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a + (b - a) * t
}

/// Compute the electric potential at a point due to all bodies.
pub fn compute_potential_at_point(
    bodies: &[Body],
    pos: Vec2,
    config: &crate::config::SimConfig,
) -> f32 {
    let mut potential = 0.0f32;

    if config.isoline_field_mode != crate::config::IsolineFieldMode::ExternalOnly {
        for body in bodies {
            let r = pos - body.pos;
            let dist = r.mag().max(1e-4);
            potential += crate::simulation::forces::K_E * body.charge / dist;
        }
    }

    if config.isoline_field_mode != crate::config::IsolineFieldMode::BodyOnly {
        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        let background = Vec2::new(theta.cos(), theta.sin()) * mag;
        potential += -background.dot(pos);
    }

    potential
}