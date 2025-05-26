use super::state::*;
use palette::{Hsluv, IntoColor, Srgba};
use ultraviolet::Vec2;
use crate::quadtree::Quadtree;
use crate::body::{Species, Body};

impl super::Renderer {
    pub fn draw(&mut self, ctx: &mut quarkstrom::RenderContext) {
        {
            let mut lock = UPDATE_LOCK.lock();
            if *lock {
                std::mem::swap(&mut self.bodies, &mut BODIES.lock());
                std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());
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
                        //Species::Electron => [255, 0, 0, 255],        // Rd
                        Species::LithiumMetal => [192, 192, 192, 255],// Silverish
                    };

					ctx.draw_circle(body.pos, body.radius, color);

                    if body.species == Species::LithiumMetal {
                        for electron in &body.electrons {
                            let electron_pos = body.pos + electron.rel_pos;
                            ctx.draw_circle(electron_pos, body.radius * 0.3, [0, 128, 255, 255]); // blue
                        }
                    }
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

            if let Some(id) = self.selected_particle_id {
                if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                    // Draw a larger, semi-transparent circle as a halo
                    ctx.draw_circle(body.pos, body.radius * 2.0, [255, 255, 0, 128]); // yellow, semi-transparent
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

// Helper function to compute the electric field at a point
pub fn compute_field_at_point(bodies: &[Body], pos: Vec2, _config: &crate::config::SimConfig) -> Vec2 {
    let mut field = Vec2::zero();
    for body in bodies {
        let r = pos - body.pos;
        let dist2 = r.mag_sq().max(1e-4); // avoid div by zero
        let e = body.charge * r / (dist2 * dist2.sqrt()); // Coulomb's law (unitless K)
        field += e;
    }
    field
}