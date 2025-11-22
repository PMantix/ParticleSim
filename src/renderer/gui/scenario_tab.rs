use super::*;
use crate::renderer::{ComponentMode, ElectrolyteComponent};
use std::collections::HashSet;

impl super::super::Renderer {
    pub fn show_scenario_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🌐 Scenario & Domain");
        ui.group(|ui| {
            ui.label("️ Delete Particles");
            ui.horizontal(|ui| {
                ui.label("Delete:");
                egui::ComboBox::from_id_source("delete_species_combo")
                    .selected_text(match self.selected_delete_option {
                        crate::renderer::DeleteOption::AllSpecies => "All Species",
                        crate::renderer::DeleteOption::LithiumIon => "Li+ Ions",
                        crate::renderer::DeleteOption::LithiumMetal => "Li Metal",
                        crate::renderer::DeleteOption::FoilMetal => "Foil Metal",
                        crate::renderer::DeleteOption::ElectrolyteAnion => "Anions",
                        crate::renderer::DeleteOption::EC => "EC",
                        crate::renderer::DeleteOption::DMC => "DMC",
                        crate::renderer::DeleteOption::VC => "VC",
                        crate::renderer::DeleteOption::FEC => "FEC",
                        crate::renderer::DeleteOption::EMC => "EMC",
                        crate::renderer::DeleteOption::LLZO => "LLZO",
                        crate::renderer::DeleteOption::LLZT => "LLZT",
                        crate::renderer::DeleteOption::S40B => "S40B",
                        crate::renderer::DeleteOption::SEI => "SEI",
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
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::EC,
                            "EC",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::DMC,
                            "DMC",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::VC,
                            "VC",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::FEC,
                            "FEC",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::EMC,
                            "EMC",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::LLZO,
                            "LLZO",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::LLZT,
                            "LLZT",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::S40B,
                            "S40B",
                        );
                        ui.selectable_value(
                            &mut self.selected_delete_option,
                            crate::renderer::DeleteOption::SEI,
                            "SEI",
                        );
                    });
            });
            ui.horizontal(|ui| {
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
                        crate::renderer::DeleteOption::EC => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::EC,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::DMC => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::DMC,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::VC => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::VC,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::FEC => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::FEC,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::EMC => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::EMC,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::LLZO => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::LLZO,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::LLZT => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::LLZT,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::S40B => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::S40B,
                                })
                                .unwrap();
                        }
                        crate::renderer::DeleteOption::SEI => {
                            SIM_COMMAND_SENDER
                                .lock()
                                .as_ref()
                                .unwrap()
                                .send(SimCommand::DeleteSpecies {
                                    species: Species::SEI,
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
                let drag_value = ui.add(
                    egui::DragValue::new(&mut domain_width)
                        .speed(10.0)
                        .clamp_range(100.0..=5000.0),
                );

                if drag_value.changed() {
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
                let drag_value = ui.add(
                    egui::DragValue::new(&mut domain_height)
                        .speed(10.0)
                        .clamp_range(100.0..=5000.0),
                );

                if drag_value.changed() {
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
                        ui.selectable_value(&mut self.scenario_species, Species::EC, "EC");
                        ui.selectable_value(&mut self.scenario_species, Species::DMC, "DMC");
                        ui.selectable_value(&mut self.scenario_species, Species::VC, "VC");
                        ui.selectable_value(&mut self.scenario_species, Species::FEC, "FEC");
                        ui.selectable_value(&mut self.scenario_species, Species::EMC, "EMC");
                        ui.selectable_value(&mut self.scenario_species, Species::LLZO, "LLZO");
                        ui.selectable_value(&mut self.scenario_species, Species::LLZT, "LLZT");
                        ui.selectable_value(&mut self.scenario_species, Species::S40B, "S40B");
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

            // Electrolyte solution controls
            ui.separator();
            ui.label("🧪 Electrolyte Solution");
            ui.horizontal(|ui| {
                ui.label("Molarity:");
                ui.add(
                    egui::DragValue::new(&mut self.electrolyte_molarity)
                        .speed(0.1)
                        .clamp_range(0.05..=5.0),
                );
                ui.label("M LiPF6");
            });
            ui.horizontal(|ui| {
                ui.label("Total particles:");
                ui.add(
                    egui::DragValue::new(&mut self.electrolyte_total_particles)
                        .speed(25.0)
                        .clamp_range(0..=50_000),
                );
            });
            ui.add_space(6.0);
            show_electrolyte_component_table(ui, self);

            let plan = compute_electrolyte_plan(self);
            ui.add_space(6.0);
            show_electrolyte_plan_preview(ui, &plan, self.electrolyte_total_particles);

            let can_spawn = plan.is_actionable(self.electrolyte_total_particles);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_spawn,
                        egui::Button::new("Add Electrolyte Mixture"),
                    )
                    .on_hover_text(
                        "Adds every enabled component using the counts in the preview above.",
                    )
                    .clicked()
                {
                    spawn_electrolyte_plan(self, &plan);
                }

                if ui
                    .add_enabled(
                        can_spawn,
                        egui::Button::new("Rebalance Electrolyte"),
                    )
                    .on_hover_text(
                        "Deletes the currently enabled electrolyte species before re-adding them with the new composition.",
                    )
                    .clicked()
                {
                    delete_plan_species(&plan);
                    spawn_electrolyte_plan(self, &plan);
                }
            });
        });

