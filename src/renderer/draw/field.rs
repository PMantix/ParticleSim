use crate::body::Body;
use crate::renderer::state::{FIELD_DIRECTION, FIELD_MAGNITUDE};
use crate::renderer::Renderer;
use rayon::prelude::*;
use ultraviolet::Vec2;

impl Renderer {
    /// Draw electric field isolines using a simple marching squares algorithm.
    pub fn draw_field_isolines(&mut self, ctx: &mut quarkstrom::RenderContext) {
        // Use full current viewport and aspect-aware bounds
        let half_view = Vec2::new(
            self.scale * (self.window_width as f32 / self.window_height as f32),
            self.scale,
        );
        let min = self.pos - half_view;
        let max = self.pos + half_view;

        // Fidelity: aim for target samples across the short side
        let target = self.sim_config.isoline_target_samples.max(10) as f32;
        let short_side = (max.x - min.x).min(max.y - min.y);
        let mut grid_spacing = short_side / target;
        grid_spacing = grid_spacing.clamp(2.0, 100.0);

        let nx = ((max.x - min.x) / grid_spacing).ceil() as usize + 1;
        let ny = ((max.y - min.y) / grid_spacing).ceil() as usize + 1;
        let stride_x = 1usize;
        let stride_y = 1usize;

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

        let num_levels = self.sim_config.isoline_count.max(3);
        let mut sorted_samples = samples.clone();
        // Filter out NaN and infinite values before sorting
        sorted_samples.retain(|x| x.is_finite());
        if sorted_samples.is_empty() {
            return; // No valid samples
        }
        sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Apply percentile clipping and bias + nonlinear distribution gamma
        let clip = self.sim_config.isoline_clip_margin.clamp(0.0, 0.49);
        let bias = self.sim_config.isoline_bias.clamp(-0.5, 0.5);
        let span = (1.0 - 2.0 * clip).max(1e-6);
        let dist_gamma = self.sim_config.isoline_distribution_gamma.max(0.001);

        let mut iso_values = Vec::with_capacity(num_levels);
        for i in 0..num_levels {
            let base_p = i as f32 / (num_levels - 1) as f32; // 0..1
                                                             // Nonlinear warp: p' = p^gamma (gamma>1 pushes toward 0; invert halves symmetrically around 0.5 by blending)
            let warped = if base_p <= 0.5 {
                0.5 * (2.0 * base_p).powf(dist_gamma)
            } else {
                1.0 - 0.5 * (2.0 * (1.0 - base_p)).powf(dist_gamma)
            };
            // Clip extremes, then apply bias around center
            let clipped = clip + span * warped;
            let biased = ((clipped - 0.5) + bias + 0.5).clamp(clip, 1.0 - clip);
            let idx = ((sorted_samples.len() - 1) as f32 * biased).round() as usize;
            iso_values.push(sorted_samples[idx]);
        }
        iso_values.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        let mut field_grid = vec![0.0f32; nx * ny];
        field_grid.par_iter_mut().enumerate().for_each(|(i, val)| {
            let ix = i % nx;
            let iy = i / nx;
            let x = min.x + ix as f32 * grid_spacing;
            let y = min.y + iy as f32 * grid_spacing;
            let pos = Vec2::new(x, y);
            *val = compute_potential_at_point(&self.bodies, pos, &self.sim_config);
        });

        // Optionally draw filled isobands first (between consecutive iso values)
        if self.sim_config.isoline_filled && iso_values.len() >= 2 {
            let alpha = self.sim_config.isoline_fill_alpha;
            for band in 0..iso_values.len() - 1 {
                let v0 = iso_values[band];
                let v1 = iso_values[band + 1];
                let mid = 0.5 * (v0 + v1);
                let mut color = map_iso_color(
                    mid,
                    abs_max,
                    self.sim_config.isoline_color_strength,
                    self.sim_config.isoline_color_gamma,
                );
                color[3] = alpha;
                // Simple rect fill per grid cell if band spans it
                for ix in 0..nx - 1 {
                    for iy in 0..ny - 1 {
                        let i00 = iy * nx + ix;
                        let i10 = iy * nx + ix + 1;
                        let i01 = (iy + 1) * nx + ix;
                        let i11 = (iy + 1) * nx + ix + 1;
                        let vmin = field_grid[i00]
                            .min(field_grid[i10])
                            .min(field_grid[i01])
                            .min(field_grid[i11]);
                        let vmax = field_grid[i00]
                            .max(field_grid[i10])
                            .max(field_grid[i01])
                            .max(field_grid[i11]);
                        if vmin <= v1 && vmax >= v0 {
                            // cell intersects band
                            let rect_min = Vec2::new(
                                min.x + ix as f32 * grid_spacing,
                                min.y + iy as f32 * grid_spacing,
                            );
                            let rect_max = rect_min + Vec2::new(grid_spacing, grid_spacing);
                            ctx.draw_rect(rect_min, rect_max, color);
                        }
                    }
                }
            }
        }

        for &iso in &iso_values {
            let mut color = map_iso_color(
                iso,
                abs_max,
                self.sim_config.isoline_color_strength,
                self.sim_config.isoline_color_gamma,
            );
            color[3] = 255;

            // Optionally refine a band of iso levels
            let level_t = if num_levels > 1 {
                iso_values.iter().position(|v| *v == iso).unwrap_or(0) as f32
                    / (num_levels - 1) as f32
            } else {
                0.0
            };
            let refine_enabled = self.sim_config.isoline_local_refine
                && self.sim_config.isoline_local_refine_factor > 1
                && (level_t - 0.5).abs()
                    <= (self.sim_config.isoline_local_refine_band.clamp(0.1, 1.0) * 0.5);

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

                    let p00 = Vec2::new(
                        min.x + ix as f32 * grid_spacing,
                        min.y + iy as f32 * grid_spacing,
                    );
                    let p10 = Vec2::new(
                        min.x + (ix + 1) as f32 * grid_spacing,
                        min.y + iy as f32 * grid_spacing,
                    );
                    let p01 = Vec2::new(
                        min.x + ix as f32 * grid_spacing,
                        min.y + (iy + 1) as f32 * grid_spacing,
                    );
                    let p11 = Vec2::new(
                        min.x + (ix + 1) as f32 * grid_spacing,
                        min.y + (iy + 1) as f32 * grid_spacing,
                    );

                    // If refinement enabled and the coarse cell crosses this iso, render refined sub-cells
                    let coarse_crosses = (v00 - iso) * (v10 - iso) < 0.0
                        || (v10 - iso) * (v11 - iso) < 0.0
                        || (v11 - iso) * (v01 - iso) < 0.0
                        || (v01 - iso) * (v00 - iso) < 0.0;

                    if refine_enabled && coarse_crosses {
                        let n = self.sim_config.isoline_local_refine_factor.max(2);
                        let inv_n = 1.0 / n as f32;
                        for sx in 0..n {
                            for sy in 0..n {
                                let fx0 = sx as f32 * inv_n;
                                let fx1 = (sx as f32 + 1.0) * inv_n;
                                let fy0 = sy as f32 * inv_n;
                                let fy1 = (sy as f32 + 1.0) * inv_n;

                                let pv00 = bilerp(v00, v10, v01, v11, fx0, fy0);
                                let pv10 = bilerp(v00, v10, v01, v11, fx1, fy0);
                                let pv01 = bilerp(v00, v10, v01, v11, fx0, fy1);
                                let pv11 = bilerp(v00, v10, v01, v11, fx1, fy1);

                                let pp00 = Vec2::new(
                                    p00.x + (p10.x - p00.x) * fx0 + (p01.x - p00.x) * fy0,
                                    p00.y + (p10.y - p00.y) * fx0 + (p01.y - p00.y) * fy0,
                                );
                                let pp10 = Vec2::new(
                                    p00.x + (p10.x - p00.x) * fx1 + (p01.x - p00.x) * fy0,
                                    p00.y + (p10.y - p00.y) * fx1 + (p01.y - p00.y) * fy0,
                                );
                                let pp01 = Vec2::new(
                                    p00.x + (p10.x - p00.x) * fx0 + (p01.x - p00.x) * fy1,
                                    p00.y + (p10.y - p00.y) * fx0 + (p01.y - p00.y) * fy1,
                                );
                                let pp11 = Vec2::new(
                                    p00.x + (p10.x - p00.x) * fx1 + (p01.x - p00.x) * fy1,
                                    p00.y + (p10.y - p00.y) * fx1 + (p01.y - p00.y) * fy1,
                                );

                                let mut sub_pts = Vec::new();
                                if (pv00 - iso) * (pv10 - iso) < 0.0 {
                                    let t = (iso - pv00) / (pv10 - pv00);
                                    sub_pts.push(lerp(pp00, pp10, t));
                                }
                                if (pv10 - iso) * (pv11 - iso) < 0.0 {
                                    let t = (iso - pv10) / (pv11 - pv10);
                                    sub_pts.push(lerp(pp10, pp11, t));
                                }
                                if (pv11 - iso) * (pv01 - iso) < 0.0 {
                                    let t = (iso - pv11) / (pv01 - pv11);
                                    sub_pts.push(lerp(pp11, pp01, t));
                                }
                                if (pv01 - iso) * (pv00 - iso) < 0.0 {
                                    let t = (iso - pv01) / (pv00 - pv01);
                                    sub_pts.push(lerp(pp01, pp00, t));
                                }
                                if sub_pts.len() == 2 {
                                    ctx.draw_line(sub_pts[0], sub_pts[1], color);
                                }
                            }
                        }
                    } else {
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

fn bilerp(v00: f32, v10: f32, v01: f32, v11: f32, fx: f32, fy: f32) -> f32 {
    // Bilinear interpolation within a unit square
    let a = v00 + (v10 - v00) * fx;
    let b = v01 + (v11 - v01) * fx;
    a + (b - a) * fy
}

fn map_iso_color(value: f32, abs_max: f32, strength: f32, gamma: f32) -> [u8; 4] {
    let mut t = (value / abs_max).clamp(-1.0, 1.0);
    // Apply gamma to magnitude while preserving sign
    let sign = if t < 0.0 {
        -1.0
    } else if t > 0.0 {
        1.0
    } else {
        0.0
    };
    t = sign * t.abs().powf(gamma.max(0.01));
    // Blend between white and color by strength
    let (r, g, b) = if t < 0.0 {
        let f = t.abs();
        let cr = (255.0 * (1.0 - f) + 0.0 * f) as u8;
        let cg = (255.0 * (1.0 - f) + 128.0 * f) as u8;
        let cb = 255u8;
        (cr, cg, cb)
    } else if t > 0.0 {
        let f = t;
        let cr = 255u8;
        let cg = (255.0 * (1.0 - f) + 64.0 * f) as u8;
        let cb = (255.0 * (1.0 - f) + 64.0 * f) as u8;
        (cr, cg, cb)
    } else {
        (255, 255, 255)
    };
    let s = strength.clamp(0.0, 1.0);
    let r = ((1.0 - s) * 255.0 + s * r as f32) as u8;
    let g = ((1.0 - s) * 255.0 + s * g as f32) as u8;
    let b = ((1.0 - s) * 255.0 + s * b as f32) as u8;
    [r, g, b, 255]
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
            // Skip bodies with invalid positions or charges
            if !body.pos.x.is_finite() || !body.pos.y.is_finite() || !body.charge.is_finite() {
                continue;
            }
            let r = pos - body.pos;
            let dist = r.mag().max(1e-4);
            let contribution = config.coulomb_constant * body.charge / dist;
            if contribution.is_finite() {
                potential += contribution;
            }
        }
    }

    if config.isoline_field_mode != crate::config::IsolineFieldMode::BodyOnly {
        let mag = *FIELD_MAGNITUDE.lock();
        let theta = (*FIELD_DIRECTION.lock()).to_radians();
        let background = Vec2::new(theta.cos(), theta.sin()) * mag;
        let external_contribution = -background.dot(pos);
        if external_contribution.is_finite() {
            potential += external_contribution;
        }
    }

    // Ensure we return a finite value
    if potential.is_finite() {
        potential
    } else {
        0.0
    }
}
