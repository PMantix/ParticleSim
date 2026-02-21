// renderer/gui/eis_tab.rs
// EIS configuration, Nyquist plot, signal time series, and CSV export

use crate::renderer::state::{SimCommand, SIM_COMMAND_SENDER};
use crate::simulation::eis::EIS_RESULTS;
use quarkstrom::egui;

impl super::super::Renderer {
    pub fn show_eis_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Electrochemical Impedance Spectroscopy");
        ui.separator();

        let shared = EIS_RESULTS.lock().clone();

        // Configuration panel
        egui::CollapsingHeader::new("Configuration")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Amplitude (e/fs):");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_amplitude)
                            .speed(0.0001)
                            .clamp_range(1e-8..=1.0)
                            .max_decimals(6),
                    );
                });
                let dt = *crate::renderer::state::TIMESTEP.lock();

                // Helper: convert freq → steps and clamp to a safe range
                let freq_to_steps = |f: f32| -> u64 {
                    if f > 0.0 && dt > 0.0 {
                        ((1.0 / (f * dt)).round() as u64).max(1)
                    } else {
                        1
                    }
                };

                // f_min — editable as frequency OR as steps/period
                ui.horizontal(|ui| {
                    ui.label("f_min:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_f_min)
                            .speed(1e-7)
                            .clamp_range(1e-10..=1.0)
                            .max_decimals(8)
                            .suffix(" 1/fs"),
                    );
                    ui.label("=");
                    let mut steps = freq_to_steps(self.eis_f_min);
                    let prev = steps;
                    ui.add(egui::DragValue::new(&mut steps).speed(10).clamp_range(1..=10_000_000u64).suffix(" steps"));
                    if steps != prev && steps > 0 && dt > 0.0 {
                        self.eis_f_min = (1.0 / (steps as f32 * dt)).clamp(1e-10, 1.0);
                    }
                });

                // f_max — editable as frequency OR as steps/period
                ui.horizontal(|ui| {
                    ui.label("f_max:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_f_max)
                            .speed(1e-4)
                            .clamp_range(1e-8..=1.0)
                            .max_decimals(6)
                            .suffix(" 1/fs"),
                    );
                    ui.label("=");
                    let mut steps = freq_to_steps(self.eis_f_max);
                    let prev = steps;
                    ui.add(egui::DragValue::new(&mut steps).speed(10).clamp_range(1..=10_000_000u64).suffix(" steps"));
                    if steps != prev && steps > 0 && dt > 0.0 {
                        self.eis_f_max = (1.0 / (steps as f32 * dt)).clamp(1e-8, 1.0);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Points per decade:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_points_per_decade)
                            .speed(0.5)
                            .clamp_range(1.0..=20.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Recording periods:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_periods_per_freq)
                            .speed(1)
                            .clamp_range(1..=50),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Settle periods:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_settle_periods)
                            .speed(1)
                            .clamp_range(0..=20),
                    );
                });

                // Show estimated time before starting
                {
                    let freqs = crate::simulation::eis::EisConfig::log_spaced_frequencies(
                        self.eis_f_min,
                        self.eis_f_max,
                        self.eis_points_per_decade,
                    );
                    let est_cfg = crate::simulation::eis::EisConfig {
                        amplitude: self.eis_amplitude,
                        frequencies: freqs,
                        periods_per_freq: self.eis_periods_per_freq,
                        settle_periods: self.eis_settle_periods,
                    };
                    let est_fs = est_cfg.estimated_total_fs();
                    let n_freqs = est_cfg.frequencies.len();
                    ui.label(format!(
                        "Sweep: {} frequencies, est. {:.0} fs ({:.1} ps)",
                        n_freqs, est_fs, est_fs / 1000.0
                    ));
                }

                // Show foil group assignments
                if !shared.group_a_ids.is_empty() || !shared.group_b_ids.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Group A (+):");
                        ui.label(
                            shared
                                .group_a_ids
                                .iter()
                                .map(|id| format!("Foil {}", id))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Group B (-):");
                        ui.label(
                            shared
                                .group_b_ids
                                .iter()
                                .map(|id| format!("Foil {}", id))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    });
                } else if !shared.is_running {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "No foil groups assigned. Set groups in the Charging tab first.",
                    );
                }

                ui.separator();
                if shared.is_running {
                    ui.horizontal(|ui| {
                        if ui.button("Stop EIS").clicked() {
                            if let Some(tx) = SIM_COMMAND_SENDER.lock().as_ref() {
                                let _ = tx.send(SimCommand::StopEIS);
                            }
                        }
                        ui.label(format!(
                            "Freq {}/{} ({:.2e} 1/fs)",
                            shared.current_freq_idx + 1,
                            shared.total_frequencies,
                            shared.current_freq,
                        ));
                    });
                    ui.label(format!("Phase: {}", shared.phase));
                    ui.label(format!(
                        "Elapsed: {:.0} / {:.0} fs ({:.1}%)",
                        shared.elapsed_fs,
                        shared.needed_fs,
                        if shared.needed_fs > 0.0 {
                            100.0 * shared.elapsed_fs / shared.needed_fs
                        } else {
                            0.0
                        }
                    ));
                    ui.label(format!("Samples: {}", shared.sample_count));
                    // Progress bar
                    let frac = if shared.needed_fs > 0.0 {
                        (shared.elapsed_fs / shared.needed_fs).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(frac).show_percentage());
                } else {
                    if ui.button("Start EIS Sweep").clicked() {
                        if let Some(tx) = SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = tx.send(SimCommand::StartEIS {
                                amplitude: self.eis_amplitude,
                                f_min: self.eis_f_min,
                                f_max: self.eis_f_max,
                                points_per_decade: self.eis_points_per_decade,
                                periods_per_freq: self.eis_periods_per_freq,
                                settle_periods: self.eis_settle_periods,
                            });
                        }
                    }
                    if !shared.points.is_empty() {
                        ui.label(format!("{} points collected", shared.points.len()));
                    }
                }
            });

        ui.separator();

        // Signal Time Series — visible while running or once data has been collected
        if shared.is_running || !shared.ts_t_rel.is_empty() {
            egui::CollapsingHeader::new("Signal Time Series")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.eis_show_fit, "Show best-fit sinusoid");
                    self.draw_timeseries_plot(ui, &shared);
                });
        }

        if shared.points.is_empty() {
            if !shared.is_running {
                ui.label("No EIS data yet. Configure and start a sweep.");
            }
            return;
        }

        // Nyquist plot: -Im(Z) vs Re(Z)
        egui::CollapsingHeader::new("Nyquist Plot")
            .default_open(true)
            .show(ui, |ui| {
                self.draw_nyquist_plot(ui, &shared.points);
            });

        // Data table
        egui::CollapsingHeader::new("Data Table")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("eis_data_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Freq (1/fs)");
                        ui.label("Re(Z)");
                        ui.label("-Im(Z)");
                        ui.label("|Z|");
                        ui.label("Phase (deg)");
                        ui.label("R²");
                        ui.end_row();

                        for pt in &shared.points {
                            ui.label(format!("{:.2e}", pt.frequency));
                            ui.label(format!("{:.4e}", pt.z_real));
                            ui.label(format!("{:.4e}", -pt.z_imag));
                            ui.label(format!("{:.4e}", pt.magnitude));
                            ui.label(format!("{:.2}", pt.phase_deg));
                            ui.label(format!("{:.4}", pt.fit_r2));
                            ui.end_row();
                        }
                    });
            });

        // Export button
        ui.separator();
        if ui.button("Export CSV").clicked() {
            self.export_eis_csv(&shared.points);
        }
    }

    fn draw_nyquist_plot(
        &self,
        ui: &mut egui::Ui,
        points: &[crate::simulation::eis::EisPoint],
    ) {
        const LEFT_MARGIN: f32 = 62.0;
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - LEFT_MARGIN - 10.0, 300.0);

        // Indent right so y-axis tick labels have room on the left
        let (rect, response) = ui
            .horizontal(|ui| {
                ui.add_space(LEFT_MARGIN);
                ui.allocate_exact_size(plot_size, egui::Sense::hover())
            })
            .inner;

        if !ui.is_rect_visible(rect) || points.is_empty() {
            return;
        }

        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(240));
        ui.painter()
            .rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

        // Compute ranges for Re(Z) and -Im(Z)
        let re_vals: Vec<f64> = points.iter().map(|p| p.z_real).collect();
        let neg_im_vals: Vec<f64> = points.iter().map(|p| -p.z_imag).collect();

        let re_min = re_vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let re_max = re_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let im_min = neg_im_vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let im_max = neg_im_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let re_range = (re_max - re_min).max(1e-20);
        let im_range = (im_max - im_min).max(1e-20);
        let re_pad = re_range * 0.1;
        let im_pad = im_range * 0.1;
        let x_min = re_min - re_pad;
        let x_max = re_max + re_pad;
        let y_min = im_min - im_pad;
        let y_max = im_max + im_pad;

        let data_color = egui::Color32::from_rgb(0, 100, 255);

        // Draw data points and lines
        let mut screen_pts = Vec::new();
        for (re, neg_im) in re_vals.iter().zip(neg_im_vals.iter()) {
            let x_norm = (re - x_min) / (x_max - x_min);
            let y_norm = 1.0 - (neg_im - y_min) / (y_max - y_min);
            let sx = rect.min.x + x_norm as f32 * rect.width();
            let sy = rect.min.y + y_norm as f32 * rect.height();
            screen_pts.push(egui::Pos2::new(sx, sy));
        }

        for i in 0..screen_pts.len().saturating_sub(1) {
            ui.painter().line_segment(
                [screen_pts[i], screen_pts[i + 1]],
                egui::Stroke::new(1.5, data_color),
            );
        }
        for pt in &screen_pts {
            ui.painter().circle_filled(*pt, 3.0, data_color);
        }

        // Axis labels — use theme text color so they're readable in dark mode
        let label_color = ui.visuals().text_color();
        let font = egui::FontId::proportional(12.0);
        ui.painter().text(
            egui::Pos2::new(rect.center().x, rect.max.y + 15.0),
            egui::Align2::CENTER_TOP,
            "Re(Z)",
            font.clone(),
            label_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.min.x - 32.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            "-Im(Z)",
            font,
            label_color,
        );

        // Tick labels
        for i in 0..=4 {
            let frac = i as f32 / 4.0;
            let x_val = x_min + (x_max - x_min) * frac as f64;
            let y_val = y_min + (y_max - y_min) * (1.0 - frac as f64);
            ui.painter().text(
                egui::Pos2::new(rect.min.x + frac * rect.width(), rect.max.y + 3.0),
                egui::Align2::CENTER_TOP,
                format!("{:.2e}", x_val),
                egui::FontId::proportional(9.0),
                label_color,
            );
            ui.painter().text(
                egui::Pos2::new(rect.min.x - 4.0, rect.min.y + frac * rect.height()),
                egui::Align2::RIGHT_CENTER,
                format!("{:.2e}", y_val),
                egui::FontId::proportional(9.0),
                label_color,
            );
        }

        // Reserve vertical space for the x-axis label below the plot
        ui.add_space(25.0);

        // Hover tooltip: highlight nearest point within 30 px and show values
        let hover_pos = if response.hovered() {
            ui.ctx().pointer_hover_pos()
        } else {
            None
        };
        if let Some(hover_pos) = hover_pos {
            if let Some((idx, dist_sq)) = screen_pts
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    (
                        i,
                        (p.x - hover_pos.x).powi(2) + (p.y - hover_pos.y).powi(2),
                    )
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            {
                if dist_sq < 30.0f32.powi(2) {
                    let pt = &points[idx];
                    // Ring around the hovered point
                    ui.painter().circle_stroke(
                        screen_pts[idx],
                        7.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );
                    // Popup box slightly to the right of the cursor
                    let lines = [
                        format!("f = {:.3e} 1/fs", pt.frequency),
                        format!("Re(Z)  = {:.4e}", pt.z_real),
                        format!("-Im(Z) = {:.4e}", -pt.z_imag),
                        format!("R\u{00B2}     = {:.4}", pt.fit_r2),
                    ];
                    let tip_font = egui::FontId::proportional(11.0);
                    let bg = egui::Color32::from_rgba_unmultiplied(25, 25, 25, 220);
                    let text_color = egui::Color32::from_gray(230);
                    let origin = hover_pos + egui::Vec2::new(14.0, -10.0);
                    let line_h = 16.0;
                    let pad = 5.0;
                    let box_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(origin.x - pad, origin.y - pad),
                        egui::Vec2::new(178.0, lines.len() as f32 * line_h + 2.0 * pad),
                    );
                    ui.painter().rect_filled(box_rect, 4.0, bg);
                    ui.painter().rect_stroke(
                        box_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                    );
                    for (i, line) in lines.iter().enumerate() {
                        ui.painter().text(
                            egui::Pos2::new(origin.x, origin.y + i as f32 * line_h),
                            egui::Align2::LEFT_TOP,
                            line,
                            tip_font.clone(),
                            text_color,
                        );
                    }
                }
            }
        }
    }

    fn draw_timeseries_plot(
        &self,
        ui: &mut egui::Ui,
        shared: &crate::simulation::eis::EisSharedState,
    ) {
        const LEFT_MARGIN: f32 = 62.0;
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - LEFT_MARGIN - 10.0, 280.0);

        let (rect, _response) = ui
            .horizontal(|ui| {
                ui.add_space(LEFT_MARGIN);
                ui.allocate_exact_size(plot_size, egui::Sense::hover())
            })
            .inner;

        if !ui.is_rect_visible(rect) {
            return;
        }

        // Background
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(30));
        ui.painter()
            .rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));

        let ts = &shared.ts_t_rel;
        let vs = &shared.ts_v;
        let is = &shared.ts_i;
        let phases = &shared.ts_is_recording;

        if ts.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Waiting for data...",
                egui::FontId::proportional(13.0),
                egui::Color32::from_gray(160),
            );
            return;
        }

        let t_min = ts.iter().cloned().fold(f32::INFINITY, f32::min);
        let t_max = ts.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let t_range = (t_max - t_min).max(1e-20);

        // Settle/record shading: find where recording starts
        let first_record_t = ts
            .iter()
            .zip(phases.iter())
            .find(|(_, &rec)| rec)
            .map(|(&t, _)| t);

        if let Some(rec_t) = first_record_t {
            let x_split = rect.min.x + ((rec_t - t_min) / t_range) as f32 * rect.width();
            let settle_rect = egui::Rect::from_min_max(
                rect.min,
                egui::Pos2::new(x_split.min(rect.max.x), rect.max.y),
            );
            let record_rect = egui::Rect::from_min_max(
                egui::Pos2::new(x_split.max(rect.min.x), rect.min.y),
                rect.max,
            );
            ui.painter()
                .rect_filled(settle_rect, 0.0, egui::Color32::from_rgba_unmultiplied(80, 60, 20, 60));
            ui.painter()
                .rect_filled(record_rect, 0.0, egui::Color32::from_rgba_unmultiplied(20, 60, 80, 60));
            // Phase labels
            let font_small = egui::FontId::proportional(10.0);
            if settle_rect.width() > 30.0 {
                ui.painter().text(
                    egui::Pos2::new(settle_rect.min.x + 4.0, settle_rect.min.y + 4.0),
                    egui::Align2::LEFT_TOP,
                    "settling",
                    font_small.clone(),
                    egui::Color32::from_rgb(200, 160, 80),
                );
            }
            if record_rect.width() > 40.0 {
                ui.painter().text(
                    egui::Pos2::new(record_rect.min.x + 4.0, record_rect.min.y + 4.0),
                    egui::Align2::LEFT_TOP,
                    "recording",
                    font_small,
                    egui::Color32::from_rgb(80, 180, 220),
                );
            }
        }

        let v_color = egui::Color32::from_rgb(80, 200, 120);   // green for voltage
        let i_color = egui::Color32::from_rgb(220, 120, 60);   // orange for current
        let label_color = egui::Color32::from_gray(200);
        let font = egui::FontId::proportional(11.0);

        // Helper: compute normalised y-range for a slice, return (min, max)
        let data_range = |vals: &[f32]| -> (f32, f32) {
            let lo = vals.iter().cloned().fold(f32::INFINITY, f32::min);
            let hi = vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let span = (hi - lo).max(1e-30);
            let pad = span * 0.15;
            (lo - pad, hi + pad)
        };

        // Helper: map a (t, val) into screen space given a y range
        let to_screen = |t: f32, val: f32, y_lo: f32, y_hi: f32| -> egui::Pos2 {
            let x = rect.min.x + ((t - t_min) / t_range) * rect.width();
            let y = rect.max.y - ((val - y_lo) / (y_hi - y_lo)) * rect.height();
            egui::Pos2::new(x, y)
        };

        // Draw voltage trace
        let (v_lo, v_hi) = data_range(vs);
        {
            let pts: Vec<egui::Pos2> = ts
                .iter()
                .zip(vs.iter())
                .map(|(&t, &v)| to_screen(t, v, v_lo, v_hi))
                .collect();
            for w in pts.windows(2) {
                ui.painter()
                    .line_segment([w[0], w[1]], egui::Stroke::new(1.5, v_color));
            }
        }

        // Draw best-fit sinusoid overlay (recording region only)
        if self.eis_show_fit && (shared.fit_v_re != 0.0 || shared.fit_v_im != 0.0) {
            if let Some(rec_start) = first_record_t {
                let omega = 2.0 * std::f32::consts::PI * shared.current_freq;
                let fit_color = egui::Color32::from_rgb(255, 230, 80); // bright yellow
                // Generate a smooth curve over the recording region
                let n_fit = 300usize;
                let t_rec_end = t_max;
                let fit_pts: Vec<egui::Pos2> = (0..=n_fit)
                    .map(|i| {
                        let t = rec_start + (t_rec_end - rec_start) * i as f32 / n_fit as f32;
                        let t_fit = t - rec_start;
                        // Add DC offset so the fit rides on top of the actual signal
                        let v = shared.fit_v_dc
                            + (shared.fit_v_re * (omega * t_fit).cos() as f64
                                + shared.fit_v_im * (omega * t_fit).sin() as f64)
                                as f32;
                        to_screen(t, v, v_lo, v_hi)
                    })
                    .collect();
                // Clip the fit curve strictly to the plot rect so early/wild
                // estimates don't bleed outside the figure bounds.
                let fit_painter = ui.painter().with_clip_rect(rect);
                for w in fit_pts.windows(2) {
                    fit_painter
                        .line_segment([w[0], w[1]], egui::Stroke::new(1.5, fit_color));
                }
            }
        }

        // Draw current trace — normalised independently so it fills the plot height
        let (i_lo, i_hi) = data_range(is);
        {
            let pts: Vec<egui::Pos2> = ts
                .iter()
                .zip(is.iter())
                .map(|(&t, &i)| to_screen(t, i, i_lo, i_hi))
                .collect();
            for w in pts.windows(2) {
                ui.painter()
                    .line_segment([w[0], w[1]], egui::Stroke::new(1.5, i_color));
            }
        }

        // Legend
        ui.painter().text(
            egui::Pos2::new(rect.max.x - 8.0, rect.min.y + 6.0),
            egui::Align2::RIGHT_TOP,
            format!("V  [{:.2e} … {:.2e}]", v_lo, v_hi),
            font.clone(),
            v_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.max.x - 8.0, rect.min.y + 20.0),
            egui::Align2::RIGHT_TOP,
            format!("I  [{:.2e} … {:.2e}]", i_lo, i_hi),
            font.clone(),
            i_color,
        );

        // X-axis label
        ui.painter().text(
            egui::Pos2::new(rect.center().x, rect.max.y + 14.0),
            egui::Align2::CENTER_TOP,
            format!(
                "t (fs)   freq = {:.2e} 1/fs   {} pts",
                shared.current_freq,
                ts.len()
            ),
            egui::FontId::proportional(10.0),
            label_color,
        );

        // Y-axis min/max tick labels (V left, I right)
        let font_tick = egui::FontId::proportional(9.0);
        ui.painter().text(
            egui::Pos2::new(rect.min.x - 2.0, rect.min.y),
            egui::Align2::RIGHT_TOP,
            format!("{:.1e}", v_hi),
            font_tick.clone(),
            v_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.min.x - 2.0, rect.max.y),
            egui::Align2::RIGHT_BOTTOM,
            format!("{:.1e}", v_lo),
            font_tick.clone(),
            v_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.max.x + 2.0, rect.min.y),
            egui::Align2::LEFT_TOP,
            format!("{:.1e}", i_hi),
            font_tick.clone(),
            i_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.max.x + 2.0, rect.max.y),
            egui::Align2::LEFT_BOTTOM,
            format!("{:.1e}", i_lo),
            font_tick,
            i_color,
        );

        // Reserve vertical space for the x-axis label below the plot
        ui.add_space(25.0);
    }

    fn export_eis_csv(&self, points: &[crate::simulation::eis::EisPoint]) {
        let mut csv = String::from("frequency,z_real,z_imag,z_magnitude,phase_deg,fit_r2\n");
        for pt in points {
            csv.push_str(&format!(
                "{:.6e},{:.6e},{:.6e},{:.6e},{:.4},{:.6}\n",
                pt.frequency, pt.z_real, pt.z_imag, pt.magnitude, pt.phase_deg, pt.fit_r2
            ));
        }
        let path = "eis_results.csv";
        match std::fs::write(path, &csv) {
            Ok(_) => println!("EIS data exported to {}", path),
            Err(e) => eprintln!("Failed to export EIS CSV: {}", e),
        }
    }
}