        ui.separator();

        // Save/Load State
        ui.group(|ui| {
            ui.label("💾 Save/Load State");
            ui.vertical(|ui| {
                // --- Save State UI ---
                use std::fs;
                use std::path::PathBuf;
                let saved_state_dir = PathBuf::from("saved_state");
                let _ = fs::create_dir_all(&saved_state_dir);
                let mut state_files: Vec<String> = fs::read_dir(&saved_state_dir)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .filter(|e| {
                                if let Some(name) = e.file_name().to_str() {
                                    name.ends_with(".json") || name.ends_with(".json.gz") || name.ends_with(".bin") || name.ends_with(".bin.gz")
                                } else { false }
                            })
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                state_files.sort();

                // Save format (own line)
                {
                    use crate::renderer::state::SaveFormat;
                    let mut fmt = *crate::renderer::state::SAVE_FORMAT.lock();
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        egui::ComboBox::from_id_source("save_format_combo")
                            .selected_text(match fmt { SaveFormat::Json => "JSON", SaveFormat::Binary => "Binary" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut fmt, SaveFormat::Json, "JSON");
                                ui.selectable_value(&mut fmt, SaveFormat::Binary, "Binary");
                            });
                    });
                    *crate::renderer::state::SAVE_FORMAT.lock() = fmt;
                }

                // Compression (own line)
                {
                    let mut compress = *crate::renderer::state::SAVE_COMPRESS.lock();
                    let mut uncompressed = !compress;
                    if ui.checkbox(&mut uncompressed, "Save uncompressed (debug)")
                        .on_hover_text("Unchecked = gzip compress saves (default). Checked = plain JSON.")
                        .changed()
                    {
                        compress = !uncompressed;
                        *crate::renderer::state::SAVE_COMPRESS.lock() = compress;
                    }
                }

                // Include history (own line)
                {
                    let mut include_history = *crate::renderer::state::SAVE_INCLUDE_HISTORY.lock();
                    if ui.checkbox(&mut include_history, "Include history")
                        .on_hover_text("If unchecked, only the current frame is saved (much smaller/faster). History playback data omitted.")
                        .changed()
                    {
                        *crate::renderer::state::SAVE_INCLUDE_HISTORY.lock() = include_history;
                    }
                }

                // Save name and button (own line)
                ui.horizontal(|ui| {
                    ui.label("Save as:");
                    let save_name = &mut self.save_state_name;
                    let save_clicked = ui.text_edit_singleline(save_name).lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Save State").clicked() || save_clicked {
                        let mut name = save_name.trim().to_string();
                        let fmt = *crate::renderer::state::SAVE_FORMAT.lock();
                        let compress = *crate::renderer::state::SAVE_COMPRESS.lock();
                        if name.is_empty() {
                            let mut idx = 1;
                            loop {
                                let candidate = format!("save_{:02}.{}", idx, fmt.extension(compress));
                                if !state_files.iter().any(|f| f == &candidate) { name = candidate; break; }
                                idx += 1;
                            }
                        } else {
                            for suf in [".json.gz", ".json", ".bin.gz", ".bin"] { if name.ends_with(suf) { name = name.trim_end_matches(suf).to_string(); break; } }
                            name = format!("{name}.{}", fmt.extension(compress));
                        }
                        let path = saved_state_dir.join(&name);
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::SaveState { path: path.to_string_lossy().to_string() }).unwrap();
                        self.save_state_name.clear();
                    }
                });

                // Load (stacked vertically)
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("Load:");
                    let selected = &mut self.load_state_selected;
                    egui::ComboBox::from_id_source("load_state_combo")
                        .selected_text(selected.as_deref().unwrap_or("Select state"))
                        .show_ui(ui, |ui| { for file in &state_files { ui.selectable_value(selected, Some(file.clone()), file); } });
                    if ui.button("Load State").clicked() {
                        if let Some(selected_file) = selected.clone() {
                            let path = saved_state_dir.join(selected_file);
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::LoadState { path: path.to_string_lossy().to_string() }).unwrap();
                        }
                    }
                });
            });
        });
    }
}

