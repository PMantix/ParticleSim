use super::*;

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
                ui.add(egui::DragValue::new(&mut self.electrolyte_molarity).speed(0.1));
                ui.label("M LiPF6");
            });
            ui.horizontal(|ui| {
                ui.label("Total particles:");
                ui.add(egui::DragValue::new(&mut self.electrolyte_total_particles).speed(10.0));
            });
            ui.horizontal(|ui| {
                if ui.button("Add Electrolyte (EC/DMC)").clicked() {
                    // Calculate particle counts based on molarity and proportions
                    let total = self.electrolyte_total_particles;

                    // For xM LiPF6 in EC/DMC (1:1 vol ratio):
                    // LiPF6 dissociates to Li+ + PF6-
                    // Typical EC:DMC ratio is 1:1 by volume
                    // Rough calculation: ~10-20 solvent molecules per salt molecule
                    let solvent_to_salt_ratio = 15.0; // EC+DMC molecules per LiPF6

                    let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
                    let lipf6_count = (total as f32 * salt_fraction * self.electrolyte_molarity
                        / 1.0)
                        .round() as usize;
                    let li_count = lipf6_count; // 1:1 stoichiometry
                    let pf6_count = lipf6_count; // 1:1 stoichiometry

                    let remaining = total.saturating_sub(li_count + pf6_count);
                    let ec_count = remaining / 2; // 1:1 EC:DMC
                    let dmc_count = remaining - ec_count;

                    // Add Li+ ions
                    if li_count > 0 {
                        let li_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::LithiumIon,
                        );
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::AddRandom {
                                body: li_body,
                                count: li_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            })
                            .unwrap();
                    }

                    // Add PF6- anions
                    if pf6_count > 0 {
                        let pf6_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::ElectrolyteAnion,
                        );
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::AddRandom {
                                body: pf6_body,
                                count: pf6_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            })
                            .unwrap();
                    }

                    // Add EC solvent
                    if ec_count > 0 {
                        let ec_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::EC,
                        );
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::AddRandom {
                                body: ec_body,
                                count: ec_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            })
                            .unwrap();
                    }

                    // Add DMC solvent
                    if dmc_count > 0 {
                        let dmc_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::DMC,
                        );
                        SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::AddRandom {
                                body: dmc_body,
                                count: dmc_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            })
                            .unwrap();
                    }

                    eprintln!(
                        "[Electrolyte] Added molarity={:.2}M: {} Li+, {} PF6-, {} EC, {} DMC (total {} particles)",
                        self.electrolyte_molarity, li_count, pf6_count, ec_count, dmc_count, total
                    );
                }
            });

            // Option to rebalance existing electrolyte mixture to current settings
            ui.horizontal(|ui| {
                if ui
                    .button("Rebalance Electrolyte (delete & re-add)")
                    .on_hover_text(
                        "Removes existing Li+, PF6-, EC and DMC, then re-adds them using the current Molarity and Total particles",
                    )
                    .clicked()
                {
                    // Delete existing electrolyte-related species
                    for species in [
                        Species::LithiumIon,
                        Species::ElectrolyteAnion,
                        Species::EC,
                        Species::DMC,
                    ] {
                        let _ = SIM_COMMAND_SENDER
                            .lock()
                            .as_ref()
                            .unwrap()
                            .send(SimCommand::DeleteSpecies { species });
                    }

                    // Re-add according to current settings
                    let total = self.electrolyte_total_particles;
                    let solvent_to_salt_ratio = 15.0; // EC+DMC molecules per LiPF6
                    let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
                    let lipf6_count = (total as f32
                        * salt_fraction
                        * self.electrolyte_molarity
                        / 1.0)
                        .round() as usize;
                    let li_count = lipf6_count; // 1:1 stoichiometry
                    let pf6_count = lipf6_count; // 1:1 stoichiometry

                    let remaining = total.saturating_sub(li_count + pf6_count);
                    let ec_count = remaining / 2; // 1:1 EC:DMC
                    let dmc_count = remaining - ec_count;

                    if li_count > 0 {
                        let li_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::LithiumIon,
                        );
                        let _ = SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                            SimCommand::AddRandom {
                                body: li_body,
                                count: li_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            },
                        );
                    }

                    if pf6_count > 0 {
                        let pf6_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::ElectrolyteAnion,
                        );
                        let _ = SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                            SimCommand::AddRandom {
                                body: pf6_body,
                                count: pf6_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            },
                        );
                    }

                    if ec_count > 0 {
                        let ec_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::EC,
                        );
                        let _ = SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                            SimCommand::AddRandom {
                                body: ec_body,
                                count: ec_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            },
                        );
                    }

                    if dmc_count > 0 {
                        let dmc_body = make_body_with_species(
                            ultraviolet::Vec2::zero(),
                            ultraviolet::Vec2::zero(),
                            Species::DMC,
                        );
                        let _ = SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                            SimCommand::AddRandom {
                                body: dmc_body,
                                count: dmc_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            },
                        );
                    }

                    eprintln!(
                        "[Electrolyte] Rebalanced molarity={:.2}M: {} Li+, {} PF6-, {} EC, {} DMC (total {} particles)",
                        self.electrolyte_molarity, li_count, pf6_count, ec_count, dmc_count, total
                    );
                }
            });

            // Show composition breakdown
            let total = self.electrolyte_total_particles;
            let solvent_to_salt_ratio = 15.0;
            let salt_fraction = 1.0 / (1.0 + solvent_to_salt_ratio);
            let lipf6_count =
                (total as f32 * salt_fraction * self.electrolyte_molarity / 1.0).round() as usize;
            let remaining = total.saturating_sub(lipf6_count * 2);
            let ec_count = remaining / 2;
            let dmc_count = remaining - ec_count;

            ui.horizontal(|ui| {
                ui.label("Composition:");
                ui.label(format!(
                    "Li+: {}, PF6-: {}, EC: {}, DMC: {}",
                    lipf6_count, lipf6_count, ec_count, dmc_count
                ));
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
        Species::EC | Species::DMC => {
            // Neutral solvent molecules with a single drifting electron
            body.electrons.push(Electron {
                rel_pos: Vec2::zero(),
                vel: Vec2::zero(),
            });
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
