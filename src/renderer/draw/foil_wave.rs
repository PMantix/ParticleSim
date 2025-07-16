use crate::renderer::Renderer;
use ultraviolet::Vec2;

impl Renderer {
    /// Update history of selected foil on/off states.
    pub fn update_foil_wave_history(&mut self) {
        if self.selected_foil_ids.is_empty() {
            return;
        }

        let is_paused = crate::renderer::state::PAUSED.load(std::sync::atomic::Ordering::Relaxed);
        if is_paused {
            return;
        }

        let time = *crate::renderer::state::SIM_TIME.lock();
        for id in &self.selected_foil_ids {
            if let Some(foil) = self.foils.iter().find(|f| f.id == *id) {
                let effective_current = if foil.switch_hz > 0.0 {
                    let ac_component = if (time * foil.switch_hz) % 1.0 < 0.5 {
                        foil.ac_current
                    } else {
                        -foil.ac_current
                    };
                    foil.dc_current + ac_component
                } else {
                    foil.dc_current
                };

                let state = if effective_current.abs() > f32::EPSILON {
                    effective_current.signum()
                } else {
                    0.0
                };

                let entry = self.foil_wave_history.entry(*id).or_insert_with(Vec::new);
                if let Some(&(_, last)) = entry.last() {
                    if (last - state).abs() > f32::EPSILON {
                        entry.push((time, last));
                        entry.push((time, state));
                    }
                }
                entry.push((time, state));

                if entry.len() > 2000 {
                    let excess = entry.len() - 2000;
                    entry.drain(0..excess);
                }
            }
        }
        self.foil_wave_history.retain(|id, _| self.selected_foil_ids.contains(id));
    }

    /// Draw square-wave lines for selected foils using stored history.
    pub fn draw_foil_square_waves(&self, ctx: &mut quarkstrom::RenderContext) {
        if self.selected_foil_ids.is_empty() {
            return;
        }

        let current_time = *crate::renderer::state::SIM_TIME.lock();
        let max_time = 10.0;
        let start_time = current_time - max_time;

        let amplitude = self.scale * 0.05;
        let spacing = amplitude * 2.0;
        let base_x = self.pos.x - self.scale;
        let base_y = self.pos.y - self.scale + spacing;
        let x_scale = (2.0 * self.scale) / max_time;

        for (idx, id) in self.selected_foil_ids.iter().enumerate() {
            if let Some(history) = self.foil_wave_history.get(id) {
                let y_base = base_y + idx as f32 * spacing;
                let mut prev: Option<(f32, f32)> = None;
                for &(t, state) in history {
                    if t < start_time { continue; }
                    if let Some((pt, pv)) = prev {
                        let x0 = base_x + (pt - start_time) * x_scale;
                        let x1 = base_x + (t - start_time) * x_scale;
                        ctx.draw_line(
                            Vec2::new(x0, y_base + pv * amplitude),
                            Vec2::new(x1, y_base + pv * amplitude),
                            [255, 255, 255, 255],
                        );
                    }
                    prev = Some((t, state));
                }

                if let Some((pt, pv)) = prev {
                    let x0 = base_x + (pt - start_time) * x_scale;
                    let x1 = base_x + (current_time - start_time) * x_scale;
                    ctx.draw_line(
                        Vec2::new(x0, y_base + pv * amplitude),
                        Vec2::new(x1, y_base + pv * amplitude),
                        [255, 255, 255, 255],
                    );
                } else if let Some(&(_, last)) = history.last() {
                    let x0 = base_x;
                    let x1 = base_x + (current_time - start_time) * x_scale;
                    ctx.draw_line(
                        Vec2::new(x0, y_base + last * amplitude),
                        Vec2::new(x1, y_base + last * amplitude),
                        [255, 255, 255, 255],
                    );
                }
            }
        }
    }
}