const ELECTROLYTE_SPECIES: &[Species] = &[
    Species::EC,
    Species::DMC,
    Species::VC,
    Species::FEC,
    Species::EMC,
    Species::LLZO,
    Species::LLZT,
    Species::S40B,
];

struct ComponentPlanEntry {
    species: Species,
    normalized_weight: f32,
    count: usize,
}

struct ElectrolytePlan {
    entries: Vec<ComponentPlanEntry>,
}

impl ElectrolytePlan {
    fn is_actionable(&self, total_particles: usize) -> bool {
        total_particles > 0 && self.entries.iter().any(|entry| entry.count > 0)
    }
}

fn show_electrolyte_component_table(ui: &mut egui::Ui, renderer: &mut super::super::Renderer) {
    ui.group(|ui| {
        ui.label("Composition Components");
        let mut remove_idx = None;
        egui::Grid::new("electrolyte_component_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Species");
                ui.label("Mode");
                ui.label("Value");
                ui.label("Result");
                ui.label(" ");
                ui.end_row();

                for (idx, component) in renderer.electrolyte_components.iter_mut().enumerate() {
                    egui::ComboBox::from_id_source(format!("component_species_{idx}"))
                        .selected_text(species_display_name(component.species))
                        .show_ui(ui, |ui| {
                            for species in ELECTROLYTE_SPECIES {
                                ui.selectable_value(
                                    &mut component.species,
                                    *species,
                                    species_display_name(*species),
                                );
                            }
                        });

                    egui::ComboBox::from_id_source(format!("component_mode_{idx}"))
                        .selected_text(match component.mode {
                            ComponentMode::Fraction => "Fraction",
                            ComponentMode::Part => "Part",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut component.mode,
                                ComponentMode::Fraction,
                                "Fraction",
                            );
                            ui.selectable_value(&mut component.mode, ComponentMode::Part, "Part");
                        });

                    ui.add(
                        egui::DragValue::new(&mut component.input_value)
                            .speed(0.01)
                            .clamp_range(0.0..=100.0),
                    );

                    ui.label(format!("{:.1}%", component.fraction * 100.0));

                    if ui.button("Remove").clicked() {
                        remove_idx = Some(idx);
                    }
                    ui.end_row();
                }
            });

        if let Some(idx) = remove_idx {
            renderer.electrolyte_components.remove(idx);
        }

        if ui.button("+ Add component").clicked() {
            renderer
                .electrolyte_components
                .push(default_component_template());
        }

        recalculate_fractions(&mut renderer.electrolyte_components);
    });
}

fn default_component_template() -> ElectrolyteComponent {
    ElectrolyteComponent {
        species: Species::EC,
        fraction: 1.0,
        mode: ComponentMode::Part,
        input_value: 1.0,
    }
}

