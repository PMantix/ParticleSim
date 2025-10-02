use super::*;
use egui::{Grid, RichText};
use crate::manual_measurement::{ManualMeasurementConfig, ManualMeasurementPoint};

impl super::super::Renderer {
    pub fn show_measurement_tab(&mut self, ui: &mut egui::Ui) {
        // Manual Measurement System Section
        ui.collapsing("🎯 Auto-Recording Measurements", |ui| {
            ui.label("Set up measurement points that automatically record to CSV at regular intervals.");
            ui.separator();
            
            // Pull latest measurement results from shared state
            self.manual_measurement_last_results = crate::renderer::state::MANUAL_MEASUREMENT_RESULTS.lock().clone();
            
            // Recording status (check if we have recent results which indicates recording is active)
            let is_recording = !self.manual_measurement_last_results.is_empty();
            let measurement_count = self.manual_measurement_last_results.len();
            
            if is_recording {
                ui.colored_label(egui::Color32::GREEN, format!("🔴 RECORDING ({} measurements)", measurement_count));
            } else {
                ui.label("⏸️ Not recording");
            }
            
            ui.horizontal(|ui| {
                if ui.button("� Refresh Configs").on_hover_text("Scan workspace for manual_measurements_*.toml files").clicked() {
                    // Find available configs in current directory
                    let mut found = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(".") {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                                if name.starts_with("manual_measurements_") && name.ends_with(".toml") {
                                    found.push(name.to_string());
                                }
                            }
                        }
                    }
                    found.sort();
                    self.available_measurement_configs = found;
                }

                ui.separator();

                egui::ComboBox::from_id_source("available_measurement_configs")
                    .selected_text(self.selected_measurement_config.clone().unwrap_or_else(|| "Select config".into()))
                    .show_ui(ui, |ui| {
                        for fname in &self.available_measurement_configs {
                            if ui.selectable_label(self.selected_measurement_config.as_deref() == Some(fname.as_str()), fname).clicked() {
                                self.selected_measurement_config = Some(fname.clone());
                            }
                        }
                    });

                if ui.add_enabled(self.selected_measurement_config.is_some(), egui::Button::new("📂 Load Selected")).clicked() {
                    if let Some(fname) = &self.selected_measurement_config {
                        match ManualMeasurementConfig::from_file(fname) {
                            Ok(config) => {
                                self.manual_measurement_ui_config = config;
                                self.manual_measurement_recorder = None;
                                self.show_manual_measurements = true;
                                println!("✓ Loaded manual measurement config from: {}", fname);
                            }
                            Err(e) => eprintln!("✗ Failed to load {}: {}", fname, e),
                        }
                    }
                }

                ui.separator();

                if ui.button("�💾 Save Config").clicked() {
                    let config_name = self.manual_measurement_ui_config.name.replace(" ", "_");
                    let path = format!("manual_measurements_{}.toml", config_name);
                    match self.manual_measurement_ui_config.to_file(&path) {
                        Ok(_) => println!("✓ Saved manual measurement config to: {}", path),
                        Err(e) => eprintln!("✗ Failed to save config: {}", e),
                    }
                }
                
                if ui.button("📂 Load Config").clicked() {
                    // TODO: Add file picker - for now use default name
                    let path = "manual_measurements.toml";
                    match ManualMeasurementConfig::from_file(path) {
                        Ok(config) => {
                            self.manual_measurement_ui_config = config;
                            self.manual_measurement_recorder = None; // Reset recorder with new config
                            println!("✓ Loaded manual measurement config from: {}", path);
                        }
                        Err(e) => {
                            eprintln!("✗ Failed to load config: {}", e);
                        }
                    }
                }
            });
            
            ui.separator();
            
            // Config name and output file
            ui.horizontal(|ui| {
                ui.label("Config Name:");
                ui.text_edit_singleline(&mut self.manual_measurement_ui_config.name);
            });
            
            ui.horizontal(|ui| {
                ui.label("Output CSV:");
                ui.text_edit_singleline(&mut self.manual_measurement_ui_config.output_file);
            });
            
            ui.horizontal(|ui| {
                ui.label("Interval (fs):");
                ui.add(egui::DragValue::new(&mut self.manual_measurement_ui_config.interval_fs)
                    .speed(100.0)
                    .clamp_range(100.0..=100000.0));
            });
            
            ui.checkbox(&mut self.show_manual_measurements, "Show measurement regions in simulation");
            
            ui.separator();
            ui.label("Measurement Points:");

            // Generator for evenly distributed measurement points along a foil
            ui.collapsing("⚙️ Generate points from foil", |ui| {
                // Choose foil by ID
                ui.horizontal(|ui| {
                    ui.label("Foil:");
                    // Build a simple list of foil IDs
                    let foil_ids: Vec<u64> = self.foils.iter().map(|f| f.id).collect();
                    let current = self.gen_selected_foil;
                    egui::ComboBox::from_id_source("gen_foil_combo")
                        .selected_text(current.map(|id| format!("Foil {}", id)).unwrap_or_else(|| "Select Foil".into()))
                        .show_ui(ui, |ui| {
                            for id in foil_ids {
                                let sel = Some(id) == self.gen_selected_foil;
                                if ui.selectable_label(sel, format!("Foil {}", id)).clicked() {
                                    self.gen_selected_foil = Some(id);
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Direction:");
                    egui::ComboBox::from_id_source("gen_dir_combo")
                        .selected_text(self.gen_direction.clone())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.gen_direction, "left".into(), "left");
                            ui.selectable_value(&mut self.gen_direction, "right".into(), "right");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Max length (Å):");
                    ui.add(egui::DragValue::new(&mut self.gen_max_length).speed(1.0).clamp_range(10.0..=1000.0));
                    ui.label("Point count:");
                    ui.add(egui::DragValue::new(&mut self.gen_point_count).speed(1.0).clamp_range(1..=20));
                });

                if ui.add_enabled(self.gen_selected_foil.is_some(), egui::Button::new("Generate")).clicked() {
                    if let Some(foil_id) = self.gen_selected_foil {
                        // Build points based on foil's Y span and center X
                        if let Some(foil) = self.foils.iter().find(|f| f.id == foil_id) {
                            // Compute foil bounds from its bodies
                            let mut y_min = f32::INFINITY;
                            let mut y_max = f32::NEG_INFINITY;
                            let mut x_center_acc = 0.0f32;
                            let mut count = 0usize;
                            for b in &self.bodies {
                                if foil.body_ids.contains(&b.id) {
                                    y_min = y_min.min(b.pos.y);
                                    y_max = y_max.max(b.pos.y);
                                    x_center_acc += b.pos.x;
                                    count += 1;
                                }
                            }
                            if count > 0 && y_max.is_finite() && y_min.is_finite() {
                                let x_center = x_center_acc / count as f32;
                                let foil_height = y_max - y_min;
                                let bins = self.gen_point_count.max(1) as f32;
                                let bin_height = foil_height / bins;
                                let box_height = bin_height * 0.9; // 90% to avoid overlap
                                let width = self.gen_max_length;   // extend left/right from center by this length
                                let dir = self.gen_direction.clone();

                                let mut new_points = Vec::new();
                                for i in 0..self.gen_point_count.max(1) {
                                    let i_f = i as f32 + 0.5; // center of bin
                                    let y_center = y_min + i_f * bin_height;
                                    let label = format!("Foil{}_Ybin{}", foil_id, i + 1);
                                    let point = ManualMeasurementPoint {
                                        x: x_center,
                                        y: y_center,
                                        width,
                                        height: box_height,
                                        direction: dir.clone(),
                                        label,
                                    };
                                    new_points.push(point);
                                }
                                self.manual_measurement_ui_config.points = new_points;
                                self.show_manual_measurements = true;
                            }
                        }
                    }
                }
            });
            
            // List of measurement points
            let mut point_to_delete = None;
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for (idx, point) in self.manual_measurement_ui_config.points.iter_mut().enumerate() {
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
                                
                                if ui.small_button("�").clicked() {
                                    point_to_delete = Some(idx);
                                }
                            });
                        });
                }
            });
            
            if let Some(idx) = point_to_delete {
                self.manual_measurement_ui_config.points.remove(idx);
                if self.manual_measurement_selected_point >= self.manual_measurement_ui_config.points.len() {
                    self.manual_measurement_selected_point = self.manual_measurement_ui_config.points.len().saturating_sub(1);
                }
            }
            
            // Add new point button
            if ui.button("➕ Add Measurement Point").clicked() {
                let mut new_point = ManualMeasurementPoint::default();
                new_point.label = format!("Measurement_{}", self.manual_measurement_ui_config.points.len() + 1);
                self.manual_measurement_ui_config.points.push(new_point);
                self.manual_measurement_selected_point = self.manual_measurement_ui_config.points.len() - 1;
            }
            
            // Edit selected point
            if !self.manual_measurement_ui_config.points.is_empty() && 
               self.manual_measurement_selected_point < self.manual_measurement_ui_config.points.len() {
                ui.separator();
                ui.label(RichText::new("Edit Selected Point").strong());
                
                let point = &mut self.manual_measurement_ui_config.points[self.manual_measurement_selected_point];
                
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
                    ui.add(egui::DragValue::new(&mut point.width).speed(1.0).clamp_range(10.0..=500.0));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut point.height).speed(1.0).clamp_range(10.0..=500.0));
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
            
            // Recording controls
            ui.horizontal(|ui| {
                if !is_recording {
                    if ui.button("▶️ Start Recording").clicked() {
                        // Send command to simulation thread
                        if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = tx.send(crate::renderer::state::SimCommand::StartManualMeasurement {
                                config: self.manual_measurement_ui_config.clone(),
                            });
                            self.show_manual_measurements = true;
                        }
                    }
                } else {
                    if ui.button("⏹️ Stop Recording").clicked() {
                        // Send command to simulation thread
                        if let Some(tx) = crate::renderer::state::SIM_COMMAND_SENDER.lock().as_ref() {
                            let _ = tx.send(crate::renderer::state::SimCommand::StopManualMeasurement);
                        }
                    }
                }
            });
            
            // Display last measurement results
            if !self.manual_measurement_last_results.is_empty() {
                ui.separator();
                ui.label(RichText::new("Latest Measurements:").strong());
                
                Grid::new("manual_measurement_results")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Label");
                        ui.label("Edge (Å)");
                        ui.label("Li Metal");
                        ui.label("Li Ion");
                        ui.end_row();
                        
                        for result in &self.manual_measurement_last_results {
                            ui.label(&result.label);
                            ui.label(format!("{:.2}", result.edge_position));
                            ui.label(format!("{}", result.li_metal_count));
                            ui.label(format!("{}", result.li_ion_count));
                            ui.end_row();
                        }
                    });
            }
        });
        
        ui.separator();
        
        // Original Manual Measurement Tool
        ui.heading("�📏 Manual Measurement Tool");
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
                let label = if self.measurement_direction.is_some() { "Projected distance" } else { "Current distance" };
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
            ui.small(format!("Direction set: ({:.2}, {:.2}) — measurements are projected onto this axis.", dir.x, dir.y));
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
                                ui.label(record.switch_step.map(|v| v.to_string()).unwrap_or_else(|| "".into()));
                                ui.label(record.switch_mode.clone().unwrap_or_default());
                                ui.label(record.switch_value.map(|v| format!("{:.3}", v)).unwrap_or_default());
                                ui.label(record.pos_role.clone().unwrap_or_default());
                                ui.label(record.neg_role.clone().unwrap_or_default());
                                ui.end_row();
                            }
                        });
                });
        }
    }
}
