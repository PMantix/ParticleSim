use super::state::*;
use crate::body::foil::LinkMode;
use quarkstrom::egui;
use crate::renderer::Species;
use ultraviolet::Vec2;
use crate::renderer::Body;
use crate::body::Electron;
use crate::config::IsolineFieldMode;

impl super::Renderer {
    pub fn show_gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                // Use actual simulation time, not renderer time
                let sim_time = *SIM_TIME.lock();
                ui.label(format!("Time: {:.2} s", sim_time));
                
                // Show pause status
                let is_paused = PAUSED.load(std::sync::atomic::Ordering::Relaxed);
                if is_paused {
                    ui.colored_label(egui::Color32::YELLOW, "⏸ PAUSED");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "▶ RUNNING");
                }
                // --- Field Controls ---
                egui::CollapsingHeader::new("Field Controls").default_open(true).show(ui, |ui| {
                    let mut mag = *FIELD_MAGNITUDE.lock();
                    ui.add(
                        egui::Slider::new(&mut mag, 0.0..=200.0)
                            .text("Field |E|")
                            .clamp_to_range(true)
                            .step_by(1.0), // Set increment to 1
                    );
                    *FIELD_MAGNITUDE.lock() = mag;

                    let mut dir = *FIELD_DIRECTION.lock();
                    ui.add(
                        egui::Slider::new(&mut dir, 0.0..=360.0)
                            .text("Field θ (deg)")
                            .clamp_to_range(true),
                    );
                    *FIELD_DIRECTION.lock() = dir;
                });

                ui.separator();

                // --- Display Options ---
                egui::CollapsingHeader::new("Display Options").default_open(true).show(ui, |ui| {
                    ui.checkbox(&mut self.show_bodies, "Show Bodies");
                    ui.checkbox(&mut self.show_quadtree, "Show Quadtree");
                });

                ui.separator();

                // --- Simulation Controls ---
                egui::CollapsingHeader::new("Simulation Controls").default_open(true).show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut *TIMESTEP.lock(), 0.0001..=0.1)
                            .text("Timestep (dt)")
                            .step_by(0.001),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.sim_config.damping_base, 0.95..=1.0)
                            .text("Damping Base")
                            .step_by(0.0001),
                    );

                    let mut passes = COLLISION_PASSES.lock();
                    ui.add(
                        egui::Slider::new(&mut *passes, 2..=20)
                            .text("Collision Passes")
                            .clamp_to_range(true),
                    );

                    if ui.button("Step Simulation").clicked() {
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::StepOnce).unwrap();
                    }

                    if self.show_quadtree {
                        let range = &mut self.depth_range;
                        ui.horizontal(|ui| {
                            ui.label("Depth Range:");
                            ui.add(egui::DragValue::new(&mut range.0).speed(0.05));
                            ui.label("to");
                            ui.add(egui::DragValue::new(&mut range.1).speed(0.05));
                        });
                    }
                });

                ui.separator();

                // --- Visualization Overlays ---
                egui::CollapsingHeader::new("Visualization Overlays").default_open(true).show(ui, |ui| {
                    ui.checkbox(&mut self.sim_config.show_field_isolines, "Show Field Isolines");
                    ui.checkbox(&mut self.sim_config.show_velocity_vectors, "Show Velocity Vectors");
                    ui.checkbox(&mut self.sim_config.show_charge_density, "Show Charge Density");
                    ui.checkbox(&mut self.sim_config.show_field_vectors, "Show Field Vectors"); // NEW
                    egui::ComboBox::from_label("Isoline Field Mode")
                        .selected_text(format!("{:?}", self.sim_config.isoline_field_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.sim_config.isoline_field_mode,
                                IsolineFieldMode::Total,
                                "Total",
                            );
                            ui.selectable_value(
                                &mut self.sim_config.isoline_field_mode,
                                IsolineFieldMode::ExternalOnly,
                                "External Only",
                            );
                            ui.selectable_value(
                                &mut self.sim_config.isoline_field_mode,
                                IsolineFieldMode::BodyOnly,
                                "Body Only",
                            );
                        });
                    ui.add(
                        egui::Slider::new(&mut self.velocity_vector_scale, 0.01..=1.0)
                            .text("Velocity Vector Scale")
                            .step_by(0.01),
                    );
                });

                ui.separator();

                // --- Lennard-Jones Parameters ---
                egui::CollapsingHeader::new("Lennard-Jones Parameters").default_open(true).show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut self.sim_config.lj_force_epsilon, 0.0..=5000.0)
                        .text("LJ Epsilon (attraction strength)")
                        .step_by(1.0));
                    ui.add(egui::Slider::new(&mut self.sim_config.lj_force_sigma, 0.1..=5.0)
                        .text("LJ Sigma (particle size)")
                        .step_by(0.01));
                    ui.add(egui::Slider::new(&mut self.sim_config.lj_force_cutoff, 0.5..=10.0)
                        .text("LJ Cutoff (range factor)")
                        .step_by(0.01));
                });

                ui.separator();

                // --- Butler-Volmer Parameters ---
                egui::CollapsingHeader::new("Butler-Volmer Parameters").default_open(true).show(ui, |ui| {
                    ui.checkbox(&mut self.sim_config.use_butler_volmer, "Use Butler-Volmer");
                    ui.add(
                        egui::Slider::new(&mut self.sim_config.bv_exchange_current, 0.0..=1.0e6)
                            .text("Exchange Current i0")
                            .step_by(1.0),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.sim_config.bv_transfer_coeff, 0.0..=1.0)
                            .text("Transfer Coeff α")
                            .step_by(0.01),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.sim_config.bv_overpotential_scale, 0.0..=1.0)
                            .text("Overpotential Scale")
                            .step_by(0.0001),
                    );
                });

                ui.separator();

                // --- Scenario Controls ---
                egui::CollapsingHeader::new("Scenario").default_open(true).show(ui, |ui| {
                    if ui.button("Delete All Particles").clicked() {
                        SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::DeleteAll).unwrap();
                    }

                    // --- Domain Size Controls ---
                    ui.separator();
                    ui.label("🌐 Computational Domain:");
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        let mut domain_width = self.domain_width;
                        if ui.add(egui::DragValue::new(&mut domain_width).speed(10.0).clamp_range(100.0..=5000.0)).changed() {
                            self.domain_width = domain_width;
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::SetDomainSize { 
                                width: self.domain_width, 
                                height: self.domain_height 
                            }).unwrap();
                        }
                        ui.label("Height:");
                        let mut domain_height = self.domain_height;
                        if ui.add(egui::DragValue::new(&mut domain_height).speed(10.0).clamp_range(100.0..=5000.0)).changed() {
                            self.domain_height = domain_height;
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::SetDomainSize { 
                                width: self.domain_width, 
                                height: self.domain_height 
                            }).unwrap();
                        }
                    });
                    ui.label("⚠️ Particles outside domain will be removed");

                    ui.separator();

                    // Common controls for all Add scenarios
                    ui.horizontal(|ui| {
                        use crate::species::SPECIES_PROPERTIES;
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut self.scenario_x).speed(0.1));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut self.scenario_y).speed(0.1));
                        ui.label("Particle Radius:");
                        // Set default radius from species if changed
                        if let Some(props) = SPECIES_PROPERTIES.get(&self.scenario_species) {
                            if (self.scenario_particle_radius - props.radius).abs() > f32::EPSILON && ui.button("Reset to Default").clicked() {
                                self.scenario_particle_radius = props.radius;
                            }
                        }
                        ui.add(egui::DragValue::new(&mut self.scenario_particle_radius).speed(0.05));
                        egui::ComboBox::from_label("Species")
                            .selected_text(format!("{:?}", self.scenario_species))
                            .show_ui(ui, |ui| {
                                use crate::renderer::Species;
                                ui.selectable_value(&mut self.scenario_species, Species::LithiumMetal, "Lithium Metal");
                                ui.selectable_value(&mut self.scenario_species, Species::LithiumIon, "Lithium Ion");
                                ui.selectable_value(&mut self.scenario_species, Species::ElectrolyteAnion, "Electrolyte Anion");
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
                            let props = crate::species::SPECIES_PROPERTIES.get(&spec).unwrap();
                            let body = make_body_with_species(
                                ultraviolet::Vec2::zero(),
                                ultraviolet::Vec2::zero(),
                                props.mass,
                                props.radius,
                                spec,
                            );
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRing {
                                body,
                                x: self.scenario_x,
                                y: self.scenario_y,
                                radius: self.scenario_radius,
                            }).unwrap();
                        }
                        if ui.button("Add Filled Circle").clicked() {
                            let spec = self.scenario_species;
                            let props = crate::species::SPECIES_PROPERTIES.get(&spec).unwrap();
                            let body = make_body_with_species(
                                ultraviolet::Vec2::zero(),
                                ultraviolet::Vec2::zero(),
                                props.mass,
                                props.radius,
                                spec,
                            );
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddCircle {
                                body,
                                x: self.scenario_x,
                                y: self.scenario_y,
                                radius: self.scenario_radius,
                            }).unwrap();
                        }
                    });

                    // Add Rectangle and Add Foil (using common width/height)
                    ui.horizontal(|ui| {
                        if ui.button("Add Rectangle").clicked() {
                            let spec = self.scenario_species;
                            let props = crate::species::SPECIES_PROPERTIES.get(&spec).unwrap();
                            let body = make_body_with_species(
                                ultraviolet::Vec2::zero(),
                                ultraviolet::Vec2::zero(),
                                props.mass,
                                props.radius,
                                spec,
                            );
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRectangle {
                                body,
                                x: self.scenario_x - self.scenario_width / 2.0,
                                y: self.scenario_y - self.scenario_height / 2.0,
                                width: self.scenario_width,
                                height: self.scenario_height,
                            }).unwrap();
                        }
                        if ui.button("Add Foil").clicked() {
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddFoil {
                                width: self.scenario_width,
                                height: self.scenario_height,
                                x: self.scenario_x - self.scenario_width / 2.0,
                                y: self.scenario_y - self.scenario_height / 2.0,
                                particle_radius: self.scenario_particle_radius,
                                current: 0.0, // Always start with 0 current
                            }).unwrap();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        ui.add(egui::DragValue::new(&mut self.scenario_random_count).speed(1.0));
                        if ui.button("Add Random").clicked() {
                            let spec = self.scenario_species;
                            let props = crate::species::SPECIES_PROPERTIES.get(&spec).unwrap();
                            let body = make_body_with_species(
                                ultraviolet::Vec2::zero(),
                                ultraviolet::Vec2::zero(),
                                props.mass,
                                props.radius,
                                spec,
                            );
                            SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::AddRandom {
                                body,
                                count: self.scenario_random_count,
                                domain_width: self.domain_width,
                                domain_height: self.domain_height,
                            }).unwrap();
                        }
                    });

                    ui.horizontal(|ui| {
                        // --- Save State UI ---
                        use std::fs;
                        use std::path::PathBuf;
                        let saved_state_dir = PathBuf::from("saved_state");
                        // List all .json files in saved_state
                        let mut state_files: Vec<String> = fs::read_dir(&saved_state_dir)
                            .map(|rd| rd.filter_map(|e| e.ok())
                                .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
                                .map(|e| e.file_name().to_string_lossy().to_string())
                                .collect())
                            .unwrap_or_default();
                        state_files.sort();

                        // Save name input
                        ui.label("Save as:");
                        let save_name = &mut self.save_state_name;
                        let save_clicked = ui.text_edit_singleline(save_name).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
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
                                .send(SimCommand::SaveState { path: path.to_string_lossy().to_string() })
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
                                    .send(SimCommand::LoadState { path: path.to_string_lossy().to_string() })
                                    .unwrap();
                            }
                        }
                    });
                });

                // --- Foil Current Controls for Selected Foil ---
                if let Some(selected_id) = self.selected_particle_id {
                    let maybe_foil = {
                        let foils = FOILS.lock();
                        foils.iter().find(|f| f.body_ids.contains(&selected_id)).cloned()
                    };
                    if let Some(foil) = maybe_foil {

                            ui.separator();
                            ui.label("DC + AC Current Components:");
                            
                            // DC Current control
                            let mut dc_current = foil.dc_current;
                            ui.horizontal(|ui| {
                                ui.label("DC Current:");
                                if ui.button("-").clicked() { dc_current -= 1.0; }
                                if ui.button("+").clicked() { dc_current += 1.0; }
                                if ui.button("0").clicked() { dc_current = 0.0; }
                                ui.add(egui::Slider::new(&mut dc_current, -500.0..=500.00).step_by(0.1));
                            });
                            if (dc_current - foil.dc_current).abs() > f32::EPSILON {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                                    SimCommand::SetFoilDCCurrent { foil_id: selected_id, dc_current }
                                ).unwrap();
                            }

                            // AC Current control
                            let mut ac_current = foil.ac_current;
                            ui.horizontal(|ui| {
                                ui.label("AC Amplitude:");
                                if ui.button("-").clicked() { ac_current -= 1.0; }
                                if ui.button("+").clicked() { ac_current += 1.0; }
                                if ui.button("0").clicked() { ac_current = 0.0; }
                                ui.add(egui::Slider::new(&mut ac_current, 0.0..=500.00).step_by(0.1));
                            });
                            if (ac_current - foil.ac_current).abs() > f32::EPSILON {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                                    SimCommand::SetFoilACCurrent { foil_id: selected_id, ac_current }
                                ).unwrap();
                            }

                            let mut hz = foil.switch_hz;
                            ui.horizontal(|ui| {
                                ui.label("Switch Hz:");
                                ui.add(egui::DragValue::new(&mut hz).speed(0.1));
                            });
                            if (hz - foil.switch_hz).abs() > f32::EPSILON {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(
                                    SimCommand::SetFoilFrequency { foil_id: selected_id, switch_hz: hz }
                                ).unwrap();
                            }

                            use egui::plot::{Plot, Line, PlotPoints};
                            let seconds = 5.0;
                            let steps = 200;
                            // Use actual simulation time and respect pause state
                            let sim_time = *SIM_TIME.lock();
                            let is_paused = PAUSED.load(std::sync::atomic::Ordering::Relaxed);
                            let current_time = if is_paused {
                                // When paused, freeze the time display
                                sim_time
                            } else {
                                sim_time
                            };
                            let selected_ids = self.selected_foil_ids.clone();
                            Plot::new("foil_wave_plot").height(100.0).allow_scroll(false).allow_zoom(false).show(ui, |plot_ui| {
                                let colors = [egui::Color32::LIGHT_BLUE, egui::Color32::LIGHT_RED, egui::Color32::LIGHT_GREEN, egui::Color32::YELLOW];
                                let foils = FOILS.lock();
                                for (idx, fid) in selected_ids.iter().enumerate() {
                                    if let Some(f) = foils.iter().find(|f| f.id == *fid) {
                                        let dt = seconds / steps as f32;
                                        let mut points_vec: Vec<[f64; 2]> = Vec::with_capacity(steps + 1);
                                        for i in 0..=steps {
                                            let t = i as f32 * dt;
                                            let effective_current = {
                                                // DC component is always active
                                                let mut current = f.dc_current;
                                                // Add AC component only if frequency is set
                                                if f.switch_hz > 0.0 {
                                                    let plot_time = current_time + t;
                                                    let ac_component = if (plot_time * f.switch_hz) % 1.0 < 0.5 { 
                                                        f.ac_current 
                                                    } else { 
                                                        -f.ac_current 
                                                    };
                                                    current += ac_component;
                                                }
                                                current
                                            };
                                            points_vec.push([t as f64, effective_current as f64]);
                                        }
                                        let points = PlotPoints::from(points_vec);
                                        plot_ui.line(Line::new(points).color(colors[idx % colors.len()]));
                                    }
                                }
                            });
                    }
                }

                // --- Foil Linking Controls ---
                ui.separator();
                egui::CollapsingHeader::new("Foil Links").default_open(true).show(ui, |ui| {
                    if self.selected_foil_ids.len() == 2 {
                        let a = self.selected_foil_ids[0];
                        let b = self.selected_foil_ids[1];
                        let foils = FOILS.lock();
                        let linked = foils.iter().find(|f| f.id == a).and_then(|f| f.link_id).map(|id| id == b).unwrap_or(false);
                        if linked {
                            if ui.button("Unlink Foils").clicked() {
                                SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::UnlinkFoils { a, b }).unwrap();
                            }
                        } else {
                            ui.horizontal(|ui| {
                                if ui.button("Link Parallel").clicked() {
                                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::LinkFoils { a, b, mode: LinkMode::Parallel }).unwrap();
                                }
                                if ui.button("Link Opposite").clicked() {
                                    SIM_COMMAND_SENDER.lock().as_ref().unwrap().send(SimCommand::LinkFoils { a, b, mode: LinkMode::Opposite }).unwrap();
                                }
                            });
                        }
                    }
                });

                // --- Debug/Diagnostics ---
                ui.separator();
                egui::CollapsingHeader::new("Debug/Diagnostics").default_open(true).show(ui, |ui| {
                    ui.checkbox(&mut self.sim_config.show_lj_vs_coulomb_ratio, "Show LJ/Coulomb Force Ratio");
                    ui.checkbox(&mut self.show_electron_deficiency, "Show Electron Deficiency/Excess");
                });

                // --- Plotting & Analysis ---
                ui.separator();
                crate::plotting::gui::show_plotting_controls(
                    ui,
                    &mut self.plotting_system,
                    &mut self.show_plotting_window,
                    &mut self.new_plot_type,
                    &mut self.new_plot_quantity,
                    &mut self.new_plot_sampling_mode,
                    &mut self.new_plot_title,
                    &mut self.new_plot_spatial_bins,
                    &mut self.new_plot_time_window,
                    &mut self.new_plot_update_frequency,
                );
            });

        // Show plotting control window if open
        if self.show_plotting_window {
            crate::plotting::gui::show_plotting_window(
                ctx,
                &mut self.plotting_system,
                &mut self.show_plotting_window,
                &mut self.new_plot_type,
                &mut self.new_plot_quantity,
                &mut self.new_plot_sampling_mode,
                &mut self.new_plot_title,
                &mut self.new_plot_spatial_bins,
                &mut self.new_plot_time_window,
                &mut self.new_plot_update_frequency,
            );
        }

        // Show individual plot windows
        crate::plotting::gui::show_plot_windows(ctx, &mut self.plotting_system);
    }
}

