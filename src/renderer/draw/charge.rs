use super::super::state::*;
use ultraviolet::Vec2;
use rayon::prelude::*;

impl super::Renderer {
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
        let max_abs = samples
            .par_iter_mut()
            .enumerate()
            .map(|(i, sample)| {
                let ix = i % nx;
                let iy = i / nx;
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
                *sample = density;
                density.abs()
            })
            .reduce(|| 0.0f32, f32::max);

        let max_abs = max_abs.max(1e-6);

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
                    min.x + ix as f32 * grid_spacing,
                    min.y + iy as f32 * grid_spacing,
                );
                let rect_max = rect_min + Vec2::new(grid_spacing, grid_spacing);
                ctx.draw_rect(rect_min, rect_max, color);
            }
        }
    }
}
