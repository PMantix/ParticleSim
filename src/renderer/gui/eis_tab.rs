// renderer/gui/eis_tab.rs
// EIS configuration, Nyquist/Bode plots, and CSV export

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
                ui.horizontal(|ui| {
                    ui.label("f_min (1/fs):");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_f_min)
                            .speed(1e-7)
                            .clamp_range(1e-10..=1.0)
                            .max_decimals(8),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("f_max (1/fs):");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_f_max)
                            .speed(1e-4)
                            .clamp_range(1e-8..=1.0)
                            .max_decimals(6),
                    );
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

        if shared.points.is_empty() {
            ui.label("No EIS data yet. Configure and start a sweep.");
            return;
        }

        // Nyquist plot: -Im(Z) vs Re(Z)
        egui::CollapsingHeader::new("Nyquist Plot")
            .default_open(true)
            .show(ui, |ui| {
                self.draw_nyquist_plot(ui, &shared.points);
            });

        // Bode plot: |Z| and phase vs log(freq)
        egui::CollapsingHeader::new("Bode Plot")
            .default_open(false)
            .show(ui, |ui| {
                self.draw_bode_plot(ui, &shared.points);
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
                        ui.end_row();

                        for pt in &shared.points {
                            ui.label(format!("{:.2e}", pt.frequency));
                            ui.label(format!("{:.4e}", pt.z_real));
                            ui.label(format!("{:.4e}", -pt.z_imag));
                            ui.label(format!("{:.4e}", pt.magnitude));
                            ui.label(format!("{:.2}", pt.phase_deg));
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
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - 20.0, 250.0_f32.min(available.y - 50.0));
        let (rect, _response) = ui.allocate_exact_size(plot_size, egui::Sense::hover());

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

        // Axis labels
        let label_color = egui::Color32::BLACK;
        let font = egui::FontId::proportional(12.0);
        ui.painter().text(
            egui::Pos2::new(rect.center().x, rect.max.y + 15.0),
            egui::Align2::CENTER_TOP,
            "Re(Z)",
            font.clone(),
            label_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.min.x - 30.0, rect.center().y),
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
                egui::Pos2::new(rect.min.x - 3.0, rect.min.y + frac * rect.height()),
                egui::Align2::RIGHT_CENTER,
                format!("{:.2e}", y_val),
                egui::FontId::proportional(9.0),
                label_color,
            );
        }
    }

    fn draw_bode_plot(
        &self,
        ui: &mut egui::Ui,
        points: &[crate::simulation::eis::EisPoint],
    ) {
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - 20.0, 200.0_f32.min(available.y - 50.0));
        let (rect, _response) = ui.allocate_exact_size(plot_size, egui::Sense::hover());

        if !ui.is_rect_visible(rect) || points.is_empty() {
            return;
        }

        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(240));
        ui.painter()
            .rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

        // Use log10(freq) for x-axis, log10(|Z|) for left y-axis
        let log_freqs: Vec<f64> = points.iter().map(|p| (p.frequency as f64).log10()).collect();
        let log_mags: Vec<f64> = points.iter().map(|p| p.magnitude.log10()).collect();
        let phases: Vec<f64> = points.iter().map(|p| p.phase_deg).collect();

        let lf_min = log_freqs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let lf_max = log_freqs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lm_min = log_mags.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let lm_max = log_mags.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let x_range = (lf_max - lf_min).max(1e-10);
        let y_range = (lm_max - lm_min).max(1e-10);
        let x_pad = x_range * 0.05;
        let y_pad = y_range * 0.05;

        let mag_color = egui::Color32::from_rgb(0, 100, 255);
        let phase_color = egui::Color32::from_rgb(200, 50, 50);

        // Draw |Z| line (left y-axis)
        let mut mag_pts = Vec::new();
        for (&lf, &lm) in log_freqs.iter().zip(log_mags.iter()) {
            let x_norm = (lf - (lf_min - x_pad)) / (x_range + 2.0 * x_pad);
            let y_norm = 1.0 - (lm - (lm_min - y_pad)) / (y_range + 2.0 * y_pad);
            let sx = rect.min.x + x_norm as f32 * rect.width();
            let sy = rect.min.y + y_norm as f32 * rect.height();
            mag_pts.push(egui::Pos2::new(sx, sy));
        }
        for i in 0..mag_pts.len().saturating_sub(1) {
            ui.painter().line_segment(
                [mag_pts[i], mag_pts[i + 1]],
                egui::Stroke::new(2.0, mag_color),
            );
        }
        for pt in &mag_pts {
            ui.painter().circle_filled(*pt, 2.5, mag_color);
        }

        // Draw phase line (right y-axis, mapped to same rect)
        let ph_min = phases.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let ph_max = phases.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let ph_range = (ph_max - ph_min).max(1.0);
        let ph_pad = ph_range * 0.1;

        let mut phase_pts = Vec::new();
        for (&lf, &ph) in log_freqs.iter().zip(phases.iter()) {
            let x_norm = (lf - (lf_min - x_pad)) / (x_range + 2.0 * x_pad);
            let y_norm = 1.0 - (ph - (ph_min - ph_pad)) / (ph_range + 2.0 * ph_pad);
            let sx = rect.min.x + x_norm as f32 * rect.width();
            let sy = rect.min.y + y_norm as f32 * rect.height();
            phase_pts.push(egui::Pos2::new(sx, sy));
        }
        for i in 0..phase_pts.len().saturating_sub(1) {
            ui.painter().line_segment(
                [phase_pts[i], phase_pts[i + 1]],
                egui::Stroke::new(1.5, phase_color),
            );
        }
        for pt in &phase_pts {
            ui.painter().circle_filled(*pt, 2.5, phase_color);
        }

        // Legend
        let label_color = egui::Color32::BLACK;
        let font = egui::FontId::proportional(11.0);
        ui.painter().text(
            egui::Pos2::new(rect.min.x + 5.0, rect.min.y + 5.0),
            egui::Align2::LEFT_TOP,
            "|Z| (blue)",
            font.clone(),
            mag_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.min.x + 5.0, rect.min.y + 18.0),
            egui::Align2::LEFT_TOP,
            "Phase (red)",
            font.clone(),
            phase_color,
        );
        ui.painter().text(
            egui::Pos2::new(rect.center().x, rect.max.y + 15.0),
            egui::Align2::CENTER_TOP,
            "log10(Freq)",
            font,
            label_color,
        );
    }

    fn export_eis_csv(&self, points: &[crate::simulation::eis::EisPoint]) {
        let mut csv = String::from("frequency,z_real,z_imag,z_magnitude,phase_deg\n");
        for pt in points {
            csv.push_str(&format!(
                "{:.6e},{:.6e},{:.6e},{:.6e},{:.4}\n",
                pt.frequency, pt.z_real, pt.z_imag, pt.magnitude, pt.phase_deg
            ));
        }
        let path = "eis_results.csv";
        match std::fs::write(path, &csv) {
            Ok(_) => println!("EIS data exported to {}", path),
            Err(e) => eprintln!("Failed to export EIS CSV: {}", e),
        }
    }
}
