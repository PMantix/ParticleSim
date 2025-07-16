use super::*;

impl super::Renderer {
    fn show_scenario_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🌐 Scenario & Domain");

        // Delete Controls
        ui.group(|ui| {
            ui.label("🗑️ Delete Particles");
            ui.horizontal(|ui| {
                ui.label("Delete:");
                egui::ComboBox::from_id_source("delete_species_combo")
                    .selected_text(match self.selected_delete_option {
                        crate::renderer::DeleteOption::AllSpecies => "All Species",
                        crate::renderer::DeleteOption::LithiumIon => "Li+ Ions",
                        crate::renderer::DeleteOption::LithiumMetal => "Li Metal",
                        crate::renderer::DeleteOption::FoilMetal => "Foil Metal",
                        crate::renderer::DeleteOption::ElectrolyteAnion => "Anions",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::AllSpecies,
                            "All Species",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::LithiumIon,
                            "Li+ Ions",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::LithiumMetal,
                            "Li Metal",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::FoilMetal,
                            "Foil Metal",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::ElectrolyteAnion,
                            "Anions",
                        );
                    });

                // Delete button that actually performs the deletion
                if ui.button("Delete").clicked() {
                    match self.selected_delete_option {
                        crate::renderer::DeleteOption::AllSpecies => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteAll)
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::LithiumIon => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::LithiumIon,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::LithiumMetal => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::LithiumMetal,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::FoilMetal => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::FoilMetal,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::ElectrolyteAnion => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::ElectrolyteAnion,
                                })
                                .unwrap();
                        }
                    }
                }
            });
        });

        ui.separator();

        // Domain Size Controls
        ui.group(|ui| {
            ui.label("🌐 Computational Domain");
            ui.horizontal(|ui| {
                ui.label("Width:");
                let mut domain_width = self.domain_width;
                if ui
                    .add(
                        egui::DragValue::new(&mut domain_width)
                            .speed(10.0)
                            .clamp_range(100.0..=5000.0),
                    )
                    .changed()
                {
                    self.domain_width = domain_width;
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::SetDomainSize {
                            width: self.domain_width,
                            height: self.domain_height,
                        })
                        .unwrap();
                }
                ui.label("Height:");
                let mut domain_height = self.domain_height;
                if ui
                    .add(
                        egui::DragValue::new(&mut domain_height)
                            .speed(10.0)
                            .clamp_range(100.0..=5000.0),
                    )
                    .changed()
                {
                    self.domain_height = domain_height;
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::SetDomainSize {
                            width: self.domain_width,
                            height: self.domain_height,
                        })
                        .unwrap();
                }
            });
            ui.label("⚠️ Particles outside domain will be removed");
        });

        ui.separator();

        // Add Particles Controls
        ui.group(|ui| {
            ui.label("➕ Add Particles");

            // Common controls for all Add scenarios
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                egui::ComboBox::from_label("Species")
                    .selected_text(format!("{:?}", self.scenario_species))
                    .show_ui(ui, |ui| {
                        use crate::renderer::Species;
                        ui.selectable_value(
                            &mut self.scenario_species,
                            Species::LithiumMetal,
                            "Lithium Metal",
                        );
                        ui.selectable_value(
                            &mut self.scenario_species,
                            Species::LithiumIon,
                            "Lithium Ion",
                        );
                        ui.selectable_value(
                            &mut self.scenario_species,
                            Species::ElectrolyteAnion,
                            "Electrolyte Anion",
                        );
                    });
            });

            // Common Width/Height controls (used by Rectangle and Foil)
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut self.scenario_width).speed(0.1));
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut self.scenario_height).speed(0.1));
            });

            // Add Ring / Filled Circle
            ui.horizontal(|ui| {
                ui.label("Radius:");
                ui.add(egui::DragValue::new(&mut self.scenario_radius).speed(0.1));
                if ui.button("Add Ring").clicked() {
                    let spec = self.scenario_species;
                    let body = make_body_with_species(
                        ultraviolet::Vec2::zero(),
                        ultraviolet::Vec2::zero(),
                        spec.mass(),
                        spec.radius(),
                        spec,
                    );
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::AddRing {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius,
                        })
                        .unwrap();
                }
                if ui.button("Add Filled Circle").clicked() {
                    let spec = self.scenario_species;
                    let body = make_body_with_species(
                        ultraviolet::Vec2::zero(),
                        ultraviolet::Vec2::zero(),
                        spec.mass(),
                        spec.radius(),
                        spec,
                    );
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::AddCircle {
                            body,
                            x: self.scenario_x,
                            y: self.scenario_y,
                            radius: self.scenario_radius,
                        })
                        .unwrap();
                }
            });

            // Add Rectangle and Add Foil (using common width/height)
            ui.horizontal(|ui| {
                if ui.button("Add Rectangle").clicked() {
                    let spec = self.scenario_species;
                    let body = make_body_with_species(
                        ultraviolet::Vec2::zero(),
                        ultraviolet::Vec2::zero(),
                        spec.mass(),
                        spec.radius(),
                        spec,
                    );
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::AddRectangle {
                            body,
                            x: self.scenario_x - self.scenario_width / 2.0,
                            y: self.scenario_y - self.scenario_height / 2.0,
                            width: self.scenario_width,
                            height: self.scenario_height,
                        })
                        .unwrap();
                }
                if ui.button("Add Foil").clicked() {
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::AddFoil {
                            width: self.scenario_width,
                            height: self.scenario_height,
                            x: self.scenario_x - self.scenario_width / 2.0,
                            y: self.scenario_y - self.scenario_height / 2.0,
                            particle_radius: Species::FoilMetal.radius(),
                            current: 0.0, // Always start with 0 current
                        })
                        .unwrap();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Count:");
                ui.add(egui::DragValue::new(&mut self.scenario_random_count).speed(1.0));
                if ui.button("Add Random").clicked() {
                    let spec = self.scenario_species;
                    let body = make_body_with_species(
                        ultraviolet::Vec2::zero(),
                        ultraviolet::Vec2::zero(),
                        spec.mass(),
                        spec.radius(),
                        spec,
                    );
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::AddRandom {
                            body,
                            count: self.scenario_random_count,
                            domain_width: self.domain_width,
                            domain_height: self.domain_height,
                        })
                        .unwrap();
                }
            });
        });

        ui.separator();

        // Save/Load State
        ui.group(|ui| {
            ui.label("💾 Save/Load State");
            ui.horizontal(|ui| {
                // --- Save State UI ---
                use std::fs;
                use std::path::PathBuf;
                let saved_state_dir = PathBuf::from("saved_state");
                // Ensure directory exists
                let _ = fs::create_dir_all(&saved_state_dir);
                // List all .json files in saved_state
                let mut state_files: Vec<String> = fs::read_dir(&saved_state_dir)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .map(|ext| ext == "json")
                                    .unwrap_or(false)
                            })
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                state_files.sort();

                // Save name input
                ui.label("Save as:");
                let save_name = &mut self.save_state_name;
                let save_clicked = ui.text_edit_singleline(save_name).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Save State").clicked() || save_clicked {
                    // If no name, auto-increment
                    let mut name = save_name.trim().to_string();
                    if name.is_empty() {
                        // Find next available save_XX.json
                        let mut idx = 1;
                        loop {
                            let candidate = format!("save_{:02}.json", idx);
                            if !state_files.iter().any(|f| f == &candidate) {
                                name = candidate;
                                break;
                            }
                            idx += 1;
                        }
                    } else if !name.ends_with(".json") {
                        name.push_str(".json");
                    }
                    let path = saved_state_dir.join(&name);
                    SIM_COMMAND_SENDER
                        .lock()
                        .as_ref()
                        .unwrap()
                        .send(SimCommand::SaveState {
                            path: path.to_string_lossy().to_string(),
                        })
                        .unwrap();
                    self.save_state_name.clear();
                }

                // --- Load State UI ---
                ui.label("Load:");
                let selected = &mut self.load_state_selected;
                egui::ComboBox::from_id_source("load_state_combo")
                    .selected_text(selected.as_deref().unwrap_or("Select state"))
                    .show_ui(ui, |ui| {
                        for file in &state_files {
                            ui.selectable_value(selected, Some(file.clone()), file);
                        }
                    });
                if ui.button("Load State").clicked() {
                    if let Some(selected_file) = selected.clone() {
                        let path = saved_state_dir.join(selected_file);
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::LoadState {
                                path: path.to_string_lossy().to_string(),
                            })
                            .unwrap();
                    }
                }
            });
        });
    }
}
pub fn make_body_with_species(
    pos: Vec2,
    vel: Vec2,
    mass: f32,
    radius: f32,
    species: Species,
) -> Body {
    use crate::config::{FOIL_NEUTRAL_ELECTRONS, LITHIUM_METAL_NEUTRAL_ELECTRONS};
    let mut body = Body::new(pos, vel, mass, radius, 0.0, species);
    body.electrons.clear();
    match species {
        Species::LithiumMetal => {
            for _ in 0..LITHIUM_METAL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron {
                    rel_pos: Vec2::zero(),
                    vel: Vec2::zero(),
                });
            }
        }
        Species::FoilMetal => {
            for _ in 0..FOIL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron {
                    rel_pos: Vec2::zero(),
                    vel: Vec2::zero(),
                });
            }
        }
        Species::LithiumIon => {
            // Ions: one less electron than neutral metal, positive charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS - 1) {
                    body.electrons.push(Electron {
                        rel_pos: Vec2::zero(),
                        vel: Vec2::zero(),
                    });
                }
            }
        }
        Species::ElectrolyteAnion => {
            // Anions: one more electron than neutral metal, negative charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS + 1) {
                    body.electrons.push(Electron {
                        rel_pos: Vec2::zero(),
                        vel: Vec2::zero(),
                    });
                }
            }
        }
    }
    body.update_charge_from_electrons();
    body.update_species();
    body
}