pub fn make_body_with_species(pos: Vec2, vel: Vec2, mass: f32, radius: f32, species: Species) -> Body {
    use crate::config::{LITHIUM_METAL_NEUTRAL_ELECTRONS, FOIL_NEUTRAL_ELECTRONS};
    let mut body = Body::new(pos, vel, mass, radius, 0.0, species);
    body.electrons.clear();
    match species {
        Species::LithiumMetal => {
            for _ in 0..LITHIUM_METAL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        Species::FoilMetal => {
            for _ in 0..FOIL_NEUTRAL_ELECTRONS {
                body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
            }
        }
        Species::LithiumIon => {
            // Ions: one less electron than neutral metal, positive charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS - 1) {
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
            }
        }
        Species::ElectrolyteAnion => {
            // Anions: one more electron than neutral metal, negative charge
            if LITHIUM_METAL_NEUTRAL_ELECTRONS > 0 {
                for _ in 0..(LITHIUM_METAL_NEUTRAL_ELECTRONS + 1) {
                    body.electrons.push(Electron { rel_pos: Vec2::zero(), vel: Vec2::zero() });
                }
            }
        }
    }
    body.update_charge_from_electrons();
    body.update_species();
    body
}

// In your rendering/drawing code, use:
// let color = match body.species {
//     Species::LithiumMetal => /* existing color */,
//     Species::LithiumIon => /* existing color */,
//     Species::FoilMetal => egui::Color32::from_rgb(255, 128, 0), // Orange or any distinct color
// };
