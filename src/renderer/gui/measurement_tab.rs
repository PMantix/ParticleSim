use super::*;
use crate::manual_measurement::ManualMeasurementPoint;
use egui::{Grid, RichText};

impl super::super::Renderer {
    pub fn show_measurement_tab(&mut self, ui: &mut egui::Ui) {
        let compose_base = |charge: &str, amount: &str, steps: &str| -> String {
            let mut parts = Vec::new();
            for part in [charge, amount, steps] {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.replace(' ', ""));
                }
            }
            if parts.is_empty() {
                "Custom".to_string()
            } else {
                parts.join("_")
            }
        };

        self.ensure_measurement_points_seeded();
        // On first entry, if point-based is enabled by default and not yet started, start it
        if self.points_csv_enabled && !self.points_csv_armed {
            if !self.measurement_points_seeded {
                self.ensure_measurement_points_seeded();
            }

            if self
                .manual_measurement_ui_config
                .output_file
                .trim()
                .is_empty()
                || self.manual_measurement_ui_config.output_file == "manual_measurements.csv"
            {
                let base = compose_base(
                    &self.measurement_charge_type_input,
                    &self.measurement_charge_amount_input,
                    &self.measurement_steps_input,
                );
                self.manual_measurement_ui_config.output_file = format!("Point-based_{}.csv", base);
            }
            let can_start = self.measurement_points_seeded || self.foils.is_empty();
            if can_start {
                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                        config: self.manual_measurement_ui_config.clone(),
                    });
                    self.show_manual_measurements = true;
                    self.points_csv_armed = true;
                }
            }
        }
        // Unified Measurements section: time-based, foil metrics, and points (always visible)
        ui.heading("📈 Measurements");
        if let Some(diag) = &self.solvation_diagnostic {
            // Distribution summary
            ui.separator();
            ui.label("Time-based distribution (domain-wide):");
            ui.label(format!(
                "CIP: {:.3}\nSIP: {:.3}\nS2IP: {:.3}\nFD: {:.3}",
                diag.cip_fraction, diag.sip_fraction, diag.s2ip_fraction, diag.fd_fraction
            ));

            // Visual overlays
            ui.separator();
            ui.label("🔍 Visual Overlays:");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_cip_ions, "Show CIP");
                ui.checkbox(&mut self.show_sip_ions, "Show SIP");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_s2ip_ions, "Show S2IP");
                ui.checkbox(&mut self.show_fd_ions, "Show FD");
            });

            // CSV logging (Solvation + Foil + Points filenames side-by-side)
            ui.separator();
            ui.label("🗂️ CSV Logging:");

            // Time-based (formerly Solvation) enable + interval
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.solvation_csv_enabled, "Enable Time-based CSV");
                ui.label("Interval (fs):");
                ui.add(
                    egui::DragValue::new(&mut self.solvation_csv_interval_fs)
                        .speed(100.0)
                        .clamp_range(10.0..=1_000_000.0),
                );
            });

            // Foil-based enable
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.foil_metrics_enabled, "Enable Foil-based CSV");
            });

            // Shared filename helper inputs
            ui.group(|ui| {
                ui.label("Filename helper inputs");
                ui.horizontal(|ui| {
                    ui.label("Charge type:");
                    ui.text_edit_singleline(&mut self.measurement_charge_type_input);
                });
                ui.horizontal(|ui| {
                    ui.label("Charge amount:");
                    ui.text_edit_singleline(&mut self.measurement_charge_amount_input);
                });
                ui.horizontal(|ui| {
                    ui.label("Steps:");
                    ui.text_edit_singleline(&mut self.measurement_steps_input);
                });
                ui.horizontal(|ui| {
                    if ui.button("Auto Populate").clicked() {
                        if matches!(
                            self.charging_ui_mode,
                            super::super::ChargingUiMode::SwitchCharging
                        ) {
                            use crate::switch_charging::{Mode, StepSetpoint};
                            let cfg = &self.switch_ui_state.config;
                            let step_opt = *crate::renderer::state::SWITCH_STEP.lock();
                            let chosen: StepSetpoint = if cfg.use_active_inactive_setpoints {
                                if cfg.use_global_active_inactive {
                                    cfg.global_active.clone()
                                } else {
                                    let idx = step_opt.unwrap_or(0);
                                    cfg.step_active_inactive
                                        .get(&idx)
                                        .map(|s| s.active.clone())
                                        .or_else(|| {
                                            cfg.step_active_inactive
                                                .get(&0)
                                                .map(|s| s.active.clone())
                                        })
                                        .unwrap_or(cfg.global_active.clone())
                                }
                            } else {
                                let idx = step_opt.unwrap_or(0);
                                cfg.step_setpoints
                                    .get(&idx)
                                    .cloned()
                                    .or_else(|| cfg.step_setpoints.get(&0).cloned())
                                    .unwrap_or_default()
                            };
                            let mode_str = match chosen.mode {
                                Mode::Current => "SWITCH_CC",
                                Mode::Overpotential => "SWITCH_OP",
                            };
                            self.measurement_charge_type_input = mode_str.to_string();
                            self.measurement_charge_amount_input =
                                format!("{:.2}", chosen.value.abs()).replace('.', "p");
                            self.measurement_steps_input = if cfg.delta_steps > 0 {
                                cfg.delta_steps.to_string()
                            } else {
                                String::new()
                            };
                        } else if self.conventional_is_overpotential {
                            self.measurement_charge_type_input = "CONV_OP".to_string();
                            self.measurement_charge_amount_input =
                                format!("{:.2}", self.conventional_target_ratio.abs())
                                    .replace('.', "p");
                            self.measurement_steps_input.clear();
                        } else {
                            self.measurement_charge_type_input = "CONV_CC".to_string();
                            self.measurement_charge_amount_input =
                                format!("{:.2}", self.conventional_current_setpoint.abs())
                                    .replace('.', "p");
                            self.measurement_steps_input.clear();
                        }
                    }
                    if ui.button("Apply Filenames").clicked() {
                        let base = compose_base(
                            &self.measurement_charge_type_input,
                            &self.measurement_charge_amount_input,
                            &self.measurement_steps_input,
                        );
                        let time_name = format!("Time-based_{}.csv", base);
                        let foil_name = format!("Foil-based_{}.csv", base);
                        let point_name = format!("Point-based_{}.csv", base);

                        self.solvation_csv_writer.reset();
                        self.solvation_csv_filename = time_name;
                        self.foil_metrics_filename_override = foil_name.clone();
                        self.manual_measurement_ui_config.output_file = point_name.clone();

                        {
                            let mut ov =
                                crate::renderer::state::FOIL_METRICS_FILENAME_OVERRIDE.lock();
                            if foil_name.trim().is_empty() {
                                *ov = None;
                            } else {
                                *ov = Some(foil_name);
                            }
                        }

                        if self.points_csv_enabled {
                            if let Some(tx) =
                                crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref()
                            {
                                let _ = tx.send(
                                    crate::renderer::state::SimCommand::StopManualMeasurement,
                                );
                            }
                            self.points_csv_armed = false;
                            self.show_manual_measurements = true;
                        }
                    }
                });
            });

            // Stack filename inputs, left-aligned
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Time-based CSV:");
                    if ui
                        .text_edit_singleline(&mut self.solvation_csv_filename)
                        .changed()
                    {
                        self.solvation_csv_writer.reset();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Foil-based CSV:");
                    ui.text_edit_singleline(&mut self.foil_metrics_filename_override);
                });
                ui.horizontal(|ui| {
                    ui.label("Point-based CSV:");
                    if ui
                        .text_edit_singleline(&mut self.manual_measurement_ui_config.output_file)
                        .changed()
                    {
                        if self.points_csv_enabled {
                            if let Some(tx) =
                                crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref()
                            {
                                let _ = tx.send(
                                    crate::renderer::state::SimCommand::StopManualMeasurement,
                                );
                            }
                            self.points_csv_armed = false;
                            self.show_manual_measurements = true;
                        }
                    }
                });
            });

            if self.solvation_csv_enabled {
                let current_time = *crate::renderer::state::SIM_TIME.lock();
                let next_due = (self.solvation_csv_last_write_fs + self.solvation_csv_interval_fs)
                    - current_time;
                if next_due.is_finite() {
                    ui.small(format!(
                        "Next time-based write in: {:.1} fs",
                        next_due.max(0.0)
                    ));
                }
            }

            // Push foil settings to simulation globals
            crate::renderer::state::FOIL_METRICS_ENABLED.store(
                self.foil_metrics_enabled,
                std::sync::atomic::Ordering::Relaxed,
            );
            let mut ov = crate::renderer::state::FOIL_METRICS_FILENAME_OVERRIDE.lock();
            if self.foil_metrics_filename_override.trim().is_empty() {
                *ov = None;
            } else {
                *ov = Some(self.foil_metrics_filename_override.clone());
            }

            // Foil metrics field selection drop-down
            egui::ComboBox::from_id_source("foil_fields_combo")
                .selected_text("Foil fields…")
                .show_ui(ui, |ui| {
                    let mut inc_set = crate::renderer::state::FOIL_METRICS_INCLUDE_SETPOINT
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let mut inc_act = crate::renderer::state::FOIL_METRICS_INCLUDE_ACTUAL_RATIO
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let mut inc_del = crate::renderer::state::FOIL_METRICS_INCLUDE_DELTA_ELECTRONS
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let mut inc_li = crate::renderer::state::FOIL_METRICS_INCLUDE_LI_METAL
                        .load(std::sync::atomic::Ordering::Relaxed);
                    ui.checkbox(&mut inc_set, "Setpoint");
                    ui.checkbox(&mut inc_act, "Actual Ratio");
                    ui.checkbox(&mut inc_del, "ΔElectrons");
                    ui.checkbox(&mut inc_li, "Li Metal Count");
                    crate::renderer::state::FOIL_METRICS_INCLUDE_SETPOINT
                        .store(inc_set, std::sync::atomic::Ordering::Relaxed);
                    crate::renderer::state::FOIL_METRICS_INCLUDE_ACTUAL_RATIO
                        .store(inc_act, std::sync::atomic::Ordering::Relaxed);
                    crate::renderer::state::FOIL_METRICS_INCLUDE_DELTA_ELECTRONS
                        .store(inc_del, std::sync::atomic::Ordering::Relaxed);
                    crate::renderer::state::FOIL_METRICS_INCLUDE_LI_METAL
                        .store(inc_li, std::sync::atomic::Ordering::Relaxed);
                });

            ui.separator();

            // Points recording enable toggle and autopopulate controls
            // Recording status
            self.manual_measurement_last_results =
                crate::renderer::state::MANUAL_MEASUREMENT_RESULTS
                    .lock()
                    .clone();
            let was_enabled = self.points_csv_enabled;
            let resp = ui.checkbox(&mut self.points_csv_enabled, "Enable Point-based CSV");
            if resp.changed() {
                if self.points_csv_enabled && !was_enabled {
                    // Ensure auto filename is set before starting if it's default/empty
                    if self
                        .manual_measurement_ui_config
                        .output_file
                        .trim()
                        .is_empty()
                        || self.manual_measurement_ui_config.output_file
                            == "manual_measurements.csv"
                    {
                        let base = compose_base(
                            &self.measurement_charge_type_input,
                            &self.measurement_charge_amount_input,
                            &self.measurement_steps_input,
                        );
                        self.manual_measurement_ui_config.output_file =
                            format!("Point-based_{}.csv", base);
                    }
                    if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                        let _ =
                            tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                                config: self.manual_measurement_ui_config.clone(),
                            });
                        self.show_manual_measurements = true;
                    }
                }
                if !self.points_csv_enabled && was_enabled {
                    if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                        let _ = tx.send(crate::renderer::state::SimCommand::StopManualMeasurement);
                    }
                }
            }

            // Autopopulate controls
            ui.horizontal(|ui| {
                    ui.label("Autopopulate:");
                    // Foil selector (by id)
                    let mut foil_ids: Vec<u64> = self.foils.iter().map(|f| f.id).collect();
                    foil_ids.sort_unstable();
                    let current_sel = self.gen_selected_foil.unwrap_or(3);
                    let mut sel = current_sel;
                    egui::ComboBox::from_id_source("gen_foil_combo")
                        .selected_text(format!("Foil {}", sel))
                        .show_ui(ui, |ui| {
                            for id in foil_ids {
                                if ui.selectable_label(sel == id, format!("Foil {}", id)).clicked() {
                                    sel = id;
                                }
                            }
                        });
                    self.gen_selected_foil = Some(sel);
                    
                    // Show warning if selected foil doesn't exist
                    if let Some(foil_id) = self.gen_selected_foil {
                        let foil_exists = self.foils.iter().any(|f| f.id == foil_id);
                        if !foil_exists {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 150, 0),
                                format!("⚠ Foil {} not found!", foil_id)
                            );
                        }
                    }

                    ui.label("Direction:");
                    egui::ComboBox::from_id_source("gen_dir_combo")
                        .selected_text(self.gen_direction.clone())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.gen_direction, "left".into(), "left");
                            ui.selectable_value(&mut self.gen_direction, "right".into(), "right");
                        });

                    ui.label("Width (Å):");
                    ui.add(egui::DragValue::new(&mut self.gen_max_length).speed(1.0).clamp_range(10.0..=1000.0));
                    ui.label("Count:");
                    ui.add(egui::DragValue::new(&mut self.gen_point_count).speed(1.0).clamp_range(1..=20));

                    if ui.button("Generate").clicked() {
                        if let Some(foil_id) = self.gen_selected_foil {
                            // Check if the foil actually exists
                            let foil_exists = self.foils.iter().any(|f| f.id == foil_id);
                            if !foil_exists {
                                println!("⚠ Warning: Selected Foil {} no longer exists!", foil_id);
                                println!("  Available foil IDs: {:?}", self.foils.iter().map(|f| f.id).collect::<Vec<_>>());
                                println!("  Please select a different foil from the dropdown.");
                                // Try to select the first available foil
                                if let Some(first_foil) = self.foils.first() {
                                    self.gen_selected_foil = Some(first_foil.id);
                                    println!("  Auto-selected Foil {} instead.", first_foil.id);
                                }
                            }
                            
                            if let Some(new_points) = self.build_measurement_points_for_settings(foil_id) {
                                self.manual_measurement_ui_config.points = new_points;
                                self.measurement_points_seeded = true;
                                self.show_manual_measurements = true;

                                if !self.points_csv_enabled {
                                    let base = compose_base(
                                        &self.measurement_charge_type_input,
                                        &self.measurement_charge_amount_input,
                                        &self.measurement_steps_input,
                                    );
                                    self.manual_measurement_ui_config.output_file =
                                        format!("Point-based_{}.csv", base);
                                }

                                if self.points_csv_enabled {
                                    if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                                        let _ = tx.send(crate::renderer::state::SimCommand::StopManualMeasurement);
                                        let _ = tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                                            config: self.manual_measurement_ui_config.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                });
        } else {
            ui.label("❌ No time-based diagnostic data available.");
        }

        // List of measurement points
        let mut point_to_delete = None;
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, point) in self
                    .manual_measurement_ui_config
                    .points
                    .iter_mut()
                    .enumerate()
                {
                    let is_selected = self.manual_measurement_selected_point == idx;
                    let bg_color = if is_selected {
                        egui::Color32::from_rgb(50, 50, 80)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    egui::Frame::none()
                        .fill(bg_color)
                        .inner_margin(egui::Margin::same(4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(is_selected, &point.label).clicked() {
                                    self.manual_measurement_selected_point = idx;
                                }

                                ui.label(format!("({:.1}, {:.1})", point.x, point.y));
                                ui.label(format!("{}×{} Å", point.width, point.height));
                                ui.label(&point.direction);
                                if let Some(fid) = point.host_foil_id {
                                    ui.label(format!("host foil: {}", fid));
                                }

                                if ui.small_button("🗑").clicked() {
                                    point_to_delete = Some(idx);
                                }
                            });
                        });
                }
            });

        if let Some(idx) = point_to_delete {
            self.manual_measurement_ui_config.points.remove(idx);
            if self.manual_measurement_selected_point
                >= self.manual_measurement_ui_config.points.len()
            {
                self.manual_measurement_selected_point = self
                    .manual_measurement_ui_config
                    .points
                    .len()
                    .saturating_sub(1);
            }
            self.measurement_points_seeded = true;
            // If recording is enabled, restart recorder so header matches new points
            if self.points_csv_enabled {
                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = tx.send(crate::renderer::state::SimCommand::StopManualMeasurement);
                    let _ = tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                        config: self.manual_measurement_ui_config.clone(),
                    });
                }
            }
        }

        // Add new point button
        if ui.button("➕ Add Measurement Point").clicked() {
            let mut new_point = ManualMeasurementPoint::default();
            new_point.label = format!(
                "Measurement_{}",
                self.manual_measurement_ui_config.points.len() + 1
            );
            self.manual_measurement_ui_config.points.push(new_point);
            self.manual_measurement_selected_point =
                self.manual_measurement_ui_config.points.len() - 1;
            self.measurement_points_seeded = true;
            // If recording is enabled, restart recorder so header matches new points
            if self.points_csv_enabled {
                if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                    let _ = tx.send(crate::renderer::state::SimCommand::StopManualMeasurement);
                    let _ = tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                        config: self.manual_measurement_ui_config.clone(),
                    });
                }
            }
        }

        // Edit selected point
        if !self.manual_measurement_ui_config.points.is_empty()
            && self.manual_measurement_selected_point
                < self.manual_measurement_ui_config.points.len()
        {
            ui.separator();
            ui.label(RichText::new("Edit Selected Point").strong());

            let point = &mut self.manual_measurement_ui_config.points
                [self.manual_measurement_selected_point];

            ui.horizontal(|ui| {
                ui.label("Label:");
                ui.text_edit_singleline(&mut point.label);
            });

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut point.x).speed(1.0));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut point.y).speed(1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(
                    egui::DragValue::new(&mut point.width)
                        .speed(1.0)
                        .clamp_range(10.0..=500.0),
                );
                ui.label("Height:");
                ui.add(
                    egui::DragValue::new(&mut point.height)
                        .speed(1.0)
                        .clamp_range(10.0..=500.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Direction:");
                egui::ComboBox::from_id_source("measurement_direction")
                    .selected_text(&point.direction)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut point.direction, "left".to_string(), "Left");
                        ui.selectable_value(&mut point.direction, "right".to_string(), "Right");
                        ui.selectable_value(&mut point.direction, "up".to_string(), "Up");
                        ui.selectable_value(&mut point.direction, "down".to_string(), "Down");
                    });
            });
        }

        ui.separator();

        // Recording controls moved into unified section toggle

        // Display last measurement results
        if !self.manual_measurement_last_results.is_empty() {
            ui.separator();
            ui.label(RichText::new("Latest Measurements:").strong());

            Grid::new("manual_measurement_results")
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Label");
                    ui.label("Edge (Å)");
                    ui.end_row();

                    for result in &self.manual_measurement_last_results {
                        ui.label(&result.label);
                        ui.label(format!("{:.2}", result.edge_position));
                        ui.end_row();
                    }
                });
        }

        ui.separator();

        // Original Manual Measurement Tool
        ui.heading("📏 Manual Measurement Tool");
        ui.label("Select a starting point, then (optionally) define a direction. Live distances will be projected onto that direction if set.");

        ui.horizontal(|ui| {
            if ui.button("🎯 Select Start Point").clicked() {
                self.measurement_selecting_start = true;
                // Reset direction when selecting a new start
                self.measurement_direction = None;
                self.measurement_selecting_direction = false;
            }

            let copy_enabled = !self.measurement_history.is_empty();
            let copy_button =
                ui.add_enabled(copy_enabled, egui::Button::new("📋 Copy History (.csv)"));
            if copy_button.clicked() {
                let mut csv = String::from("step,time_fs,distance,switch_step,switch_mode,switch_value,pos_role,neg_role\n");
                for record in &self.measurement_history {
                    let ss = record.switch_step.map(|v| v.to_string()).unwrap_or_default();
                    let sm = record.switch_mode.clone().unwrap_or_default();
                    let sv = record.switch_value.map(|v| format!("{:.6}", v)).unwrap_or_default();
                    let pr = record.pos_role.clone().unwrap_or_default();
                    let nr = record.neg_role.clone().unwrap_or_default();
                    csv.push_str(&format!(
                        "{},{:.6},{:.6},{},{},{},{},{}\n",
                        record.step, record.time_fs, record.distance, ss, sm, sv, pr, nr
                    ));
                }
                ui.output_mut(|o| o.copied_text = csv);
                ui.label(RichText::new("History copied to clipboard").italics());
            }

            let clear_button = ui.add_enabled(copy_enabled, egui::Button::new("🧹 Clear History"));
            if clear_button.clicked() {
                self.clear_measurement();
                self.measurement_history.clear();
            }

            let dir_enabled = self.measurement_start.is_some();
            let dir_label = if self.measurement_direction.is_some() { "🧭 Redefine Direction" } else { "🧭 Define Direction" };
            let dir_button = ui.add_enabled(dir_enabled, egui::Button::new(dir_label));
            if dir_button.clicked() {
                self.measurement_selecting_direction = true;
            }
        });

        if self.measurement_selecting_start {
            ui.label(
                RichText::new("Click in the simulation view to set the starting point.").italics(),
            );
        }

        if self.measurement_selecting_direction {
            ui.label(
                RichText::new("Click in the simulation view to set the measurement direction (from the start point toward your click).").italics(),
            );
        }

        ui.separator();

        match self.measurement_start {
            Some(start) => {
                ui.label(format!("Starting point: ({:.2}, {:.2})", start.x, start.y));
            }
            None => {
                ui.label("No starting point selected yet.");
            }
        }

        match self.current_measurement_distance() {
            Some(distance) => {
                let label = if self.measurement_direction.is_some() {
                    "Projected distance"
                } else {
                    "Current distance"
                };
                ui.label(format!("{}: {:.3}", label, distance));
            }
            None => {
                if self.measurement_start.is_some() {
                    ui.label("Current distance: move the mouse over the simulation view.");
                } else {
                    ui.label("Current distance: select a starting point to begin measuring.");
                }
            }
        }

        if let Some(cursor) = self.measurement_cursor {
            ui.label(format!(
                "Current cursor: ({:.2}, {:.2})",
                cursor.x, cursor.y
            ));
        }

        if let Some(dir) = self.measurement_direction {
            ui.small(format!(
                "Direction set: ({:.2}, {:.2}) — measurements are projected onto this axis.",
                dir.x, dir.y
            ));
        } else if self.measurement_start.is_some() {
            ui.small("Tip: Define a direction to lock measurements to a single axis (use the button above).");
        }

        ui.small("Left-click in the simulation view to record a measurement. Right-click or switch tabs to exit measurement mode.");

        ui.separator();
        ui.label("📚 Measurement History");

        if self.measurement_history.is_empty() {
            ui.label("No measurements recorded yet.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    Grid::new("measurement_history_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Step").strong());
                            ui.label(RichText::new("Time (fs)").strong());
                            ui.label(RichText::new("Distance").strong());
                            ui.label(RichText::new("Switch Step").strong());
                            ui.label(RichText::new("Mode").strong());
                            ui.label(RichText::new("Value").strong());
                            ui.label(RichText::new("+Role").strong());
                            ui.label(RichText::new("-Role").strong());
                            ui.end_row();

                            for record in self.measurement_history.iter().rev() {
                                ui.label(format!("{}", record.step));
                                ui.label(format!("{:.3}", record.time_fs));
                                ui.label(format!("{:.3}", record.distance));
                                ui.label(
                                    record
                                        .switch_step
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "".into()),
                                );
                                ui.label(record.switch_mode.clone().unwrap_or_default());
                                ui.label(
                                    record
                                        .switch_value
                                        .map(|v| format!("{:.3}", v))
                                        .unwrap_or_default(),
                                );
                                ui.label(record.pos_role.clone().unwrap_or_default());
                                ui.label(record.neg_role.clone().unwrap_or_default());
                                ui.end_row();
                            }
                        });
                });
        }
    }

    fn ensure_measurement_points_seeded(&mut self) {
        if self.measurement_points_seeded {
            return;
        }

        let using_placeholder = match self.manual_measurement_ui_config.points.as_slice() {
            [] => true,
            [single] => single.host_foil_id.is_none() && single.label.starts_with("Measurement_"),
            _ => false,
        };

        if !using_placeholder {
            self.measurement_points_seeded = true;
            return;
        }

        let foil_id = match self.gen_selected_foil {
            Some(id) => id,
            None => return,
        };

        if let Some(points) = self.build_measurement_points_for_settings(foil_id) {
            self.manual_measurement_ui_config.points = points;
            self.measurement_points_seeded = true;
            self.show_manual_measurements = true;
        }
    }

    fn build_measurement_points_for_settings(
        &self,
        foil_id: u64,
    ) -> Option<Vec<ManualMeasurementPoint>> {
        let foil = self.foils.iter().find(|f| f.id == foil_id)?;

        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let mut x_center_acc = 0.0f32;
        let mut count = 0usize;

        for body in &self.bodies {
            if foil.body_ids.contains(&body.id) {
                y_min = y_min.min(body.pos.y);
                y_max = y_max.max(body.pos.y);
                x_center_acc += body.pos.x;
                count += 1;
            }
        }

        if count == 0 || !y_min.is_finite() || !y_max.is_finite() {
            return None;
        }

        let x_center = x_center_acc / count as f32;
        let foil_height = y_max - y_min;
        if foil_height <= 0.0 || !foil_height.is_finite() {
            return None;
        }

        let bins = self.gen_point_count.max(1) as f32;
        let bin_height = foil_height / bins;
        let box_height = bin_height * 0.9;
        let width = self.gen_max_length;
        let direction = self.gen_direction.clone();
        let mut points = Vec::with_capacity(self.gen_point_count.max(1));

        for i in 0..self.gen_point_count.max(1) {
            let y_center = y_min + (i as f32 + 0.5) * bin_height;
            let label = format!("Foil{}_Ybin{}", foil_id, i + 1);
            points.push(ManualMeasurementPoint {
                x: x_center,
                y: y_center,
                width,
                height: box_height,
                direction: direction.clone(),
                label,
                host_foil_id: Some(foil_id),
            });
        }

        Some(points)
    }
}