fn recalculate_fractions(components: &mut [ElectrolyteComponent]) {
    // 1. Sum explicit fractions
    let sum_fractions: f32 = components
        .iter()
        .filter(|c| matches!(c.mode, ComponentMode::Fraction))
        .map(|c| c.input_value)
        .sum();

    // 2. Handle Fraction overflow
    if sum_fractions >= 1.0 {
        // Normalize fractions to sum to 1.0
        let scale = if sum_fractions > 0.0 {
            1.0 / sum_fractions
        } else {
            0.0
        };
        for c in components.iter_mut() {
            if matches!(c.mode, ComponentMode::Fraction) {
                c.fraction = c.input_value * scale;
            } else {
                c.fraction = 0.0;
            }
        }
        return;
    }

    // 3. Distribute remainder to Parts
    let remaining_fraction = 1.0 - sum_fractions;
    let sum_parts: f32 = components
        .iter()
        .filter(|c| matches!(c.mode, ComponentMode::Part))
        .map(|c| c.input_value)
        .sum();

    if sum_parts > 0.0 {
        let part_scale = remaining_fraction / sum_parts;
        for c in components.iter_mut() {
            if matches!(c.mode, ComponentMode::Part) {
                c.fraction = c.input_value * part_scale;
            } else if matches!(c.mode, ComponentMode::Fraction) {
                c.fraction = c.input_value;
            }
        }
    } else {
        // No parts to distribute to, just set fractions as is
        for c in components.iter_mut() {
            if matches!(c.mode, ComponentMode::Fraction) {
                c.fraction = c.input_value;
            } else {
                c.fraction = 0.0;
            }
        }
    }
}

fn compute_electrolyte_plan(renderer: &super::super::Renderer) -> ElectrolytePlan {
    let total_particles = renderer.electrolyte_total_particles as f32;
    let molarity = renderer.electrolyte_molarity;

    // 1. Calculate Solvent vs Salt split
    // Heuristic: 1.0 M => 1/15 ratio of salt to solvent molecules
    // S = total / (1 + 2 * M / 15)
    let solvent_count_f32 = total_particles / (1.0 + 2.0 * molarity / 15.0);
    let salt_count_f32 = solvent_count_f32 * (molarity / 15.0);

    let li_count = salt_count_f32.round() as usize;
    let anion_count = salt_count_f32.round() as usize;
    let total_solvent_count = solvent_count_f32.round() as usize;

    let mut entries = Vec::new();

    // Add Ions
    if li_count > 0 {
        entries.push(ComponentPlanEntry {
            species: Species::LithiumIon,
            normalized_weight: 0.0, // Not used for ions in this logic
            count: li_count,
        });
    }
    if anion_count > 0 {
        entries.push(ComponentPlanEntry {
            species: Species::ElectrolyteAnion,
            normalized_weight: 0.0,
            count: anion_count,
        });
    }

    // 2. Distribute Solvent
    let enabled_solvents: Vec<_> = renderer
        .electrolyte_components
        .iter()
        .filter(|c| c.fraction > 0.0)
        .collect();

    if !enabled_solvents.is_empty() && total_solvent_count > 0 {
        let total_fraction: f32 = enabled_solvents.iter().map(|c| c.fraction).sum();

        let mut solvent_entries = Vec::new();
        for comp in &enabled_solvents {
            let weight = comp.fraction / total_fraction;
            let exact = weight * total_solvent_count as f32;
            let count = exact.floor() as usize;
            solvent_entries.push(ComponentPlanEntry {
                species: comp.species,
                normalized_weight: weight,
                count,
            });
        }

        let current_allocated: usize = solvent_entries.iter().map(|e| e.count).sum();
        let mut remainder = total_solvent_count.saturating_sub(current_allocated);

        // Simple distribution of remainder
        for entry in solvent_entries.iter_mut() {
            if remainder == 0 {
                break;
            }
            entry.count += 1;
            remainder -= 1;
        }

        entries.extend(solvent_entries);
    }

    ElectrolytePlan { entries }
}

fn show_electrolyte_plan_preview(
    ui: &mut egui::Ui,
    plan: &ElectrolytePlan,
    total_particles: usize,
) {
    ui.group(|ui| {
        ui.label("Distribution Preview");
        if plan.entries.is_empty() {
            ui.label("Enable at least one component with a non-zero fraction to preview counts.");
            if total_particles == 0 {
                ui.label("Total particles must also be greater than zero to spawn the mixture.");
            }
            return;
        }

        for entry in &plan.entries {
            ui.horizontal(|ui| {
                ui.label(species_display_name(entry.species));
                if entry.normalized_weight > 0.0 {
                    ui.label(format!("{:.1}%", entry.normalized_weight * 100.0));
                }
                if total_particles > 0 {
                    ui.label(format!("{} particles", entry.count));
                }
            });
        }

        if total_particles > 0 {
            let planned: usize = plan.entries.iter().map(|entry| entry.count).sum();
            ui.label(format!("Planned total: {} / {}", planned, total_particles));
        } else {
            ui.label("Increase Total particles to > 0 to enable spawning.");
        }
    });
}

