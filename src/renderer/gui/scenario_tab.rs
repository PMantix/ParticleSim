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
                        .clamp_range(0.0..=5.0),
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
                        "Deletes ALL electrolyte species (ions, solvents, solids) before re-adding the new composition.",
                    )
                    .clicked()
                {
                    delete_all_electrolyte();
                    spawn_electrolyte_plan(self, &plan);
                }

                if ui
                    .button("Delete Electrolyte")
                    .on_hover_text("Removes all electrolyte particles, ions, and SEI.")
                    .clicked()
                {
                    delete_all_electrolyte();
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
    charge_override: Option<f32>,
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
                ui.label("Stoich.");
                ui.label(" ");
                ui.end_row();

                for (idx, component) in renderer.electrolyte_components.iter_mut().enumerate() {
                    egui::ComboBox::from_id_source(format!("component_species_{idx}"))
                        .selected_text(species_display_name(component.species))
                        .show_ui(ui, |ui| {
                            for species in ELECTROLYTE_SPECIES {
                                if ui.selectable_value(
                                    &mut component.species,
                                    *species,
                                    species_display_name(*species),
                                )
                                .changed()
                                {
                                    component.lithium_stoichiometry = match species {
                                        Species::LLZO | Species::LLZT | Species::S40B => 1.0,
                                        _ => 0.0,
                                    };
                                }
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

                    if matches!(
                        component.species,
                        Species::LLZO | Species::LLZT | Species::S40B
                    ) {
                        ui.add(
                            egui::DragValue::new(&mut component.lithium_stoichiometry)
                                .speed(0.1)
                                .clamp_range(0.0..=20.0),
                        );
                    } else {
                        ui.label("-");
                    }

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
        lithium_stoichiometry: 0.0,
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
    let total_particles = renderer.electrolyte_total_particles;
    let molarity = renderer.electrolyte_molarity;

    // 1. Calculate Solvent vs Salt split
    // Heuristic: 1.0 M => 1/15 ratio of salt to solvent molecules
    let solvent_to_salt_ratio = 15.0;
    let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
    let lipf6_count = (total_particles as f32 * salt_fraction * molarity / 1.0).round() as usize;

    let li_count = lipf6_count;
    let anion_count = lipf6_count;
    let total_solvent_count = total_particles.saturating_sub(li_count + anion_count);

    let mut entries = Vec::new();

    // Add Ions
    if li_count > 0 {
        entries.push(ComponentPlanEntry {
            species: Species::LithiumIon,
            normalized_weight: 0.0, // Not used for ions in this logic
            count: li_count,
            charge_override: None,
        });
    }
    if anion_count > 0 {
        entries.push(ComponentPlanEntry {
            species: Species::ElectrolyteAnion,
            normalized_weight: 0.0,
            count: anion_count,
            charge_override: None,
        });
    }

    // 2. Distribute Solvent
    let enabled_solvents: Vec<_> = renderer
        .electrolyte_components
        .iter()
        .filter(|c| c.fraction > 0.0)
        .collect();

    if !enabled_solvents.is_empty() && total_solvent_count > 0 {
        // Calculate expansion factor due to solid electrolyte stoichiometry
        // Total Particles = Base Particles + (Base Particles * Stoichiometry)
        // Expansion Factor = 1 + Stoichiometry
        // Weighted Average Expansion = Sum(Fraction * (1 + Stoich))
        let total_fraction: f32 = enabled_solvents.iter().map(|c| c.fraction).sum();
        let weighted_expansion: f32 = enabled_solvents
            .iter()
            .map(|c| {
                let stoich = if matches!(c.species, Species::LLZO | Species::LLZT | Species::S40B) {
                    c.lithium_stoichiometry
                } else {
                    0.0
                };
                (c.fraction / total_fraction) * (1.0 + stoich)
            })
            .sum();

        // Adjust the available count for base particles
        let adjusted_solvent_count = if weighted_expansion > 1.0 {
            (total_solvent_count as f32 / weighted_expansion).floor() as usize
        } else {
            total_solvent_count
        };

        // Check if all solvents are in Part mode (volume-based)
        let all_parts_mode = enabled_solvents
            .iter()
            .all(|c| matches!(c.mode, ComponentMode::Part));

        let mut solvent_entries = Vec::new();
        let mut extra_li_count = 0;

        if all_parts_mode {
            // Use volume-based calculation for Part mode
            let solvent_parts: Vec<_> = enabled_solvents
                .iter()
                .map(|c| (c.species, c.input_value))
                .collect();

            let particle_counts = crate::species::calculate_solvent_particle_counts(
                &solvent_parts,
                adjusted_solvent_count,
            );

            for (species, count) in particle_counts {
                let comp = enabled_solvents
                    .iter()
                    .find(|c| c.species == species)
                    .unwrap();
                let weight = comp.fraction / total_fraction;

                let mut charge_override = None;
                if matches!(species, Species::LLZO | Species::LLZT | Species::S40B) {
                    let stoich = comp.lithium_stoichiometry;
                    if stoich > 0.0 {
                        let extra = (count as f32 * stoich).round() as usize;
                        extra_li_count += extra;
                        charge_override = Some(-1.0 * stoich);
                    }
                }

                solvent_entries.push(ComponentPlanEntry {
                    species,
                    normalized_weight: weight,
                    count,
                    charge_override,
                });
            }
        } else {
            // Mixed mode or all Fraction mode
            // Separate components by mode
            let fraction_components: Vec<_> = enabled_solvents
                .iter()
                .filter(|c| matches!(c.mode, ComponentMode::Fraction))
                .collect();
            let part_components: Vec<_> = enabled_solvents
                .iter()
                .filter(|c| matches!(c.mode, ComponentMode::Part))
                .collect();

            // First, allocate particles for Fraction mode components
            let mut particles_for_parts = adjusted_solvent_count;

            for comp in &fraction_components {
                let count = (comp.fraction * adjusted_solvent_count as f32).round() as usize;
                particles_for_parts = particles_for_parts.saturating_sub(count);

                let mut charge_override = None;
                if matches!(comp.species, Species::LLZO | Species::LLZT | Species::S40B) {
                    let stoich = comp.lithium_stoichiometry;
                    if stoich > 0.0 {
                        let extra = (count as f32 * stoich).round() as usize;
                        extra_li_count += extra;
                        charge_override = Some(-1.0 * stoich);
                    }
                }

                solvent_entries.push(ComponentPlanEntry {
                    species: comp.species,
                    normalized_weight: comp.fraction,
                    count,
                    charge_override,
                });
            }

            // Then, allocate remaining particles for Part mode components using volume-based calculation
            if !part_components.is_empty() && particles_for_parts > 0 {
                let solvent_parts: Vec<_> = part_components
                    .iter()
                    .map(|c| (c.species, c.input_value))
                    .collect();

                let particle_counts = crate::species::calculate_solvent_particle_counts(
                    &solvent_parts,
                    particles_for_parts,
                );

                for (species, count) in particle_counts {
                    let comp = part_components
                        .iter()
                        .find(|c| c.species == species)
                        .unwrap();

                    let mut charge_override = None;
                    if matches!(species, Species::LLZO | Species::LLZT | Species::S40B) {
                        let stoich = comp.lithium_stoichiometry;
                        if stoich > 0.0 {
                            let extra = (count as f32 * stoich).round() as usize;
                            extra_li_count += extra;
                            charge_override = Some(-1.0 * stoich);
                        }
                    }

                    solvent_entries.push(ComponentPlanEntry {
                        species,
                        normalized_weight: comp.fraction,
                        count,
                        charge_override,
                    });
                }
            }
        }

        entries.extend(solvent_entries);

        if extra_li_count > 0 {
            entries.push(ComponentPlanEntry {
                species: Species::LithiumIon,
                normalized_weight: 0.0,
                count: extra_li_count,
                charge_override: None,
            });
        }
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
        let mut body = make_body_with_species(
            ultraviolet::Vec2::zero(),
            ultraviolet::Vec2::zero(),
            entry.species,
        );

        if let Some(charge) = entry.charge_override {
            body.charge = charge;
        }

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

#[allow(dead_code)]
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

fn delete_all_electrolyte() {
    let sender_opt = SIM_COMMAND_SENDER.lock().clone();
    let Some(tx) = sender_opt else { return };

    let species_to_delete = [
        Species::LithiumIon,
        Species::ElectrolyteAnion,
        Species::EC,
        Species::DMC,
        Species::VC,
        Species::FEC,
        Species::EMC,
        Species::LLZO,
        Species::LLZT,
        Species::S40B,
        Species::SEI,
    ];

    for species in species_to_delete {
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
        // Intercalation electrode materials
        Species::Graphite => "Graphite",
        Species::HardCarbon => "Hard Carbon",
        Species::SiliconOxide => "SiOx",
        Species::LTO => "LTO",
        Species::LFP => "LFP",
        Species::LMFP => "LMFP",
        Species::NMC => "NMC",
        Species::NCA => "NCA",
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
        // Intercalation electrode materials - neutral, no electrons
        Species::Graphite | Species::HardCarbon | Species::SiliconOxide | Species::LTO
        | Species::LFP | Species::LMFP | Species::NMC | Species::NCA => {
            // Electrode materials are neutral solids
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
