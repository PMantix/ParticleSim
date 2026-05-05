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
                // Mode selector
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.selectable_value(
                        &mut self.eis_mode,
                        crate::simulation::eis::EisMode::Galvanostatic,
                        "Galvanostatic (current)",
                    );
                    ui.selectable_value(
                        &mut self.eis_mode,
                        crate::simulation::eis::EisMode::Potentiostatic,
                        "Potentiostatic (voltage)",
                    );
                });

                // Amplitude — label changes with mode
                let amp_label = match self.eis_mode {
                    crate::simulation::eis::EisMode::Galvanostatic => "Amplitude (e/fs):",
                    crate::simulation::eis::EisMode::Potentiostatic => "Amplitude (Δratio):",
                };
                ui.horizontal(|ui| {
                    ui.label(amp_label);
                    ui.add(
                        egui::DragValue::new(&mut self.eis_amplitude)
                            .speed(0.0001)
                            .clamp_range(1e-8..=100.0)
                            .max_decimals(6),
                    );
                    if self.eis_mode == crate::simulation::eis::EisMode::Potentiostatic {
                        ui.label(egui::RichText::new("(foil must be in overpotential mode)")
                            .small()
                            .color(egui::Color32::from_rgb(200, 160, 80)));
                    }
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
                            .clamp_range(1.0..=100.0),
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
                ui.horizontal(|ui| {
                    ui.label("Repeats per freq:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_repeats_per_freq)
                            .speed(1)
                            .clamp_range(1..=20),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Voltage probes:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_voltage_probes)
                            .speed(1)
                            .clamp_range(0..=200usize),
                    );
                    if self.eis_voltage_probes == 0 {
                        ui.label(egui::RichText::new("(all)").small());
                    }
                    ui.checkbox(&mut self.eis_show_probes, "Show");
                });

                // Diagnostic controls
                ui.separator();
                ui.label(egui::RichText::new("Diagnostics").strong());
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.eis_show_actual_i, "Show actual electron I");
                    ui.label(egui::RichText::new("(cyan trace)").small().color(egui::Color32::from_rgb(80, 220, 220)));
                });
                ui.horizontal(|ui| {
                    ui.label("Virtual cap C:");
                    ui.add(
                        egui::DragValue::new(&mut self.eis_c_virtual)
                            .speed(1e-5)
                            .clamp_range(1e-6..=1.0)
                            .max_decimals(6),
                    );
                    ui.label(egui::RichText::new("(green dashed V_cap)").small().color(egui::Color32::from_rgb(80, 200, 120)));
                });
                ui.separator();

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
                        mode: self.eis_mode,
                        repeats_per_freq: self.eis_repeats_per_freq,
                        voltage_probes: self.eis_voltage_probes,
                        c_virtual: self.eis_c_virtual,
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
                        let freq_label = format!(
                            "Freq {}/{} ({:.2e} 1/fs)",
                            shared.current_freq_idx + 1,
                            shared.total_frequencies,
                            shared.current_freq,
                        );
                        if shared.total_repeats > 1 {
                            ui.label(format!(
                                "{}, repeat {}/{}",
                                freq_label,
                                shared.current_repeat + 1,
                                shared.total_repeats,
                            ));
                        } else {
                            ui.label(freq_label);
                        }
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
                                mode: self.eis_mode,
                                repeats_per_freq: self.eis_repeats_per_freq,
                                voltage_probes: self.eis_voltage_probes,
                                c_virtual: self.eis_c_virtual,
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
                // Debug reference lines
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.eis_hline_enabled, "H line  -Im(Z) =");
                    ui.add_enabled(
                        self.eis_hline_enabled,
                        egui::DragValue::new(&mut self.eis_hline_val)
                            .speed(1e-4)
                            .max_decimals(6),
                    );
                    ui.add_space(16.0);
                    ui.checkbox(&mut self.eis_vline_enabled, "V line  Re(Z) =");
                    ui.add_enabled(
                        self.eis_vline_enabled,
                        egui::DragValue::new(&mut self.eis_vline_val)
                            .speed(1e-4)
                            .max_decimals(6),
                    );
                });
                // Zoom controls
                ui.horizontal(|ui| {
                    let label = if self.eis_nyquist_set_range { "Set Range (active)" } else { "Set Range" };
                    if ui.selectable_label(self.eis_nyquist_set_range, label).clicked() {
                        self.eis_nyquist_set_range = !self.eis_nyquist_set_range;
                        if !self.eis_nyquist_set_range {
                            self.eis_nyquist_drag_start = None;
                        }
                    }
                    if ui.add_enabled(self.eis_nyquist_bounds.is_some(), egui::Button::new("Reset View")).clicked() {
                        self.eis_nyquist_bounds = None;
                    }
                });
                self.draw_nyquist_plot(ui, &shared.points);
            });

        // Bode plot: |Z| and phase(Z) vs log(freq)
        egui::CollapsingHeader::new("Bode Plot")
            .default_open(true)
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
                        ui.label("R²(V)");
                        ui.label("R²(I)");
                        ui.label("V amp");
                        ui.label("V phase (deg)");
                        ui.label("I amp");
                        ui.label("I phase (deg)");
                        ui.end_row();

                        for pt in &shared.points {
                            ui.label(format!("{:.2e}", pt.frequency));
                            ui.label(format!("{:.4e}", pt.z_real));
                            ui.label(format!("{:.4e}", -pt.z_imag));
                            ui.label(format!("{:.4e}", pt.magnitude));
                            ui.label(format!("{:.2}", pt.phase_deg));
                            ui.label(format!("{:.4}", pt.fit_r2_v));
                            ui.label(format!("{:.4}", pt.fit_r2_i));
                            ui.label(format!("{:.4e}", pt.fit_v_amp));
                            ui.label(format!("{:.2}", pt.fit_v_phase_deg));
                            ui.label(format!("{:.4e}", pt.fit_i_amp));
                            ui.label(format!("{:.2}", pt.fit_i_phase_deg));
                            ui.end_row();
                        }
                    });
            });

        // Export button
        ui.separator();
        if ui.button("Export CSV").clicked() {
            self.export_eis_csv(&shared.points);
        }

        // ----- Phase 4.2: Morphology metrics live display + CSV log toggle ----
        ui.separator();
        egui::CollapsingHeader::new("Morphology metrics (Phase 4.2)")
            .default_open(false)
            .show(ui, |ui| {
                let snapshot = crate::renderer::state::MORPHOLOGY_LATEST.lock().clone();
                ui.horizontal(|ui| {
                    ui.label("Latest values:");
                    if snapshot.is_none() {
                        ui.label("(not yet computed — enable logging or run a sim step)");
                    }
                });
                if let Some(m) = snapshot {
                    egui::Grid::new("morphology_metrics_grid")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("arc_length / lateral");
                            ui.label(format!("{:.4}", m.interface_arc_length_per_unit_lateral));
                            ui.end_row();
                            ui.label("roughness_rms (Å)");
                            ui.label(format!("{:.3}", m.interface_roughness_rms_angstroms));
                            ui.end_row();
                            ui.label("dead_li_fraction");
                            ui.label(format!("{:.4}", m.dead_li_fraction));
                            ui.end_row();
                            ui.label("accessible_surface_atoms");
                            ui.label(format!("{}", m.accessible_surface_atoms));
                            ui.end_row();
                        });
                }

                ui.separator();
                ui.label("CSV logging:");
                ui.horizontal(|ui| {
                    ui.label("path");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.morphology_log_path)
                            .desired_width(280.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("log every");
                    ui.add(
                        egui::DragValue::new(&mut self.morphology_log_every_frames)
                            .clamp_range(1..=1_000_000)
                            .speed(10.0),
                    );
                    ui.label("frames");
                });
                ui.horizontal(|ui| {
                    let label = if self.morphology_log_enabled {
                        "Stop logging"
                    } else {
                        "Start logging"
                    };
                    if ui.button(label).clicked() {
                        if let Some(tx) = SIM_COMMAND_SENDER.lock().as_ref() {
                            if self.morphology_log_enabled {
                                let _ = tx.send(SimCommand::StopMorphologyLog);
                                self.morphology_log_enabled = false;
                            } else {
                                let _ = tx.send(SimCommand::StartMorphologyLog {
                                    path: std::path::PathBuf::from(self.morphology_log_path.clone()),
                                    log_every_frames: self.morphology_log_every_frames,
                                });
                                self.morphology_log_enabled = true;
                            }
                        }
                    }
                    if self.morphology_log_enabled {
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "● recording");
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "CSV: frame,time_fs,arc_length_norm,roughness_rms,dead_li_frac,accessible_atoms",
                    )
                    .small()
                    .color(egui::Color32::GRAY),
                );
            });
    }

    fn draw_nyquist_plot(
        &mut self,
        ui: &mut egui::Ui,
        points: &[crate::simulation::eis::EisPoint],
    ) {
        let hline = if self.eis_hline_enabled { Some(self.eis_hline_val) } else { None };
        let vline = if self.eis_vline_enabled { Some(self.eis_vline_val) } else { None };
        const LEFT_MARGIN: f32 = 62.0;
        let available = ui.available_size();
        let plot_size = egui::Vec2::new(available.x - LEFT_MARGIN - 10.0, 300.0);

        // Use click_and_drag sense when Set Range mode is active, hover otherwise
        let sense = if self.eis_nyquist_set_range {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::hover()
        };

        let (rect, response) = ui
            .horizontal(|ui| {
                ui.add_space(LEFT_MARGIN);
                ui.allocate_exact_size(plot_size, sense)
            })
            .inner;

        if !ui.is_rect_visible(rect) || points.is_empty() {
            return;
        }

        // Dark background (matches Signal Time Series style)
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_gray(30));
        ui.painter()
            .rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));

        // Compute data ranges
        let re_vals: Vec<f64> = points.iter().map(|p| p.z_real).collect();
        let neg_im_vals: Vec<f64> = points.iter().map(|p| -p.z_imag).collect();

        // Use custom bounds if set, otherwise auto-fit
        let (x_min, x_max, y_min, y_max) = if let Some(b) = self.eis_nyquist_bounds {
            (b[0], b[1], b[2], b[3])
        } else {
            let re_min = re_vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let re_max = re_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let im_min = neg_im_vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let im_max = neg_im_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let re_range = (re_max - re_min).max(1e-20);
            let im_range = (im_max - im_min).max(1e-20);
            let re_pad = re_range * 0.1;
            let im_pad = im_range * 0.1;
            (re_min - re_pad, re_max + re_pad, im_min - im_pad, im_max + im_pad)
        };

        let x_span = (x_max - x_min).max(1e-30);
        let y_span = (y_max - y_min).max(1e-30);

        // Frequency gradient colors: green (high freq) → orange (low freq)
        let color_hi = [80u8, 200, 120];  // green — high frequency
        let color_lo = [220u8, 120, 60];  // orange — low frequency

        // Compute frequency range for normalization
        let freq_min = points.iter().map(|p| p.frequency).fold(f32::INFINITY, f32::min);
        let freq_max = points.iter().map(|p| p.frequency).fold(f32::NEG_INFINITY, f32::max);
        let freq_range = (freq_max - freq_min).max(1e-30);

        let color_for_freq = |freq: f32| -> egui::Color32 {
            let t = ((freq - freq_min) / freq_range).clamp(0.0, 1.0); // 0=low, 1=high
            let r = color_lo[0] as f32 + (color_hi[0] as f32 - color_lo[0] as f32) * t;
            let g = color_lo[1] as f32 + (color_hi[1] as f32 - color_lo[1] as f32) * t;
            let b = color_lo[2] as f32 + (color_hi[2] as f32 - color_lo[2] as f32) * t;
            egui::Color32::from_rgb(r as u8, g as u8, b as u8)
        };

        // Convert data → screen coords
        let mut screen_pts = Vec::new();
        for (re, neg_im) in re_vals.iter().zip(neg_im_vals.iter()) {
            let x_norm = (re - x_min) / x_span;
            let y_norm = 1.0 - (neg_im - y_min) / y_span;
            let sx = rect.min.x + x_norm as f32 * rect.width();
            let sy = rect.min.y + y_norm as f32 * rect.height();
            screen_pts.push(egui::Pos2::new(sx, sy));
        }

        // Clip all data drawing to the plot rectangle
        let plot_painter = ui.painter().with_clip_rect(rect);

        // Draw line segments with frequency gradient
        for i in 0..screen_pts.len().saturating_sub(1) {
            let avg_freq = (points[i].frequency + points[i + 1].frequency) * 0.5;
            let c = color_for_freq(avg_freq);
            plot_painter.line_segment(
                [screen_pts[i], screen_pts[i + 1]],
                egui::Stroke::new(1.5, c),
            );
        }
        // Draw points with frequency gradient
        for (i, pt) in screen_pts.iter().enumerate() {
            let c = color_for_freq(points[i].frequency);
            plot_painter.circle_filled(*pt, 3.0, c);
        }

        // Debug reference lines
        let ref_color = egui::Color32::from_rgb(220, 60, 60);
        let ref_stroke = egui::Stroke::new(1.5, ref_color);
        if let Some(h_val) = hline {
            let y_norm = 1.0 - ((h_val - y_min) / y_span) as f32;
            let sy = rect.min.y + y_norm * rect.height();
            plot_painter.line_segment(
                [egui::Pos2::new(rect.min.x, sy), egui::Pos2::new(rect.max.x, sy)],
                ref_stroke,
            );
        }
        if let Some(v_val) = vline {
            let x_norm = ((v_val - x_min) / x_span) as f32;
            let sx = rect.min.x + x_norm * rect.width();
            plot_painter.line_segment(
                [egui::Pos2::new(sx, rect.min.y), egui::Pos2::new(sx, rect.max.y)],
                ref_stroke,
            );
        }

        // Axis labels
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

        // --- Drag-to-zoom logic (only when Set Range mode is active) ---
        if self.eis_nyquist_set_range {
            // Helper: convert screen pos → data coords
            let screen_to_data = |sp: egui::Pos2| -> (f64, f64) {
                let xn = ((sp.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
                let yn = ((sp.y - rect.min.y) / rect.height()).clamp(0.0, 1.0) as f64;
                let data_x = x_min + xn * x_span;
                let data_y = y_min + (1.0 - yn) * y_span;
                (data_x, data_y)
            };

            if response.drag_started() {
                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                    self.eis_nyquist_drag_start = Some(pos);
                }
            }

            // Draw selection rectangle while dragging
            if let Some(start) = self.eis_nyquist_drag_start {
                if let Some(current) = ui.ctx().pointer_interact_pos() {
                    let sel_rect = egui::Rect::from_two_pos(start, current).intersect(rect);
                    ui.painter().rect_filled(
                        sel_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(80, 200, 120, 40),
                    );
                    ui.painter().rect_stroke(
                        sel_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 200, 120)),
                    );
                }
            }

            if response.drag_released() {
                if let Some(start) = self.eis_nyquist_drag_start.take() {
                    if let Some(end) = ui.ctx().pointer_interact_pos() {
                        let sel = egui::Rect::from_two_pos(start, end).intersect(rect);
                        // Only apply if the selection is large enough (>5px each axis)
                        if sel.width() > 5.0 && sel.height() > 5.0 {
                            let (dx0, dy0) = screen_to_data(sel.min);
                            let (dx1, dy1) = screen_to_data(sel.max);
                            self.eis_nyquist_bounds = Some([
                                dx0.min(dx1), dx0.max(dx1),
                                dy0.min(dy1), dy0.max(dy1),
                            ]);
                        }
                    }
                }
                self.eis_nyquist_set_range = false;
            }
        }

        // Hover tooltip (only when NOT in Set Range mode)
        if !self.eis_nyquist_set_range {
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
                        plot_painter.circle_stroke(
                            screen_pts[idx],
                            7.0,
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                        let lines = [
                            format!("f = {:.3e} 1/fs", pt.frequency),
                            format!("Re(Z)  = {:.4e}", pt.z_real),
                            format!("-Im(Z) = {:.4e}", -pt.z_imag),
                            format!("R\u{00B2}(V)  = {:.4}", pt.fit_r2_v),
                            format!("R\u{00B2}(I)  = {:.4}", pt.fit_r2_i),
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
        // Draw both traces first so we have both y-ranges before the fit overlay
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

        let actual_i_color = egui::Color32::from_rgb(80, 220, 220); // cyan for actual electron I
        let vcap_color = egui::Color32::from_rgb(120, 220, 120); // light green for V_cap

        // Draw actual electron current trace (cyan) — normalised independently
        if self.eis_show_actual_i && !shared.ts_actual_i.is_empty() {
            let (ai_lo, ai_hi) = data_range(&shared.ts_actual_i);
            let pts: Vec<egui::Pos2> = ts
                .iter()
                .zip(shared.ts_actual_i.iter())
                .map(|(&t, &ai)| to_screen(t, ai, ai_lo, ai_hi))
                .collect();
            for w in pts.windows(2) {
                ui.painter()
                    .line_segment([w[0], w[1]], egui::Stroke::new(1.0, actual_i_color));
            }
        }

        // Draw virtual capacitor V_cap trace (dashed green) — normalised to V range
        if !shared.ts_v_cap.is_empty() {
            let (vc_lo, vc_hi) = data_range(&shared.ts_v_cap);
            let pts: Vec<egui::Pos2> = ts
                .iter()
                .zip(shared.ts_v_cap.iter())
                .map(|(&t, &vc)| to_screen(t, vc, vc_lo, vc_hi))
                .collect();
            // Dashed line
            for w in pts.windows(2) {
                let dx = w[1].x - w[0].x;
                let dy = w[1].y - w[0].y;
                let seg_len = (dx * dx + dy * dy).sqrt();
                if seg_len < 3.0 {
                    // Short segment — draw solid
                    ui.painter().line_segment([w[0], w[1]], egui::Stroke::new(1.0, vcap_color));
                } else {
                    // Draw dashes
                    let dash = 4.0;
                    let gap = 3.0;
                    let total = dash + gap;
                    let steps = (seg_len / total).ceil() as usize;
                    for s in 0..steps {
                        let t0 = (s as f32 * total / seg_len).min(1.0);
                        let t1 = ((s as f32 * total + dash) / seg_len).min(1.0);
                        let p0 = egui::Pos2::new(w[0].x + dx * t0, w[0].y + dy * t0);
                        let p1 = egui::Pos2::new(w[0].x + dx * t1, w[0].y + dy * t1);
                        ui.painter().line_segment([p0, p1], egui::Stroke::new(1.0, vcap_color));
                    }
                }
            }
        }

        let fit_painter = ui.painter().with_clip_rect(rect);
        let n_fit = 300usize;

        // Best-fit overlay on V (Coulomb potential) — always, both modes.
        if self.eis_show_fit && (shared.fit_v_re != 0.0 || shared.fit_v_im != 0.0) {
            if let Some(rec_start) = first_record_t {
                let omega = 2.0 * std::f32::consts::PI * shared.current_freq;
                let fit_color = egui::Color32::from_rgb(255, 230, 80); // yellow on V
                let t_rec_end = t_max;
                let fit_pts: Vec<egui::Pos2> = (0..=n_fit)
                    .map(|i| {
                        let t = rec_start + (t_rec_end - rec_start) * i as f32 / n_fit as f32;
                        let t_fit = t - rec_start;
                        let val = shared.fit_v_dc
                            + (shared.fit_v_re * (omega * t_fit).cos() as f64
                                + shared.fit_v_im * (omega * t_fit).sin() as f64) as f32;
                        to_screen(t, val, v_lo, v_hi)
                    })
                    .collect();
                for w in fit_pts.windows(2) {
                    fit_painter.line_segment([w[0], w[1]], egui::Stroke::new(1.5, fit_color));
                }
            }
        }

        // Best-fit overlay on I — potentiostatic mode only (I is also a measured signal).
        if self.eis_show_fit
            && shared.mode == crate::simulation::eis::EisMode::Potentiostatic
            && (shared.fit_i_re != 0.0 || shared.fit_i_im != 0.0)
        {
            if let Some(rec_start) = first_record_t {
                let omega = 2.0 * std::f32::consts::PI * shared.current_freq;
                let fit_color = egui::Color32::from_rgb(255, 180, 60); // amber on I
                let t_rec_end = t_max;
                let fit_pts: Vec<egui::Pos2> = (0..=n_fit)
                    .map(|i| {
                        let t = rec_start + (t_rec_end - rec_start) * i as f32 / n_fit as f32;
                        let t_fit = t - rec_start;
                        let val = shared.fit_i_dc
                            + (shared.fit_i_re * (omega * t_fit).cos() as f64
                                + shared.fit_i_im * (omega * t_fit).sin() as f64) as f32;
                        to_screen(t, val, i_lo, i_hi)
                    })
                    .collect();
                for w in fit_pts.windows(2) {
                    fit_painter.line_segment([w[0], w[1]], egui::Stroke::new(1.5, fit_color));
                }
            }
        }

        // Legend
        let mut legend_y = rect.min.y + 6.0;
        let legend_step = 14.0;
        ui.painter().text(
            egui::Pos2::new(rect.max.x - 8.0, legend_y),
            egui::Align2::RIGHT_TOP,
            format!("V  [{:.2e} … {:.2e}]", v_lo, v_hi),
            font.clone(),
            v_color,
        );
        legend_y += legend_step;
        ui.painter().text(
            egui::Pos2::new(rect.max.x - 8.0, legend_y),
            egui::Align2::RIGHT_TOP,
            format!("I  [{:.2e} … {:.2e}]", i_lo, i_hi),
            font.clone(),
            i_color,
        );
        if self.eis_show_actual_i && !shared.ts_actual_i.is_empty() {
            legend_y += legend_step;
            ui.painter().text(
                egui::Pos2::new(rect.max.x - 8.0, legend_y),
                egui::Align2::RIGHT_TOP,
                "I_actual (electron hops)",
                font.clone(),
                actual_i_color,
            );
        }
        if !shared.ts_v_cap.is_empty() {
            legend_y += legend_step;
            let _ = legend_y; // suppress unused warning
            ui.painter().text(
                egui::Pos2::new(rect.max.x - 8.0, legend_y),
                egui::Align2::RIGHT_TOP,
                "V_cap (virtual capacitor)",
                font.clone(),
                vcap_color,
            );
        }

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

    fn draw_bode_plot(
        &self,
        ui: &mut egui::Ui,
        points: &[crate::simulation::eis::EisPoint],
    ) {
        if points.len() < 2 {
            ui.label("Need at least 2 points for Bode plot.");
            return;
        }

        const LEFT_MARGIN: f32 = 62.0;
        let available = ui.available_size();
        let plot_w = available.x - LEFT_MARGIN - 10.0;
        let sub_h = 120.0;
        let gap = 8.0;

        // Frequency gradient colors (same as Nyquist)
        let color_hi = [80u8, 200, 120]; // green — high freq
        let color_lo = [220u8, 120, 60]; // orange — low freq
        let freq_min = points.iter().map(|p| p.frequency).fold(f32::INFINITY, f32::min);
        let freq_max = points.iter().map(|p| p.frequency).fold(f32::NEG_INFINITY, f32::max);
        let freq_range = (freq_max - freq_min).max(1e-30);
        let color_for_freq = |freq: f32| -> egui::Color32 {
            let t = ((freq - freq_min) / freq_range).clamp(0.0, 1.0);
            let r = color_lo[0] as f32 + (color_hi[0] as f32 - color_lo[0] as f32) * t;
            let g = color_lo[1] as f32 + (color_hi[1] as f32 - color_lo[1] as f32) * t;
            let b = color_lo[2] as f32 + (color_hi[2] as f32 - color_lo[2] as f32) * t;
            egui::Color32::from_rgb(r as u8, g as u8, b as u8)
        };

        let label_color = ui.visuals().text_color();
        let font_tick = egui::FontId::proportional(9.0);

        // X-axis: log10(freq) — shared
        let log_freqs: Vec<f64> = points.iter().map(|p| (p.frequency as f64).log10()).collect();
        let lf_min = log_freqs.iter().cloned().fold(f64::INFINITY, f64::min);
        let lf_max = log_freqs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lf_span = (lf_max - lf_min).max(1e-10);
        let lf_pad = lf_span * 0.05;
        let x_lo = lf_min - lf_pad;
        let x_hi = lf_max + lf_pad;
        let x_span = x_hi - x_lo;

        // --- Sub-plot 1: log10(|Z|) ---
        let log_mags: Vec<f64> = points.iter().map(|p| p.magnitude.max(1e-30).log10()).collect();
        let mag_min = log_mags.iter().cloned().fold(f64::INFINITY, f64::min);
        let mag_max = log_mags.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mag_span = (mag_max - mag_min).max(1e-10);
        let mag_pad = mag_span * 0.1;
        let y1_lo = mag_min - mag_pad;
        let y1_hi = mag_max + mag_pad;
        let y1_span = y1_hi - y1_lo;

        let (rect1, _) = ui
            .horizontal(|ui| {
                ui.add_space(LEFT_MARGIN);
                ui.allocate_exact_size(egui::Vec2::new(plot_w, sub_h), egui::Sense::hover())
            })
            .inner;

        if ui.is_rect_visible(rect1) {
            ui.painter().rect_filled(rect1, 2.0, egui::Color32::from_gray(30));
            ui.painter().rect_stroke(rect1, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));

            let clip = ui.painter().with_clip_rect(rect1);
            // Points + lines
            let mut screen_pts = Vec::with_capacity(points.len());
            for i in 0..points.len() {
                let sx = rect1.min.x + ((log_freqs[i] - x_lo) / x_span) as f32 * rect1.width();
                let sy = rect1.max.y - ((log_mags[i] - y1_lo) / y1_span) as f32 * rect1.height();
                screen_pts.push(egui::Pos2::new(sx, sy));
            }
            for i in 0..screen_pts.len().saturating_sub(1) {
                let c = color_for_freq((points[i].frequency + points[i + 1].frequency) * 0.5);
                clip.line_segment([screen_pts[i], screen_pts[i + 1]], egui::Stroke::new(1.5, c));
            }
            for (i, sp) in screen_pts.iter().enumerate() {
                clip.circle_filled(*sp, 3.0, color_for_freq(points[i].frequency));
            }

            // Y-axis label
            ui.painter().text(
                egui::Pos2::new(rect1.min.x - 36.0, rect1.center().y),
                egui::Align2::CENTER_CENTER,
                "log|Z|",
                egui::FontId::proportional(11.0),
                label_color,
            );
            // Y ticks
            for i in 0..=4 {
                let frac = i as f32 / 4.0;
                let y_val = y1_lo + (y1_hi - y1_lo) * (1.0 - frac as f64);
                ui.painter().text(
                    egui::Pos2::new(rect1.min.x - 4.0, rect1.min.y + frac * rect1.height()),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.2}", y_val),
                    font_tick.clone(),
                    label_color,
                );
            }
        }

        ui.add_space(gap);

        // --- Sub-plot 2: phase(Z) in degrees ---
        let phases: Vec<f64> = points.iter().map(|p| p.phase_deg).collect();
        let actual_phases: Vec<f64> = points.iter().map(|p| p.z_actual_imag.atan2(p.z_actual_real).to_degrees()).collect();
        let cap_phases: Vec<f64> = points.iter().map(|p| p.z_cap_imag.atan2(p.z_cap_real).to_degrees()).collect();

        // Expand y-range to encompass all phase traces
        let mut ph_min = phases.iter().cloned().fold(f64::INFINITY, f64::min);
        let mut ph_max = phases.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let has_actual = points.iter().any(|p| p.z_actual_real != 0.0 || p.z_actual_imag != 0.0);
        let has_cap = points.iter().any(|p| p.z_cap_real != 0.0 || p.z_cap_imag != 0.0);
        if has_actual {
            ph_min = ph_min.min(actual_phases.iter().cloned().fold(f64::INFINITY, f64::min));
            ph_max = ph_max.max(actual_phases.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        }
        if has_cap {
            ph_min = ph_min.min(cap_phases.iter().cloned().fold(f64::INFINITY, f64::min));
            ph_max = ph_max.max(cap_phases.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        }
        let ph_span = (ph_max - ph_min).max(1e-10);
        let ph_pad = ph_span * 0.15;
        let y2_lo = ph_min - ph_pad;
        let y2_hi = ph_max + ph_pad;
        let y2_span = y2_hi - y2_lo;

        let (rect2, _) = ui
            .horizontal(|ui| {
                ui.add_space(LEFT_MARGIN);
                ui.allocate_exact_size(egui::Vec2::new(plot_w, sub_h), egui::Sense::hover())
            })
            .inner;

        if ui.is_rect_visible(rect2) {
            ui.painter().rect_filled(rect2, 2.0, egui::Color32::from_gray(30));
            ui.painter().rect_stroke(rect2, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));

            let clip = ui.painter().with_clip_rect(rect2);

            // Phase=0 dashed line (capacitive/inductive boundary)
            if y2_lo < 0.0 && y2_hi > 0.0 {
                let y_zero = rect2.max.y - ((0.0 - y2_lo) / y2_span) as f32 * rect2.height();
                let dash_len = 6.0;
                let gap_len = 4.0;
                let mut x = rect2.min.x;
                while x < rect2.max.x {
                    let x_end = (x + dash_len).min(rect2.max.x);
                    clip.line_segment(
                        [egui::Pos2::new(x, y_zero), egui::Pos2::new(x_end, y_zero)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
                    );
                    x += dash_len + gap_len;
                }
            }

            // Points + lines (Z_pid phase)
            let mut screen_pts = Vec::with_capacity(points.len());
            for i in 0..points.len() {
                let sx = rect2.min.x + ((log_freqs[i] - x_lo) / x_span) as f32 * rect2.width();
                let sy = rect2.max.y - ((phases[i] - y2_lo) / y2_span) as f32 * rect2.height();
                screen_pts.push(egui::Pos2::new(sx, sy));
            }
            for i in 0..screen_pts.len().saturating_sub(1) {
                let c = color_for_freq((points[i].frequency + points[i + 1].frequency) * 0.5);
                clip.line_segment([screen_pts[i], screen_pts[i + 1]], egui::Stroke::new(1.5, c));
            }
            for (i, sp) in screen_pts.iter().enumerate() {
                clip.circle_filled(*sp, 3.0, color_for_freq(points[i].frequency));
            }

            // Z_actual phase overlay (dashed cyan)
            if has_actual {
                let actual_color = egui::Color32::from_rgb(80, 220, 220);
                let mut apt = Vec::with_capacity(points.len());
                for i in 0..points.len() {
                    let sx = rect2.min.x + ((log_freqs[i] - x_lo) / x_span) as f32 * rect2.width();
                    let sy = rect2.max.y - ((actual_phases[i] - y2_lo) / y2_span) as f32 * rect2.height();
                    apt.push(egui::Pos2::new(sx, sy));
                }
                for w in apt.windows(2) {
                    // Dashed
                    let dx = w[1].x - w[0].x;
                    let dy = w[1].y - w[0].y;
                    let seg = (dx * dx + dy * dy).sqrt();
                    let dash = 5.0;
                    let gap = 3.0;
                    let total = dash + gap;
                    let steps = (seg / total).ceil() as usize;
                    for s in 0..steps {
                        let t0 = (s as f32 * total / seg).min(1.0);
                        let t1 = ((s as f32 * total + dash) / seg).min(1.0);
                        clip.line_segment(
                            [egui::Pos2::new(w[0].x + dx * t0, w[0].y + dy * t0),
                             egui::Pos2::new(w[0].x + dx * t1, w[0].y + dy * t1)],
                            egui::Stroke::new(1.5, actual_color),
                        );
                    }
                }
            }

            // Z_cap phase overlay (dotted green)
            if has_cap {
                let cap_color = egui::Color32::from_rgb(120, 220, 120);
                let mut cpt = Vec::with_capacity(points.len());
                for i in 0..points.len() {
                    let sx = rect2.min.x + ((log_freqs[i] - x_lo) / x_span) as f32 * rect2.width();
                    let sy = rect2.max.y - ((cap_phases[i] - y2_lo) / y2_span) as f32 * rect2.height();
                    cpt.push(egui::Pos2::new(sx, sy));
                }
                for w in cpt.windows(2) {
                    // Dotted (short dashes)
                    let dx = w[1].x - w[0].x;
                    let dy = w[1].y - w[0].y;
                    let seg = (dx * dx + dy * dy).sqrt();
                    let dot = 2.0;
                    let gap = 3.0;
                    let total = dot + gap;
                    let steps = (seg / total).ceil() as usize;
                    for s in 0..steps {
                        let t0 = (s as f32 * total / seg).min(1.0);
                        let t1 = ((s as f32 * total + dot) / seg).min(1.0);
                        clip.line_segment(
                            [egui::Pos2::new(w[0].x + dx * t0, w[0].y + dy * t0),
                             egui::Pos2::new(w[0].x + dx * t1, w[0].y + dy * t1)],
                            egui::Stroke::new(1.5, cap_color),
                        );
                    }
                }
            }

            // Y-axis label
            ui.painter().text(
                egui::Pos2::new(rect2.min.x - 36.0, rect2.center().y),
                egui::Align2::CENTER_CENTER,
                "phase(°)",
                egui::FontId::proportional(11.0),
                label_color,
            );
            // Y ticks
            for i in 0..=4 {
                let frac = i as f32 / 4.0;
                let y_val = y2_lo + (y2_hi - y2_lo) * (1.0 - frac as f64);
                ui.painter().text(
                    egui::Pos2::new(rect2.min.x - 4.0, rect2.min.y + frac * rect2.height()),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.1}", y_val),
                    font_tick.clone(),
                    label_color,
                );
            }
        }

        // Shared x-axis ticks
        for i in 0..=4 {
            let frac = i as f32 / 4.0;
            let x_val = x_lo + (x_hi - x_lo) * frac as f64;
            ui.painter().text(
                egui::Pos2::new(rect2.min.x + frac * rect2.width(), rect2.max.y + 3.0),
                egui::Align2::CENTER_TOP,
                format!("{:.2}", x_val),
                font_tick.clone(),
                label_color,
            );
        }

        // X-axis label
        ui.painter().text(
            egui::Pos2::new(rect2.center().x, rect2.max.y + 15.0),
            egui::Align2::CENTER_TOP,
            "log₁₀(freq)  [1/fs]",
            egui::FontId::proportional(11.0),
            label_color,
        );

        ui.add_space(25.0);
    }

    fn export_eis_csv(&self, points: &[crate::simulation::eis::EisPoint]) {
        let mut csv = String::from(
            "frequency,z_real,z_imag,z_magnitude,phase_deg,fit_r2_v,fit_r2_i,fit_v_amp,fit_v_phase_deg,fit_i_amp,fit_i_phase_deg,z_actual_real,z_actual_imag,z_cap_real,z_cap_imag\n"
        );
        for pt in points {
            csv.push_str(&format!(
                "{:.6e},{:.6e},{:.6e},{:.6e},{:.4},{:.6},{:.6},{:.6e},{:.4},{:.6e},{:.4},{:.6e},{:.6e},{:.6e},{:.6e}\n",
                pt.frequency, pt.z_real, pt.z_imag, pt.magnitude, pt.phase_deg,
                pt.fit_r2_v, pt.fit_r2_i,
                pt.fit_v_amp, pt.fit_v_phase_deg, pt.fit_i_amp, pt.fit_i_phase_deg,
                pt.z_actual_real, pt.z_actual_imag, pt.z_cap_real, pt.z_cap_imag,
            ));
        }
        let path = "eis_results.csv";
        match std::fs::write(path, &csv) {
            Ok(_) => println!("EIS data exported to {}", path),
            Err(e) => eprintln!("Failed to export EIS CSV: {}", e),
        }
    }
}
