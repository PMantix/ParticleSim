// Drawing routines split into focused modules

mod field;
mod charge;
mod foil_wave;

pub use field::{compute_field_at_point, compute_potential_at_point};

pub use foil_wave::{update_foil_wave_history, draw_foil_square_waves};

use super::state::*;
use palette::{Hsluv, IntoColor, Srgba};
use ultraviolet::Vec2;
use crate::quadtree::Quadtree;
use crate::body::{Species, Body};
use rayon::prelude::*;

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
                self.frame = self.frame.wrapping_add(1);
                self.update_foil_wave_history();

                // Update plotting system with new data
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                self.plotting_system.update_plots(&self.bodies, &self.foils, current_time);

                // Update diagnostics
                if let Some(ref mut diagnostic) = self.transference_number_diagnostic {
                    diagnostic.calculate(&self.bodies);
                }
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
                    let mut color = body.species.color();
                    if body.species == Species::LithiumIon {
                        if body.surrounded_by_metal {
                            if self.show_electron_deficiency {
                                color = [192, 190, 190, 255];
                            } else {
                                color = [192, 192, 192, 255];
                            }
                        }
                    }

                    if body.species == Species::FoilMetal {
                        if let Some(foil) = self.foils.iter().find(|f| f.body_ids.contains(&body.id)) {
                            if self.selected_foil_ids.contains(&foil.id) {
                                ctx.draw_circle(body.pos, body.radius * 1.1, [255, 255, 0, 32]);
                            }
                        }
                    }

                    ctx.draw_circle(body.pos, body.radius, color);

                    // Visualize electron count for FoilMetal
                    if self.show_electron_deficiency && body.species == Species::FoilMetal {
                        let neutral_electrons = crate::config::FOIL_NEUTRAL_ELECTRONS;
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
                    }

                    // Visualize electron count for LithiumMetal
                    if self.show_electron_deficiency && body.species == Species::LithiumMetal {
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
                ctx.draw_circle(body.pos, body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            if let Some(body) = &self.spawn_body {
                ctx.draw_circle(body.pos, body.radius, [0xff; 4]);
                ctx.draw_line(body.pos, body.pos + body.vel, [0xff; 4]);
            }

            if let Some(id) = self.selected_particle_id {
                if let Some(body) = self.bodies.iter().find(|b| b.id == id) {
                    ctx.draw_circle(body.pos, body.radius * 1.5, [255, 255, 0, 32]);
                }
            }

            for id in &self.selected_particle_ids {
                if let Some(body) = self.bodies.iter().find(|b| b.id == *id) {
                    ctx.draw_circle(body.pos, body.radius * 3.0, [255, 255, 0, 128]);
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
    }
}
