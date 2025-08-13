use crate::renderer::Renderer;
use crate::renderer::state::{FIELD_MAGNITUDE, FIELD_DIRECTION};
use ultraviolet::Vec2;
use crate::body::Body;
use rayon::prelude::*;

impl Renderer {
    /// Draw electric field isolines using a simple marching squares algorithm.
    pub fn draw_field_isolines(&mut self, ctx: &mut quarkstrom::RenderContext) {
        let min_spacing = 8.0;
        let max_spacing = 60.0;
        let grid_spacing = (self.scale / 10.0).clamp(min_spacing, max_spacing);

        let half_view = Vec2::new(self.scale, self.scale) * 1.2;
        let min = self.pos - half_view;
        let max = self.pos + half_view;

        let nx = ((max.x - min.x) / grid_spacing).ceil() as usize + 1;
        let ny = ((max.y - min.y) / grid_spacing).ceil() as usize + 1;
        let stride_x = (nx as f32 / 200.0).ceil().max(1.0) as usize;
        let stride_y = (ny as f32 / 200.0).ceil().max(1.0) as usize;

        let mut samples = Vec::with_capacity((nx / stride_x + 1) * (ny / stride_y + 1));
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

        let num_levels = 11;
        let mut sorted_samples = samples.clone();
        sorted_samples.sort_by(|a, b| a.total_cmp(b));
        let mut iso_values = Vec::with_capacity(num_levels);
        for i in 0..num_levels {
            let p = i as f32 / (num_levels - 1) as f32;
            let idx = ((sorted_samples.len() - 1) as f32 * p).round() as usize;
            iso_values.push(sorted_samples[idx]);
        }
        iso_values.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        let mut field_grid = vec![0.0f32; nx * ny];
        field_grid
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, val)| {
                let ix = i % nx;
                let iy = i / nx;
                let x = min.x + ix as f32 * grid_spacing;
                let y = min.y + iy as f32 * grid_spacing;
                let pos = Vec2::new(x, y);
                *val = compute_potential_at_point(&self.bodies, pos, &self.sim_config);
            });

        for &iso in &iso_values {
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
}

/// Compute the electric field at a point.
pub fn compute_field_at_point(
    bodies: &[Body],
    pos: Vec2,
    config: &crate::config::SimConfig,
) -> Vec2 {
    let mut field = Vec2::zero();

    if config.isoline_field_mode != crate::config::IsolineFieldMode::ExternalOnly {
        for body in bodies {
            let r = pos - body.pos;
            let dist2 = r.mag_sq().max(1e-4);
            let e = body.charge * r / (dist2 * dist2.sqrt());
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
            potential += config.coulomb_constant * body.charge / dist;
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
