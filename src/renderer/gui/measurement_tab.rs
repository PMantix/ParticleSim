use super::*;
use egui::{Grid, RichText};

impl super::super::Renderer {
    pub fn show_measurement_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("📏 Measurement Tool");
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