fn spawn_electrolyte_plan(
    renderer: &mut super::super::Renderer,
    plan: &ElectrolytePlan,
) {
    if plan.entries.is_empty() {
        return;
    }

    let sender_opt = SIM_COMMAND_SENDER.lock().clone();
    let Some(tx) = sender_opt else { return };

    for entry in &plan.entries {
        if entry.count == 0 {
            continue;
        }
        let body = make_body_with_species(
            ultraviolet::Vec2::zero(),
            ultraviolet::Vec2::zero(),
            entry.species,
        );
        let _ = tx.send(SimCommand::AddRandom {
            body,
            count: entry.count,
            domain_width: renderer.domain_width,
            domain_height: renderer.domain_height,
        });
    }

    let total_added: usize = plan.entries.iter().map(|entry| entry.count).sum();
    eprintln!(
        "[Electrolyte] Added {} particles: {}",
        total_added,
        plan
            .entries
            .iter()
            .filter(|entry| entry.count > 0)
            .map(|entry| format!(
                "{}={}",
                species_display_name(entry.species),
                entry.count
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn delete_plan_species(plan: &ElectrolytePlan) {
    let mut species: HashSet<Species> = HashSet::new();
    for entry in &plan.entries {
        species.insert(entry.species);
    }
    if species.is_empty() {
        return;
    }
    let sender_opt = SIM_COMMAND_SENDER.lock().clone();
    let Some(tx) = sender_opt else { return };
    for species in species {
        let _ = tx.send(SimCommand::DeleteSpecies { species });
    }
}

fn species_display_name(species: Species) -> &'static str {
    match species {
        Species::LithiumIon => "Li+",
        Species::LithiumMetal => "Li Metal",
        Species::FoilMetal => "Foil Metal",
        Species::ElectrolyteAnion => "PF6-",
        Species::EC => "EC",
        Species::DMC => "DMC",
        Species::VC => "VC",
        Species::FEC => "FEC",
        Species::EMC => "EMC",
        Species::LLZO => "LLZO",
        Species::LLZT => "LLZT",
        Species::S40B => "S40B",
        Species::SEI => "SEI",
    }
}

pub fn make_body_with_species(pos: Vec2, vel: Vec2, species: Species) -> Body {
    use crate::config::{FOIL_NEUTRAL_ELECTRONS, LITHIUM_METAL_NEUTRAL_ELECTRONS};
    // Always use species properties for mass and radius to ensure consistency
    let mut body = Body::new(pos, vel, species.mass(), species.radius(), 0.0, species);
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
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 1 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS - 1) {
                    body.electrons.push(Electron {
                        rel_pos: Vec2::zero(),
                        vel: Vec2::zero(),
                    });
                }
            }
            // If LITHIUM_METAL_NEUTRAL_ELECTRONS is 1, then Li+ has 0 electrons (which is correct)
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
        Species::EC | Species::DMC | Species::VC | Species::FEC | Species::EMC => {
            // Neutral solvent molecules with a single drifting electron
            body.electrons.push(Electron {
                rel_pos: Vec2::zero(),
                vel: Vec2::zero(),
            });
        }
        Species::LLZO | Species::LLZT | Species::S40B => {
            // Solid electrolytes treated like neutral bodies without drift electrons
        }
        Species::SEI => {
            // SEI is neutral and has no electrons
        }
    }
    body.update_charge_from_electrons();
    body.update_species();
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_body_with_species_uses_correct_radius() {
        let ion = make_body_with_species(Vec2::zero(), Vec2::zero(), Species::LithiumIon);
        let metal = make_body_with_species(Vec2::zero(), Vec2::zero(), Species::LithiumMetal);

        assert_eq!(ion.radius, Species::LithiumIon.radius());
        assert_eq!(ion.mass, Species::LithiumIon.mass());
        assert_eq!(ion.species, Species::LithiumIon);

        assert_eq!(metal.radius, Species::LithiumMetal.radius());
        assert_eq!(metal.mass, Species::LithiumMetal.mass());
        assert_eq!(metal.species, Species::LithiumMetal);

        // Should be different radii
        assert_ne!(ion.radius, metal.radius);
    }
}
